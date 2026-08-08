package tmux

// Session is the top-level tmux object. Name is the stable identifier.
type Session struct {
	Name      string `json:"name"`
	Windows   int    `json:"windows"`
	Attached  int    `json:"attached"`
	CreatedAt int64  `json:"createdAt"`
	Width     int    `json:"width"`
	Height    int    `json:"height"`
}

// Window uses its tmux stable ID (@N) as identifier; Index is display-only.
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

// Pane uses its tmux stable ID (%N) as identifier; Index is display-only.
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

// Snapshot is a full state dump for one session.
type Snapshot struct {
	Session      Session  `json:"session"`
	Windows      []Window `json:"windows"`
	Panes        []Pane   `json:"panes"`
	ActiveWindow string   `json:"activeWindow"`
	ActivePane   string   `json:"activePane"`
}

// Tree is the sidebar representation: sessions with windows, panes grouped by window.
type Tree struct {
	Sessions []SessionTreeNode `json:"sessions"`
}

type SessionTreeNode struct {
	Session Session `json:"session"`
	Windows []WindowTreeNode `json:"windows"`
}

type WindowTreeNode struct {
	Window Window `json:"window"`
	Panes  []Pane `json:"panes"`
}
