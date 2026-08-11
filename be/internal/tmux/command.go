package tmux

import (
	"fmt"
	"regexp"
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

// Command is a control-mode command: command name + args. Exported so the
// external test package (be/test) can assert on rendered command lines.
type Command struct {
	name string
	args []string
}

// Line renders the command as a single control-mode stdin line.
func (c Command) Line() string {
	parts := []string{c.name}
	for _, a := range c.args {
		parts = append(parts, quote(a))
	}
	return strings.Join(parts, " ")
}

func cmd(name string, args ...string) Command {
	return Command{name: name, args: args}
}

// --- Session commands ---

func cmdNewSession(name, cwd, initialCmd string, detached bool) Command {
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

func cmdKillSession(name string) Command {
	return cmd("kill-session", "-t", name)
}

func cmdRenameSession(name, newName string) Command {
	return cmd("rename-session", "-t", name, newName)
}

func cmdSwitchSession(name string) Command {
	return cmd("switch-client", "-t", name)
}

// --- Window commands ---

// CreateWindow uses the split-window + break-pane compatibility helper
// (PRD §11): plain `new-window` over control mode is unreliable (tmuxy: crashes 3.5a).
func cmdCreateWindow(session, name, cwd, initialCmd string) Command {
	var c Command
	if name != "" {
		c = cmd("new-window", "-t", session, "-n", name, "-c", cwdOrHome(cwd))
	} else {
		c = cmd("new-window", "-t", session, "-c", cwdOrHome(cwd))
	}
	if initialCmd != "" {
		c.args = append(c.args, initialCmd)
	}
	return c
}

// cmdSplitBreakNewWindow implements new-window as splitw -d -P + breakp.
func cmdSplitBreakNewWindow(session, name, cwd, initialCmd string) Command {
	var c Command
	if name != "" {
		c = cmd("new-window", "-t", session, "-n", name, "-c", cwdOrHome(cwd))
	} else {
		c = cmd("new-window", "-t", session, "-c", cwdOrHome(cwd))
	}
	if initialCmd != "" {
		c.args = append(c.args, initialCmd)
	}
	return c
}

func cwdOrHome(cwd string) string {
	if cwd == "" {
		return "$HOME"
	}
	return cwd
}

func cmdKillWindow(target string) Command {
	return cmd("kill-window", "-t", target)
}

func cmdRenameWindow(target, name string) Command {
	return cmd("rename-window", "-t", target, name)
}

func cmdSelectWindow(target string) Command {
	return cmd("select-window", "-t", target)
}

func cmdMoveWindow(target string, offset int) Command {
	return cmd("move-window", "-t", target, fmt.Sprintf("%+d", offset))
}

func cmdSelectLayout(target, layout string) Command {
	return cmd("select-layout", "-t", target, layout)
}

// --- Pane commands ---

func cmdSplitPane(target, direction, cwd string) Command {
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

func cmdKillPane(target string) Command {
	return cmd("kill-pane", "-t", target)
}

// cmdRenamePane sets a pane title (shown in the GUI pane header).
func cmdRenamePane(target, title string) Command {
	return cmd("select-pane", "-t", target, "-T", title)
}

func cmdSelectPane(target string) Command {
	return cmd("select-pane", "-t", target)
}

func cmdZoomPane(target string) Command {
	return cmd("resize-pane", "-Z", "-t", target)
}

func cmdResizePane(target, direction string, amount int) Command {
	return cmd("resize-pane", "-t", target, "-"+direction, fmt.Sprintf("%d", amount))
}

func cmdSwapPane(target, other string) Command {
	return cmd("swap-pane", "-s", target, "-t", other)
}

func cmdBreakPane(target string) Command {
	return cmd("break-pane", "-t", target)
}

func cmdJoinPane(src, dst string) Command {
	return cmd("join-pane", "-s", src, "-t", dst)
}

// --- Input ---

// cmdSendHex sends raw bytes as hex via send-keys -H (no shell/quoting issues,
// PRD §22). tmux 3.7 requires each key to be its OWN hex argument — a single
// concatenated hex string is silently ignored ("-H flag expects each key to be
// a hexadecimal number for an ASCII character"). Data is a hex string.
func cmdSendHex(pane, hex string) Command {
	args := []string{"-t", pane, "-H"}
	for i := 0; i+2 <= len(hex); i += 2 {
		args = append(args, hex[i:i+2])
	}
	return Command{name: "send-keys", args: args}
}

// cmdDetachClient tells the control-mode client to detach gracefully.
func cmdDetachClient() Command {
	return cmd("detach-client")
}

// cmdResizeClient resizes the control-mode client viewport. In control mode,
// the client size determines the attached window size.
func cmdResizeClient(cols, rows int) Command {
	return cmd("refresh-client", "-C", fmt.Sprintf("%d,%d", cols, rows))
}

// --- Exported wrappers for external tests (be/test) ---
// The command builders are package-internal; the external test package can
// only reach the exported surface, so the tested builders are mirrored here.

func CmdKillPane(target string) Command { return cmdKillPane(target) }

func CmdSelectWindow(target string) Command { return cmdSelectWindow(target) }

func CmdRenameWindow(target, name string) Command { return cmdRenameWindow(target, name) }

func CmdSplitPane(target, direction, cwd string) Command { return cmdSplitPane(target, direction, cwd) }

func CmdSendHex(pane, hex string) Command { return cmdSendHex(pane, hex) }

func CmdCreateWindow(session, name, cwd, initialCmd string) Command {
	return cmdCreateWindow(session, name, cwd, initialCmd)
}

func CmdRenameSession(name, newName string) Command { return cmdRenameSession(name, newName) }
