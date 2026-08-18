//go:build windows

package tmux

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"strconv"
	"strings"
	"sync"
)

// Control is the native-Windows command adapter.
//
// arndawg/tmux-windows currently does not implement tmux control mode (-C),
// so Windows cannot use the persistent client used on Unix. We keep the same
// Control surface for the monitor, but execute each typed command as a normal
// one-shot tmux invocation. The monitor supplies the missing asynchronous
// events by polling snapshots and capture-pane output.
type Control struct {
	session string
	socket  Socket
	log     *slog.Logger
	exec    *Executor

	mu        sync.Mutex
	commandMu sync.Mutex
	closed    bool
	running   bool

	// Kept for the platform-neutral Monitor wiring. One-shot commands do not
	// have a persistent child whose exit needs to be reported asynchronously.
	OnExit func(err error)
}

func NewControl(session string, socket Socket, log *slog.Logger) *Control {
	return &Control{
		session: session,
		socket:  socket,
		log:     log,
		exec:    NewExecutor(socket),
	}
}

// IsSynchronous reports that commands complete in RunCommand. Unix control
// mode completes them later through %end/%error markers.
func (c *Control) IsSynchronous() bool { return true }

// Start marks the adapter as ready. There is no persistent tmux child because
// the native Windows port does not support -C.
func (c *Control) Start() error {
	c.mu.Lock()
	c.closed = false
	c.running = true
	c.mu.Unlock()
	c.log.Debug("using native Windows tmux one-shot adapter", "session", c.session)
	return nil
}

// RunCommand executes a typed mutation with argv preserved. Using argv instead
// of command.line() is important: line() is control-mode syntax and must not
// be reparsed as shell words.
func (c *Control) RunCommand(command Command) error {
	c.mu.Lock()
	if c.closed || !c.running {
		c.mu.Unlock()
		return errors.New("tmux native Windows adapter is not running")
	}
	c.mu.Unlock()

	// Input batches and UI actions must not race one another on the same
	// session. tmux itself is safe, but serialization preserves command order.
	c.commandMu.Lock()
	defer c.commandMu.Unlock()

	args := append([]string{command.name}, command.args...)
	if _, err := c.exec.Run(context.Background(), args...); err != nil {
		return err
	}
	return nil
}

// Resize updates the active tmux window directly. refresh-client -C only
// applies to a control-mode client, which is unavailable on native Windows.
func (c *Control) Resize(cols, rows int) error {
	return c.RunCommand(Command{
		name: "resize-window",
		args: []string{"-t", c.session, "-x", strconv.Itoa(cols), "-y", strconv.Itoa(rows)},
	})
}

// Write is retained for the platform-neutral Control API. Native Windows tmux
// has no control-mode stdin; Monitor uses RunCommand instead.
func (c *Control) Write(line string) error {
	return fmt.Errorf("tmux control mode is unavailable on native Windows (command %q)", line)
}

// ReadLine is retained for the platform-neutral Control API. Events are
// produced by Monitor's polling loop on native Windows.
func (c *Control) ReadLine() (string, error) {
	return "", errors.New("tmux control mode is unavailable on native Windows")
}

func StripControlModeWrapper(line string) string {
	line = strings.TrimPrefix(line, "\x1bP1000p")
	return strings.TrimSuffix(line, "\x1b\\")
}

// Stop closes the adapter. tmux sessions are intentionally left alive.
func (c *Control) Stop() {
	c.mu.Lock()
	c.closed = true
	c.running = false
	c.mu.Unlock()
}
