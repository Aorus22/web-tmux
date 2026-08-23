package backend

import (
	"bufio"
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"sync"
	"time"
)

type Manager struct {
	path      string
	userData  string
	cmd       *exec.Cmd
	portCh    chan int
	errCh     chan error
	stoppedCh chan struct{}
	logFile   *os.File
	mu        sync.Mutex
	stopped   bool
}

func NewManager(path, userData string) *Manager {
	return &Manager{path: path, userData: userData, portCh: make(chan int, 1), errCh: make(chan error, 1), stoppedCh: make(chan struct{})}
}

func (m *Manager) Port() <-chan int     { return m.portCh }
func (m *Manager) Errors() <-chan error { return m.errCh }

func (m *Manager) Start(ctx context.Context) error {
	if err := os.MkdirAll(m.userData, 0o755); err != nil {
		return fmt.Errorf("create user data directory: %w", err)
	}
	// cmd.Dir is the per-user data directory, so a relative backend path would
	// otherwise be resolved against that directory instead of the caller's
	// working directory (notably when launched by `go run`).
	backendPath := m.path
	if !filepath.IsAbs(backendPath) {
		abs, err := filepath.Abs(backendPath)
		if err != nil {
			return fmt.Errorf("resolve backend path: %w", err)
		}
		backendPath = abs
	}
	cmd := exec.CommandContext(ctx, backendPath)
	cmd.Dir = m.userData
	cmd.Env = backendEnv()
	applyPlatformSysProcAttr(cmd)
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return err
	}
	logPath := filepath.Join(m.userData, "backend.log")
	if f, openErr := os.OpenFile(logPath, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0o644); openErr == nil {
		m.logFile = f
		cmd.Stderr = f
	} else {
		cmd.Stderr = os.Stderr
	}
	if err := cmd.Start(); err != nil {
		return fmt.Errorf("start %s: %w", m.path, err)
	}
	m.cmd = cmd
	go m.scanStdout(stdout)
	go func() {
		err := cmd.Wait()
		m.mu.Lock()
		stopped := m.stopped
		m.mu.Unlock()
		if !stopped {
			select {
			case m.errCh <- err:
			default:
			}
		}
		close(m.stoppedCh)
		if m.logFile != nil {
			_ = m.logFile.Close()
		}
	}()
	return nil
}

func (m *Manager) scanStdout(r io.Reader) {
	s := bufio.NewScanner(r)
	s.Buffer(make([]byte, 64*1024), 1024*1024)
	for s.Scan() {
		line := s.Text()
		if m.logFile != nil {
			_, _ = fmt.Fprintln(m.logFile, line)
		}
		if !strings.HasPrefix(line, "BACKEND_PORT:") {
			continue
		}
		port, err := strconv.Atoi(strings.TrimSpace(strings.TrimPrefix(line, "BACKEND_PORT:")))
		if err == nil && port > 0 {
			select {
			case m.portCh <- port:
			default:
			}
		}
	}
}

func (m *Manager) Stop(ctx context.Context) error {
	m.mu.Lock()
	if m.stopped {
		m.mu.Unlock()
		return nil
	}
	m.stopped = true
	cmd := m.cmd
	m.mu.Unlock()
	if cmd == nil || cmd.Process == nil {
		return nil
	}
	_ = signalStop(cmd.Process.Pid)
	select {
	case <-m.stoppedCh:
		return nil
	case <-time.After(3 * time.Second):
		_ = cmd.Process.Kill()
		return nil
	case <-ctx.Done():
		_ = cmd.Process.Kill()
		return ctx.Err()
	}
}

func backendEnv() []string {
	env := os.Environ()
	env = setEnv(env, "TMUXGUI_HOST", "127.0.0.1")
	env = setEnv(env, "TMUXGUI_PORT", "0")
	env = removeEnv(env, "TMUX")
	env = removeEnv(env, "TMUX_PANE")
	if runtime.GOOS == "windows" && os.Getenv("TMUXGUI_TMUX_BIN") == "" {
		if local := os.Getenv("LOCALAPPDATA"); local != "" {
			candidate := filepath.Join(local, "Microsoft", "WinGet", "Links", "tmux.exe")
			if _, err := os.Stat(candidate); err == nil {
				env = setEnv(env, "TMUXGUI_TMUX_BIN", candidate)
			}
		}
	}
	return env
}

func setEnv(env []string, key, value string) []string {
	env = removeEnv(env, key)
	return append(env, key+"="+value)
}

func removeEnv(env []string, key string) []string {
	prefix := strings.ToUpper(key) + "="
	out := env[:0]
	for _, item := range env {
		if !strings.HasPrefix(strings.ToUpper(item), prefix) {
			out = append(out, item)
		}
	}
	return out
}

func UserDataDir() string {
	if runtime.GOOS == "windows" {
		if base := os.Getenv("APPDATA"); base != "" {
			return filepath.Join(base, "tmux-gui-gtk")
		}
	}
	if base, err := os.UserConfigDir(); err == nil {
		return filepath.Join(base, "tmux-gui-gtk")
	}
	return ".tmux-gui-gtk"
}

var _ = errors.Is
