package tmux

import (
	"context"
	"encoding/hex"
	"errors"
	"fmt"
	"log/slog"
	"os"
	"reflect"
	"runtime"
	"sync"
	"time"
)

// MonitorEvent is a message emitted by a Monitor to subscribers (the realtime layer).
type MonitorEvent struct {
	Type string // "output" | "state" | "ready" | "disconnected" | "reconnecting"

	// output
	PaneID     string
	Data       []byte
	Replace    bool // full screen replacement (native Windows polling)
	ScreenRows int  // leading Data lines above this count are scrollback history

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

	mu             sync.Mutex
	control        *Control
	batcher        *InputBatcher
	snapshot       *Snapshot
	pending        []*PendingCommand // FIFO correlation (PRD §28)
	subs           map[chan MonitorEvent]struct{}
	captures       map[string]string      // last visible capture per pane (Windows)
	activity       map[string]time.Time   // last typed input per pane (Windows fast poll)
	streams        map[string]*PipeStream // active pipe-pane streams per pane (Windows)
	pendingStreams map[string]bool        // streams currently arming
	seq            uint64
	reconnect      bool
	lastErr        error
	resyncCh       chan struct{}

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
		session:        session,
		socket:         socket,
		exec:           exec,
		reader:         NewSnapshotReader(exec),
		parser:         NewParser(),
		log:            log.With("session", session),
		scrollback:     scrollback,
		subs:           make(map[chan MonitorEvent]struct{}),
		captures:       make(map[string]string),
		activity:       make(map[string]time.Time),
		streams:        make(map[string]*PipeStream),
		pendingStreams: make(map[string]bool),
		resyncCh:       make(chan struct{}, 1),
		ctx:            ctx,
		cancel:         cancel,
		done:           make(chan struct{}),
		swapCh:         make(chan struct{}),
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
	return m.runCommand(c, requestID, true)
}

// runCommandInput executes a non-structural command from the input path
// (batched keystrokes). On Windows it skips the post-command resync hook:
// every keystroke batch must not trigger a topology refresh nor invalidate
// pane captures — the periodic topology tick remains the safety net for
// structural changes. On Unix this is identical to RunCommand because the
// synchronous-resync branch never fires (control mode reports completion
// asynchronously via %end/%error).
func (m *Monitor) runCommandInput(c Command) error {
	return m.runCommand(c, "", false)
}

