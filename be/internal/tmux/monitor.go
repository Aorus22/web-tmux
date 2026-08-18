package tmux

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"reflect"
	"runtime"
	"sync"
	"time"
)

// MonitorEvent is a message emitted by a Monitor to subscribers (the realtime layer).
type MonitorEvent struct {
	Type string // "output" | "state" | "ready" | "disconnected" | "reconnecting"

	// output
	PaneID string
	Data   []byte

	// state: full snapshot after a topology change
	Snapshot *Snapshot

	// command result correlation
	RequestID string
	Err       error
}

const (
	EvOutput        = "output"
	EvState         = "state"
	EvReady         = "ready"
	EvDisconnected  = "disconnected"
	EvReconnecting  = "reconnecting"
	EvCommandResult = "command"
)

// Monitor owns the tmux connection for one session. Unix uses a persistent
// `tmux -CC` child and asynchronous control events. Native Windows tmux does
// not implement -C, so the Windows adapter uses one-shot commands and this
// monitor polls snapshots and pane captures instead.
type Monitor struct {
	session    string
	socket     Socket
	exec       *Executor
	reader     *SnapshotReader
	parser     *Parser
	log        *slog.Logger
	scrollback int

	mu        sync.Mutex
	control   *Control
	batcher   *InputBatcher
	snapshot  *Snapshot
	pending   []*PendingCommand // FIFO correlation (PRD §28)
	subs      map[chan MonitorEvent]struct{}
	captures  map[string]string // last visible capture per pane (Windows)
	seq       uint64
	reconnect bool
	lastErr   error
	resyncCh  chan struct{}

	ctx    context.Context
	cancel context.CancelFunc
	done   chan struct{}
	wg     sync.WaitGroup

	swapCh chan struct{} // closed when the control is replaced (reconnect)
}

// PendingCommand tracks a mutating command awaiting %begin/%end/%error.
type PendingCommand struct {
	RequestID string
	errored   bool
	msg       string
}

// NewMonitor creates a monitor for one session. It does not start until Start().
func NewMonitor(session string, socket Socket, exec *Executor, log *slog.Logger, scrollback int) *Monitor {
	ctx, cancel := context.WithCancel(context.Background())
	return &Monitor{
		session:    session,
		socket:     socket,
		exec:       exec,
		reader:     NewSnapshotReader(exec),
		parser:     NewParser(),
		log:        log.With("session", session),
		scrollback: scrollback,
		subs:       make(map[chan MonitorEvent]struct{}),
		captures:   make(map[string]string),
		resyncCh:   make(chan struct{}, 1),
		ctx:        ctx,
		cancel:     cancel,
		done:       make(chan struct{}),
		swapCh:     make(chan struct{}),
	}
}

// Start launches the control connection and event loop.
func (m *Monitor) Start() error {
	m.mu.Lock()
	if m.control != nil {
		m.mu.Unlock()
		return nil
	}
	ctrl := NewControl(m.session, m.socket, m.log)
	ctrl.OnExit = m.onControlExit
	m.control = ctrl
	m.mu.Unlock()

	if err := ctrl.Start(); err != nil {
		return fmt.Errorf("control start: %w", err)
	}

	m.wg.Add(1)
	go m.run()
	return nil
}

// Subscribe registers a channel for monitor events. Buffered to avoid blocking.
func (m *Monitor) Subscribe() chan MonitorEvent {
	ch := make(chan MonitorEvent, 512)
	m.mu.Lock()
	m.subs[ch] = struct{}{}
	m.mu.Unlock()
	return ch
}

// Unsubscribe removes a subscriber channel.
func (m *Monitor) Unsubscribe(ch chan MonitorEvent) {
	m.mu.Lock()
	if _, ok := m.subs[ch]; ok {
		delete(m.subs, ch)
		close(ch)
	}
	m.mu.Unlock()
}

func (m *Monitor) broadcast(ev MonitorEvent) {
	m.mu.Lock()
	defer m.mu.Unlock()
	for ch := range m.subs {
		select {
		case ch <- ev:
		default:
			// Slow subscriber: drop rather than block the loop.
		}
	}
}

