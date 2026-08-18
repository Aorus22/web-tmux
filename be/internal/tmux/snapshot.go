package tmux

import (
	"context"
	"fmt"
	"sort"
	"strconv"
	"strings"
)

// SnapshotReader builds full state dumps using one-shot read-only queries
// (PRD §9). These queries are safe to run while a control-mode client exists.

// Format strings used by one-shot read-only queries. Exported so external
// tests can assert the field counts.
const (
	SessionFormat = "#{session_name}|#{session_windows}|#{session_attached}|#{session_created}|#{session_width}|#{session_height}"
	WindowFormat  = "#{window_id}|#{window_index}|#{window_name}|#{window_active}|#{window_panes}|#{window_width}|#{window_height}|#{window_layout}"
	PaneFormat    = "#{pane_id}|#{pane_index}|#{window_id}|#{pane_active}|#{window_zoomed_flag}|#{pane_left}|#{pane_top}|#{pane_width}|#{pane_height}|#{pane_pid}|#{pane_current_command}|#{pane_current_path}|#{pane_title}"
)

type SnapshotReader struct {
	exec *Executor
}

func NewSnapshotReader(exec *Executor) *SnapshotReader {
	return &SnapshotReader{exec: exec}
}

// ListSessions returns all sessions on the server.
func (s *SnapshotReader) ListSessions(ctx context.Context) ([]Session, error) {
	lines, err := s.exec.Output(ctx, "list-sessions", "-F", SessionFormat)
	if err != nil {
		return nil, err
	}
	var out []Session
	for _, l := range lines {
		if strings.TrimSpace(l) == "" {
			continue
		}
		ses, ok := ParseSession(l)
		if ok {
			out = append(out, ses)
		}
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Name < out[j].Name })
	return out, nil
}

// ListWindows returns windows for a session.
func (s *SnapshotReader) ListWindows(ctx context.Context, session string) ([]Window, error) {
	lines, err := s.exec.Output(ctx, "list-windows", "-t", session, "-F", WindowFormat)
	if err != nil {
		return nil, err
	}
	var out []Window
	for _, l := range lines {
		if strings.TrimSpace(l) == "" {
			continue
		}
		if w, ok := ParseWindow(l); ok {
			out = append(out, w)
		}
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Index < out[j].Index })
	return out, nil
}

// ListPanes returns panes for a window (or all windows of a session when window is "").
func (s *SnapshotReader) ListPanes(ctx context.Context, target string) ([]Pane, error) {
	lines, err := s.exec.Output(ctx, "list-panes", "-t", target, "-F", PaneFormat)
	if err != nil {
		return nil, err
	}
	var out []Pane
	for _, l := range lines {
		if strings.TrimSpace(l) == "" {
			continue
		}
		if p, ok := ParsePane(l); ok {
			out = append(out, p)
		}
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Index < out[j].Index })
	return out, nil
}

// Snapshot returns the full state of one session (windows + panes).
func (s *SnapshotReader) Snapshot(ctx context.Context, session string) (*Snapshot, error) {
	sessions, err := s.ListSessions(ctx)
	if err != nil {
		return nil, err
	}
	var found *Session
	for i := range sessions {
		if sessions[i].Name == session {
			found = &sessions[i]
			break
		}
	}
	if found == nil {
		return nil, fmt.Errorf("session %q not found", session)
	}

	windows, err := s.ListWindows(ctx, session)
	if err != nil {
		return nil, err
	}
	panes, err := s.ListPanes(ctx, session)
	if err != nil {
		return nil, err
	}

	snap := &Snapshot{Session: *found, Windows: windows, Panes: panes}
	for _, w := range windows {
		if w.Active {
			snap.ActiveWindow = w.ID
		}
	}
	for _, p := range panes {
		if p.Active {
			snap.ActivePane = p.ID
		}
	}
	return snap, nil
}

