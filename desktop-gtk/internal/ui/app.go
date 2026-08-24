package ui

import (
	"context"
	"fmt"
	"log"
	"os"
	"strings"
	"sync/atomic"
	"time"

	"github.com/diamondburned/gotk4-adwaita/pkg/adw"
	"github.com/diamondburned/gotk4/pkg/gio/v2"
	"github.com/diamondburned/gotk4/pkg/glib/v2"

	"tmux-gui/desktop-gtk/internal/api"
	"tmux-gui/desktop-gtk/internal/backend"
	"tmux-gui/desktop-gtk/internal/config"
	"tmux-gui/desktop-gtk/internal/protocol"
)

type AppConfig struct {
	ID          string
	Name        string
	Version     string
	UserDataDir string
	Manager     *backend.Manager
	PortChannel <-chan int
}

type App struct {
	cfg      AppConfig
	adwApp   *adw.Application
	window   *MainWindow
	client   *api.Client
	port     int
	settings config.Settings
	ctx      context.Context
	cancel   context.CancelFunc
	views    map[string]*SessionView
	active   string
	tree     protocol.Tree
	request  atomic.Uint64
}

func NewApp(cfg AppConfig) (*App, error) {
	a := &App{cfg: cfg, views: map[string]*SessionView{}}
	a.adwApp = adw.NewApplication(cfg.ID, gio.ApplicationFlagsNone)
	a.adwApp.Application.ConnectActivate(a.onActivate)
	a.adwApp.Application.ConnectShutdown(a.onShutdown)
	return a, nil
}

func (a *App) Run(parent context.Context) int {
	a.ctx, a.cancel = context.WithCancel(parent)
	go func() {
		<-a.ctx.Done()
		glib.IdleAdd(func() bool { a.adwApp.Quit(); return false })
	}()
	return a.adwApp.Run([]string{os.Args[0]})
}

func (a *App) onActivate() {
	a.settings = config.Load(a.cfg.UserDataDir)
	ApplyTheme(a.settings.UITheme)
	a.window = NewMainWindow(a.adwApp, a.cfg.Name, a.cfg.Version, WindowCallbacks{
		Open:       a.open,
		CloseTab:   a.closeTab,
		Command:    a.runToolbarCommand,
		NewSession: a.showNewSessionDialog,
		Palette:    a.showCommandPalette,
		Settings:   a.showSettings,
	})
	a.window.AdwWin.ConnectCloseRequest(func() bool { a.adwApp.Quit(); return false })
	a.registerShortcuts()
	a.window.Present()
	go a.awaitBackend()
}

func (a *App) awaitBackend() {
	port, ok := <-a.cfg.PortChannel
	if !ok || port <= 0 {
		a.idle(func() { a.window.SetStatus("Backend tidak dapat dijalankan") })
		return
	}
	a.client = api.New(port)
	a.port = port
	ctx, cancel := context.WithTimeout(a.ctx, 8*time.Second)
	defer cancel()
	info, err := a.client.Info(ctx)
	if err != nil {
		a.idle(func() { a.window.SetStatus("Backend belum siap: " + err.Error()) })
		return
	}
	if path := strings.TrimSpace(a.settings.TmuxBinary); path != "" {
		selected, selectErr := a.client.SetTmuxBinary(ctx, path)
		if selectErr != nil {
			a.idle(func() { a.window.SetStatus("tmux binary tidak valid: " + selectErr.Error()) })
			return
		}
		info = selected
	}
	a.idle(func() { a.window.SetStatus("Connected · tmux " + info.Version) })
	a.pollTree()
}

func (a *App) pollTree() {
	ticker := time.NewTicker(1500 * time.Millisecond)
	defer ticker.Stop()
	for {
		ctx, cancel := context.WithTimeout(a.ctx, 4*time.Second)
		tree, err := a.client.Tree(ctx)
		cancel()
		if err == nil {
			a.idle(func() {
				a.tree = tree
				a.window.UpdateTree(tree)
			})
		}
		select {
		case <-ticker.C:
		case <-a.ctx.Done():
			return
		}
	}
}

