package server_test

import (
	"tmux-gui/be/internal/server"
	"bytes"
	"context"
	"encoding/json"
	"io"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"os"
	"strings"
	"testing"

	"tmux-gui/be/internal/config"
	"tmux-gui/be/internal/realtime"
	"tmux-gui/be/internal/tmux"
)

// Integration test for POST /api/sessions against a dedicated tmux socket
// (PRD §75). Never touches the user's tmux server; skipped if tmux is
// unavailable. Uses a distinct socket name so it can't collide with the
// service integration tests.

const restTestSocket = "tmux-gui-test-rest"

func restTestServer(t *testing.T) *httptest.Server {
	t.Helper()
	socket := tmux.Socket{Name: restTestSocket}
	exec := tmux.NewExecutor(socket)
	if _, err := exec.Run(context.Background(), "-V"); err != nil {
		t.Skip("tmux not installed:", err)
	}
	log := slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelError}))
	svc := tmux.NewService(socket, log, 2000)
	hub := realtime.NewHub(svc, log)
	cfg := &config.Config{Host: "127.0.0.1", Port: 0}
	return httptest.NewServer(server.NewRouter(cfg, svc, hub, log))
}

func restCleanup(t *testing.T) {
	t.Helper()
	exec := tmux.NewExecutor(tmux.Socket{Name: restTestSocket})
	_, _ = exec.Run(context.Background(), "kill-server")
}

func restPost(t *testing.T, url string, body map[string]string) *http.Response {
	t.Helper()
	raw, _ := json.Marshal(body)
	resp, err := http.Post(url, "application/json", bytes.NewReader(raw))
	if err != nil {
		t.Fatalf("POST %s: %v", url, err)
	}
	t.Cleanup(func() { _ = resp.Body.Close() })
	return resp
}

func restGet(t *testing.T, url string) string {
	t.Helper()
	resp, err := http.Get(url)
	if err != nil {
		t.Fatalf("GET %s: %v", url, err)
	}
	defer resp.Body.Close()
	data, _ := io.ReadAll(resp.Body)
	if resp.StatusCode != http.StatusOK {
		t.Fatalf("GET %s: status %d: %s", url, resp.StatusCode, data)
	}
	return string(data)
}

// The whole point of the REST create endpoint: creating the FIRST session
// with zero existing sessions must work (no WebSocket/control-mode client
// exists to carry the command).
func TestCreateSessionWithZeroSessions(t *testing.T) {
	restCleanup(t)
	defer restCleanup(t)

	ts := restTestServer(t)
	defer ts.Close()

	// Sanity: no sessions yet.
	if got := restGet(t, ts.URL+"/api/sessions"); strings.TrimSpace(got) != `{"sessions":[]}` {
		t.Fatalf("expected empty tree, got %s", got)
	}

	resp := restPost(t, ts.URL+"/api/sessions", map[string]string{"name": "rest-first"})
	if resp.StatusCode != http.StatusCreated {
		t.Fatalf("create: status %d", resp.StatusCode)
	}

	// The session is now visible in the tree.
	tree := restGet(t, ts.URL+"/api/sessions")
	if !bytes.Contains([]byte(tree), []byte("rest-first")) {
		t.Fatalf("created session missing from tree: %s", tree)
	}
}

func TestCreateSessionValidation(t *testing.T) {
	restCleanup(t)
	defer restCleanup(t)

	ts := restTestServer(t)
	defer ts.Close()

	// Invalid name -> 400.
	resp := restPost(t, ts.URL+"/api/sessions", map[string]string{"name": "bad:name"})
	if resp.StatusCode != http.StatusBadRequest {
		t.Fatalf("invalid name: expected 400, got %d", resp.StatusCode)
	}

	// Duplicate name -> 409.
	if resp := restPost(t, ts.URL+"/api/sessions", map[string]string{"name": "dup"}); resp.StatusCode != http.StatusCreated {
		t.Fatalf("first create: expected 201, got %d", resp.StatusCode)
	}
	resp = restPost(t, ts.URL+"/api/sessions", map[string]string{"name": "dup"})
	if resp.StatusCode != http.StatusConflict {
		t.Fatalf("duplicate name: expected 409, got %d", resp.StatusCode)
	}
}