// Tree returns the complete session tree for the sidebar.
func (s *SnapshotReader) Tree(ctx context.Context) (*Tree, error) {
	sessions, err := s.ListSessions(ctx)
	if err != nil {
		return nil, err
	}
	tree := &Tree{Sessions: []SessionTreeNode{}}
	for _, ses := range sessions {
		windows, err := s.ListWindows(ctx, ses.Name)
		if err != nil {
			// Session may have died between queries; skip it.
			continue
		}
		panes, err := s.ListPanes(ctx, ses.Name)
		if err != nil {
			continue
		}
		node := SessionTreeNode{Session: ses, Windows: []WindowTreeNode{}}
		byWindow := map[string]*WindowTreeNode{}
		for i := range windows {
			w := windows[i]
			// Initialize Panes to an empty slice: a nil slice would marshal to
			// `"panes": null` in JSON, crashing the sidebar's panes.map() when
			// a window races between list-windows and list-panes.
			byWindow[w.ID] = &WindowTreeNode{Window: w, Panes: []Pane{}}
		}
		for _, p := range panes {
			if wn, ok := byWindow[p.WindowID]; ok {
				wn.Panes = append(wn.Panes, p)
			}
		}
		for _, w := range windows {
			node.Windows = append(node.Windows, *byWindow[w.ID])
		}
		tree.Sessions = append(tree.Sessions, node)
	}
	return tree, nil
}

// CapturePane captures terminal content for the initial snapshot (PRD §24).
func (s *SnapshotReader) CapturePane(ctx context.Context, paneID string, scrollback int) (string, error) {
	return s.exec.Run(ctx, "capture-pane", "-p", "-e", "-J", "-t", paneID, "-S", fmt.Sprintf("-%d", scrollback))
}

// TmuxVersion returns the installed tmux version string, or an error.
func (s *SnapshotReader) TmuxVersion(ctx context.Context) (string, error) {
	raw, err := s.exec.Run(ctx, "-V")
	if err != nil {
		return "", err
	}
	return strings.TrimSpace(strings.TrimPrefix(raw, "tmux ")), nil
}

// HasSession checks whether a session exists.
func (s *SnapshotReader) HasSession(ctx context.Context, name string) (bool, error) {
	_, err := s.exec.Run(ctx, "has-session", "-t", name)
	if err != nil {
		return false, nil
	}
	return true, nil
}

// --- parsers ---

func ParseSession(l string) (Session, bool) {
	p := strings.Split(l, "|")
	if len(p) < 6 {
		return Session{}, false
	}
	created, _ := strconv.ParseInt(p[3], 10, 64)
	width, _ := strconv.Atoi(p[4])
	height, _ := strconv.Atoi(p[5])
	return Session{
		Name:      p[0],
		Windows:   atoiSafe(p[1]),
		Attached:  atoiSafe(p[2]),
		CreatedAt: created,
		Width:     width,
		Height:    height,
	}, true
}

func ParseWindow(l string) (Window, bool) {
	p := strings.Split(l, "|")
	if len(p) < 8 {
		return Window{}, false
	}
	return Window{
		ID:     p[0],
		Index:  atoiSafe(p[1]),
		Name:   p[2],
		Active: p[3] == "1",
		Panes:  atoiSafe(p[4]),
		Width:  atoiSafe(p[5]),
		Height: atoiSafe(p[6]),
		Layout: p[7],
	}, true
}

func ParsePane(l string) (Pane, bool) {
	p := strings.Split(l, "|")
	if len(p) < 13 {
		return Pane{}, false
	}
	return Pane{
		ID:       p[0],
		Index:    atoiSafe(p[1]),
		WindowID: p[2],
		Active:   p[3] == "1",
		// tmux only reports window_zoomed_flag (per-window, true for every pane
		// in the zoomed window); the zoomed pane is the active one, so a pane is
		// "zoomed" only when the window is zoomed AND this pane is active.
		Zoomed:         p[4] == "1" && p[3] == "1",
		Left:           atoiSafe(p[5]),
		Top:            atoiSafe(p[6]),
		Width:          atoiSafe(p[7]),
		Height:         atoiSafe(p[8]),
		PID:            atoiSafe(p[9]),
		CurrentCommand: p[10],
		CurrentPath:    p[11],
		Title:          p[12],
	}, true
}

func atoiSafe(s string) int {
	n, _ := strconv.Atoi(s)
	return n
}
