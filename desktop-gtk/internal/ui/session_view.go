package ui

import (
	"context"
	"fmt"
	"strings"

	"github.com/diamondburned/gotk4/pkg/glib/v2"
	"github.com/diamondburned/gotk4/pkg/gtk/v4"

	"tmux-gui/desktop-gtk/internal/api"
	"tmux-gui/desktop-gtk/internal/config"
	"tmux-gui/desktop-gtk/internal/geometry"
	"tmux-gui/desktop-gtk/internal/protocol"
	"tmux-gui/desktop-gtk/internal/terminal"
	"tmux-gui/desktop-gtk/internal/ws"
)

type pendingTerminal struct {
	data    string
	replace bool
}

type PaneWidget struct {
	pane    protocol.Pane
	frame   *gtk.Box
	header  *gtk.Label
	surface *terminal.Surface
}

type dividerWidget struct {
	widget *gtk.Box
	data   geometry.Divider
}

type SessionView struct {
	ctx        context.Context
	session    string
	settings   config.Settings
	send       func(protocol.Incoming)
	toast      func(string)
	onSnapshot func(*protocol.Snapshot)
	root       *gtk.Overlay
	fixed      *gtk.Fixed
	empty      *gtk.Box
	state      *gtk.Label
	client     *ws.Client
	snapshot   *protocol.Snapshot
	panes      map[string]*PaneWidget
	pending    map[string]pendingTerminal
	dividers   []dividerWidget
	lastW      int
	lastH      int
	closed     bool
}

func NewSessionView(ctx context.Context, session string, settings config.Settings, send func(protocol.Incoming), toast func(string), onSnapshot func(*protocol.Snapshot)) *SessionView {
	v := &SessionView{ctx: ctx, session: session, settings: settings, send: send, toast: toast, onSnapshot: onSnapshot, panes: map[string]*PaneWidget{}, pending: map[string]pendingTerminal{}}
	v.fixed = gtk.NewFixed()
	v.fixed.SetHExpand(true)
	v.fixed.SetVExpand(true)
	v.fixed.SetOverflow(gtk.OverflowHidden)
	v.fixed.AddCSSClass("terminal-workspace")
	v.root = gtk.NewOverlay()
	v.root.SetChild(v.fixed)
	v.empty = gtk.NewBox(gtk.OrientationVertical, 8)
	v.empty.SetHAlign(gtk.AlignCenter)
	v.empty.SetVAlign(gtk.AlignCenter)
	v.empty.Append(gtk.NewImageFromIconName("utilities-terminal-symbolic"))
	v.state = gtk.NewLabel("Connecting to " + session + "…")
	v.state.AddCSSClass("dim-label")
	v.empty.Append(v.state)
	v.root.AddOverlay(v.empty)
	glib.TimeoutAdd(100, func() bool {
		if v.closed {
			return false
		}
		w, h := v.fixed.AllocatedWidth(), v.fixed.AllocatedHeight()
		if w != v.lastW || h != v.lastH {
			v.lastW, v.lastH = w, h
			v.layout()
			v.reportViewport()
		}
		return true
	})
	return v
}

func (v *SessionView) Widget() gtk.Widgetter        { return v.root }
func (v *SessionView) Snapshot() *protocol.Snapshot { return v.snapshot }
func (v *SessionView) ActivePaneID() string {
	if v.snapshot == nil {
		return ""
	}
	return v.snapshot.ActivePane
}

func (v *SessionView) ActiveWindowID() string {
	if v.snapshot == nil {
		return ""
	}
	return v.snapshot.ActiveWindow
}

func (v *SessionView) OtherPaneIDs() []string {
	if v.snapshot == nil {
		return nil
	}
	var ids []string
	for _, pane := range v.snapshot.Panes {
		if pane.WindowID == v.snapshot.ActiveWindow && pane.ID != v.snapshot.ActivePane {
			ids = append(ids, pane.ID)
		}
	}
	return ids
}

func (v *SessionView) CopyActive() {
	if pane := v.panes[v.ActivePaneID()]; pane != nil {
		pane.surface.Copy()
	}
}

