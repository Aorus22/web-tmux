package tmux

import (
	"errors"
	"fmt"
	"log/slog"
	"net"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

// Pipe-pane streaming (native Windows). While a WS client is connected, each
// pane gets ONE combined pipe-pane carrying BOTH directions:
//
//		tmux pipe-pane -IO -t <pane> \
//		  "exec 3<>/dev/tcp/127.0.0.1/<port>; stdbuf -o0 cat <&3 & stdbuf -o0 cat >> <file>"
//
//	  - OUTPUT: tmux writes pane bytes to the command's stdin; the foreground
//	    cat appends them to a per-pane log file this process tails itself
//	    (10ms polls, plain reads — microseconds, no forks; `tail -f` polls
//	    ~1/s and is never used).
//	  - INPUT: the backend writes raw keystroke bytes to the same localhost
//	    socket; the background cat relays them into the pane's keyboard input —
//	    zero process spawns per keystroke batch.
//
// A pane has a SINGLE pipe slot, so both directions MUST share one invocation
// (arming `pipe-pane -I` alone clobbers an existing `-O` pipe). Empirically
// validated against tmux 3.7b MSYS2: the input pump works in the background,
// while the output drain must stay in the foreground. stdbuf lives at
// /usr/bin/stdbuf; tmux's default shell is /usr/bin/bash, which accepts
// forward-slash drive paths (C:/Users/...).
const (
	streamPollInterval  = 10 * time.Millisecond // file read cadence
	streamOpenTimeout   = 3 * time.Second       // wait for tmux to create the log
	streamAcceptTimeout = 15 * time.Second      // bash connects shortly after arming
	streamMaxLogBytes   = 16 << 20              // rotate beyond ~16MB (heavy-output panes)
	streamBarrierCap    = 256 << 10             // pre-snapshot buffer cap
	streamReadBufSize   = 64 << 10              // per-read scratch allocation
	inputWriteTimeout   = 2 * time.Second       // per-write deadline
)

const pipeDirName = "web-tmux-pipes"

// activePipeFiles tracks log files owned by live streams so WipePipeDir at a
// monitor startup never deletes another session's active pipe logs.
var activePipeFiles sync.Map

func pipeDir() string { return filepath.Join(os.TempDir(), pipeDirName) }

// sanitizePaneID makes a pane ID safe as a file name. Pane IDs look like "%0"
// (already legal), but stay defensive about future ID shapes.
func sanitizePaneID(id string) string {
	var b strings.Builder
	for _, r := range id {
		switch {
		case r >= 'a' && r <= 'z', r >= 'A' && r <= 'Z', r >= '0' && r <= '9',
			r == '.', r == '_', r == '-':
			b.WriteRune(r)
		default:
			b.WriteByte('_')
		}
	}
	return b.String()
}

// WipePipeDir removes stale pipe logs left over from previous runs. Files
// owned by currently-live streams are skipped so multi-session setups do not
// break each other when a second monitor starts.
func WipePipeDir() {
	entries, err := os.ReadDir(pipeDir())
	if err != nil {
		return // directory absent: nothing stale
	}
	for _, e := range entries {
		if e.IsDir() {
			continue
		}
		full := filepath.Join(pipeDir(), e.Name())
		if _, inUse := activePipeFiles.Load(full); inUse {
			continue
		}
		_ = os.Remove(full)
	}
}

// errRotated tells the read loop the log was rotated and a fresh file should
// be opened from offset 0.
var errRotated = errors.New("pipe log rotated")

// PipeStream streams one pane's output AND input through a combined
// pipe-pane: output lands in a tailed log file, input arrives over a
// localhost socket this process owns.
//
// Bootstrap ordering (snapshot barrier): Arm buffers every output chunk;
// once the caller has sent the initial full frame it calls ApplySnapshot,
// which replays the buffer in order and switches to live delivery. If the
// buffer overflows before the snapshot lands, the stream drops the buffer and
// fires onOverflow so the caller can push an immediate integrity frame.
type PipeStream struct {
	paneID     string
	run        func(args ...string) error // executes one tmux command
	deliver    func(chunk []byte)         // live delivery (post-barrier)
	onOverflow func()                     // barrier overflow: request integrity frame
	onFail     func(err error)            // fatal failure: caller falls back to polling
	log        *slog.Logger

	logPath  string // OS path of the output log (Go-side reads)
	tmuxPath string // forward-slash path embedded in the pipe command

	mu              sync.Mutex
	stopCh          chan struct{}
	stopped         bool
	listener        net.Listener
	conn            net.Conn // current pane↔backend connection (may swap on respawn)
	inputDead       bool
	inputDeadLogged bool

	// Reader-goroutine state (single owner, no lock needed).
	offset int64

	// Barrier state (barrierMu): chunks arriving between Arm and
	// ApplySnapshot are buffered to preserve output order.
	barrierMu   sync.Mutex
	buffered    [][]byte
	bufferedLen int
	applied     bool
	overflowed  bool

	wg sync.WaitGroup
}

// NewPipeStream builds a stream for one pane. run executes tmux commands
// (wired to the monitor's command path); deliver receives ordered output
// chunks after the snapshot barrier releases.
func NewPipeStream(
	paneID string,
	run func(args ...string) error,
	deliver func(chunk []byte),
	onOverflow func(),
	onFail func(err error),
	log *slog.Logger,
) *PipeStream {
	name := sanitizePaneID(paneID) + ".log"
	logPath := filepath.Join(pipeDir(), name)
	return &PipeStream{
		paneID:     paneID,
		logPath:    logPath,
		tmuxPath:   filepath.ToSlash(logPath),
		run:        run,
		deliver:    deliver,
		onOverflow: onOverflow,
		onFail:     onFail,
		log:        log,
		stopCh:     make(chan struct{}),
	}
}

// pipeCommand is the shell line tmux runs for the pane: one socket carries
// input, one foreground drain appends output to the log. The output drain
// MUST stay foreground — see the type comment.
func (p *PipeStream) pipeCommand(port int) string {
	return fmt.Sprintf(
		"exec 3<>/dev/tcp/127.0.0.1/%d; stdbuf -o0 cat <&3 & stdbuf -o0 cat >> %s",
		port, p.tmuxPath,
	)
}

// Arm turns on the combined pipe-pane for the pane and starts pumping.
func (p *PipeStream) Arm() error {
	p.mu.Lock()
	if p.stopped {
		p.mu.Unlock()
		return errors.New("pipe stream already stopped")
	}
	p.mu.Unlock()

	if err := os.MkdirAll(pipeDir(), 0o755); err != nil {
		return fmt.Errorf("pipe log dir: %w", err)
	}
	_ = os.Remove(p.logPath)
	activePipeFiles.Store(p.logPath, struct{}{})

	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		activePipeFiles.Delete(p.logPath)
		return fmt.Errorf("pipe listen: %w", err)
	}
	port := ln.Addr().(*net.TCPAddr).Port
	if err := p.run("pipe-pane", "-IO", "-t", p.paneID, p.pipeCommand(port)); err != nil {
		_ = ln.Close()
		activePipeFiles.Delete(p.logPath)
		return fmt.Errorf("pipe-pane arm: %w", err)
	}

	p.mu.Lock()
	if p.stopped {
		p.mu.Unlock()
		_ = ln.Close()
		_ = p.run("pipe-pane", "-t", p.paneID)
		activePipeFiles.Delete(p.logPath)
		return errors.New("pipe stream stopped during arm")
	}
	p.listener = ln
	p.mu.Unlock()

	p.wg.Add(2)
	go p.acceptLoop(ln)
	go p.readLoop()
	return nil
}

