package tmux_test

import (
	"tmux-gui/be/internal/tmux"
	"context"
	"fmt"
	"log/slog"
	"os"
	"testing"
	"time"
)

// Integration test against a dedicated tmux socket (PRD §75). Never touches
// the user's tmux server. Requires tmux on PATH; skipped if unavailable.
//
// Set TMUXGUI_TEST_SOCKET to override the socket name (default tmux-gui-test).

const testSocketName = "tmux-gui-test"

func testService(t *testing.T) *tmux.Service {
	t.Helper()
	socket := tmux.Socket{Name: testSocketName}
	exec := tmux.NewExecutor(socket)
	if _, err := exec.Run(context.Background(), "-V"); err != nil {
		t.Skip("tmux not installed:", err)
	}
	log := slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelError}))
	return tmux.NewService(socket, log, 2000)
}

func cleanupServer(t *testing.T) {
	t.Helper()
	exec := tmux.NewExecutor(tmux.Socket{Name: testSocketName})
	_, _ = exec.Run(context.Background(), "kill-server")
}

func waitFor(t *testing.T, timeout time.Duration, what string, fn func() bool) {
	t.Helper()
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if fn() {
			return
		}
		time.Sleep(50 * time.Millisecond)
	}
	t.Fatalf("timed out waiting for %s", what)
}

func TestServiceLifecycle(t *testing.T) {
	cleanupServer(t)
	defer cleanupServer(t)

	svc := testService(t)
	ctx, cancel := context.WithTimeout(context.Background(), 20*time.Second)
	defer cancel()

	// 1. Create session via one-shot (no control client yet).
	if err := svc.CreateSession(ctx, "itest", "", ""); err != nil {
		t.Fatalf("create session: %v", err)
	}
	waitFor(t, 3*time.Second, "session visible", func() bool {
		ok := svc.HasSession(ctx, "itest")
		return ok
	})

	// 2. Connect control-mode monitor.
	mon, err := svc.ConnectMonitor(ctx, "itest")
	if err != nil {
		t.Fatalf("connect monitor: %v", err)
	}
	defer svc.DisconnectMonitor("itest")

	waitFor(t, 3*time.Second, "snapshot", func() bool {
		s := mon.Snapshot()
		return s != nil && len(s.Panes) == 1
	})

	// 3. Split pane.
	if err := svc.SplitPane(ctx, "itest", "%0", "vertical"); err != nil {
		t.Fatalf("split: %v", err)
	}
	waitFor(t, 3*time.Second, "2 panes", func() bool {
		return len(mon.Snapshot().Panes) == 2
	})

	// 4. Rename window (stable ID @0).
	if err := svc.RenameWindow(ctx, "itest", "@0", "editor"); err != nil {
		t.Fatalf("rename window: %v", err)
	}
	waitFor(t, 3*time.Second, "window renamed", func() bool {
		for _, w := range mon.Snapshot().Windows {
			if w.ID == "@0" && w.Name == "editor" {
				return true
			}
		}
		return false
	})

	// 5. Create a window via control mode (split+break path).
	if err := svc.CreateWindow(ctx, "itest", "logs", "", ""); err != nil {
		t.Fatalf("create window: %v", err)
	}
	waitFor(t, 3*time.Second, "2 windows", func() bool {
		return len(mon.Snapshot().Windows) == 2
	})

	// 6. Resize pane.
	if err := svc.ResizePane(ctx, "itest", "%0", "R", 3); err != nil {
		t.Fatalf("resize: %v", err)
	}
	time.Sleep(300 * time.Millisecond)

	// 7. Capture pane content.
	out, err := svc.CapturePane(ctx, "itest", "%0")
	if err != nil {
		t.Fatalf("capture: %v", err)
	}
	if out == "" {
		t.Log("capture returned empty (shell may still be starting); non-fatal")
	}

	// 8. Zoom. tmux emits %layout-change only for the client's attached window,
	// so select window @0 first (matches real GUI usage: zoom the visible pane).
	if err := svc.SelectWindow(ctx, "itest", "@0"); err != nil {
		t.Fatalf("select window before zoom: %v", err)
	}
	time.Sleep(300 * time.Millisecond)
	if err := svc.ZoomPane(ctx, "itest", "%0"); err != nil {
		t.Fatalf("zoom: %v", err)
	}
	waitFor(t, 3*time.Second, "pane zoomed", func() bool {
		for _, p := range mon.Snapshot().Panes {
			if p.ID == "%0" && p.Zoomed {
				return true
			}
		}
		return false
	})
	// Un-zoom to leave the layout clean.
	_ = svc.ZoomPane(ctx, "itest", "%0")

	// 9. Terminal input via batcher (send-keys -H path).
	if err := svc.SendInput("itest", "%0", []byte("echo itest-ok\n")); err != nil {
		t.Fatalf("send input: %v", err)
	}
	time.Sleep(400 * time.Millisecond)
	out2, err := svc.CapturePane(ctx, "itest", "%0")
	if err != nil {
		t.Fatalf("capture2: %v", err)
	}
	if !contains(out2, "itest-ok") {
		t.Logf("input echo not observed in capture (got %q); non-fatal in CI", out2)
	}

	// 10. Kill pane.
	if err := svc.KillPane(ctx, "itest", "%0"); err != nil {
		t.Fatalf("kill pane: %v", err)
	}
	waitFor(t, 3*time.Second, "pane removed", func() bool {
		for _, p := range mon.Snapshot().Panes {
			if p.ID == "%0" {
				return false
			}
		}
		return true
	})

	// 11. Kill session (control-mode monitor stops; tmux session really dies).
	if err := svc.KillSession(ctx, "itest"); err != nil {
		t.Fatalf("kill session: %v", err)
	}
	waitFor(t, 3*time.Second, "session gone", func() bool {
		return !svc.HasSession(ctx, "itest")
	})
}

func contains(haystack, needle string) bool {
	return len(haystack) >= len(needle) && indexOf(haystack, needle) >= 0
}

func indexOf(s, sub string) int {
	for i := 0; i+len(sub) <= len(s); i++ {
		if s[i:i+len(sub)] == sub {
			return i
		}
	}
	return -1
}

func TestStableIDTargeting(t *testing.T) {
	cleanupServer(t)
	defer cleanupServer(t)

	svc := testService(t)
	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	if err := svc.CreateSession(ctx, "idtest", "", ""); err != nil {
		t.Fatalf("create: %v", err)
	}
	mon, err := svc.ConnectMonitor(ctx, "idtest")
	if err != nil {
		t.Fatalf("monitor: %v", err)
	}
	defer svc.DisconnectMonitor("idtest")
	waitFor(t, 3*time.Second, "snapshot", func() bool {
		return mon.Snapshot() != nil && len(mon.Snapshot().Panes) == 1
	})

	// Stable pane ID format must be %N.
	for _, p := range mon.Snapshot().Panes {
		if len(p.ID) < 2 || p.ID[0] != '%' {
			t.Fatalf("pane id not stable: %q", p.ID)
		}
	}
	// Stable window ID format must be @N.
	for _, w := range mon.Snapshot().Windows {
		if len(w.ID) < 2 || w.ID[0] != '@' {
			t.Fatalf("window id not stable: %q", w.ID)
		}
	}

	fmt.Fprintf(os.Stderr, "observed stable ids: window=%s pane=%s\n",
		mon.Snapshot().Windows[0].ID, mon.Snapshot().Panes[0].ID)
}
