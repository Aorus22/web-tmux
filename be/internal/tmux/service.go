package tmux

import (
	"context"
	"fmt"
	"log/slog"
	"sync"
)

// Service exposes high-level, typed tmux operations (PRD §87: every GUI action
// maps to a typed Go method — the frontend never sends raw tmux commands).
//
// Mutating commands go through the session Monitor's control-mode channel
// (PRD §8). Read-only queries use one-shot commands (PRD §9).
type Service struct {
	exec       *Executor
	reader     *SnapshotReader
	socket     Socket
	log        *slog.Logger
	scrollback int

	mu       sync.Mutex
	monitors map[string]*Monitor
}

func NewService(socket Socket, log *slog.Logger, scrollback int) *Service {
	exec := NewExecutor(socket)
	return &Service{
		exec:       exec,
		reader:     NewSnapshotReader(exec),
		socket:     socket,
		log:        log,
		scrollback: scrollback,
		monitors:   make(map[string]*Monitor),
	}
}

// --- global / read-only ---

// TmuxVersion returns the installed tmux version, e.g. "3.7b".
func (s *Service) TmuxVersion(ctx context.Context) (string, error) {
	return s.reader.TmuxVersion(ctx)
}

// ListSessions returns all sessions.
func (s *Service) ListSessions(ctx context.Context) ([]Session, error) {
	return s.reader.ListSessions(ctx)
}

// Tree returns the full session/window/pane tree for the sidebar.
func (s *Service) Tree(ctx context.Context) (*Tree, error) {
	return s.reader.Tree(ctx)
}

// HasSession reports whether a session exists.
func (s *Service) HasSession(ctx context.Context, name string) bool {
	ok, _ := s.reader.HasSession(ctx, name)
	return ok
}

// EnsureServer boots a tmux server if none exists by creating a hidden
// throwaway session (PRD §10). Safe to call repeatedly.
func (s *Service) EnsureServer(ctx context.Context) error {
	sessions, err := s.reader.ListSessions(ctx)
	if err == nil && len(sessions) > 0 {
		return nil
	}
	// Server absent or empty: bootstrap so control mode can attach later.
	// The session is killed right away; it only exists to start the server.
	if _, err := s.exec.Run(ctx, "new-session", "-d", "-s", "__tmux_gui_bootstrap__"); err != nil {
		return fmt.Errorf("bootstrap tmux server: %w", err)
	}
	return nil
}

// --- session operations ---

// CreateSession creates a new session (PRD §14). Mutating operations use the
// session monitor when one is available; native Windows monitors execute the
// same typed command as a one-shot tmux process.
func (s *Service) CreateSession(ctx context.Context, name, cwd, initialCmd string) error {
	if err := ValidateSessionName(name); err != nil {
		return err
	}
	if m := s.getMonitor(""); m != nil {
		return m.RunCommand(cmdNewSession(name, cwd, initialCmd, true), "")
	}
	c := cmdNewSession(name, cwd, initialCmd, true)
	return s.execOneShot(ctx, c)
}

// RenameSession renames a session.
func (s *Service) RenameSession(ctx context.Context, name, newName string) error {
	if err := ValidateSessionName(newName); err != nil {
		return err
	}
	if m := s.getMonitor(name); m != nil {
		return m.RunCommand(cmdRenameSession(name, newName), "")
	}
	return s.execOneShot(ctx, cmdRenameSession(name, newName))
}

// KillSession kills a session. The app closing never kills sessions — only an
// explicit user action does (PRD §49).
func (s *Service) KillSession(ctx context.Context, name string) error {
	if m := s.getMonitor(name); m != nil {
		if err := m.RunCommand(cmdKillSession(name), ""); err != nil {
			// Control mode unavailable (e.g. mid-reconnect): fall back to
			// one-shot so the kill is never silently dropped.
			s.log.Debug("kill-session via control failed, falling back", "session", name, "err", err)
			if err := s.execOneShot(ctx, cmdKillSession(name)); err != nil {
				return err
			}
		}
	} else {
		if err := s.execOneShot(ctx, cmdKillSession(name)); err != nil {
			return err
		}
	}
	// Stop our control connection to a dead session.
	if m := s.takeMonitor(name); m != nil {
		m.Stop()
	}
	return nil
}

// --- window operations ---

// CreateWindow creates a window in a session (PRD §15, §11).
func (s *Service) CreateWindow(ctx context.Context, session, name, cwd, initialCmd string) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.RunCommand(cmdCreateWindow(session, name, cwd, initialCmd), "")
}

// RenameWindow renames a window by stable ID (PRD §6).
func (s *Service) RenameWindow(ctx context.Context, session, windowID, name string) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.RunCommand(cmdRenameWindow(windowID, name), "")
}

// KillWindow kills a window by stable ID.
func (s *Service) KillWindow(ctx context.Context, session, windowID string) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.RunCommand(cmdKillWindow(windowID), "")
}

// SelectWindow focuses a window by stable ID.
func (s *Service) SelectWindow(ctx context.Context, session, windowID string) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.RunCommand(cmdSelectWindow(windowID), "")
}

// MoveWindow shifts a window left/right (negative/positive offset).
func (s *Service) MoveWindow(ctx context.Context, session, windowID string, offset int) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.RunCommand(cmdMoveWindow(windowID, offset), "")
}

// SelectLayout applies a layout preset to a window (PRD §16).
func (s *Service) SelectLayout(ctx context.Context, session, windowID, layout string) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.RunCommand(cmdSelectLayout(windowID, layout), "")
}

// --- pane operations ---

// SplitPane splits a pane by stable ID (PRD §18).
func (s *Service) SplitPane(ctx context.Context, session, paneID, direction string) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.RunCommand(cmdSplitPane(paneID, direction, ""), "")
}

