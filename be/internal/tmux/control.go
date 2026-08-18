//go:build !windows

package tmux

import (
	"bufio"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"os"
	"os/exec"
	"strings"
	"sync"
	"syscall"
	"time"

	"github.com/creack/pty"
)

// Control manages a persistent `tmux -CC attach-session -t <session>` child
// process (PRD §7).
//
// tmux control mode requires a real PTY (tcgetattr on the client terminal;
// tmuxy: "openpty + controlling terminal — required by -CC"). The backend owns
// the PTY master: mutating commands are written to it, and the control
// protocol (events starting with '%') arrives on it.
type Control struct {
	session string
	socket  Socket
	log     *slog.Logger

	mu     sync.Mutex
	cmd    *exec.Cmd
	master *os.File // PTY master
	reader *bufio.Reader
	closed bool

	// waitDone is closed exactly once, after the single cmd.Wait() (spawned in
	// Start) reaps the child. Stop waits on it instead of calling Wait again —
	// concurrent Wait calls on one *exec.Cmd race on its internals.
	waitDone chan struct{}

	// OnExit is called (in a goroutine) when the process exits unexpectedly.
	OnExit func(err error)
}

func NewControl(session string, socket Socket, log *slog.Logger) *Control {
	return &Control{
		session:  session,
		socket:   socket,
		log:      log,
		waitDone: make(chan struct{}),
	}
}

// IsSynchronous reports whether RunCommand completes immediately. Unix
// control mode returns false because completion arrives via %end/%error.
func (c *Control) IsSynchronous() bool { return false }

// RunCommand sends a typed command through the control-mode parser.
func (c *Control) RunCommand(command Command) error {
	return c.Write(command.Line())
}

// Start launches the control-mode client on a PTY. The session must exist.
func (c *Control) Start() error {
	args := append(c.socket.Args(), "-CC", "attach-session", "-t", c.session)
	c.log.Debug("starting tmux control mode", "args", args)

	cmd := exec.Command("tmux", args...)
	master, slave, err := pty.Open()
	if err != nil {
		return fmt.Errorf("pty open: %w", err)
	}
	// close slave after the child inherits it
	defer slave.Close()

	cmd.Stdin = slave
	cmd.Stdout = slave
	cmd.Stderr = slave
	cmd.SysProcAttr = &syscall.SysProcAttr{Setsid: true, Setctty: true}

	if err := cmd.Start(); err != nil {
		master.Close()
		return fmt.Errorf("tmux -CC attach: %w", err)
	}
	// The child holds the slave open now; release our copy.
	_ = slave.Close()

	c.mu.Lock()
	c.cmd = cmd
	c.master = master
	c.reader = bufio.NewReader(master)
	c.closed = false
	c.mu.Unlock()

	go func() {
		err := cmd.Wait()
		c.mu.Lock()
		wasClosed := c.closed
		c.mu.Unlock()
		if !wasClosed {
			c.log.Warn("tmux control client exited", "session", c.session, "err", err)
			if c.OnExit != nil {
				c.OnExit(err)
			}
		}
		// Signal any concurrent Stop that the child has been reaped. This is
		// the ONLY cmd.Wait() on this process.
		close(c.waitDone)
	}()
	return nil
}

// Resize sets the PTY window size, which resizes the attached tmux window.
func (c *Control) Resize(cols, rows int) error {
	c.mu.Lock()
	master := c.master
	c.mu.Unlock()
	if master == nil {
		return errors.New("tmux control client is not running")
	}
	return pty.Setsize(master, &pty.Winsize{Cols: uint16(cols), Rows: uint16(rows)})
}

// Write sends one command line (without trailing newline) to control-mode stdin.
func (c *Control) Write(line string) error {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.closed || c.master == nil {
		return errors.New("tmux control client is not running")
	}
	if _, err := io.WriteString(c.master, line+"\n"); err != nil {
		return fmt.Errorf("tmux stdin write: %w", err)
	}
	return nil
}

// ReadLine blocks for the next line from control-mode output.
func (c *Control) ReadLine() (string, error) {
	c.mu.Lock()
	r := c.reader
	c.mu.Unlock()
	if r == nil {
		return "", errors.New("tmux control client is not running")
	}
	line, err := r.ReadString('\n')
	if err != nil {
		return "", err
	}
	return StripControlModeWrapper(strings.TrimRight(line, "\r\n")), nil
}

// stripControlModeWrapper removes the DCS envelope tmux wraps control-mode
// event batches in. A batch is emitted as `\x1bP1000p<events...>\x1b\`: the
// start marker rides on the first event line and the terminator `\x1b\` is
// its own line. Without stripping, the parser would treat the first line as
// raw output (it no longer starts with '%') and broadcast the `%begin`
// marker to clients as garbage with an empty pane id.
func StripControlModeWrapper(line string) string {
	line = strings.TrimPrefix(line, "\x1bP1000p")
	return strings.TrimSuffix(line, "\x1b\\")
}

// Stop detaches the control client gracefully. It sends `detach-client` and
// waits for the process to exit (tmuxy: never SIGKILL a CC client — always
// detach first), falling back to SIGTERM after a timeout. The child is reaped
// by Start's single cmd.Wait(); Stop just waits for that reaping (or kills the
// process after 2s to unblock it).
func (c *Control) Stop() {
	c.mu.Lock()
	if c.closed || c.cmd == nil || c.cmd.Process == nil {
		c.mu.Unlock()
		return
	}
	c.closed = true
	cmd := c.cmd
	master := c.master
	waitDone := c.waitDone
	c.master = nil
	c.reader = nil
	c.mu.Unlock()

	// Graceful detach. Errors are expected when the client already exited.
	if master != nil {
		_, _ = io.WriteString(master, "detach-client\n")
	}

	select {
	case <-waitDone:
	case <-time.After(2 * time.Second):
		_ = cmd.Process.Kill()
		<-waitDone
	}
	if master != nil {
		_ = master.Close()
	}
}
