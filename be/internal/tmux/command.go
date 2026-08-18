package tmux

import (
	"fmt"
	"os"
	"regexp"
	"runtime"
	"strings"
)

// This file builds typed, mutating tmux commands as control-mode stdin lines.
// Commands are sent to the persistent `tmux -CC` client — never as `exec` subprocesses.
//
// Quoting: control-mode stdin is parsed by tmux's command parser, so args are
// double-quoted with backslash escaping. Command separators (`;`) are bare.

var validSessionName = regexp.MustCompile(`^[A-Za-z0-9][A-Za-z0-9_./-]*$`)

// ValidateSessionName checks a new/renamed session name against tmux rules.
// Colons and dots break `-t` targeting, and a leading `$` is reserved.
func ValidateSessionName(name string) error {
	if name == "" {
		return fmt.Errorf("session name is required")
	}
	if len(name) > 200 {
		return fmt.Errorf("session name too long")
	}
	if strings.ContainsAny(name, ":.") {
		return fmt.Errorf("session name must not contain ':' or '.'")
	}
	if strings.HasPrefix(name, "$") {
		return fmt.Errorf("session name must not start with '$'")
	}
	if !validSessionName.MatchString(name) {
		return fmt.Errorf("invalid session name %q", name)
	}
	return nil
}

// quote wraps an arg for tmux's control-mode command parser.
func quote(s string) string {
	s = strings.ReplaceAll(s, `\`, `\\`)
	s = strings.ReplaceAll(s, `"`, `\"`)
	return `"` + s + `"`
}

// command is a control-mode command: command name + args.
type command struct {
	name string
	args []string
}

// line renders the command as a single control-mode stdin line.
func (c command) line() string {
	parts := []string{c.name}
	for _, a := range c.args {
		parts = append(parts, quote(a))
	}
	return strings.Join(parts, " ")
}

func cmd(name string, args ...string) command {
	return command{name: name, args: args}
}

// --- Session commands ---

func cmdNewSession(name, cwd, initialCmd string, detached bool) command {
	args := []string{"-d", "-s", name}
	if cwd != "" {
		args = append(args, "-c", cwd)
	}
	c := cmd("new-session", args...)
	if initialCmd != "" {
		c.args = append(c.args, initialCmd)
	}
	return c
}

func cmdKillSession(name string) command {
	return cmd("kill-session", "-t", name)
}

func cmdRenameSession(name, newName string) command {
	return cmd("rename-session", "-t", name, newName)
}

func cmdSwitchSession(name string) command {
	return cmd("switch-client", "-t", name)
}

// --- Window commands ---

// CreateWindow uses the split-window + break-pane compatibility helper
// (PRD §11): plain `new-window` over control mode is unreliable (tmuxy: crashes 3.5a).
func cmdCreateWindow(session, name, cwd, initialCmd string) command {
	var c command
	if name != "" {
		c = cmd("new-window", "-t", session, "-n", name)
	} else {
		c = cmd("new-window", "-t", session)
	}
	c.args = appendNewWindowArgs(c.args, cwd, initialCmd)
	return c
}

// cmdSplitBreakNewWindow implements new-window as splitw -d -P + breakp.
func cmdSplitBreakNewWindow(session, name, cwd, initialCmd string) command {
	var c command
	if name != "" {
		c = cmd("new-window", "-t", session, "-n", name)
	} else {
		c = cmd("new-window", "-t", session)
	}
	c.args = appendNewWindowArgs(c.args, cwd, initialCmd)
	return c
}

// appendNewWindowArgs handles the platform-specific working-directory
// behavior of the native Windows tmux port. Its -c implementation currently
// returns "spawn failed", so use cmd.exe's `cd /d` in the shell command. A
// bare Windows window still starts normally when cwd is empty.
func appendNewWindowArgs(args []string, cwd, initialCmd string) []string {
	if runtime.GOOS == "windows" {
		if cwd != "" {
			path := strings.ReplaceAll(cwd, `"`, `""`)
			shellCmd := `cd /d "` + path + `"`
			if initialCmd != "" {
				shellCmd += " && " + initialCmd
			} else {
				shellCmd += " && cmd.exe"
			}
			return append(args, shellCmd)
		}
		if initialCmd != "" {
			return append(args, initialCmd)
		}
		return args
	}

	args = append(args, "-c", cwdOrHome(cwd))
	if initialCmd != "" {
		args = append(args, initialCmd)
	}
	return args
}

func cwdOrHome(cwd string) string {
	if cwd != "" {
		return cwd
	}
	if runtime.GOOS == "windows" {
		if home, err := os.UserHomeDir(); err == nil && home != "" {
			return home
		}
		return "."
	}
	return "$HOME"
}

func cmdKillWindow(target string) command {
	return cmd("kill-window", "-t", target)
}

func cmdRenameWindow(target, name string) command {
	return cmd("rename-window", "-t", target, name)
}

func cmdSelectWindow(target string) command {
	return cmd("select-window", "-t", target)
}

func cmdMoveWindow(target string, offset int) command {
	return cmd("move-window", "-t", target, fmt.Sprintf("%+d", offset))
}

func cmdSelectLayout(target, layout string) command {
	return cmd("select-layout", "-t", target, layout)
}

// --- Pane commands ---

func cmdSplitPane(target, direction, cwd string) command {
	// direction: "horizontal" = split left/right (tmux -h), "vertical" = top/bottom (-v)
	args := []string{"-t", target}
	if direction == "vertical" {
		args = append(args, "-v")
	} else {
		args = append(args, "-h")
	}
	if cwd != "" {
		args = append(args, "-c", cwd)
	}
	return cmd("split-window", args...)
}

func cmdKillPane(target string) command {
	return cmd("kill-pane", "-t", target)
}

// cmdRenamePane sets a pane title (shown in the GUI pane header).
func cmdRenamePane(target, title string) command {
	return cmd("select-pane", "-t", target, "-T", title)
}

func cmdSelectPane(target string) command {
	return cmd("select-pane", "-t", target)
}

func cmdZoomPane(target string) command {
	return cmd("resize-pane", "-Z", "-t", target)
}

func cmdResizePane(target, direction string, amount int) command {
	return cmd("resize-pane", "-t", target, "-"+direction, fmt.Sprintf("%d", amount))
}

func cmdSwapPane(target, other string) command {
	return cmd("swap-pane", "-s", target, "-t", other)
}

func cmdBreakPane(target string) command {
	return cmd("break-pane", "-t", target)
}

func cmdJoinPane(src, dst string) command {
	return cmd("join-pane", "-s", src, "-t", dst)
}

// --- Input ---

// cmdSendHex sends raw bytes as hex via send-keys -H (no shell/quoting issues,
// PRD §22). tmux 3.7 requires each key to be its OWN hex argument — a single
// concatenated hex string is silently ignored ("-H flag expects each key to be
// a hexadecimal number for an ASCII character"). Data is a hex string.
func cmdSendHex(pane, hex string) command {
	args := []string{"-t", pane, "-H"}
	for i := 0; i+2 <= len(hex); i += 2 {
		args = append(args, hex[i:i+2])
	}
	return command{name: "send-keys", args: args}
}

// cmdDetachClient tells the control-mode client to detach gracefully.
func cmdDetachClient() command {
	return cmd("detach-client")
}

// cmdResizeClient resizes the control-mode client viewport. In control mode,
// the client size determines the attached window size.
func cmdResizeClient(cols, rows int) command {
	return cmd("refresh-client", "-C", fmt.Sprintf("%d,%d", cols, rows))
}
