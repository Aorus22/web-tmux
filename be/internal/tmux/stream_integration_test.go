package tmux

import (
	"context"
	"encoding/hex"
	"fmt"
	"os"
	"os/exec"
	"strings"
	"sync"
	"testing"
	"time"
)

// Real-tmux integration tests for pipe-pane streaming (stream.go). Skipped
// when no usable tmux is installed or -short is set; otherwise they exercise
// the actual pane↔pipe↔socket→Go chain against the local tmux server.

func requireTmux(t *testing.T) {
	t.Helper()
	if testing.Short() {
		t.Skip("skipping real-tmux test in short mode")
	}
	bin := ""
	for _, candidate := range []string{"tmux.exe", "tmux"} {
		if path, err := exec.LookPath(candidate); err == nil {
			bin = path
			break
		}
	}
	if bin == "" {
		t.Skip("tmux not installed")
	}
	out, err := exec.Command(bin, "-V").CombinedOutput()
	if err != nil {
		t.Skipf("tmux -V failed: %v (%s)", err, out)
	}
}

// chunkCollector gathers delivered chunks behind a mutex.
type chunkCollector struct {
	mu     sync.Mutex
	chunks []string
}

func (c *chunkCollector) add(chunk []byte) {
	c.mu.Lock()
	c.chunks = append(c.chunks, string(chunk))
	c.mu.Unlock()
}

func (c *chunkCollector) joined() string {
	c.mu.Lock()
	defer c.mu.Unlock()
	return strings.Join(c.chunks, "")
}

func (c *chunkCollector) reset() {
	c.mu.Lock()
	c.chunks = nil
	c.mu.Unlock()
}

// waitFor polls cond until it holds or the deadline passes.
func waitFor(t *testing.T, timeout time.Duration, what string, cond func() bool) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if cond() {
			return
		}
		time.Sleep(20 * time.Millisecond)
	}
	t.Fatalf("timed out after %v waiting for %s", timeout, what)
}

// pipeTestEnv is an isolated tmux server + uniquely named session.
type pipeTestEnv struct {
	run     func(args ...string) error          // mutating commands (output discarded)
	capture func(paneID string) (string, error) // read-only capture-pane
	session string
	paneID  string
}

func startPipeSession(t *testing.T) *pipeTestEnv {
	t.Helper()
	requireTmux(t)

	suffix := fmt.Sprintf("%d", time.Now().UnixNano())
	sockName := "wtpipe_test_" + suffix
	env := &pipeTestEnv{session: "wtpipe_test_" + suffix}

	executor := NewExecutor(Socket{Name: sockName})
	env.run = func(args ...string) error {
		ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		_, err := executor.Run(ctx, args...)
		return err
	}
	env.capture = func(paneID string) (string, error) {
		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		return executor.Run(ctx, "capture-pane", "-p", "-t", paneID)
	}

	if err := env.run("new-session", "-d", "-s", env.session); err != nil {
		t.Fatalf("new-session: %v", err)
	}
	t.Cleanup(func() {
		_ = env.run("kill-session", "-t", env.session)
	})

	paneOut, err := env.capture(env.session)
	if err != nil {
		t.Fatalf("list panes: %v", err)
	}
	// list-panes with a format would need brace-safe quoting; capture of a
	// fresh session is enough to know the pane exists — resolve its ID via
	// display-message instead.
	idOut, err := executor.Run(context.Background(), "display-message", "-p", "-t", env.session, "#{pane_id}")
	if err != nil {
		t.Fatalf("resolve pane id: %v", err)
	}
	env.paneID = strings.TrimSpace(idOut)
	if env.paneID == "" {
		t.Fatalf("no pane found in session %s", env.session)
	}
	_ = paneOut
	return env
}