func (m *Monitor) runCommand(c Command, requestID string, resync bool) error {
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
	if resync && ctrl.IsSynchronous() {
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
	if runtime.GOOS != "windows" {
		// Preserve the Unix control-mode resize path. The Windows adapter has
		// no -C client and overrides this with resize-window below.
		return m.RunCommand(cmdResizeClient(cols, rows), "")
	}
	return ctrl.Resize(cols, rows)
}

// SendInput batches raw terminal bytes to a pane (PRD §22). The pane is also
// marked active so the Windows poller captures it at the fast cadence and
// echoed keystrokes show up immediately.
func (m *Monitor) SendInput(paneID string, data []byte) {
	// Batcher is created per-start below; guarded by mu.
	m.mu.Lock()
	batcher := m.batcher
	if m.activity != nil && paneID != "" {
		m.activity[paneID] = time.Now()
	}
	m.mu.Unlock()
	if batcher != nil {
		batcher.Write(paneID, data)
	}
}

// recentInput reports whether paneID received typed input within window.
func (m *Monitor) recentInput(paneID string, window time.Duration) bool {
	m.mu.Lock()
	last, ok := m.activity[paneID]
	m.mu.Unlock()
	return ok && time.Since(last) <= window
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

	// Input batcher lives per control connection. The 6ms cadence coalesces
	// typing bursts; flushes go through the pane's pipe socket (zero spawns)
	// when active and fall back to send-keys otherwise. The flush path skips
	// the post-command resync (runCommandInput) so keystrokes never trigger a
	// topology refresh. The 512-byte cap keeps the fallback's per-byte hex
	// argv bounded (~512 args, well under Windows cmdline limits).
	batcher := NewInputBatcher(6*time.Millisecond, 512, m.deliverInput)
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
//
// Live output comes from pipe-pane streams (stream.go): each pane's bytes are
// teed to a log file the backend tails itself. Capture-polling demotes to the
// initial snapshot plus a slow integrity poll that heals any frontend
// divergence (e.g. after reconnect). Panes whose stream failed keep the old
// fast/idle capture cadence as fallback.
func (m *Monitor) runWindowsPolling() {
	const (
		fastCadence   = 30 * time.Millisecond
		idleCadence   = 250 * time.Millisecond
		activeWindow  = 1500 * time.Millisecond
		schedulerStep = 25 * time.Millisecond
		// Streamed panes skip capture rotation; this slow poll only compares
		// captures and broadcasts when they diverge.
		integrityCadence = 2 * time.Second
	)
	lastCapture := make(map[string]time.Time)
	lastIntegrity := make(map[string]time.Time)
	topologyDue := time.Now()
	// Monitor lifetime == viewer lifetime (the hub starts it on the first WS
	// client and stops it on the last), so stale logs from previous runs can
	// be wiped now and everything disarmed on exit.
	WipePipeDir()
	defer m.teardownStreams()
	ticker := time.NewTicker(schedulerStep)
	defer ticker.Stop()
	for {
		select {
		case <-m.ctx.Done():
			return
		case <-m.resyncCh:
			m.refreshTopology()
			topologyDue = time.Now().Add(idleCadence)
			clear(lastCapture)
		case now := <-ticker.C:
			if !now.Before(topologyDue) {
				m.refreshTopology()
				topologyDue = now.Add(idleCadence)
			}
			// The first snapshot may still be failing; skip this tick rather
			// than dereferencing a nil cached snapshot.
			snap := m.Snapshot()
			if snap == nil {
				continue
			}
			panes := snap.Panes
			live := make(map[string]struct{}, len(panes))
			for _, pane := range panes {
				live[pane.ID] = struct{}{}
				if m.streamFor(pane.ID) != nil {
					// Streamed pane: live bytes arrive via the pipe; only the
					// slow integrity poll runs here.
					if last, ok := lastIntegrity[pane.ID]; !ok || now.Sub(last) >= integrityCadence {
						lastIntegrity[pane.ID] = now
						m.captureVisiblePane(pane.ID, true)
					}
					continue
				}
				cadence := idleCadence
				if m.recentInput(pane.ID, activeWindow) {
					cadence = fastCadence
				}
				if last, ok := lastCapture[pane.ID]; ok && now.Sub(last) < cadence {
					continue
				}
				lastCapture[pane.ID] = now
				m.captureVisiblePane(pane.ID, false)
			}
			for id := range lastCapture {
				if _, ok := live[id]; !ok {
					delete(lastCapture, id)
					m.disarmStream(id)
				}
			}
			for id := range lastIntegrity {
				if _, ok := live[id]; !ok {
					delete(lastIntegrity, id)
				}
			}
		}
	}
}

// refreshTopology diffs and broadcasts the session snapshot without touching
// pane captures. Newly discovered panes get a pipe stream armed lazily while
// viewers are connected (the monitor itself only exists while they are).
func (m *Monitor) refreshTopology() {
	previous := m.Snapshot()
	if err := m.refreshSnapshot(); err != nil {
		m.log.Debug("Windows tmux poll failed", "err", err)
		return
	}
	current := m.Snapshot()
	if !reflect.DeepEqual(previous, current) {
		m.broadcast(MonitorEvent{Type: EvState, Snapshot: current})
	}
	for _, pane := range current.Panes {
		m.ensureStream(pane.ID)
	}
}

// captureVisiblePane replaces the client's screen for one pane when its
// visible content changed since the previous poll. The capture includes
// scrollback history; ScreenRows tells the client where the live screen starts.
// force broadcasts even on the first successful capture (stream bootstrap and
// integrity healing always need a frame when content differs from nothing).
func (m *Monitor) captureVisiblePane(paneID string, force bool) {
	ctx, cancel := context.WithTimeout(m.ctx, 2*time.Second)
	defer cancel()
	data, err := m.reader.CapturePaneScreen(ctx, paneID, m.scrollback)
	if err != nil {
		return
	}
	screenRows := 0
	// refreshTopology may have failed on the first ticks; guard against a nil
	// cached snapshot so a capture can never take the process down.
	if snap := m.Snapshot(); snap != nil {
		for _, pane := range snap.Panes {
			if pane.ID == paneID {
				screenRows = pane.Height
				break
			}
		}
	}
	m.mu.Lock()
	previousCapture, exists := m.captures[paneID]
	m.captures[paneID] = data
	m.mu.Unlock()
	// Without force, the initial terminal.capture request supplies the first
	// screen and scrollback; do not immediately overwrite it with the first
	// poll.
	if (force || exists) && previousCapture != data {
		m.broadcast(MonitorEvent{
			Type:       EvOutput,
			PaneID:     paneID,
			Data:       []byte(data),
			Replace:    true,
			ScreenRows: screenRows,
		})
	}
}

// --- pipe-pane streaming (native Windows) ---

// streamingEnabled reports whether pipe-pane streaming applies. Native
// Windows tmux has no control-mode notifications, so live output needs the
// pipe; Unix control mode already delivers asynchronous updates and must stay
// untouched.
func (m *Monitor) streamingEnabled() bool {
	m.mu.Lock()
	ctrl := m.control
	m.mu.Unlock()
	return ctrl != nil && ctrl.IsSynchronous()
}

// ensureStream arms a pipe stream for paneID unless one exists or is already
// being armed. Arming runs asynchronously: it spawns tmux processes and waits
// for the log file, which must not stall the polling loop.
func (m *Monitor) ensureStream(paneID string) {
	if !m.streamingEnabled() {
		return
	}
	m.mu.Lock()
	if _, ok := m.streams[paneID]; ok {
		m.mu.Unlock()
		return
	}
	if m.pendingStreams[paneID] {
		m.mu.Unlock()
		return
	}
	m.pendingStreams[paneID] = true
	m.mu.Unlock()
	go m.armStream(paneID)
}

// armStream arms the pipe, sends the initial full frame through the existing
// capture+screenRows path, then releases the chunk barrier so buffered output
// replays in order before live delivery begins. On failure the pane silently
// stays on the capture-polling fallback.
func (m *Monitor) armStream(paneID string) {
	defer func() {
		m.mu.Lock()
		delete(m.pendingStreams, paneID)
		m.mu.Unlock()
	}()

	st := NewPipeStream(paneID,
		func(args ...string) error {
			// pipe-pane bookkeeping is not structural: skip the post-command
			// resync hook (same rule as the input path).
			return m.runCommand(cmd("pipe-pane", args...), "", false)
		},
		func(chunk []byte) {
			// Normal incremental output event (replace=false): the shape the
			// frontends already consume by default.
			m.broadcast(MonitorEvent{Type: EvOutput, PaneID: paneID, Data: chunk})
		},
		func() { go m.captureVisiblePane(paneID, true) }, // barrier overflow → immediate integrity frame
		func(err error) { go m.handleStreamFailure(paneID, err) },
		m.log,
	)
	if err := st.Arm(); err != nil {
		m.log.Warn("pipe-pane arm failed; pane stays on capture polling", "pane", paneID, "err", err)
		return
	}
	m.mu.Lock()
	m.streams[paneID] = st
	m.mu.Unlock()

	// Bootstrap ordering per pane: arm → buffer → initial replace frame →
	// apply → flush buffer → live.
	m.captureVisiblePane(paneID, true)
	st.ApplySnapshot()
}

// handleStreamFailure drops a failed stream so the pane falls back to the
// regular capture cadence. Runs off the reader goroutine because Disarm
// blocks on that goroutine exiting.
func (m *Monitor) handleStreamFailure(paneID string, err error) {
	m.log.Warn("pipe-pane stream failed; pane falls back to capture polling", "pane", paneID, "err", err)
	m.disarmStream(paneID)
}

// disarmStream stops and forgets a pane's stream (idempotent).
func (m *Monitor) disarmStream(paneID string) {
	m.mu.Lock()
	st := m.streams[paneID]
	delete(m.streams, paneID)
	m.mu.Unlock()
	if st != nil {
		st.Disarm()
	}
	m.mu.Lock()
	delete(m.captures, paneID)
	m.mu.Unlock()
}

// deliverInput sends one raw input batch to a pane: over the pane's pipe
// socket when active (zero process spawns), else the send-keys -H fallback
// (hex-encoded, one argv element per byte — tmux 3.7 silently ignores a
// concatenated hex blob). A failed pipe write is logged once by the stream;
// the batch then goes out via the fallback so no keystrokes are lost.
func (m *Monitor) deliverInput(paneID string, data []byte) {
	if st := m.streamFor(paneID); st != nil {
		if err := st.WriteInput(data); err == nil {
			return
		}
	}
	_ = m.runCommandInput(cmdSendHex(paneID, hex.EncodeToString(data)))
}

// streamFor returns the active stream for a pane, if any.
func (m *Monitor) streamFor(paneID string) *PipeStream {
	m.mu.Lock()
	defer m.mu.Unlock()
	return m.streams[paneID]
}

// teardownStreams disarms every stream and removes the temp dir. Called when
// the polling loop exits (last viewer disconnected → monitor stopped). The
// directory removal only succeeds once no other session's monitor still owns
// files there.
func (m *Monitor) teardownStreams() {
	m.mu.Lock()
	streams := m.streams
	m.streams = make(map[string]*PipeStream)
	m.mu.Unlock()
	for _, st := range streams {
		st.Disarm()
	}
	_ = os.Remove(pipeDir())
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
