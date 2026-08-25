//go:build windows

package tmux

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"golang.org/x/sys/windows"
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
//
// The resolved Windows binary path is passed through explicitly (never bare
// `tmux`): bash's own PATH may shadow it — e.g. a Linux tmux under
// /usr/local/bin reachable via WSL interop — which would silently retarget
// every command, including the spawned server, to a different tmux.
//
// When the one-time direct-spawn probe succeeds, tmux.exe is executed
// directly and the bash wrapper is skipped entirely — wrapping doubles the
// process creation cost of every command.
func tmuxCommand(ctx context.Context, binary string, args []string) *exec.Cmd {
	var cmd *exec.Cmd
	if bash := msysBash(binary); bash != "" && !directSpawnSupported(binary) {
		parts := make([]string, 0, len(args)+1)
		parts = append(parts, shellQuote(filepath.ToSlash(binary)))
		for _, arg := range args {
			parts = append(parts, shellQuote(arg))
		}
		script := "exec " + strings.Join(parts, " ")
		cmd = exec.CommandContext(ctx, bash, "--noprofile", "--norc", "-c", script)
	} else {
		cmd = exec.CommandContext(ctx, binary, args...)
	}
	// Never flash a console window for background tmux/bash spawns.
	cmd.SysProcAttr = &windows.SysProcAttr{HideWindow: true, CreationFlags: windows.CREATE_NO_WINDOW}
	return cmd
}

// Direct-spawn probe state: resolved once per process, safe for concurrent use.
var (
	spawnProbeOnce sync.Once
	directSpawnOK  bool
)

// directSpawnSupported reports whether tmux.exe can be executed directly,
// without the MSYS2 bash wrapper. It runs two one-time startup probes with a
// short timeout and caches the result:
//
//  1. `<tmux> -V` — is the binary directly executable at all?
//  2. `<tmux> '#{…}'` — do braces survive a direct spawn? Launching an MSYS2
//     tmux.exe natively strips braces from arguments (#{session_name} becomes
//     #session_name), which silently breaks every format query. An invalid
//     command name is rejected client-side without touching a server, and the
//     error echoes it back, making it a cheap preservation check.
//
// If either probe fails, callers keep the bash wrapper path so environments
// that genuinely need it are unaffected.
func directSpawnSupported(binary string) bool {
	spawnProbeOnce.Do(func() {
		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		// Build the commands directly (not via tmuxCommand) to avoid
		// re-entering this probe.
		version := exec.CommandContext(ctx, binary, "-V")
		version.SysProcAttr = &windows.SysProcAttr{HideWindow: true, CreationFlags: windows.CREATE_NO_WINDOW}
		if err := version.Run(); err != nil {
			return // not directly executable; keep the bash wrapper
		}
		const probeArg = "#{webtmux-direct-spawn-probe}"
		braces := exec.CommandContext(ctx, binary, probeArg)
		braces.SysProcAttr = &windows.SysProcAttr{HideWindow: true, CreationFlags: windows.CREATE_NO_WINDOW}
		out, _ := braces.CombinedOutput()
		directSpawnOK = strings.Contains(string(out), probeArg)
	})
	return directSpawnOK
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
