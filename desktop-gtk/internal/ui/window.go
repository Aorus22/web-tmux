package ui

import (
	"fmt"

	"github.com/diamondburned/gotk4-adwaita/pkg/adw"
	"github.com/diamondburned/gotk4/pkg/gtk/v4"

	"tmux-gui/desktop-gtk/internal/protocol"
)

type WindowCallbacks struct {
	Open                          func(session, windowID, paneID string)
	CloseTab                      func(session string)
	Command                       func(command, value string)
	NewSession, Palette, Settings func()
}

type treeTarget struct{ session, window, pane string }

type MainWindow struct {
	AdwWin       *adw.ApplicationWindow
	toastOverlay *adw.ToastOverlay
	sidebar      *gtk.ListBox
	treeTargets  []treeTarget
	sessionTabs  *gtk.Box
	windowTabs   *gtk.Box
	stack        *gtk.Stack
	settingsPage gtk.Widgetter
	chrome       []gtk.Widgetter
	status       *gtk.Label
	callbacks    WindowCallbacks
	active       string
}

func NewMainWindow(app *adw.Application, name, version string, callbacks WindowCallbacks) *MainWindow {
	w := &MainWindow{callbacks: callbacks}
	w.AdwWin = adw.NewApplicationWindow((*gtk.Application)(&app.Application))
	w.AdwWin.SetTitle(fmt.Sprintf("%s — v%s", name, version))
	w.AdwWin.SetDefaultSize(1280, 820)

	toolbarView := adw.NewToolbarView()
	header := adw.NewHeaderBar()
	title := adw.NewWindowTitle(name, "Native GTK · libvterm")
	header.SetTitleWidget(title)
	header.PackStart(iconButton("view-list-symbolic", "Command palette (Ctrl+Shift+P)", callbacks.Palette))
	header.PackEnd(iconButton("preferences-system-symbolic", "Settings (Ctrl+,)", callbacks.Settings))
	header.PackEnd(iconButton("list-add-symbolic", "New session (Ctrl+Shift+N)", callbacks.NewSession))
	toolbarView.AddTopBar(header)

	w.sidebar = gtk.NewListBox()
	w.sidebar.AddCSSClass("navigation-sidebar")
	w.sidebar.AddCSSClass("tmux-sidebar")
	w.sidebar.SetSelectionMode(gtk.SelectionSingle)
	w.sidebar.ConnectRowActivated(func(row *gtk.ListBoxRow) {
		idx := row.Index()
		if idx >= 0 && idx < len(w.treeTargets) {
			t := w.treeTargets[idx]
			w.callbacks.Open(t.session, t.window, t.pane)
		}
	})
	scroll := gtk.NewScrolledWindow()
	scroll.SetMinContentWidth(230)
	scroll.SetChild(w.sidebar)

	side := gtk.NewBox(gtk.OrientationVertical, 0)
	side.AddCSSClass("sidebar-pane")
	side.Append(sidebarHeading("SESSIONS"))
	side.Append(scroll)
	newSession := gtk.NewButtonWithLabel("New session")
	newSession.SetMarginStart(10)
	newSession.SetMarginEnd(10)
	newSession.SetMarginTop(8)
	newSession.SetMarginBottom(10)
	newSession.ConnectClicked(callbacks.NewSession)
	side.Append(newSession)
	settingsButton := gtk.NewButtonWithLabel("Settings")
	settingsButton.SetIconName("preferences-system-symbolic")
	settingsButton.SetTooltipText("Open settings")
	settingsButton.SetMarginStart(10)
	settingsButton.SetMarginEnd(10)
	settingsButton.SetMarginBottom(10)
	settingsButton.ConnectClicked(callbacks.Settings)
	side.Append(settingsButton)

	w.sessionTabs = gtk.NewBox(gtk.OrientationHorizontal, 4)
	w.sessionTabs.AddCSSClass("tmux-tabbar")
	w.windowTabs = gtk.NewBox(gtk.OrientationHorizontal, 4)
	w.windowTabs.AddCSSClass("tmux-tabbar")

	tools := gtk.NewBox(gtk.OrientationHorizontal, 4)
	tools.AddCSSClass("tmux-toolbar")
	tools.Append(iconButton("window-new-symbolic", "New window (Ctrl+N)", func() { callbacks.Command(protocol.WindowCreate, "") }))
	tools.Append(iconButton("view-split-left-right-symbolic", "Split left/right (Ctrl+D)", func() { callbacks.Command(protocol.PaneSplit, "vertical") }))
	tools.Append(iconButton("view-split-top-bottom-symbolic", "Split top/bottom (Ctrl+Shift+D)", func() { callbacks.Command(protocol.PaneSplit, "horizontal") }))
	tools.Append(iconButton("view-fullscreen-symbolic", "Zoom active pane", func() { callbacks.Command(protocol.PaneZoom, "") }))
	for _, layout := range []struct{ label, value string }{{"Even H", "even-horizontal"}, {"Even V", "even-vertical"}, {"Main H", "main-horizontal"}, {"Main V", "main-vertical"}, {"Tiled", "tiled"}} {
		l := layout
		b := gtk.NewButtonWithLabel(l.label)
		b.SetTooltipText("Apply " + l.value + " layout")
		b.ConnectClicked(func() { callbacks.Command(protocol.WindowLayout, l.value) })
		tools.Append(b)
	}

	w.stack = gtk.NewStack()
	w.stack.SetHExpand(true)
	w.stack.SetVExpand(true)
	w.stack.SetTransitionType(gtk.StackTransitionTypeCrossfade)
	empty := emptyPage("Open a tmux session", "Choose a session from the sidebar, or create a new one.")
	w.stack.AddNamed(empty, "__empty")
	w.stack.SetVisibleChildName("__empty")

	w.status = gtk.NewLabel("Starting backend…")
	w.status.SetHAlign(gtk.AlignStart)
	w.status.AddCSSClass("statusbar")
	w.status.SetEllipsize(3)

	content := gtk.NewBox(gtk.OrientationVertical, 0)
	content.AddCSSClass("content-pane")
	content.Append(w.sessionTabs)
	content.Append(w.windowTabs)
	content.Append(tools)
	content.Append(w.stack)
	content.Append(w.status)
	w.chrome = []gtk.Widgetter{w.sessionTabs, w.windowTabs, tools, w.status}

	paned := gtk.NewPaned(gtk.OrientationHorizontal)
	paned.SetStartChild(side)
	paned.SetEndChild(content)
	paned.SetPosition(240)
	paned.SetResizeStartChild(false)
	paned.SetShrinkStartChild(false)
	paned.SetResizeEndChild(true)

	w.toastOverlay = adw.NewToastOverlay()
	w.toastOverlay.SetChild(paned)
	toolbarView.SetContent(w.toastOverlay)
	w.AdwWin.SetContent(toolbarView)
	return w
}

