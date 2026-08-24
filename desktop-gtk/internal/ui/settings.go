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
	a.window.SetSettingsPage(a.buildSettingsPage())
	a.window.ShowSettings()
}

func (a *App) buildSettingsPage() gtk.Widgetter {
	page := adw.NewPreferencesPage()

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

	terminalGroup := adw.NewPreferencesGroup()
	terminalGroup.SetTitle("Terminal")
	terminalGroup.SetDescription("Terminal colors and tmux connection settings.")

	termLabels := make([]string, len(TerminalThemes)+1)
	termLabels[0] = "Follow interface theme"
	selectedTerminal := 0
	for i, terminalTheme := range TerminalThemes {
		termLabels[i+1] = terminalTheme.Label
		if a.settings.TerminalTheme != nil && terminalTheme.Name == *a.settings.TerminalTheme {
			selectedTerminal = i + 1
		}
	}
	termDropDown := gtk.NewDropDownFromStrings(termLabels)
	termDropDown.SetSelected(uint(selectedTerminal))
	termDropDown.SetVAlign(gtk.AlignCenter)
	termRow := adw.NewActionRow()
	termRow.SetTitle("Terminal colors")
	termRow.SetSubtitle("Use a dedicated terminal palette or follow the UI theme")
	termRow.AddSuffix(termDropDown)
	termRow.SetActivatableWidget(termDropDown)
	termDropDown.Object.NotifyProperty("selected", func() {
		idx := int(termDropDown.Selected())
		s := a.settings
		if idx == 0 {
			s.TerminalTheme = nil
		} else if idx-1 >= 0 && idx-1 < len(TerminalThemes) {
			name := TerminalThemes[idx-1].Name
			s.TerminalTheme = &name
		} else {
			return
		}
		a.applySettings(s)
	})
	terminalGroup.Add(termRow)

	font := gtk.NewEntry()
	font.SetText(a.settings.FontFamily)
	font.SetPlaceholderText("JetBrains Mono, Menlo, Consolas, monospace")
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

	tmuxBinary := gtk.NewEntry()
	tmuxBinary.SetText(a.settings.TmuxBinary)
	tmuxBinary.SetPlaceholderText(`C:\path\to\tmux.exe or tmux`)
	tmuxBinary.SetWidthChars(28)
	tmuxBinary.SetVAlign(gtk.AlignCenter)
	tmuxApply := gtk.NewButtonWithLabel("Apply")
	tmuxApply.SetVAlign(gtk.AlignCenter)
	tmuxRow := adw.NewActionRow()
	tmuxRow.SetTitle("tmux binary")
	tmuxRow.SetSubtitle("Select the exact tmux installation that owns your sessions")
	tmuxRow.AddSuffix(tmuxBinary)
	tmuxRow.AddSuffix(tmuxApply)
	tmuxRow.SetActivatableWidget(tmuxBinary)
	tmuxApply.ConnectClicked(func() { a.applyTmuxBinary(tmuxBinary.Text()) })
	tmuxBinary.ConnectActivate(func() { a.applyTmuxBinary(tmuxBinary.Text()) })
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