// KillPane kills a pane by stable ID.
func (s *Service) KillPane(ctx context.Context, session, paneID string) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.RunCommand(cmdKillPane(paneID), "")
}

// RenamePane sets a pane's title (shown in the pane header).
func (s *Service) RenamePane(ctx context.Context, session, paneID, title string) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.RunCommand(cmdRenamePane(paneID, title), "")
}

// SelectPane focuses a pane by stable ID.
func (s *Service) SelectPane(ctx context.Context, session, paneID string) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.RunCommand(cmdSelectPane(paneID), "")
}

// ZoomPane toggles zoom on a pane (PRD §20).
func (s *Service) ZoomPane(ctx context.Context, session, paneID string) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.RunCommand(cmdZoomPane(paneID), "")
}

// ResizePane resizes a pane by cell amounts (PRD §19).
func (s *Service) ResizePane(ctx context.Context, session, paneID, direction string, amount int) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.RunCommand(cmdResizePane(paneID, direction, amount), "")
}

// SwapPane swaps two panes by stable IDs.
func (s *Service) SwapPane(ctx context.Context, session, paneID, otherPaneID string) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.RunCommand(cmdSwapPane(paneID, otherPaneID), "")
}

// BreakPane moves a pane into its own window (PRD §17).
func (s *Service) BreakPane(ctx context.Context, session, paneID string) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.RunCommand(cmdBreakPane(paneID), "")
}

// BreakActivePane moves the active pane of a window into its own window.
func (s *Service) BreakActivePane(ctx context.Context, session, windowID string) error {
	snap, err := s.Snapshot(ctx, session)
	if err != nil {
		return err
	}
	var active string
	for _, p := range snap.Panes {
		if p.WindowID == windowID && p.Active {
			active = p.ID
			break
		}
	}
	if active == "" {
		return fmt.Errorf("no active pane in window %s", windowID)
	}
	return s.BreakPane(ctx, session, active)
}

// --- terminal ---

// SendInput forwards raw terminal input to a pane (PRD §22). Batched.
func (s *Service) SendInput(session, paneID string, data []byte) error {
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	m.SendInput(paneID, data)
	return nil
}

// ResizeTerminal resizes the control client's viewport (PRD §26 terminal.resize).
//
// Raw per-client resizes no longer flow straight from the WebSocket handler:
// the realtime hub aggregates client viewports (minimum across the session,
// debounced) and is the only caller of this method (§83). The size bounds
// (cols >= 2, rows >= 1) mirror the hub's UpdateViewport validation.
func (s *Service) ResizeTerminal(session string, cols, rows int) error {
	if cols < 2 || rows < 1 {
		return fmt.Errorf("invalid terminal size %dx%d", cols, rows)
	}
	m := s.monitorOrNil(session)
	if m == nil {
		return fmt.Errorf("session %q is not connected", session)
	}
	return m.ResizeTerminal(cols, rows)
}

// CapturePane fetches terminal content for the initial snapshot (PRD §24).
func (s *Service) CapturePane(ctx context.Context, session, paneID string) (string, error) {
	return s.reader.CapturePane(ctx, paneID, s.scrollback)
}

// Snapshot returns a fresh snapshot of a session.
func (s *Service) Snapshot(ctx context.Context, session string) (*Snapshot, error) {
	return s.reader.Snapshot(ctx, session)
}

// Monitor returns the live monitor for a session (nil if not connected).
func (s *Service) Monitor(session string) *Monitor {
	return s.monitorOrNil(session)
}

// ConnectMonitor ensures a control-mode monitor exists for a session (PRD §7).
// Callers must pair with DisconnectMonitor.
func (s *Service) ConnectMonitor(ctx context.Context, session string) (*Monitor, error) {
	if !s.HasSession(ctx, session) {
		return nil, fmt.Errorf("session %q does not exist", session)
	}
	s.mu.Lock()
	m, ok := s.monitors[session]
	if !ok {
		m = NewMonitor(session, s.socket, s.exec, s.log, s.scrollback)
		s.monitors[session] = m
	}
	s.mu.Unlock()
	if err := m.Start(); err != nil {
		return nil, err
	}
	return m, nil
}

// DisconnectMonitor stops the monitor for a session when no clients remain.
func (s *Service) DisconnectMonitor(session string) {
	if m := s.takeMonitor(session); m != nil {
		m.Stop()
	}
}

// --- helpers ---

func (s *Service) monitorOrNil(session string) *Monitor {
	s.mu.Lock()
	defer s.mu.Unlock()
	return s.monitors[session]
}

func (s *Service) getMonitor(session string) *Monitor {
	if session == "" {
		// Prefer any connected monitor for server-level commands.
		s.mu.Lock()
		defer s.mu.Unlock()
		for _, m := range s.monitors {
			return m
		}
		return nil
	}
	return s.monitorOrNil(session)
}

func (s *Service) takeMonitor(session string) *Monitor {
	s.mu.Lock()
	defer s.mu.Unlock()
	m := s.monitors[session]
	delete(s.monitors, session)
	return m
}

// execOneShot runs a mutating command via a one-shot `tmux` invocation.
// Only used when no control-mode client exists (PRD §10 bootstrap path).
// Uses raw argv (command.args), NOT command.line() — line() is control-mode
// stdin syntax with tmux quoting, which must not be re-split as shell words.
func (s *Service) execOneShot(ctx context.Context, c command) error {
	argv := append([]string{c.name}, c.args...)
	if len(argv) == 0 {
		return fmt.Errorf("empty command")
	}
	_, err := s.exec.Run(ctx, argv...)
	return err
}