func (w *MainWindow) Present() { w.AdwWin.Present() }
func (w *MainWindow) Toast(message string) {
	toast := adw.NewToast(message)
	toast.SetTimeout(4)
	w.toastOverlay.AddToast(toast)
}
func (w *MainWindow) SetStatus(status string) { w.status.SetText(status) }

// SetSettingsPage installs the settings view in the main stack. Replacing the
// widget keeps the page lifecycle simple while ensuring it is never a dialog.
func (w *MainWindow) SetSettingsPage(widget gtk.Widgetter) {
	if w.settingsPage != nil {
		w.stack.Remove(w.settingsPage)
	}
	w.settingsPage = widget
	if widget != nil {
		w.stack.AddNamed(widget, "__settings")
	}
}

func (w *MainWindow) ShowSettings() {
	if w.settingsPage != nil {
		w.setChromeVisible(false)
		w.stack.SetVisibleChildName("__settings")
	}
}

func (w *MainWindow) setChromeVisible(visible bool) {
	for _, widget := range w.chrome {
		gtk.BaseWidget(widget).SetVisible(visible)
	}
}

func (w *MainWindow) UpdateTree(tree protocol.Tree) {
	w.sidebar.RemoveAll()
	w.treeTargets = w.treeTargets[:0]
	for _, session := range tree.Sessions {
		w.appendTreeRow("utilities-terminal-symbolic", session.Session.Name, fmt.Sprintf("%d windows", session.Session.Windows), treeTarget{session: session.Session.Name}, 0)
		for _, window := range session.Windows {
			w.appendTreeRow("focus-windows-symbolic", fmt.Sprintf("%d: %s", window.Window.Index, window.Window.Name), fmt.Sprintf("%d panes", window.Window.Panes), treeTarget{session: session.Session.Name, window: window.Window.ID}, 18)
			for _, pane := range window.Panes {
				title := pane.Title
				if title == "" {
					title = pane.CurrentCommand
				}
				w.appendTreeRow("prompt-symbolic", fmt.Sprintf("%d · %s", pane.Index, title), pane.CurrentPath, treeTarget{session: session.Session.Name, window: window.Window.ID, pane: pane.ID}, 36)
			}
		}
	}
}