func (v *SessionView) PasteActive() {
	if pane := v.panes[v.ActivePaneID()]; pane != nil {
		pane.surface.PasteClipboard()
	}
}

func (v *SessionView) ToggleTUIScroll() bool {
	id := v.ActivePaneID()
	enabled := !v.settings.TUIScrollPanes[id]
	v.settings.TUIScrollPanes[id] = enabled
	if pane := v.panes[id]; pane != nil {
		pane.surface.SetTUIScroll(enabled)
	}
	return enabled
}

func (v *SessionView) ApplySettings(settings config.Settings) {
	v.settings = settings
	palette := PaletteForTheme(FindTheme(settings.UITheme))
	if settings.TerminalTheme != nil {
		if terminalTheme := findTerminalTheme(*settings.TerminalTheme); terminalTheme != nil {
			t := *FindTheme(settings.UITheme)
			t.TerminalTheme = terminalTheme.Name
			palette = PaletteForTheme(&t)
		}
	}
	for id, pane := range v.panes {
		pane.surface.SetFont(settings.FontFamily, settings.FontSize, settings.LineHeight)
		pane.surface.SetPalette(palette)
		pane.surface.SetTUIScroll(settings.TUIScrollPanes[id])
	}
	v.layout()
}

func (v *SessionView) Start(client *api.Client, port int) {
	go func() {
		snapshot, err := client.Snapshot(v.ctx, v.session)
		if err == nil {
			glib.IdleAdd(func() bool { v.applySnapshot(&snapshot); return false })
		}
	}()
	v.client = ws.New(port, v.session, v.handleMessage, v.handleState)
	v.client.Start(v.ctx)
}

func (v *SessionView) Send(msg protocol.Incoming) error {
	if v.client == nil {
		return fmt.Errorf("session %s belum terhubung", v.session)
	}
	return v.client.Send(msg)
}

func (v *SessionView) Close() {
	v.closed = true
	if v.client != nil {
		v.client.Close()
	}
	for _, pane := range v.panes {
		pane.surface.Dispose()
	}
	v.panes = map[string]*PaneWidget{}
}

func (v *SessionView) handleState(state string) {
	glib.IdleAdd(func() bool {
		if v.closed {
			return false
		}
		v.state.SetText(strings.ToUpper(state[:1]) + state[1:] + " · " + v.session)
		gtk.BaseWidget(v.empty).SetVisible(state != "connected" || v.snapshot == nil)
		return false
	})
}

func (v *SessionView) handleMessage(msg protocol.Outgoing) {
	glib.IdleAdd(func() bool {
		if v.closed {
			return false
		}
		switch msg.Type {
		case protocol.StateSnapshot, protocol.StateDelta:
			if msg.Snapshot != nil && (msg.Session == "" || msg.Session == v.session) {
				v.applySnapshot(msg.Snapshot)
			}
		case protocol.TerminalSnapshot:
			v.handleTerminal(msg.PaneID, msg.Data, true)
		case protocol.TerminalOutput:
			v.handleTerminal(msg.PaneID, msg.Data, msg.Replace)
		case protocol.CommandError, protocol.ServerError:
			v.toast(msg.Message)
		case protocol.TmuxDisconnected:
			v.state.SetText("tmux disconnected · reconnecting…")
			gtk.BaseWidget(v.empty).SetVisible(true)
		case protocol.TmuxReconnecting:
			v.state.SetText("Reconnecting to tmux…")
			gtk.BaseWidget(v.empty).SetVisible(true)
		}
		return false
	})
}

func (v *SessionView) handleTerminal(paneID, data string, replace bool) {
	if pane := v.panes[paneID]; pane != nil {
		if replace {
			pane.surface.ReplaceScreen(data)
		} else {
			pane.surface.Feed([]byte(data))
		}
		return
	}
	p := v.pending[paneID]
	if replace {
		p = pendingTerminal{data: data, replace: true}
	} else {
		p.data += data
		if len(p.data) > 1<<20 {
			p.data = p.data[len(p.data)-(1<<20):]
		}
	}
	v.pending[paneID] = p
}

