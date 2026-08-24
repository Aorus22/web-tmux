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
	stack        *adw.ViewStack
	settingsPage gtk.Widgetter
	chrome       []gtk.Widgetter
	status       *gtk.Label
	callbacks    WindowCallbacks
	active       string
}

func NewMainWindow(app *adw.Application, name, version string, callbacks WindowCallbacks, settingsPage gtk.Widgetter) *MainWindow {
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
	scroll.SetVExpand(true)
	scroll.SetHExpand(true)
	// The tree can contain hundreds of rows. Its natural content height must
	// never become the application's minimum window height; the viewport is
	// the thing that expands and the rows belong inside its scrollbar.
	scroll.SetPropagateNaturalHeight(false)
	scroll.SetChild(w.sidebar)

	side := gtk.NewBox(gtk.OrientationVertical, 0)
	side.SetVExpand(true)
	side.SetHExpand(true)
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

	// Split/zoom/new-window controls live on each pane header and the window
	// tab row, like the web UI. The global toolbar only keeps tmux layouts.
	tools := gtk.NewBox(gtk.OrientationHorizontal, 4)
	tools.AddCSSClass("tmux-toolbar")
	for _, layout := range []struct{ label, value string }{{"Even H", "even-horizontal"}, {"Even V", "even-vertical"}, {"Main H", "main-horizontal"}, {"Main V", "main-vertical"}, {"Tiled", "tiled"}} {
		l := layout
		b := gtk.NewButtonWithLabel(l.label)
		b.SetTooltipText("Apply " + l.value + " layout")
		b.ConnectClicked(func() { callbacks.Command(protocol.WindowLayout, l.value) })
		tools.Append(b)
	}

	// Keep all top-level pages in an Adwaita ViewStack, just like wa-bot. The
	// stack owns both the terminal workspace and the permanent PreferencesPage;
	// switching pages therefore never adds/removes a large child from the
	// window's layout tree.
	w.stack = adw.NewViewStack()
	w.stack.SetHExpand(true)
	w.stack.SetVExpand(true)
	w.stack.SetSizeRequest(1, 1)
	w.stack.SetEnableTransitions(true)
	w.stack.SetTransitionDuration(160)
	empty := emptyPage("Welcome to Tmux GUI", "Create a tmux session, or pick one from the sidebar.", callbacks.NewSession)
	w.stack.AddNamed(empty, "__empty")
	if settingsPage != nil {
		w.settingsPage = settingsPage
		w.stack.AddNamed(settingsPage, "__settings")
	}
	w.stack.SetVisibleChildName("__empty")

	w.status = gtk.NewLabel("Starting backend…")
	w.status.SetHAlign(gtk.AlignStart)
	w.status.AddCSSClass("statusbar")
	w.status.SetEllipsize(3)

	content := gtk.NewBox(gtk.OrientationVertical, 0)
	content.SetHExpand(true)
	content.SetVExpand(true)
	content.SetSizeRequest(1, 1)
	content.AddCSSClass("content-pane")
	content.Append(w.sessionTabs)
	content.Append(w.windowTabs)
	content.Append(tools)
	stackBin := adw.NewBin()
	stackBin.SetHExpand(true)
	stackBin.SetVExpand(true)
	stackBin.SetSizeRequest(1, 1)
	stackBin.SetChild(w.stack)
	content.Append(stackBin)
	content.Append(w.status)
	w.chrome = []gtk.Widgetter{w.sessionTabs, w.windowTabs, tools, w.status}
	// The window boots into the onboarding page, which is shown without any
	// tab bars or layout buttons; they only belong to an open session view.
	w.setChromeVisible(false)

	split := adw.NewOverlaySplitView()
	split.SetHExpand(true)
	split.SetVExpand(true)
	split.SetSidebar(side)
	split.SetContent(content)
	split.SetMinSidebarWidth(240)
	split.SetMaxSidebarWidth(320)

	w.toastOverlay = adw.NewToastOverlay()
	w.toastOverlay.SetHExpand(true)
	w.toastOverlay.SetVExpand(true)
	w.toastOverlay.SetSizeRequest(1, 1)
	w.toastOverlay.SetChild(split)
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
	// Closing the last tab lands on the onboarding page: no tab bars, no
	// layout toolbar — just the welcome content.
	w.setChromeVisible(false)
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
	// The "+" trails the tab list so a new window always opens to the right of
	// the existing ones, matching the web WindowTabs row.
	w.windowTabs.Append(iconButton("list-add-symbolic", "New window (Ctrl+N)", func() { w.callbacks.Command(protocol.WindowCreate, "") }))
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

func emptyPage(title, subtitle string, newSession func()) gtk.Widgetter {
	box := gtk.NewBox(gtk.OrientationVertical, 10)
	box.SetHAlign(gtk.AlignCenter)
	box.SetVAlign(gtk.AlignCenter)
	icon := gtk.NewImageFromIconName("utilities-terminal-symbolic")
	icon.SetPixelSize(64)
	icon.SetHAlign(gtk.AlignCenter)
	box.Append(icon)
	t := gtk.NewLabel(title)
	t.AddCSSClass("empty-title")
	box.Append(t)
	s := gtk.NewLabel(subtitle)
	s.AddCSSClass("dim-label")
	box.Append(s)
	button := gtk.NewButtonWithLabel("New session")
	button.AddCSSClass("suggested-action")
	button.AddCSSClass("pill")
	button.SetHAlign(gtk.AlignCenter)
	button.SetMarginTop(8)
	button.ConnectClicked(newSession)
	box.Append(button)
	return box
}

func removeBoxChildren(box *gtk.Box) {
	for child := box.FirstChild(); child != nil; child = box.FirstChild() {
		box.Remove(child)
	}
}
