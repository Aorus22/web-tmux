package realtime

import (
	"context"
	"encoding/json"
	"log/slog"
	"net/http"

	"github.com/coder/websocket"
	"tmux-gui/be/internal/tmux"
)

// WSHandler is the WebSocket endpoint handler: GET /api/ws?session=<name>.
// It validates the Origin header (PRD §45), connects the session monitor, and
// then runs the message dispatch loop (PRD §26).
type WSHandler struct {
	hub *Hub
	svc *tmux.Service
	log *slog.Logger
}

func NewWSHandler(hub *Hub, svc *tmux.Service, log *slog.Logger) *WSHandler {
	return &WSHandler{hub: hub, svc: svc, log: log.With("component", "ws")}
}

func (h *WSHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	session := r.URL.Query().Get("session")
	if session == "" {
		http.Error(w, "missing session query parameter", http.StatusBadRequest)
		return
	}

	conn, err := websocket.Accept(w, r, &websocket.AcceptOptions{
		OriginPatterns: []string{"*"},
	})
	if err != nil {
		h.log.Warn("websocket accept failed", "err", err)
		return
	}

	client := newClient(session, conn, h.log)
	ctx, cancel := context.WithCancel(r.Context())
	defer cancel()
	defer client.close()

	// Connect monitor + register; sends the initial snapshot and readiness.
	monitor, err := h.hub.connect(ctx, session, client)
	if err != nil {
		client.Send(Outgoing{Type: EvServerError, Message: err.Error()})
		_ = client.conn.Close(websocket.StatusPolicyViolation, err.Error())
		return
	}
	defer h.hub.disconnect(session, client)

	go client.writeLoop(ctx)

	// connection.ready (PRD §27)
	client.Send(Outgoing{Type: EvConnectionReady, Session: session})
	// initial full snapshot
	if snap := monitor.Snapshot(); snap != nil {
		client.Send(Outgoing{Type: EvStateSnapshot, Session: session, Snapshot: snap})
	}

	// Dispatch loop: client → server messages.
	for {
		msg, data, err := conn.Read(ctx)
		if err != nil {
			if websocket.CloseStatus(err) != -1 {
				h.log.Debug("websocket closed", "session", session, "status", websocket.CloseStatus(err))
			}
			return
		}
		if msg != websocket.MessageText {
			continue
		}
		var in Incoming
		if err := json.Unmarshal(data, &in); err != nil {
			client.Send(Outgoing{Type: EvServerError, Message: "invalid message: " + err.Error()})
			continue
		}
		h.dispatch(ctx, client, in)
	}
}

// dispatch routes a client message to the typed service operation (PRD §87).
// Every GUI action is a typed method — the client can never send raw tmux.
func (h *WSHandler) dispatch(ctx context.Context, c *Client, in Incoming) {
	session := c.session
	req := in.RequestID
	ok := func() {
		if req != "" {
			c.Send(Outgoing{Type: EvCommandSuccess, RequestID: req})
		}
	}
	fail := func(err error) {
		if req != "" {
			c.Send(Outgoing{Type: EvCommandError, RequestID: req, Message: err.Error()})
		} else {
			c.Send(Outgoing{Type: EvServerError, Message: err.Error()})
		}
	}

	switch in.Type {
	case MsgHello:
		if in.Cols > 0 && in.Rows > 0 {
			if err := h.svc.ResizeTerminal(session, in.Cols, in.Rows); err != nil {
				h.log.Debug("initial resize failed", "err", err)
			}
		}
		ok()

	case MsgTerminalInput:
		if in.PaneID == "" {
			fail(errString("paneId required"))
			return
		}
		if err := h.svc.SendInput(session, in.PaneID, []byte(in.Data)); err != nil {
			fail(err)
		}

	case MsgTerminalResize:
		if in.Cols <= 0 || in.Rows <= 0 {
			fail(errString("cols/rows required"))
			return
		}
		if err := h.svc.ResizeTerminal(session, in.Cols, in.Rows); err != nil {
			fail(err)
		}

	case MsgTerminalCapture:
		if in.PaneID == "" {
			fail(errString("paneId required"))
			return
		}
		data, err := h.svc.CapturePane(ctx, session, in.PaneID)
		if err != nil {
			fail(err)
			return
		}
		c.Send(Outgoing{Type: EvTerminalSnapshot, PaneID: in.PaneID, Data: data})

	case MsgPaneSelect:
		failOr(fail, h.svc.SelectPane(ctx, session, in.PaneID), ok)
	case MsgPaneSplit:
		dir := in.Direction
		if dir == "" {
			dir = "horizontal"
		}
		failOr(fail, h.svc.SplitPane(ctx, session, in.PaneID, dir), ok)
	case MsgPaneResize:
		failOr(fail, h.svc.ResizePane(ctx, session, in.PaneID, in.Direction, in.Amount), ok)
	case MsgPaneKill:
		failOr(fail, h.svc.KillPane(ctx, session, in.PaneID), ok)
	case MsgPaneZoom:
		failOr(fail, h.svc.ZoomPane(ctx, session, in.PaneID), ok)
	case MsgPaneBreak:
		failOr(fail, h.svc.BreakPane(ctx, session, in.PaneID), ok)
	case MsgPaneSwap:
		failOr(fail, h.svc.SwapPane(ctx, session, in.PaneID, in.OtherPaneID), ok)

	case MsgWindowSelect:
		failOr(fail, h.svc.SelectWindow(ctx, session, in.PaneID), ok)
	case MsgWindowCreate:
		failOr(fail, h.svc.CreateWindow(ctx, session, in.Name, in.Cwd, in.InitialCommand), ok)
	case MsgWindowRename:
		failOr(fail, h.svc.RenameWindow(ctx, session, in.PaneID, in.Name), ok)
	case MsgWindowKill:
		failOr(fail, h.svc.KillWindow(ctx, session, in.PaneID), ok)
	case MsgWindowLayout:
		failOr(fail, h.svc.SelectLayout(ctx, session, in.PaneID, in.Layout), ok)
	case MsgWindowMove:
		failOr(fail, h.svc.MoveWindow(ctx, session, in.PaneID, in.Amount), ok)
	case MsgWindowBreakActive:
		failOr(fail, h.svc.BreakActivePane(ctx, session, in.PaneID), ok)

	case MsgSessionCreate:
		failOr(fail, h.svc.CreateSession(ctx, in.Session, in.Cwd, in.InitialCommand), ok)
	case MsgSessionRename:
		failOr(fail, h.svc.RenameSession(ctx, session, in.NewName), ok)
	case MsgSessionKill:
		// Multi-session: an explicit session name targets any session; the
		// default is the client's own session.
		target := session
		if in.Session != "" {
			target = in.Session
		}
		failOr(fail, h.svc.KillSession(ctx, target), ok)

	case MsgStateResync:
		snap, err := h.svc.Snapshot(ctx, session)
		if err != nil {
			fail(err)
			return
		}
		c.Send(Outgoing{Type: EvStateSnapshot, Session: session, Snapshot: snap})

	default:
		c.Send(Outgoing{Type: EvServerError, Message: "unknown message type: " + in.Type})
	}
}

func failOr(fail func(error), err error, ok func()) {
	if err != nil {
		fail(err)
		return
	}
	ok()
}

type errString string

func (e errString) Error() string { return string(e) }

// jsonMarshal is a thin wrapper so client.go can reuse it without importing
// encoding/json twice.
func jsonMarshal(v interface{}) ([]byte, error) {
	return json.Marshal(v)
}
