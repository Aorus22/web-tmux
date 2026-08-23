package ui

import (
	"context"
	"fmt"
	"strings"
	"time"

	"github.com/diamondburned/gotk4/pkg/gtk/v4"

	"tmux-gui/desktop-gtk/internal/config"
	"tmux-gui/desktop-gtk/internal/protocol"
)

func (a *App) showNewSessionDialog() {
	if a.window == nil {
		return
	}
	dialog := a.dialog("New tmux session", 500, 300)
	name := formEntry("Session name", "")
	cwd := formEntry("Working directory (optional)", "")
	command := formEntry("Initial command (optional)", "")
	content := dialog.ContentArea()
	content.Append(formRow("Name", name))
	content.Append(formRow("Directory", cwd))
	content.Append(formRow("Command", command))
	dialog.AddButton("Cancel", int(gtk.ResponseCancel))
	dialog.AddButton("Create", int(gtk.ResponseOK))
	dialog.SetDefaultResponse(int(gtk.ResponseOK))
	dialog.ConnectResponse(func(response int) {
		if response != int(gtk.ResponseOK) {
			dialog.Close()
			return
		}
		session := strings.TrimSpace(name.Text())
		if session == "" {
			a.toast("Session name is required")
			return
		}
		dialog.Close()
		if a.client == nil {
			a.toast("Backend is not ready")
			return
		}
		go func() {
			ctx, cancel := context.WithTimeout(a.ctx, 15*time.Second)
			defer cancel()
			if err := a.client.CreateSession(ctx, session, strings.TrimSpace(cwd.Text()), strings.TrimSpace(command.Text())); err != nil {
				a.idle(func() { a.toast(err.Error()) })
				return
			}
			a.idle(func() {
				a.toast("Session " + session + " created")
				a.open(session, "", "")
			})
		}()
	})
	name.ConnectActivate(func() { dialog.Response(int(gtk.ResponseOK)) })
	dialog.Present()
}

func (a *App) showSettings() {
	if a.window == nil {
		return
	}
	dialog := a.dialog("Settings", 620, 600)
	content := dialog.ContentArea()

	themeLabels := make([]string, len(Themes))
	selectedTheme := 0
	for i, theme := range Themes {
		themeLabels[i] = theme.Label
		if theme.Name == a.settings.UITheme {
			selectedTheme = i
		}
	}
	theme := gtk.NewDropDownFromStrings(themeLabels)
	theme.SetSelected(uint(selectedTheme))
	content.Append(formRow("Interface theme", theme))

	termLabels := make([]string, len(TerminalThemes)+1)
	termLabels[0] = "Follow interface theme"
	selectedTerminal := 0
	for i, terminalTheme := range TerminalThemes {
		termLabels[i+1] = terminalTheme.Label
		if a.settings.TerminalTheme != nil && terminalTheme.Name == *a.settings.TerminalTheme {
			selectedTerminal = i + 1
		}
	}
	termTheme := gtk.NewDropDownFromStrings(termLabels)
	termTheme.SetSelected(uint(selectedTerminal))
	content.Append(formRow("Terminal colors", termTheme))

	font := formEntry("Font family", a.settings.FontFamily)
	font.SetText(a.settings.FontFamily)
	content.Append(formRow("Font family", font))
	fontSize := gtk.NewSpinButtonWithRange(8, 32, 1)
	fontSize.SetValue(a.settings.FontSize)
	content.Append(formRow("Font size", fontSize))
	lineHeight := gtk.NewSpinButtonWithRange(1, 2.2, .05)
	lineHeight.SetDigits(2)
	lineHeight.SetValue(a.settings.LineHeight)
	content.Append(formRow("Line height", lineHeight))
	scrollback := gtk.NewSpinButtonWithRange(100, 100000, 100)
	scrollback.SetValue(float64(a.settings.ScrollbackLines))
	content.Append(formRow("Scrollback lines", scrollback))

	killPane := gtk.NewSwitch()
	killPane.SetActive(a.settings.ConfirmKillPane)
	content.Append(formRow("Confirm killing pane", killPane))
	killWindow := gtk.NewSwitch()
	killWindow.SetActive(a.settings.ConfirmKillWindow)
	content.Append(formRow("Confirm killing window", killWindow))
	killSession := gtk.NewSwitch()
	killSession.SetActive(a.settings.ConfirmKillSession)
	content.Append(formRow("Confirm killing session", killSession))

	dialog.AddButton("Cancel", int(gtk.ResponseCancel))
	dialog.AddButton("Apply", int(gtk.ResponseOK))
	dialog.ConnectResponse(func(response int) {
		if response == int(gtk.ResponseOK) {
			s := a.settings
			if idx := int(theme.Selected()); idx >= 0 && idx < len(Themes) {
				s.UITheme = Themes[idx].Name
			}
			idx := int(termTheme.Selected())
			if idx == 0 {
				s.TerminalTheme = nil
			} else if idx-1 < len(TerminalThemes) {
				name := TerminalThemes[idx-1].Name
				s.TerminalTheme = &name
			}
			s.FontFamily = strings.TrimSpace(font.Text())
			s.FontSize = fontSize.Value()
			s.LineHeight = lineHeight.Value()
			s.ScrollbackLines = scrollback.ValueAsInt()
			s.ConfirmKillPane = killPane.Active()
			s.ConfirmKillWindow = killWindow.Active()
			s.ConfirmKillSession = killSession.Active()
			a.settings = s
			ApplyTheme(s.UITheme)
			for _, view := range a.views {
				view.ApplySettings(s)
			}
			if err := config.Save(a.cfg.UserDataDir, s); err != nil {
				a.toast(err.Error())
			} else {
				a.toast("Settings saved")
			}
		}
		dialog.Close()
	})
	dialog.Present()
}

