package ui

import (
	"context"
	"fmt"
	"strings"
	"time"

	"github.com/diamondburned/gotk4-adwaita/pkg/adw"
	"github.com/diamondburned/gotk4/pkg/gtk/v4"

	"tmux-gui/desktop-gtk/internal/config"
)

// showSettings switches the main workspace to the native libadwaita settings
// page. Settings intentionally use PreferencesPage/PreferencesGroup/ActionRow
// like wa-bot, so the page has the same spacing, hierarchy and interaction
// model as the rest of a modern GTK application.
func (a *App) showSettings() {
	if a.window == nil {
		return
	}
	for _, view := range a.views {
		view.SetLayoutActive(false)
	}
	a.window.ShowSettings()
}

func (a *App) buildSettingsPage() gtk.Widgetter {
	page := adw.NewPreferencesPage()
	page.SetHExpand(true)
	page.SetVExpand(true)
	page.SetSizeRequest(1, 1)

	appearance := adw.NewPreferencesGroup()
	appearance.SetTitle("Appearance")
	appearance.SetDescription("Change the colors used by the entire application.")

	themeLabels := make([]string, len(Themes))
	selectedTheme := 0
	for i, theme := range Themes {
		variant := "light"
		if luminance(theme.Colors.Background) < 0.5 {
			variant = "dark"
		}
		themeLabels[i] = fmt.Sprintf("%s · %s", theme.Label, variant)
		if theme.Name == a.settings.UITheme {
			selectedTheme = i
		}
	}
	themeDropDown := gtk.NewDropDownFromStrings(themeLabels)
	themeDropDown.SetSelected(uint(selectedTheme))
	themeDropDown.SetVAlign(gtk.AlignCenter)
	themeRow := adw.NewActionRow()
	themeRow.SetTitle("Color theme")
	themeRow.SetSubtitle("Apply one palette to the sidebar, header, settings and terminal")
	themeRow.AddSuffix(themeDropDown)
	themeRow.SetActivatableWidget(themeDropDown)
	themeDropDown.Object.NotifyProperty("selected", func() {
		idx := int(themeDropDown.Selected())
		if idx < 0 || idx >= len(Themes) || Themes[idx].Name == a.settings.UITheme {
			return
		}
		s := a.settings
		s.UITheme = Themes[idx].Name
		a.applySettings(s)
	})
	appearance.Add(themeRow)

	page.Add(appearance)

	// The app and the terminal share one theme on purpose: the UI preset's
	// TerminalTheme mapping recolors both chrome and ANSI palette together.
	terminalGroup := adw.NewPreferencesGroup()
	terminalGroup.SetTitle("Terminal")
	terminalGroup.SetDescription("Fonts and tmux connection settings. Terminal colors follow the color theme.")

	font := gtk.NewEntry()
	font.SetText(a.settings.FontFamily)
	font.SetPlaceholderText(config.DefaultFontFamily)
	font.SetWidthChars(26)
	font.SetVAlign(gtk.AlignCenter)
	fontRow := adw.NewActionRow()
	fontRow.SetTitle("Font family")
	fontRow.SetSubtitle("Font used inside every terminal pane")
	fontRow.AddSuffix(font)
	fontRow.SetActivatableWidget(font)
	font.ConnectActivate(func() { a.applyFontSettings(font, nil, nil, nil) })
	terminalGroup.Add(fontRow)

	fontSize := gtk.NewSpinButtonWithRange(8, 32, 1)
	fontSize.SetValue(a.settings.FontSize)
	fontSize.SetDigits(0)
	fontSize.SetVAlign(gtk.AlignCenter)
	fontSize.SetWidthChars(5)
	fontSizeRow := adw.NewActionRow()
	fontSizeRow.SetTitle("Font size")
	fontSizeRow.SetSubtitle("Terminal text size in points")
	fontSizeRow.AddSuffix(fontSize)
	fontSizeRow.SetActivatableWidget(fontSize)
	fontSize.ConnectValueChanged(func() { a.applyFontSettings(font, fontSize, nil, nil) })
	terminalGroup.Add(fontSizeRow)

	lineHeight := gtk.NewSpinButtonWithRange(1, 2.2, .05)
	lineHeight.SetDigits(2)
	lineHeight.SetValue(a.settings.LineHeight)
	lineHeight.SetVAlign(gtk.AlignCenter)
	lineHeight.SetWidthChars(5)
	lineHeightRow := adw.NewActionRow()
	lineHeightRow.SetTitle("Line height")
	lineHeightRow.SetSubtitle("Vertical spacing between terminal rows")
	lineHeightRow.AddSuffix(lineHeight)
	lineHeightRow.SetActivatableWidget(lineHeight)
	lineHeight.ConnectValueChanged(func() { a.applyFontSettings(font, fontSize, lineHeight, nil) })
	terminalGroup.Add(lineHeightRow)

	scrollback := gtk.NewSpinButtonWithRange(100, 100000, 100)
	scrollback.SetValue(float64(a.settings.ScrollbackLines))
	scrollback.SetDigits(0)
	scrollback.SetVAlign(gtk.AlignCenter)
	scrollback.SetWidthChars(7)
	scrollbackRow := adw.NewActionRow()
	scrollbackRow.SetTitle("Scrollback lines")
	scrollbackRow.SetSubtitle("Number of terminal lines kept in memory")
	scrollbackRow.AddSuffix(scrollback)
	scrollbackRow.SetActivatableWidget(scrollback)
	scrollback.ConnectValueChanged(func() {
		s := a.settings
		s.ScrollbackLines = scrollback.ValueAsInt()
		a.applySettings(s)
	})
	terminalGroup.Add(scrollbackRow)

	// The tmux installation is picked with the OS file chooser instead of a
	// free-form entry, so the saved path is always a real executable.
	tmuxRow := adw.NewActionRow()
	tmuxRow.SetTitle("tmux binary")
	tmuxRow.SetSubtitle(binarySubtitle(a.settings.TmuxBinary))
	browse := gtk.NewButtonWithLabel("Browse…")
	browse.SetVAlign(gtk.AlignCenter)
	tmuxRow.AddSuffix(browse)
	reset := iconButton("view-refresh-symbolic", "Back to PATH lookup", func() {
		a.applyTmuxBinary("")
		tmuxRow.SetSubtitle(binarySubtitle(""))
	})
	tmuxRow.AddSuffix(reset)
	tmuxRow.SetActivatableWidget(browse)
	browse.ConnectClicked(func() {
		if a.window == nil {
			return
		}
		parent := (*gtk.Window)(&a.window.AdwWin.Window)
		chooser := gtk.NewFileChooserNative("Select tmux.exe", parent, gtk.FileChooserActionOpen, "Select", "Cancel")
		chooser.ConnectResponse(func(response int) {
			if response != int(gtk.ResponseAccept) {
				return
			}
			file := chooser.File()
			if file == nil {
				return
			}
			path := file.Path()
			if strings.TrimSpace(path) == "" {
				return
			}
			tmuxRow.SetSubtitle(path)
			a.applyTmuxBinary(path)
		})
		chooser.Show()
	})
	terminalGroup.Add(tmuxRow)

	page.Add(terminalGroup)

	behavior := adw.NewPreferencesGroup()
	behavior.SetTitle("Safety")
	behavior.SetDescription("Ask before destructive tmux operations.")
	behavior.Add(settingsSwitchRow("Confirm killing pane", "Ask before closing a pane", a.settings.ConfirmKillPane, func(v bool) {
		s := a.settings
		s.ConfirmKillPane = v
		a.applySettings(s)
	}))
	behavior.Add(settingsSwitchRow("Confirm killing window", "Ask before closing a window and its panes", a.settings.ConfirmKillWindow, func(v bool) {
		s := a.settings
		s.ConfirmKillWindow = v
		a.applySettings(s)
	}))
	behavior.Add(settingsSwitchRow("Confirm killing session", "Ask before terminating the whole session", a.settings.ConfirmKillSession, func(v bool) {
		s := a.settings
		s.ConfirmKillSession = v
		a.applySettings(s)
	}))
	page.Add(behavior)

	about := adw.NewPreferencesGroup()
	about.SetTitle("About")
	about.SetDescription("Native GTK desktop client")
	about.Add(settingsInfoRow("Application", fmt.Sprintf("%s · v%s", a.cfg.Name, a.cfg.Version)))
	about.Add(settingsInfoRow("Renderer", "GTK4 · libadwaita · libvterm"))
	about.Add(settingsInfoRow("Backend port", fmt.Sprintf("%d", a.port)))
	page.Add(about)

	return page
}