func (w *MainWindow) appendTreeRow(icon, title, subtitle string, target treeTarget, indent int) {
	row := gtk.NewListBoxRow()
	row.SetSelectable(true)
	row.SetActivatable(true)
	box := gtk.NewBox(gtk.OrientationHorizontal, 10)
	box.SetMarginStart(10 + indent)
	box.SetMarginEnd(8)
	box.SetMarginTop(5)
	box.SetMarginBottom(5)
	box.Append(gtk.NewImageFromIconName(icon))
	labels := gtk.NewBox(gtk.OrientationVertical, 1)
	t := gtk.NewLabel(title)
	t.SetHAlign(gtk.AlignStart)
	t.SetEllipsize(3)
	labels.Append(t)
	if subtitle != "" {
		s := gtk.NewLabel(subtitle)
		s.SetHAlign(gtk.AlignStart)
		s.SetEllipsize(3)
		s.AddCSSClass("dim-label")
		labels.Append(s)
	}
	box.Append(labels)
	row.SetChild(box)
	w.sidebar.Append(row)
	w.treeTargets = append(w.treeTargets, target)
}

func (w *MainWindow) AddSession(name string, widget gtk.Widgetter)    { w.stack.AddNamed(widget, name) }
func (w *MainWindow) RemoveSession(name string, widget gtk.Widgetter) { w.stack.Remove(widget) }
func (w *MainWindow) ShowSession(name string) {
	w.active = name
	w.setChromeVisible(true)
	w.stack.SetVisibleChildName(name)
}
func (w *MainWindow) ShowEmpty() {
	w.active = ""
	w.setChromeVisible(true)
	w.stack.SetVisibleChildName("__empty")
	w.UpdateWindowTabs(nil, "")
}

func (w *MainWindow) UpdateSessionTabs(names []string, active string) {
	removeBoxChildren(w.sessionTabs)
	for _, name := range names {
		n := name
		box := gtk.NewBox(gtk.OrientationHorizontal, 2)
		button := gtk.NewButtonWithLabel(n)
		if n == active {
			button.AddCSSClass("suggested-action")
		}
		button.ConnectClicked(func() { w.callbacks.Open(n, "", "") })
		box.Append(button)
		close := iconButton("window-close-symbolic", "Close tab", func() { w.callbacks.CloseTab(n) })
		close.AddCSSClass("flat")
		box.Append(close)
		w.sessionTabs.Append(box)
	}
}

func (w *MainWindow) UpdateWindowTabs(snapshot *protocol.Snapshot, requested string) {
	removeBoxChildren(w.windowTabs)
	if snapshot == nil {
		return
	}
	active := snapshot.ActiveWindow
	if requested != "" {
		active = requested
	}
	for _, window := range snapshot.Windows {
		win := window
		button := gtk.NewButtonWithLabel(fmt.Sprintf("%d: %s", win.Index, win.Name))
		button.AddCSSClass("flat")
		if win.ID == active {
			button.AddCSSClass("suggested-action")
		}
		button.ConnectClicked(func() { w.callbacks.Open(w.active, win.ID, "") })
		w.windowTabs.Append(button)
	}
}

func iconButton(icon, tooltip string, clicked func()) *gtk.Button {
	b := gtk.NewButtonFromIconName(icon)
	b.SetTooltipText(tooltip)
	b.AddCSSClass("flat")
	if clicked != nil {
		b.ConnectClicked(clicked)
	}
	return b
}

func sidebarHeading(text string) *gtk.Label {
	l := gtk.NewLabel(text)
	l.SetHAlign(gtk.AlignStart)
	l.SetMarginStart(12)
	l.SetMarginTop(12)
	l.SetMarginBottom(6)
	l.AddCSSClass("dim-label")
	return l
}

func emptyPage(title, subtitle string) gtk.Widgetter {
	box := gtk.NewBox(gtk.OrientationVertical, 8)
	box.SetHAlign(gtk.AlignCenter)
	box.SetVAlign(gtk.AlignCenter)
	icon := gtk.NewImageFromIconName("utilities-terminal-symbolic")
	icon.SetPixelSize(64)
	box.Append(icon)
	t := gtk.NewLabel(title)
	t.AddCSSClass("empty-title")
	box.Append(t)
	s := gtk.NewLabel(subtitle)
	s.AddCSSClass("dim-label")
	box.Append(s)
	return box
}

func removeBoxChildren(box *gtk.Box) {
	for child := box.FirstChild(); child != nil; child = box.FirstChild() {
		box.Remove(child)
	}
}
