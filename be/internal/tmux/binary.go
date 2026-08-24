package tmux

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"strings"
	"time"
)

// BinaryPath reports the tmux executable currently used by new commands.
// An explicit TMUXGUI_TMUX_BIN value wins; otherwise the platform resolver
// follows PATH.
func BinaryPath() string { return tmuxBinary() }

// SetBinary validates and selects a tmux executable for this backend process.
// An empty path clears the override and restores automatic PATH resolution.
func SetBinary(path string) error {
	path = strings.TrimSpace(path)
	if path == "" {
		return os.Unsetenv("TMUXGUI_TMUX_BIN")
	}

	resolved := path
	if !strings.ContainsAny(path, `/\\`) {
		if found, err := exec.LookPath(path); err == nil {
			resolved = found
		}
	}

	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, resolved, "-V")
	if output, err := cmd.CombinedOutput(); err != nil {
		message := strings.TrimSpace(string(output))
		if message != "" {
			return fmt.Errorf("tmux binary %q is not usable: %s", path, message)
		}
		return fmt.Errorf("tmux binary %q is not usable: %w", path, err)
	}

	return os.Setenv("TMUXGUI_TMUX_BIN", resolved)
}