// Snapshot returns the latest cached snapshot.
func (m *Monitor) Snapshot() *Snapshot {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.snapshot
}

// Resync forces a full snapshot refresh and broadcasts it.
func (m *Monitor) Resync() {
	select {
	case m.resyncCh <- struct{}{}:
	default:
	}
}

// RunCommand sends a mutating command through control mode and correlates the
// result with requestID (PRD §8, §28).
func (m *Monitor) RunCommand(c Command, requestID string) error {
	m.mu.Lock()
	if m.control == nil {
		m.mu.Unlock()
		return errors.New("tmux control mode not connected")
	}
	ctrl := m.control
	if !ctrl.IsSynchronous() {
		m.pending = append(m.pending, &PendingCommand{RequestID: requestID})
	}
	m.mu.Unlock()

	if err := ctrl.RunCommand(c); err != nil {
		return err
	}
	if ctrl.IsSynchronous() {
		// There is no %end marker on native Windows. The next polling tick
		// publishes the resulting snapshot and terminal contents.
		m.Resync()
	}
	return nil
}

// ResizeTerminal resizes the shared tmux viewport. Unix maps this to
// refresh-client -C; native Windows maps it to resize-window because there is
// no control-mode client to resize.
func (m *Monitor) ResizeTerminal(cols, rows int) error {
	m.mu.Lock()
	ctrl := m.control
	m.mu.Unlock()
	if ctrl == nil {
		return errors.New("tmux control mode not connected")
	}
	return ctrl.Resize(cols, rows)
}

// SendInput batches raw terminal bytes to a pane (PRD §22).
func (m *Monitor) SendInput(paneID string, data []byte) {
	// Batcher is created per-start below; guarded by mu.
	m.mu.Lock()
	batcher := m.batcher
	m.mu.Unlock()
	if batcher != nil {
		batcher.Write(paneID, data)
	}
}

// Stop terminates the control connection gracefully and stops the loop.
func (m *Monitor) Stop() {
	m.mu.Lock()
	ctrl := m.control
	m.control = nil
	batcher := m.batcher
	m.batcher = nil
	m.mu.Unlock()

	if batcher != nil {
		batcher.Stop()
	}
	if ctrl != nil {
		ctrl.Stop()
	}
	m.cancel()
	<-m.done
}

// --- internals ---

func (m *Monitor) onControlExit(err error) {
	m.mu.Lock()
	if m.control == nil {
		m.mu.Unlock()
		return
	}
	m.lastErr = err
	m.mu.Unlock()

	select {
	case m.resyncCh <- struct{}{}: // reuse channel as "retry" trigger
	default:
	}
}

func (m *Monitor) run() {
	defer m.wg.Done()
	defer close(m.done)

	// Input batcher lives per control connection.
	batcher := NewInputBatcher(10*time.Millisecond, 4096, func(pane, hexData string) {
		_ = m.RunCommand(cmdSendHex(pane, hexData), "")
	})
	m.mu.Lock()
	m.batcher = batcher
	m.mu.Unlock()
	batcher.Start()
	defer batcher.Stop()

	// Unix receives asynchronous protocol lines from the control client.
	// Native Windows tmux has no control mode, so it uses the polling loop
	// below instead.
	if runtime.GOOS != "windows" {
		m.wg.Add(1)
		go m.readLoop()
	}

	// Startup: initial snapshot + ready broadcast.
	if err := m.refreshSnapshot(); err != nil {
		m.log.Warn("initial snapshot failed", "err", err)
	}
	m.broadcast(MonitorEvent{Type: EvReady})
	m.broadcast(MonitorEvent{Type: EvState, Snapshot: m.Snapshot()})

	if runtime.GOOS == "windows" {
		m.runWindowsPolling()
		return
	}

	// Reconnect/retry backoff: 250ms, 500ms, 1s, 2s, 5s, 10s (PRD §48).
	backoff := []time.Duration{250 * time.Millisecond, 500 * time.Millisecond, time.Second, 2 * time.Second, 5 * time.Second, 10 * time.Second}
	attempt := 0

	for {
		select {
		case <-m.ctx.Done():
			return
		case <-m.resyncCh:
			m.handleResyncOrRetry(&attempt, backoff)
		}
	}
}

