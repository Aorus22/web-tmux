package tmux

import (
	"strings"
	"testing"
)

func TestCommandStableIDs(t *testing.T) {
	c := cmdKillPane("%12")
	line := c.line()
	if !strings.Contains(line, "%12") {
		t.Fatalf("kill-pane must target stable pane ID: %q", line)
	}
	c = cmdSelectWindow("@3")
	if !strings.Contains(c.line(), "@3") {
		t.Fatalf("select-window must target stable window ID: %q", c.line())
	}
}

func TestCommandQuoting(t *testing.T) {
	c := cmdRenameWindow("@3", `weird "name" \ backslash`)
	line := c.line()
	// tmux control-mode parser handles double-quoted args with backslash escapes.
	if !strings.Contains(line, `"weird \"name\" \\ backslash"`) {
		t.Fatalf("quoting broken: %q", line)
	}
}

func TestCommandSplitDirections(t *testing.T) {
	h := cmdSplitPane("%1", "horizontal", "").line()
	if !strings.Contains(h, "-h") || strings.Contains(h, "-v") {
		t.Fatalf("horizontal split: %q", h)
	}
	v := cmdSplitPane("%1", "vertical", "").line()
	if !strings.Contains(v, "-v") || strings.Contains(v, "-h") {
		t.Fatalf("vertical split: %q", v)
	}
}

func TestSendHexCommand(t *testing.T) {
	// send-keys -t %2 -H 68 65 6c 6c 6f → "hello" as per-byte hex args.
	// tmux 3.7 requires each hex byte as its own argument.
	c := cmdSendHex("%2", "68656c6c6f")
	line := c.line()
	if !strings.Contains(line, `"-t" "%2" "-H" "68" "65" "6c" "6c" "6f"`) {
		t.Fatalf("hex send broken: %q", line)
	}
	if strings.Contains(line, "hello") {
		t.Fatalf("raw payload must not appear in command: %q", line)
	}
}

func TestValidateSessionName(t *testing.T) {
	valid := []string{"dev", "my-session", "backend_2", "a/b"}
	invalid := []string{"", "has:colon", "has.dot", "$env", "with space"}
	for _, n := range valid {
		if err := ValidateSessionName(n); err != nil {
			t.Fatalf("%q should be valid: %v", n, err)
		}
	}
	for _, n := range invalid {
		if err := ValidateSessionName(n); err == nil {
			t.Fatalf("%q should be invalid", n)
		}
	}
}

func TestCommandLineNoSemicolons(t *testing.T) {
	// Mutations must be a single control-mode command (no compound statements
	// that could be misparsed).
	for _, c := range []command{
		cmdKillPane("%1"),
		cmdSplitPane("%1", "horizontal", ""),
		cmdCreateWindow("dev", "editor", "", ""),
		cmdRenameSession("dev", "prod"),
	} {
		if strings.Contains(c.line(), ";") {
			t.Fatalf("command must not contain ';': %q", c.line())
		}
	}
}
