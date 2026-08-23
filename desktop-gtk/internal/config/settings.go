package config

import (
	"encoding/json"
	"os"
	"path/filepath"
)

type Settings struct {
	UITheme            string          `json:"uiTheme"`
	TerminalTheme      *string         `json:"terminalTheme,omitempty"`
	FontFamily         string          `json:"fontFamily"`
	FontSize           float64         `json:"fontSize"`
	LineHeight         float64         `json:"lineHeight"`
	ScrollbackLines    int             `json:"scrollbackLines"`
	TUIScrollPanes     map[string]bool `json:"tuiScrollPanes"`
	ConfirmKillPane    bool            `json:"confirmKillPane"`
	ConfirmKillWindow  bool            `json:"confirmKillWindow"`
	ConfirmKillSession bool            `json:"confirmKillSession"`
}

func Defaults() Settings {
	return Settings{
		UITheme: "default-dark", FontFamily: "JetBrains Mono, Menlo, Consolas, monospace",
		FontSize: 14, LineHeight: 1.35, ScrollbackLines: 2000,
		TUIScrollPanes: map[string]bool{}, ConfirmKillPane: true,
		ConfirmKillWindow: true, ConfirmKillSession: true,
	}
}

func Load(dir string) Settings {
	s := Defaults()
	b, err := os.ReadFile(filepath.Join(dir, "settings.json"))
	if err == nil {
		_ = json.Unmarshal(b, &s)
	}
	if s.UITheme == "" {
		s.UITheme = "default-dark"
	}
	if s.FontFamily == "" {
		s.FontFamily = Defaults().FontFamily
	}
	if s.FontSize <= 0 {
		s.FontSize = 14
	}
	if s.LineHeight <= 0 {
		s.LineHeight = 1.35
	}
	if s.ScrollbackLines <= 0 {
		s.ScrollbackLines = 2000
	}
	if s.TUIScrollPanes == nil {
		s.TUIScrollPanes = map[string]bool{}
	}
	return s
}

func Save(dir string, s Settings) error {
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}
	b, err := json.MarshalIndent(s, "", "  ")
	if err != nil {
		return err
	}
	tmp := filepath.Join(dir, "settings.json.tmp")
	if err := os.WriteFile(tmp, b, 0o644); err != nil {
		return err
	}
	return os.Rename(tmp, filepath.Join(dir, "settings.json"))
}
