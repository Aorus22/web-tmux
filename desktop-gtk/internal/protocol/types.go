package protocol

type Session struct {
	Name      string `json:"name"`
	Windows   int    `json:"windows"`
	Attached  int    `json:"attached"`
	CreatedAt int64  `json:"createdAt"`
	Width     int    `json:"width"`
	Height    int    `json:"height"`
}

type Window struct {
	ID     string `json:"id"`
	Index  int    `json:"index"`
	Name   string `json:"name"`
	Active bool   `json:"active"`
	Panes  int    `json:"panes"`
	Width  int    `json:"width"`
	Height int    `json:"height"`
	Layout string `json:"layout"`
}

type Pane struct {
	ID             string `json:"id"`
	Index          int    `json:"index"`
	WindowID       string `json:"windowId"`
	Active         bool   `json:"active"`
	Zoomed         bool   `json:"zoomed"`
	Left           int    `json:"left"`
	Top            int    `json:"top"`
	Width          int    `json:"width"`
	Height         int    `json:"height"`
	PID            int    `json:"pid"`
	CurrentCommand string `json:"currentCommand"`
	CurrentPath    string `json:"currentPath"`
	Title          string `json:"title"`
}

type Snapshot struct {
	Session      Session  `json:"session"`
	Windows      []Window `json:"windows"`
	Panes        []Pane   `json:"panes"`
	ActiveWindow string   `json:"activeWindow"`
	ActivePane   string   `json:"activePane"`
}

type WindowTreeNode struct {
	Window Window `json:"window"`
	Panes  []Pane `json:"panes"`
}
type SessionTreeNode struct {
	Session Session          `json:"session"`
	Windows []WindowTreeNode `json:"windows"`
}
type Tree struct {
	Sessions []SessionTreeNode `json:"sessions"`
}
type TmuxInfo struct {
	Version string `json:"version"`
	OK      bool   `json:"ok"`
	Binary  string `json:"binary"`
}

type Incoming struct {
	Type        string `json:"type"`
	RequestID   string `json:"requestId,omitempty"`
	Cols        int    `json:"cols,omitempty"`
	Rows        int    `json:"rows,omitempty"`
	PaneID      string `json:"paneId,omitempty"`
	Data        string `json:"data,omitempty"`
	Direction   string `json:"direction,omitempty"`
	Amount      int    `json:"amount,omitempty"`
	Session     string `json:"session,omitempty"`
	Name        string `json:"name,omitempty"`
	Cwd         string `json:"cwd,omitempty"`
	Command     string `json:"command,omitempty"`
	NewName     string `json:"newName,omitempty"`
	Title       string `json:"title,omitempty"`
	OtherPaneID string `json:"otherPaneId,omitempty"`
	Layout      string `json:"layout,omitempty"`
}

type Outgoing struct {
	Type       string    `json:"type"`
	RequestID  string    `json:"requestId,omitempty"`
	Session    string    `json:"session,omitempty"`
	PaneID     string    `json:"paneId,omitempty"`
	Data       string    `json:"data,omitempty"`
	Replace    bool      `json:"replace,omitempty"`
	ScreenRows int       `json:"screenRows,omitempty"` // with Replace: Data lines above this are history
	Message    string    `json:"message,omitempty"`
	Seq        uint64    `json:"seq,omitempty"`
	Snapshot   *Snapshot `json:"snapshot,omitempty"`
}

const (
	Hello             = "hello"
	TerminalInput     = "terminal.input"
	TerminalResize    = "terminal.resize"
	TerminalCapture   = "terminal.capture"
	PaneSelect        = "pane.select"
	PaneSplit         = "pane.split"
	PaneResize        = "pane.resize"
	PaneKill          = "pane.kill"
	PaneRename        = "pane.rename"
	PaneZoom          = "pane.zoom"
	PaneBreak         = "pane.break"
	PaneSwap          = "pane.swap"
	WindowSelect      = "window.select"
	WindowCreate      = "window.create"
	WindowRename      = "window.rename"
	WindowKill        = "window.kill"
	WindowLayout      = "window.layout"
	WindowMove        = "window.move"
	WindowBreakActive = "window.break-active"
	SessionCreate     = "session.create"
	SessionRename     = "session.rename"
	SessionKill       = "session.kill"
	StateResync       = "state.resync"

	ConnectionReady  = "connection.ready"
	StateSnapshot    = "state.snapshot"
	StateDelta       = "state.delta"
	TerminalSnapshot = "terminal.snapshot"
	TerminalOutput   = "terminal.output"
	CommandSuccess   = "command.success"
	CommandError     = "command.error"
	TmuxDisconnected = "tmux.disconnected"
	TmuxReconnecting = "tmux.reconnecting"
	ServerError      = "server.error"
)
