package config

import "testing"

func TestLoadUsesPort4090ByDefault(t *testing.T) {
	t.Setenv("TMUXGUI_PORT", "")
	cfg, err := Load()
	if err != nil {
		t.Fatalf("Load() error = %v", err)
	}
	if cfg.Port != 4090 {
		t.Fatalf("default port = %d, want 4090", cfg.Port)
	}
}
