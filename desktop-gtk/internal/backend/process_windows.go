//go:build windows

package backend

import (
	"os"
	"os/exec"
)

func applyPlatformSysProcAttr(cmd *exec.Cmd) {}

func signalStop(pid int) error {
	p, err := os.FindProcess(pid)
	if err != nil {
		return err
	}
	return p.Kill()
}