// acceptLoop swaps in each connection bash opens. tmux respawns the pipe
// command if it dies (e.g. after rotation), so connections may come and go;
// the newest wins.
func (p *PipeStream) acceptLoop(ln net.Listener) {
	defer p.wg.Done()
	for {
		conn, err := ln.Accept()
		if err != nil {
			return // listener closed (disarm) or fatal
		}
		p.mu.Lock()
		if p.stopped {
			p.mu.Unlock()
			_ = conn.Close()
			return
		}
		old := p.conn
		p.conn = conn
		p.inputDead = false
		p.inputDeadLogged = false
		p.mu.Unlock()
		if old != nil {
			_ = old.Close()
		}
	}
}

// Disarm turns the pipe off, closes the socket, stops the reader, and deletes
// the log. Idempotent.
func (p *PipeStream) Disarm() {
	p.mu.Lock()
	if p.stopped {
		p.mu.Unlock()
		return
	}
	p.stopped = true
	close(p.stopCh)
	ln, conn := p.listener, p.conn
	p.listener, p.conn = nil, nil
	p.mu.Unlock()

	if ln != nil {
		_ = ln.Close()
	}
	if conn != nil {
		_ = conn.Close()
	}
	// `pipe-pane` with NO command turns the pipe off.
	_ = p.run("pipe-pane", "-t", p.paneID)
	p.wg.Wait()
	activePipeFiles.Delete(p.logPath)
	_ = os.Remove(p.logPath)
}