func (v *SessionView) applySnapshot(snapshot *protocol.Snapshot) {
	oldWindow := ""
	if v.snapshot != nil {
		oldWindow = v.snapshot.ActiveWindow
	}
	v.snapshot = snapshot
	gtk.BaseWidget(v.empty).SetVisible(false)
	v.reconcilePanes()
	v.layout()
	if v.onSnapshot != nil {
		v.onSnapshot(snapshot)
	}
	if oldWindow != "" && oldWindow != snapshot.ActiveWindow {
		v.captureVisible()
	}
}

func (v *SessionView) visiblePanes() []protocol.Pane {
	if v.snapshot == nil {
		return nil
	}
	var panes []protocol.Pane
	for _, pane := range v.snapshot.Panes {
		if pane.WindowID == v.snapshot.ActiveWindow {
			panes = append(panes, pane)
		}
	}
	for _, pane := range panes {
		if pane.Zoomed {
			return []protocol.Pane{pane}
		}
	}
	return panes
}

func (v *SessionView) reconcilePanes() {
	visible := v.visiblePanes()
	wanted := map[string]protocol.Pane{}
	for _, pane := range visible {
		wanted[pane.ID] = pane
	}
	for id, widget := range v.panes {
		if _, ok := wanted[id]; !ok {
			v.fixed.Remove(widget.frame)
			widget.surface.Dispose()
			delete(v.panes, id)
		}
	}
	for _, pane := range visible {
		widget := v.panes[pane.ID]
		if widget == nil {
			widget = v.newPane(pane)
			v.panes[pane.ID] = widget
			v.fixed.Put(widget.frame, 0, 0)
			if pending, ok := v.pending[pane.ID]; ok {
				if pending.replace {
					widget.surface.ReplaceScreen(pending.data)
				} else {
					widget.surface.Feed([]byte(pending.data))
				}
				delete(v.pending, pane.ID)
			}
			v.send(protocol.Incoming{Type: protocol.TerminalCapture, PaneID: pane.ID})
		}
		widget.pane = pane
		widget.header.SetText(paneTitle(pane))
		widget.frame.RemoveCSSClass("pane-active")
		if pane.Active {
			widget.frame.AddCSSClass("pane-active")
		}
	}
}

func (v *SessionView) newPane(pane protocol.Pane) *PaneWidget {
	frame := gtk.NewBox(gtk.OrientationVertical, 0)
	frame.AddCSSClass("pane-frame")
	frame.SetOverflow(gtk.OverflowHidden)
	headerBox := gtk.NewBox(gtk.OrientationHorizontal, 4)
	headerBox.AddCSSClass("pane-header")
	header := gtk.NewLabel(paneTitle(pane))
	header.SetHAlign(gtk.AlignStart)
	header.SetHExpand(true)
	header.SetEllipsize(3)
	header.AddCSSClass("pane-title")
	headerBox.Append(header)
	zoom := iconButton("view-fullscreen-symbolic", "Zoom pane", func() { v.send(protocol.Incoming{Type: protocol.PaneZoom, PaneID: pane.ID}) })
	headerBox.Append(zoom)
	frame.Append(headerBox)
	palette := PaletteForTheme(FindTheme(v.settings.UITheme))
	if v.settings.TerminalTheme != nil {
		if terminalTheme := findTerminalTheme(*v.settings.TerminalTheme); terminalTheme != nil {
			t := *FindTheme(v.settings.UITheme)
			t.TerminalTheme = terminalTheme.Name
			palette = PaletteForTheme(&t)
		}
	}
	surface, err := terminal.NewSurface(terminal.SurfaceOptions{
		PaneID: pane.ID, FontFamily: v.settings.FontFamily, FontSize: v.settings.FontSize,
		LineHeight: v.settings.LineHeight, Scrollback: v.settings.ScrollbackLines, Palette: palette,
		TUIScroll: v.settings.TUIScrollPanes[pane.ID],
		OnInput: func(data []byte) {
			v.send(protocol.Incoming{Type: protocol.TerminalInput, PaneID: pane.ID, Data: string(data)})
		},
		OnActivate: func() { v.send(protocol.Incoming{Type: protocol.PaneSelect, PaneID: pane.ID}) },
	})
	if err != nil {
		panic(err)
	}
	frame.Append(surface.Widget())
	return &PaneWidget{pane: pane, frame: frame, header: header, surface: surface}
}

