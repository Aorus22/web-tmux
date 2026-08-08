package tmux

import (
	"context"
	"fmt"
	"io"
	"os/exec"
	"strings"
	"time"

	"github.com/creack/pty"
)

// Executor runs one-shot, read-only tmux commands (PRD §9). It must NEVER be
// used for mutating commands while a control-mode client is attached.
//
// tmux requires a controlling terminal even for detached commands
// ("open terminal failed: not a terminal" without one). Every invocation runs
// on a PTY so the backend works when launched detached (Electron, systemd,
// nohup) — matching the control-mode client which also needs a PTY.
type Executor struct {
	socket Socket
}

func NewExecutor(socket Socket) *Executor {
	return &Executor{socket: socket}
}

// Run executes a tmux command on a PTY and returns trimmed stdout. The child's
// exit status is always captured via Wait so failing commands (e.g. has-session
// against a dead server) surface as errors instead of silent success.
func (e *Executor) Run(ctx context.Context, args ...string) (string, error) {
	cmdArgs := append(e.socket.Args(), args...)
	ctx, cancel := context.WithTimeout(ctx, 10*time.Second)
	defer cancel()

	cmd := exec.CommandContext(ctx, "tmux", cmdArgs...)

	// Allocate a PTY with a controlling terminal (Setsid + Setctty via
	// pty.Start). This is what tmux needs, detached or not. creack/pty closes
	// the parent's copy of the slave after Start, so the master hits EOF when
	// the child exits.
	ptmx, err := pty.Start(cmd)
	if err != nil {
		return "", fmt.Errorf("tmux %s: %w", strings.Join(args, " "), err)
	}
	defer ptmx.Close()

	// EIO is expected on the master when the child exits; the output is still
	// drained below. Exit status comes from Wait, not the read.
	raw, _ := io.ReadAll(ptmx)

	if err := cmd.Wait(); err != nil {
		msg := strings.TrimSpace(string(raw))
		if msg != "" {
			return "", fmt.Errorf("tmux %s: %s", strings.Join(args, " "), msg)
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
