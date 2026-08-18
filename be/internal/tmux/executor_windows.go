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

// tmuxBinary prefers the native Winget installation when it exists. Windows
// machines often also have an MSYS2 tmux on PATH; that build can use a
// different server and does not expand the format tokens used by this app.
// TMUXGUI_TMUX_BIN remains an explicit override for custom installations.
func tmuxBinary() string {
	if configured := strings.TrimSpace(os.Getenv("TMUXGUI_TMUX_BIN")); configured != "" {
		return configured
	}
	if localAppData := os.Getenv("LOCALAPPDATA"); localAppData != "" {
		candidate := filepath.Join(localAppData, "Microsoft", "WinGet", "Links", "tmux.exe")
		if _, err := os.Stat(candidate); err == nil {
			return candidate
		}
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

	cmd := exec.CommandContext(ctx, tmuxBinary(), cmdArgs...)
	raw, err := cmd.CombinedOutput()
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