// ApplySnapshot releases the barrier: buffered chunks replay in order, then
// chunks are delivered live as they arrive.
func (p *PipeStream) ApplySnapshot() {
	p.barrierMu.Lock()
	p.applied = true
	pending := p.buffered
	p.buffered = nil
	p.bufferedLen = 0
	p.overflowed = false
	p.barrierMu.Unlock()

	for _, chunk := range pending {
		p.deliver(chunk)
	}
}

// deliverChunk routes one output chunk around the snapshot barrier.
func (p *PipeStream) deliverChunk(chunk []byte) {
	p.barrierMu.Lock()
	if !p.applied {
		if p.bufferedLen+len(chunk) > streamBarrierCap {
			// Overflow: drop the buffer and ask for an immediate integrity
			// frame instead of replaying stale data.
			drop := !p.overflowed
			p.buffered = nil
			p.bufferedLen = 0
			p.overflowed = true
			p.barrierMu.Unlock()
			if drop && p.onOverflow != nil {
				p.onOverflow()
			}
			return
		}
		p.buffered = append(p.buffered, append([]byte(nil), chunk...))
		p.bufferedLen += len(chunk)
		p.barrierMu.Unlock()
		return
	}
	p.barrierMu.Unlock()
	p.deliver(chunk)
}

// --- output side (tailed log file) ---

// readLoop owns the log file across rotations: open, pump, reopen on rotate,
// exit on stop or fatal failure.
func (p *PipeStream) readLoop() {
	defer p.wg.Done()
	for {
		f, ok := p.openLog()
		if !ok {
			return
		}
		err := p.pump(f)
		_ = f.Close()
		if errors.Is(err, errRotated) {
			continue
		}
		return // stopped or fatal
	}
}

// openLog waits up to streamOpenTimeout for tmux to create the log file.
func (p *PipeStream) openLog() (*os.File, bool) {
	deadline := time.Now().Add(streamOpenTimeout)
	for {
		select {
		case <-p.stopCh:
			return nil, false
		default:
		}
		f, err := os.OpenFile(p.logPath, os.O_RDONLY, 0)
		if err == nil {
			return f, true
		}
		if time.Now().After(deadline) {
			p.fail(fmt.Errorf("pipe log never appeared: %s", p.logPath))
			return nil, false
		}
		time.Sleep(streamPollInterval)
	}
}