func settingsSwitchRow(title, subtitle string, active bool, changed func(bool)) *adw.ActionRow {
	switcher := gtk.NewSwitch()
	switcher.SetActive(active)
	switcher.SetVAlign(gtk.AlignCenter)
	row := adw.NewActionRow()
	row.SetTitle(title)
	row.SetSubtitle(subtitle)
	row.AddSuffix(switcher)
	row.SetActivatableWidget(switcher)
	switcher.ConnectStateSet(func(state bool) bool {
		changed(state)
		return false
	})
	return row
}

func settingsInfoRow(title, subtitle string) *adw.ActionRow {
	row := adw.NewActionRow()
	row.SetTitle(title)
	row.SetSubtitle(subtitle)
	return row
}

// binarySubtitle describes the active tmux executable: the pinned path, or a
// note that the backend falls back to PATH lookup.
func binarySubtitle(path string) string {
	path = strings.TrimSpace(path)
	if path == "" {
		return "Looked up from PATH — browse to pin a specific tmux.exe"
	}
	return path
}

func (a *App) applySettings(s config.Settings) {
	a.settings = s
	ApplyTheme(s.UITheme)
	for _, view := range a.views {
		view.ApplySettings(s)
	}
	if err := config.Save(a.cfg.UserDataDir, s); err != nil {
		a.toast(err.Error())
	}
}

func (a *App) applyFontSettings(font *gtk.Entry, fontSize, lineHeight, _ *gtk.SpinButton) {
	s := a.settings
	s.FontFamily = strings.TrimSpace(font.Text())
	if fontSize != nil {
		s.FontSize = fontSize.Value()
	}
	if lineHeight != nil {
		s.LineHeight = lineHeight.Value()
	}
	a.applySettings(s)
}

func (a *App) applyTmuxBinary(path string) {
	path = strings.TrimSpace(path)
	s := a.settings
	s.TmuxBinary = path
	if a.client == nil {
		a.applySettings(s)
		a.toast("tmux binary saved; backend is not connected yet")
		return
	}
	go func() {
		ctx, cancel := context.WithTimeout(a.ctx, 5*time.Second)
		defer cancel()
		if _, err := a.client.SetTmuxBinary(ctx, path); err != nil {
			a.idle(func() { a.toast("tmux binary: " + err.Error()) })
			return
		}
		a.applySettings(s)
		a.idle(func() { a.toast("tmux binary applied") })
	}()
}