func (v *SessionView) layout() {
	if v.snapshot == nil || v.lastW <= 0 || v.lastH <= 0 {
		return
	}
	for _, d := range v.dividers {
		v.fixed.Remove(d.widget)
	}
	v.dividers = nil
	window := v.activeWindow()
	if window == nil {
		return
	}
	panes := geometry.ClosePaneGaps(v.visiblePanes())
	rects := make(map[string]geometry.Rect, len(panes))
	for _, pane := range panes {
		r := geometry.PixelRect(pane, v.lastW, v.lastH, window.Width, window.Height)
		rects[pane.ID] = r
		if widget := v.panes[pane.ID]; widget != nil {
			v.fixed.Move(widget.frame, float64(r.Left), float64(r.Top))
			widget.frame.SetSizeRequest(max(1, r.Width), max(1, r.Height))
		}
	}
	if len(panes) <= 1 || panes[0].Zoomed {
		return
	}
	for _, divider := range geometry.Dividers(panes, rects) {
		v.addDivider(divider)
	}
}

func (v *SessionView) addDivider(divider geometry.Divider) {
	handle := gtk.NewBox(gtk.OrientationHorizontal, 0)
	handle.AddCSSClass("pane-divider")
	if divider.Direction == "R" {
		handle.SetCursorFromName("col-resize")
	} else {
		handle.SetCursorFromName("row-resize")
	}
	handle.SetSizeRequest(max(1, divider.Rect.Width), max(1, divider.Rect.Height))
	v.fixed.Put(handle, float64(divider.Rect.Left), float64(divider.Rect.Top))
	gesture := gtk.NewGestureDrag()
	last := 0
	gesture.ConnectDragBegin(func(_, _ float64) { last = 0 })
	gesture.ConnectDragUpdate(func(dx, dy float64) {
		position := dx
		if divider.Direction == "D" {
			position = dy
		}
		step := geometry.DragStep(position, 0, last, divider.CellPX)
		if step == 0 {
			return
		}
		last += step
		direction, amount := divider.Direction, step
		if amount < 0 {
			amount = -amount
			if direction == "R" {
				direction = "L"
			} else {
				direction = "U"
			}
		}
		v.send(protocol.Incoming{Type: protocol.PaneResize, PaneID: divider.PaneID, Direction: direction, Amount: amount})
	})
	handle.AddController(gesture)
	v.dividers = append(v.dividers, dividerWidget{widget: handle, data: divider})
}

func (v *SessionView) activeWindow() *protocol.Window {
	if v.snapshot == nil {
		return nil
	}
	for i := range v.snapshot.Windows {
		if v.snapshot.Windows[i].ID == v.snapshot.ActiveWindow {
			return &v.snapshot.Windows[i]
		}
	}
	return nil
}

func (v *SessionView) reportViewport() {
	if v.lastW <= 0 || v.lastH <= 0 {
		return
	}
	cols, rows := geometry.Viewport(v.lastW, v.lastH)
	v.send(protocol.Incoming{Type: protocol.TerminalResize, Cols: cols, Rows: rows})
}

func (v *SessionView) captureVisible() {
	for _, pane := range v.visiblePanes() {
		v.send(protocol.Incoming{Type: protocol.TerminalCapture, PaneID: pane.ID})
	}
}

func (v *SessionView) FocusTarget(windowID, paneID string) {
	if windowID != "" && (v.snapshot == nil || windowID != v.snapshot.ActiveWindow) {
		v.send(protocol.Incoming{Type: protocol.WindowSelect, PaneID: windowID})
	}
	if paneID != "" {
		v.send(protocol.Incoming{Type: protocol.PaneSelect, PaneID: paneID})
	}
}

func paneTitle(p protocol.Pane) string {
	title := p.Title
	if title == "" {
		title = p.CurrentCommand
	}
	if title == "" {
		title = p.ID
	}
	return fmt.Sprintf("%d · %s", p.Index, title)
}