// pump tails the file at ~10ms granularity with plain reads at the tracked
// offset. External truncation/rotation (size < offset) resets to 0.
func (p *PipeStream) pump(f *os.File) error {
	buf := make([]byte, streamReadBufSize)
	ticker := time.NewTicker(streamPollInterval)
	defer ticker.Stop()
	for {
		select {
		case <-p.stopCh:
			return nil
		case <-ticker.C:
		}
		info, err := f.Stat()
		if err != nil {
			continue // transient stat failure; retry next tick
		}
		size := info.Size()
		switch {
		case size > streamMaxLogBytes:
			// Self-defense against unbounded growth (heavy-output panes):
			// stop delivering, disarm, delete, re-arm fresh.
			if !p.rotate() {
				return nil
			}
			return errRotated
		case size < p.offset:
			p.offset = 0
		case size == p.offset:
			continue
		}
		for p.offset < size {
			n := size - p.offset
			if n > int64(len(buf)) {
				n = int64(len(buf))
			}
			read, err := f.ReadAt(buf[:n], p.offset)
			if read > 0 {
				p.offset += int64(read)
				chunk := make([]byte, read)
				copy(chunk, buf[:read])
				p.deliverChunk(chunk)
			}
			if err != nil || read == 0 {
				break
			}
		}
	}
}

// rotate swaps in a fresh log: pipe off, delete, pipe on (same socket port —
// the listener stays bound for the stream lifetime). Returns false on
// failure (the stream reports and dies; the pane falls back to polling).
func (p *PipeStream) rotate() bool {
	p.mu.Lock()
	ln := p.listener
	p.mu.Unlock()
	port := 0
	if ln != nil {
		port = ln.Addr().(*net.TCPAddr).Port
	}
	_ = p.run("pipe-pane", "-t", p.paneID)
	_ = os.Remove(p.logPath)
	p.offset = 0
	if port == 0 {
		p.fail(errors.New("pipe rotation without listener"))
		return false
	}
	if err := p.run("pipe-pane", "-IO", "-t", p.paneID, p.pipeCommand(port)); err != nil {
		p.fail(fmt.Errorf("pipe-pane rotation: %w", err))
		return false
	}
	return true
}

// fail reports an arm/read failure once; the monitor drops the stream and the
// pane falls back to the capture-polling path.
func (p *PipeStream) fail(err error) {
	if p.log != nil {
		p.log.Warn("pipe stream failed", "pane", p.paneID, "err", err)
	}
	if p.onFail != nil {
		p.onFail(err)
	}
}

// --- input side (localhost socket) ---

// WriteInput injects raw bytes into the pane's keyboard input over the
// socket — zero process spawns. On write failure the input half is marked
// dead (once, with a warning) and callers fall back to send-keys.
func (p *PipeStream) WriteInput(data []byte) error {
	p.mu.Lock()
	defer p.mu.Unlock()
	if p.stopped {
		return errors.New("pipe stream is stopped")
	}
	if p.inputDead {
		return errors.New("pipe input is dead")
	}
	if p.conn == nil {
		return errors.New("pipe input is not connected")
	}
	_ = p.conn.SetWriteDeadline(time.Now().Add(inputWriteTimeout))
	if _, err := p.conn.Write(data); err != nil {
		p.markInputDeadLocked(err)
		return err
	}
	return nil
}

// markInputDeadLocked retires the input half after a write failure. The
// output half keeps flowing; a tmux respawn reconnecting via acceptLoop
// clears the dead flag.
func (p *PipeStream) markInputDeadLocked(err error) {
	p.inputDead = true
	if !p.inputDeadLogged {
		p.inputDeadLogged = true
		if p.log != nil {
			p.log.Warn("pipe input failed; falling back to send-keys", "pane", p.paneID, "err", err)
		}
	}
}

// InputActive reports whether keystrokes can currently be injected.
func (p *PipeStream) InputActive() bool {
	p.mu.Lock()
	defer p.mu.Unlock()
	return !p.stopped && !p.inputDead && p.conn != nil
}

// killConn force-drops the current connection (dead-pipe test hook). Unlike
// a write failure this leaves the input half eligible for revival by the
// next accepted connection.
func (p *PipeStream) killConn() {
	p.mu.Lock()
	conn := p.conn
	p.conn = nil
	p.mu.Unlock()
	if conn != nil {
		_ = conn.Close()
	}
}