func TestPipeStreamRealTmux(t *testing.T) {
	requireTmux(t)
	env := startPipeSession(t)

	var collected chunkCollector
	stream := NewPipeStream(env.paneID,
		env.run,
		collected.add,
		nil,
		func(err error) { t.Logf("stream failure: %v", err) },
		nil,
	)
	t.Cleanup(stream.Disarm)

	// --- live delivery ---
	if err := stream.Arm(); err != nil {
		t.Fatalf("Arm: %v", err)
	}
	waitFor(t, 5*time.Second, "bash to connect to the pipe socket", stream.InputActive)
	// Open the barrier immediately: this phase asserts LIVE delivery, so
	// chunks must flow straight through (nothing to replay yet).
	stream.ApplySnapshot()
	marker := "WTPIPE_MARKER_" + env.session
	if err := env.run("send-keys", "-l", "-t", env.paneID, "echo "+marker); err != nil {
		t.Fatalf("send-keys: %v", err)
	}
	if err := env.run("send-keys", "-t", env.paneID, "Enter"); err != nil {
		t.Fatalf("send-keys Enter: %v", err)
	}
	waitFor(t, 2*time.Second, "marker chunk via pipe", func() bool {
		return strings.Contains(collected.joined(), marker)
	})
	t.Logf("live delivery OK (%d bytes total)", len(collected.joined()))

	// --- snapshot barrier buffering ---
	collected.reset()
	stream.Disarm()
	if _, statErr := os.Stat(stream.logPath); !os.IsNotExist(statErr) {
		t.Fatalf("log file not removed after Disarm: %v", statErr)
	}
	// Fresh stream, as production arms one object per pane lifecycle.
	barrier := NewPipeStream(env.paneID,
		env.run,
		collected.add,
		nil,
		func(err error) { t.Logf("barrier stream failure: %v", err) },
		nil,
	)
	t.Cleanup(barrier.Disarm)
	if err := barrier.Arm(); err != nil {
		t.Fatalf("re-Arm: %v", err)
	}
	waitFor(t, 5*time.Second, "bash to reconnect after re-arm", barrier.InputActive)
	barrierMarker := "WTPIPE_BARRIER_" + env.session
	if err := env.run("send-keys", "-l", "-t", env.paneID, "echo "+barrierMarker); err != nil {
		t.Fatalf("send-keys (barrier): %v", err)
	}
	if err := env.run("send-keys", "-t", env.paneID, "Enter"); err != nil {
		t.Fatalf("send-keys Enter (barrier): %v", err)
	}
	// Chunks produced BEFORE ApplySnapshot must stay buffered, not delivered.
	time.Sleep(300 * time.Millisecond)
	if got := collected.joined(); got != "" {
		t.Fatalf("chunks delivered before ApplySnapshot: %q", got)
	}
	barrier.ApplySnapshot()
	waitFor(t, 2*time.Second, "buffered chunk replay after ApplySnapshot", func() bool {
		return strings.Contains(collected.joined(), barrierMarker)
	})
	t.Log("barrier buffering OK")

	// --- disarm is idempotent ---
	barrier.Disarm()
	barrier.Disarm()
}

func TestInputPipeRealTmux(t *testing.T) {
	requireTmux(t)
	env := startPipeSession(t)

	var collected chunkCollector
	stream := NewPipeStream(env.paneID,
		env.run,
		collected.add,
		nil,
		func(err error) { t.Logf("stream failure: %v", err) },
		nil,
	)
	t.Cleanup(stream.Disarm)
	if err := stream.Arm(); err != nil {
		t.Fatalf("Arm: %v", err)
	}
	waitFor(t, 5*time.Second, "bash to connect to the pipe socket", stream.InputActive)
	stream.ApplySnapshot()

	paneText := func() string {
		out, err := env.capture(env.paneID)
		if err != nil {
			return ""
		}
		return out
	}

	// --- inject keystrokes over the socket; zero spawns ---
	inMark := "IMARK_" + env.session
	if err := stream.WriteInput([]byte("echo " + inMark + "\n")); err != nil {
		t.Fatalf("WriteInput: %v", err)
	}
	waitFor(t, 2*time.Second, "injected marker in capture-pane", func() bool {
		return strings.Contains(paneText(), inMark)
	})
	t.Log("input injection OK")

	// --- dead pipe: Write reports failure without panicking; the send-keys
	// fallback still delivers ---
	stream.killConn()
	if err := stream.WriteInput([]byte("echo dropped\n")); err == nil {
		t.Log("write after kill succeeded (tmux respawned the pump quickly)")
	} else {
		t.Logf("write after kill reported error as expected: %v", err)
	}
	fbMark := "FBMARK_" + env.session
	hexStr := hex.EncodeToString([]byte("echo " + fbMark + "\r"))
	args := []string{"send-keys", "-t", env.paneID, "-H"}
	for i := 0; i+2 <= len(hexStr); i += 2 {
		args = append(args, hexStr[i:i+2])
	}
	if err := env.run(args...); err != nil {
		t.Fatalf("send-keys fallback: %v", err)
	}
	waitFor(t, 2*time.Second, "fallback marker in capture-pane", func() bool {
		return strings.Contains(paneText(), fbMark)
	})
	t.Log("send-keys fallback OK")
}