// runWindowsPolling supplies the asynchronous behavior that -C would provide
// on Unix. The native Windows tmux port supports normal commands and
// capture-pane, but not control-mode notifications.
func (m *Monitor) runWindowsPolling() {
	ticker := time.NewTicker(250 * time.Millisecond)
	defer ticker.Stop()
	for {
		select {
		case <-m.ctx.Done():
			return
		case <-m.resyncCh:
			m.pollWindows()
		case <-ticker.C:
			m.pollWindows()
		}
	}
}

// pollWindows refreshes topology and sends a full visible-screen replacement
// only when a pane changed. The clear/home prefix is intentional: capture-pane
// returns a screen, while the frontend's terminal.output event is incremental.
// This preserves native Windows functionality without appending duplicate
// copies of the same captured screen.
func (m *Monitor) pollWindows() {
	previous := m.Snapshot()
	if err := m.refreshSnapshot(); err != nil {
		m.log.Debug("Windows tmux poll failed", "err", err)
		return
	}
	current := m.Snapshot()
	if !reflect.DeepEqual(previous, current) {
		m.broadcast(MonitorEvent{Type: EvState, Snapshot: current})
	}

	panes := current.Panes
	seen := make(map[string]struct{}, len(panes))
	ctx, cancel := context.WithTimeout(m.ctx, 2*time.Second)
	defer cancel()
	for _, pane := range panes {
		seen[pane.ID] = struct{}{}
		data, err := m.reader.CapturePane(ctx, pane.ID, 0)
		if err != nil {
			continue
		}
		previousCapture, exists := m.captures[pane.ID]
		m.captures[pane.ID] = data
		// The initial terminal.capture request supplies the first screen and
		// scrollback. Do not immediately overwrite it with the first poll.
		if exists && previousCapture != data {
			m.broadcast(MonitorEvent{
				Type:   EvOutput,
				PaneID: pane.ID,
				Data:   []byte("\x1b[2J\x1b[H" + data),
			})
		}
	}
	for paneID := range m.captures {
		if _, ok := seen[paneID]; !ok {
			delete(m.captures, paneID)
		}
	}
}

func (m *Monitor) handleResyncOrRetry(attempt *int, backoff []time.Duration) {
	m.mu.Lock()
	ctrl := m.control
	m.mu.Unlock()
	if ctrl == nil {
		return // stopped
	}

	// If control mode dropped, reconnect (PRD §48).
	m.mu.Lock()
	reconnecting := m.lastErr != nil
	m.mu.Unlock()
	if reconnecting {
		delay := backoff[*attempt]
		if *attempt < len(backoff)-1 {
			*attempt++
		}
		m.broadcast(MonitorEvent{Type: EvReconnecting})
		m.log.Warn("tmux control lost, reconnecting", "attempt", *attempt, "delay", delay)

		select {
		case <-time.After(delay):
		case <-m.ctx.Done():
			return
		}
		m.mu.Lock()
		old := m.control
		m.control = nil
		m.lastErr = nil
		m.mu.Unlock()
		if old != nil {
			old.Stop()
		}

		ctrl = NewControl(m.session, m.socket, m.log)
		ctrl.OnExit = m.onControlExit
		m.mu.Lock()
		m.control = ctrl
		m.lastErr = nil
		close(m.swapCh) // wake the read loop onto the new control
		m.swapCh = make(chan struct{})
		m.mu.Unlock()
		if err := ctrl.Start(); err != nil {
			m.log.Warn("reconnect failed", "err", err)
			m.onControlExit(err)
			return
		}
		*attempt = 0
		m.broadcast(MonitorEvent{Type: EvReady})
		m.log.Info("reconnected to tmux")
	}

	if err := m.refreshSnapshot(); err != nil {
		m.log.Warn("snapshot refresh failed", "err", err)
		return
	}
	m.broadcast(MonitorEvent{Type: EvState, Snapshot: m.Snapshot()})
}

