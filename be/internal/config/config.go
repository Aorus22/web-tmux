package config

import (
	"fmt"
	"os"
	"strconv"
	"strings"
)

// Config holds all backend configuration, sourced from environment variables.
type Config struct {
	Host            string
	Port            int
	TmuxSocket      string // socket name (-L) or absolute path (-S)
	LogLevel        string
	ScrollbackLines int
}

func getenv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

func getenvInt(key string, fallback int) (int, error) {
	v := os.Getenv(key)
	if v == "" {
		return fallback, nil
	}
	n, err := strconv.Atoi(strings.TrimSpace(v))
	if err != nil {
		return 0, fmt.Errorf("%s must be an integer, got %q", key, v)
	}
	return n, nil
}

// Load reads configuration from the environment.
func Load() (*Config, error) {
	port, err := getenvInt("TMUXGUI_PORT", 14101)
	if err != nil {
		return nil, err
	}
	if port < 0 || port > 65535 {
		return nil, fmt.Errorf("TMUXGUI_PORT out of range: %d", port)
	}

	scrollback, err := getenvInt("TMUXGUI_SCROLLBACK_LINES", 2000)
	if err != nil {
		return nil, err
	}
	if scrollback < 0 {
		scrollback = 0
	}

	return &Config{
		Host:            getenv("TMUXGUI_HOST", "127.0.0.1"),
		Port:            port,
		TmuxSocket:      os.Getenv("TMUXGUI_TMUX_SOCKET"),
		LogLevel:        strings.ToLower(getenv("TMUXGUI_LOG_LEVEL", "info")),
		ScrollbackLines: scrollback,
	}, nil
}

// Addr returns the host:port bind address.
func (c *Config) Addr() string {
	return fmt.Sprintf("%s:%d", c.Host, c.Port)
}