func (a *App) open(session, windowID, paneID string) {
	if session == "" || a.client == nil {
		return
	}
	v := a.views[session]
	if v == nil {
		v = NewSessionView(a.ctx, session, a.settings, func(msg protocol.Incoming) { a.send(session, msg) }, a.toast, func(snapshot *protocol.Snapshot) {
			if a.active == session {
				a.window.UpdateWindowTabs(snapshot, "")
			}
		})
		a.views[session] = v
		a.window.AddSession(session, v.Widget())
		v.Start(a.client, a.port)
	}
	a.active = session
	a.window.ShowSession(session)
	a.window.UpdateSessionTabs(a.sessionNames(), session)
	v.FocusTarget(windowID, paneID)
	a.window.UpdateWindowTabs(v.Snapshot(), windowID)
}

func (a *App) closeTab(session string) {
	v := a.views[session]
	if v == nil {
		return
	}
	v.Close()
	delete(a.views, session)
	a.window.RemoveSession(session, v.Widget())
	names := a.sessionNames()
	if a.active == session {
		a.active = ""
		if len(names) > 0 {
			a.open(names[len(names)-1], "", "")
		} else {
			a.window.ShowEmpty()
		}
	}
	a.window.UpdateSessionTabs(names, a.active)
}

func (a *App) sessionNames() []string {
	names := make([]string, 0, len(a.views))
	for _, node := range a.tree.Sessions {
		if _, ok := a.views[node.Session.Name]; ok {
			names = append(names, node.Session.Name)
		}
	}
	for name := range a.views {
		found := false
		for _, n := range names {
			found = found || n == name
		}
		if !found {
			names = append(names, name)
		}
	}
	return names
}

func (a *App) send(session string, msg protocol.Incoming) {
	if v := a.views[session]; v != nil {
		if msg.RequestID == "" && msg.Type != protocol.TerminalInput && msg.Type != protocol.TerminalResize && msg.Type != protocol.TerminalCapture {
			msg.RequestID = fmt.Sprintf("gtk-%d", a.request.Add(1))
		}
		if err := v.Send(msg); err != nil {
			a.toast(err.Error())
		}
	}
}

func (a *App) activeView() *SessionView { return a.views[a.active] }

func (a *App) runToolbarCommand(command, value string) {
	v := a.activeView()
	if v == nil {
		return
	}
	msg := protocol.Incoming{Type: command, PaneID: v.ActivePaneID()}
	switch command {
	case protocol.PaneSplit:
		msg.Direction = value
	case protocol.WindowLayout:
		msg.Layout = value
		msg.PaneID = v.ActiveWindowID()
	case protocol.WindowBreakActive, protocol.WindowMove:
		msg.PaneID = v.ActiveWindowID()
	case protocol.WindowSelect:
		msg.PaneID = value
	}
	a.send(a.active, msg)
}

func (a *App) toast(message string) {
	if a.window != nil {
		a.window.Toast(message)
	}
}

func (a *App) idle(fn func()) { glib.IdleAdd(func() bool { fn(); return false }) }

func (a *App) onShutdown() {
	if a.cancel != nil {
		a.cancel()
	}
	for _, v := range a.views {
		v.Close()
	}
	if a.cfg.Manager != nil {
		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		if err := a.cfg.Manager.Stop(ctx); err != nil {
			log.Printf("stop backend: %v", err)
		}
	}
}

func (a *App) registerShortcuts() {
	bindings := []struct {
		name, accel string
		fn          func()
	}{
		{"quit", "<Primary>Q", func() { a.adwApp.Quit() }},
		{"new-session", "<Primary><Shift>N", a.showNewSessionDialog},
		{"new-window", "<Primary>N", func() { a.runToolbarCommand(protocol.WindowCreate, "") }},
		{"split-horizontal", "<Primary><Shift>D", func() { a.runToolbarCommand(protocol.PaneSplit, "horizontal") }},
		{"split-vertical", "<Primary>D", func() { a.runToolbarCommand(protocol.PaneSplit, "vertical") }},
		{"palette", "<Primary><Shift>P", a.showCommandPalette},
		{"settings", "<Primary>comma", a.showSettings},
	}
	for _, binding := range bindings {
		action := gio.NewSimpleAction(binding.name, nil)
		fn := binding.fn
		action.ConnectActivate(func(_ *glib.Variant) { fn() })
		a.adwApp.AddAction(action)
		a.adwApp.SetAccelsForAction("app."+binding.name, []string{binding.accel})
	}
}