type paletteCommand struct {
	title, subtitle string
	run             func()
}

func (a *App) showCommandPalette() {
	if a.window == nil {
		return
	}
	dialog := a.dialog("Command palette", 620, 520)
	dialog.AddCSSClass("command-palette")
	search := gtk.NewSearchEntry()
	search.SetPlaceholderText("Type a command…")
	list := gtk.NewListBox()
	list.AddCSSClass("boxed-list")
	scroll := gtk.NewScrolledWindow()
	scroll.SetVExpand(true)
	scroll.SetChild(list)
	dialog.ContentArea().Append(search)
	dialog.ContentArea().Append(scroll)
	dialog.AddButton("Close", int(gtk.ResponseClose))

	commands := a.paletteCommands()
	visible := commands
	rebuild := func() {
		list.RemoveAll()
		visible = visible[:0]
		query := strings.ToLower(strings.TrimSpace(search.Text()))
		for _, command := range commands {
			if query != "" && !strings.Contains(strings.ToLower(command.title+" "+command.subtitle), query) {
				continue
			}
			visible = append(visible, command)
			row := gtk.NewListBoxRow()
			row.SetActivatable(true)
			box := gtk.NewBox(gtk.OrientationVertical, 2)
			box.SetMarginStart(12)
			box.SetMarginEnd(12)
			box.SetMarginTop(8)
			box.SetMarginBottom(8)
			title := gtk.NewLabel(command.title)
			title.SetHAlign(gtk.AlignStart)
			box.Append(title)
			subtitle := gtk.NewLabel(command.subtitle)
			subtitle.SetHAlign(gtk.AlignStart)
			subtitle.AddCSSClass("dim-label")
			box.Append(subtitle)
			row.SetChild(box)
			list.Append(row)
		}
	}
	rebuild()
	search.ConnectSearchChanged(rebuild)
	list.ConnectRowActivated(func(row *gtk.ListBoxRow) {
		idx := row.Index()
		if idx >= 0 && idx < len(visible) {
			run := visible[idx].run
			dialog.Close()
			run()
		}
	})
	dialog.ConnectResponse(func(_ int) { dialog.Close() })
	dialog.Present()
	search.GrabFocus()
}

