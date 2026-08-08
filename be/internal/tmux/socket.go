package tmux

import (
	"path/filepath"
	"strings"
)

// Socket is a resolved tmux socket configuration. tmux supports either a
// socket name (-L, stored under $TMPDIR) or a full path (-S).
type Socket struct {
	Name string // -L value
	Path string // -S value
}

// ResolveSocket converts a raw TMUXGUI_TMUX_SOCKET value into a Socket.
// An empty raw value means "use the user's default server".
// Absolute paths are used with -S; anything else with -L.
func ResolveSocket(raw string) Socket {
	if raw == "" {
		return Socket{}
	}
	if strings.HasPrefix(raw, "/") {
		return Socket{Path: raw}
	}
	if filepath.IsAbs(raw) {
		return Socket{Path: raw}
	}
	return Socket{Name: raw}
}

// Args returns the socket-related tmux CLI args, if any.
func (s Socket) Args() []string {
	switch {
	case s.Name != "":
		return []string{"-L", s.Name}
	case s.Path != "":
		return []string{"-S", s.Path}
	default:
		return nil
	}
}

// Env returns extra environment for tmux processes using this socket.
func (s Socket) Env() []string {
	// No-op for now; socket selection is purely CLI-based.
	return nil
}
