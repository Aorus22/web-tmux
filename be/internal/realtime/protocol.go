package realtime

// Protocol structs — PRD §26 (client → server) and §27 (server → client).
// Every GUI action carries a requestId for correlation (PRD §28).

// Incoming is a client → server WebSocket message.
type Incoming struct {
	Type     string `json:"type"`
	RequestID string `json:"requestId,omitempty"`

	// hello / terminal.resize
	Cols int `json:"cols,omitempty"`
	Rows int `json:"rows,omitempty"`

	// terminal.input
	PaneID string `json:"paneId,omitempty"`
	Data   string `json:"data,omitempty"`

	// pane.split
	Direction string `json:"direction,omitempty"`

	// pane.resize
	Amount int `json:"amount,omitempty"`

	// window.create / session.create
	Session        string `json:"session,omitempty"`
	Name           string `json:"name,omitempty"`
	Cwd            string `json:"cwd,omitempty"`
	InitialCommand string `json:"command,omitempty"`

	// session.rename
	NewName string `json:"newName,omitempty"`

	// pane.rename
	Title string `json:"title,omitempty"`

	// pane.swap
	OtherPaneID string `json:"otherPaneId,omitempty"`

	// window.layout
	Layout string `json:"layout,omitempty"`
}

// Outgoing is a server → client WebSocket message.
type Outgoing struct {
	Type      string      `json:"type"`
	RequestID string      `json:"requestId,omitempty"`
	Session   string      `json:"session,omitempty"`
	PaneID    string      `json:"paneId,omitempty"`
	Data      string      `json:"data,omitempty"`
	Message   string      `json:"message,omitempty"`
	Seq       uint64      `json:"seq,omitempty"`
	Snapshot  interface{} `json:"snapshot,omitempty"`
}

// Message types (client → server).
const (
	MsgHello          = "hello"
	MsgTerminalInput  = "terminal.input"
	MsgTerminalResize = "terminal.resize"
	MsgTerminalCapture = "terminal.capture"
	MsgPaneSelect     = "pane.select"
	MsgPaneSplit      = "pane.split"
	MsgPaneResize     = "pane.resize"
	MsgPaneKill       = "pane.kill"
	MsgPaneRename     = "pane.rename"
	MsgPaneZoom       = "pane.zoom"
	MsgPaneBreak      = "pane.break"
	MsgPaneSwap       = "pane.swap"
	MsgWindowSelect   = "window.select"
	MsgWindowCreate   = "window.create"
	MsgWindowRename   = "window.rename"
	MsgWindowKill     = "window.kill"
	MsgWindowLayout   = "window.layout"
	MsgWindowMove     = "window.move"
	MsgWindowBreakActive = "window.break-active"
	MsgSessionCreate  = "session.create"
	MsgSessionRename  = "session.rename"
	MsgSessionKill    = "session.kill"
	MsgStateResync    = "state.resync"
)

// Message types (server → client).
const (
	EvConnectionReady  = "connection.ready"
	EvStateSnapshot    = "state.snapshot"
	EvStateDelta       = "state.delta"
	EvTerminalSnapshot = "terminal.snapshot"
	EvTerminalOutput   = "terminal.output"
	EvCommandSuccess   = "command.success"
	EvCommandError     = "command.error"
	EvTmuxDisconnected = "tmux.disconnected"
	EvTmuxReconnecting = "tmux.reconnecting"
	EvServerError      = "server.error"
)