func (a *App) paletteCommands() []paletteCommand {
	commands := []paletteCommand{
		{"New session", "Create a tmux session · Ctrl+Shift+N", a.showNewSessionDialog},
		{"New window", "Create a window · Ctrl+N", func() { a.runToolbarCommand(protocol.WindowCreate, "") }},
		{"Split pane left/right", "Create a vertical divider · Ctrl+D", func() { a.runToolbarCommand(protocol.PaneSplit, "vertical") }},
		{"Split pane top/bottom", "Create a horizontal divider · Ctrl+Shift+D", func() { a.runToolbarCommand(protocol.PaneSplit, "horizontal") }},
		{"Zoom pane", "Toggle the active pane", func() { a.runToolbarCommand(protocol.PaneZoom, "") }},
		{"Break pane", "Move active pane to a new window", func() { a.runToolbarCommand(protocol.PaneBreak, "") }},
		{"Break active pane", "Move it into a named window", func() { a.runToolbarCommand(protocol.WindowBreakActive, "") }},
		{"Move window left", "Reorder current window", func() { a.sendWindowAmount(protocol.WindowMove, -1) }},
		{"Move window right", "Reorder current window", func() { a.sendWindowAmount(protocol.WindowMove, 1) }},
		{"Copy selection", "Copy from the active terminal", func() {
			if v := a.activeView(); v != nil {
				v.CopyActive()
			}
		}},
		{"Paste", "Paste clipboard into the active terminal", func() {
			if v := a.activeView(); v != nil {
				v.PasteActive()
			}
		}},
		{"Toggle TUI scroll", "Send wheel gestures as Page Up/Down", a.toggleTUIScroll},
		{"Rename pane", "Set the pane title", a.renamePane},
		{"Rename window", "Rename the active window", a.renameWindow},
		{"Rename session", "Rename the active session", a.renameSession},
		{"Swap pane", "Swap the active pane with another pane", a.swapPane},
		{"Kill pane", "Close the active pane", a.killPane},
		{"Kill window", "Close the active window", a.killWindow},
		{"Kill session", "Terminate the active session", a.killSession},
		{"Settings", "Themes, fonts and confirmations · Ctrl+,", a.showSettings},
	}
	for _, layout := range []string{"even-horizontal", "even-vertical", "main-horizontal", "main-vertical", "tiled"} {
		name := layout
		commands = append(commands, paletteCommand{"Layout: " + name, "Apply tmux layout", func() { a.runToolbarCommand(protocol.WindowLayout, name) }})
	}
	return commands
}

func (a *App) renamePane() {
	a.prompt("Rename pane", "Pane title", "", func(value string) {
		if v := a.activeView(); v != nil {
			a.send(a.active, protocol.Incoming{Type: protocol.PaneRename, PaneID: v.ActivePaneID(), Title: value})
		}
	})
}
func (a *App) renameWindow() {
	a.prompt("Rename window", "Window name", "", func(value string) {
		if v := a.activeView(); v != nil {
			a.send(a.active, protocol.Incoming{Type: protocol.WindowRename, PaneID: v.ActiveWindowID(), Name: value})
		}
	})
}
func (a *App) renameSession() {
	a.prompt("Rename session", "New session name", a.active, func(value string) {
		a.send(a.active, protocol.Incoming{Type: protocol.SessionRename, NewName: value})
	})
}
func (a *App) swapPane() {
	v := a.activeView()
	if v == nil || len(v.OtherPaneIDs()) == 0 {
		a.toast("No other pane to swap with")
		return
	}
	ids := strings.Join(v.OtherPaneIDs(), ", ")
	a.prompt("Swap pane", "Target pane ID ("+ids+")", v.OtherPaneIDs()[0], func(value string) {
		a.send(a.active, protocol.Incoming{Type: protocol.PaneSwap, PaneID: v.ActivePaneID(), OtherPaneID: value})
	})
}