// readLoop reads lines from the control client until shutdown. It survives
// reconnects: after a control exits it waits for the swap and keeps reading.
func (m *Monitor) readLoop() {
	defer m.wg.Done()
	for {
		m.mu.Lock()
		ctrl := m.control
		m.mu.Unlock()
		if ctrl == nil {
			// Stopped or waiting for a swap.
			if m.waitForSwap() {
				continue
			}
			return
		}
		line, err := ctrl.ReadLine()
		if err != nil {
			m.mu.Lock()
			still := m.control == ctrl
			m.mu.Unlock()
			if !still {
				continue // swapped during read; loop picks up the new control
			}
			// Let the retry path handle it, then wait for the swap.
			m.onControlExit(err)
			if m.waitForSwap() {
				continue
			}
			return
		}
		m.handleLine(line)
	}
}

// waitForSwap blocks until the control is replaced or the monitor stops.
// Returns true when a swap happened (keep reading), false when stopped.
func (m *Monitor) waitForSwap() bool {
	m.mu.Lock()
	ch := m.swapCh
	m.mu.Unlock()
	select {
	case <-ch:
		return true
	case <-m.ctx.Done():
		return false
	}
}

func (m *Monitor) handleLine(line string) {
	ev, ok := m.parser.ParseLine(line)
	if !ok {
		return
	}
	switch ev.Kind {
	case "output":
		m.broadcast(MonitorEvent{Type: EvOutput, PaneID: ev.PaneID, Data: DecodeOutput(ev.Data)})
	case "": // %begin / %end / %error
		m.handleCommandMarker(ev)
	case "layout-change", "window-add", "window-close", "window-renamed",
		"session-changed", "session-window-changed", "window-pane-changed",
		"sessions-changed", "pane-mode-changed", "client-session-changed":
		// Topology changed → debounced resync (PRD §25). %session-window-changed
		// fires when the current window of a session changes (e.g. the user
		// clicked a window tab): without it the frontend never learns the
		// active window moved and window switching appears broken.
		// %window-pane-changed fires when the active pane moves (pane click or
		// select-pane crossing into another window) — same requirement.
		m.Resync()
	case "unknown":
		m.log.Debug("unknown tmux control event", "raw", line)
	}
}

// handleCommandMarker correlates %begin/%end/%error with pending commands.
// Commands are sent sequentially over one stdin, so FIFO correlation is exact
// (PRD §28): %end completes the oldest pending request; %error fails it.
func (m *Monitor) handleCommandMarker(ev ControlEvent) {
	switch ev.Marker {
	case "end":
		m.resolvePending(false, "")
	case "error":
		m.resolvePending(true, ev.Error)
	}
}

// refreshSnapshot pulls the latest state from tmux (read-only queries).
func (m *Monitor) refreshSnapshot() error {
	ctx, cancel := context.WithTimeout(m.ctx, 5*time.Second)
	defer cancel()
	snap, err := m.reader.Snapshot(ctx, m.session)
	if err != nil {
		return err
	}
	m.mu.Lock()
	m.snapshot = snap
	m.seq++
	m.mu.Unlock()
	return nil
}

// resolvePending marks a pending command done. If there is no pending command
// (e.g. an untracked %error), it is dropped silently.
func (m *Monitor) resolvePending(errored bool, msg string) {
	m.mu.Lock()
	if len(m.pending) == 0 {
		m.mu.Unlock()
		return
	}
	p := m.pending[0]
	m.pending = m.pending[1:]
	m.mu.Unlock()
	if p.RequestID == "" {
		return // untracked (e.g. input batches)
	}
	m.broadcast(MonitorEvent{Type: EvCommandResult, RequestID: p.RequestID, Err: errFrom(errored, msg)})
}

func errFrom(errored bool, msg string) error {
	if !errored {
		return nil
	}
	if msg == "" {
		msg = "tmux command failed"
	}
	return errors.New(msg)
}
