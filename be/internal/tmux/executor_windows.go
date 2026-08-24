//go:build windows

package tmux

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

// Executor runs one-shot tmux commands on Windows.
//
// The Windows tmux port accepts ordinary stdin/stdout pipes for non-control
// commands, so we do not need a POSIX PTY here.
type Executor struct {
	socket Socket
}

// tmuxBinary resolves tmux the same way the user's shell does. Windows users
// can have multiple tmux ports installed (for example MSYS2 and the WinGet
// native port); each port maintains a separate server, so silently preferring
// one installation makes sessions appear to disappear. TMUXGUI_TMUX_BIN
// remains an explicit override for custom installations.
func tmuxBinary() string {
	if configured := strings.TrimSpace(os.Getenv("TMUXGUI_TMUX_BIN")); configured != "" {
		return configured
	}
	// LookPath honours PATH ordering, including the .exe extension on Windows.
	// Keep the explicit .exe lookup first so the selected path is visible in
	// diagnostics and is exactly the one PowerShell would execute.
	if path, err := exec.LookPath("tmux.exe"); err == nil {
		return path
	}
	if path, err := exec.LookPath("tmux"); err == nil {
		return path
	}
	return "tmux"
}

func NewExecutor(socket Socket) *Executor {
	return &Executor{socket: socket}
}

// Run executes a tmux command and returns trimmed stdout/stderr.
func (e *Executor) Run(ctx context.Context, args ...string) (string, error) {
	cmdArgs := append(e.socket.Args(), args...)
	ctx, cancel := context.WithTimeout(ctx, 10*time.Second)
	defer cancel()

	binary := tmuxBinary()
	cmd := tmuxCommand(ctx, binary, cmdArgs)
	var raw []byte
	var err error
	if msysBash(binary) != "" && tmuxStartsServer(cmdArgs) && !tmuxServerRunning(ctx, binary, cmdArgs) {
		// The MSYS2 server inherits stdout/stderr when it is bootstrapped by a
		// detached new-session. Keep those handles off the backend pipe so the
		// client can exit immediately; the exit status still reports failures.
		nul, openErr := os.OpenFile(os.DevNull, os.O_WRONLY, 0)
		if openErr != nil {
			return "", fmt.Errorf("tmux %s: %w", strings.Join(args, " "), openErr)
		}
		cmd.Stdout = nul
		cmd.Stderr = nul
		err = cmd.Run()
		_ = nul.Close()
	} else {
		raw, err = cmd.CombinedOutput()
	}
	if err != nil {
		msg := strings.TrimSpace(string(raw))
		if msg != "" {
			return "", fmt.Errorf("tmux %s: %s", strings.Join(args, " "), msg)
		}
		if ctx.Err() != nil {
			return "", fmt.Errorf("tmux %s: %w", strings.Join(args, " "), ctx.Err())
		}
		return "", fmt.Errorf("tmux %s: %w", strings.Join(args, " "), err)
	}

	if ctx.Err() != nil {
		return "", fmt.Errorf("tmux %s: %w", strings.Join(args, " "), ctx.Err())
	}

	return strings.TrimRight(string(raw), "\r\n"), nil
}

// tmuxCommand invokes the MSYS2 port through bash. A Windows process launch
// strips braces from arguments such as #{session_name} before the MSYS2
// runtime sees them, which turns every format query into a literal string.
// Passing a shell-quoted command line through bash preserves those tokens and
// still keeps user-controlled arguments data (not shell syntax).
func tmuxCommand(ctx context.Context, binary string, args []string) *exec.Cmd {
	if bash := msysBash(binary); bash != "" {
		parts := make([]string, 0, len(args)+1)
		parts = append(parts, "tmux")
		for _, arg := range args {
			parts = append(parts, shellQuote(arg))
		}
		script := "exec " + strings.Join(parts, " ")
		return exec.CommandContext(ctx, bash, "--noprofile", "--norc", "-c", script)
	}
	return exec.CommandContext(ctx, binary, args...)
}

func msysBash(binary string) string {
	path := filepath.Clean(binary)
	normalized := strings.ToLower(strings.ReplaceAll(path, "/", "\\"))
	if !strings.HasSuffix(normalized, `\usr\bin\tmux.exe`) {
		return ""
	}
	bash := filepath.Join(filepath.Dir(path), "bash.exe")
	if _, err := os.Stat(bash); err != nil {
		return ""
	}
	return bash
}

func shellQuote(value string) string {
	return "'" + strings.ReplaceAll(value, "'", "'\\''") + "'"
}

func tmuxStartsServer(args []string) bool {
	for i := 0; i < len(args); i++ {
		switch args[i] {
		case "-L", "-S":
			i++
		case "new-session":
			return true
		default:
			if strings.HasPrefix(args[i], "-") {
				continue
			}
			return false
		}
	}
	return false
}

func tmuxServerRunning(ctx context.Context, binary string, args []string) bool {
	probe := make([]string, 0, 5)
	for i := 0; i < len(args); i++ {
		if args[i] != "-L" && args[i] != "-S" {
			break
		}
		if i+1 >= len(args) {
			break
		}
		probe = append(probe, args[i], args[i+1])
		i++
	}
	probe = append(probe, "list-sessions", "-F", "x")
	cmd := tmuxCommand(ctx, binary, probe)
	_, err := cmd.CombinedOutput()
	return err == nil
}

// Output runs a command and returns raw output lines (split, trimmed of CR).
func (e *Executor) Output(ctx context.Context, args ...string) ([]string, error) {
	raw, err := e.Run(ctx, args...)
	if err != nil {
		return nil, err
	}
	if raw == "" {
		return nil, nil
	}
	lines := strings.Split(raw, "\n")
	for i := range lines {
		lines[i] = strings.TrimSuffix(lines[i], "\r")
	}
	return lines, nil
}