func (a *App) killPane() {
	v := a.activeView()
	if v == nil {
		return
	}
	a.confirmIf(a.settings.ConfirmKillPane, "Kill pane?", "The process in this pane will be terminated.", func() { a.send(a.active, protocol.Incoming{Type: protocol.PaneKill, PaneID: v.ActivePaneID()}) })
}
func (a *App) killWindow() {
	v := a.activeView()
	if v == nil {
		return
	}
	a.confirmIf(a.settings.ConfirmKillWindow, "Kill window?", "All panes in this window will be terminated.", func() { a.send(a.active, protocol.Incoming{Type: protocol.WindowKill, PaneID: v.ActiveWindowID()}) })
}
func (a *App) killSession() {
	session := a.active
	if session == "" {
		return
	}
	a.confirmIf(a.settings.ConfirmKillSession, "Kill session?", "Every window and process in "+session+" will be terminated.", func() { a.send(session, protocol.Incoming{Type: protocol.SessionKill, Session: session}) })
}

func (a *App) sendWindowAmount(command string, amount int) {
	if v := a.activeView(); v != nil {
		a.send(a.active, protocol.Incoming{Type: command, PaneID: v.ActiveWindowID(), Amount: amount})
	}
}
func (a *App) toggleTUIScroll() {
	if v := a.activeView(); v != nil {
		enabled := v.ToggleTUIScroll()
		a.settings.TUIScrollPanes[v.ActivePaneID()] = enabled
		_ = config.Save(a.cfg.UserDataDir, a.settings)
		a.toast(fmt.Sprintf("TUI scroll %s", map[bool]string{true: "enabled", false: "disabled"}[enabled]))
	}
}

func (a *App) prompt(title, label, initial string, accepted func(string)) {
	dialog := a.dialog(title, 480, 190)
	entry := formEntry(label, "")
	entry.SetText(initial)
	dialog.ContentArea().Append(formRow(label, entry))
	dialog.AddButton("Cancel", int(gtk.ResponseCancel))
	dialog.AddButton("Apply", int(gtk.ResponseOK))
	dialog.ConnectResponse(func(response int) {
		if response == int(gtk.ResponseOK) {
			value := strings.TrimSpace(entry.Text())
			if value == "" {
				a.toast(label + " is required")
				return
			}
			dialog.Close()
			accepted(value)
			return
		}
		dialog.Close()
	})
	entry.ConnectActivate(func() { dialog.Response(int(gtk.ResponseOK)) })
	dialog.Present()
	entry.GrabFocus()
}

func (a *App) confirmIf(enabled bool, title, body string, confirmed func()) {
	if !enabled {
		confirmed()
		return
	}
	dialog := a.dialog(title, 460, 200)
	label := gtk.NewLabel(body)
	label.SetWrap(true)
	label.SetHAlign(gtk.AlignStart)
	dialog.ContentArea().Append(label)
	dialog.AddButton("Cancel", int(gtk.ResponseCancel))
	dialog.AddButton("Continue", int(gtk.ResponseOK))
	dialog.ConnectResponse(func(response int) {
		dialog.Close()
		if response == int(gtk.ResponseOK) {
			confirmed()
		}
	})
	dialog.Present()
}

func (a *App) dialog(title string, width, height int) *gtk.Dialog {
	dialog := gtk.NewDialog()
	dialog.SetTitle(title)
	dialog.SetDefaultSize(width, height)
	dialog.SetModal(true)
	dialog.SetTransientFor((*gtk.Window)(&a.window.AdwWin.Window))
	content := dialog.ContentArea()
	content.SetSpacing(12)
	content.SetMarginStart(18)
	content.SetMarginEnd(18)
	content.SetMarginTop(18)
	content.SetMarginBottom(12)
	return dialog
}

func formEntry(placeholder, initial string) *gtk.Entry {
	entry := gtk.NewEntry()
	entry.SetPlaceholderText(placeholder)
	entry.SetText(initial)
	entry.SetHExpand(true)
	return entry
}

func formRow(label string, widget gtk.Widgetter) *gtk.Box {
	row := gtk.NewBox(gtk.OrientationHorizontal, 12)
	l := gtk.NewLabel(label)
	l.SetHAlign(gtk.AlignStart)
	l.SetSizeRequest(160, -1)
	row.Append(l)
	gtk.BaseWidget(widget).SetHExpand(true)
	row.Append(widget)
	return row
}
