package realtime

import (
	"context"
	"log/slog"
	"sync"

	"github.com/coder/websocket"
	"tmux-gui/be/internal/tmux"
)

// Hub owns the per-session monitor lifecycle and the set of connected clients.
// One control-mode monitor per session, shared by all clients of that session;
// the monitor is torn down when the last client leaves (PRD §7, §50).
type Hub struct {
	svc  *tmux.Service
	log  *slog.Logger

	mu       sync.Mutex
	sessions map[string]*sessionGroup
}

type sessionGroup struct {
	session  string
	monitor  *tmux.Monitor
	clients  map[*Client]struct{}
	closed   bool
}

func NewHub(svc *tmux.Service, log *slog.Logger) *Hub {
	return &Hub{
		svc:      svc,
		log:      log.With("component", "realtime"),
		sessions: make(map[string]*sessionGroup),
	}
}

// connect ensures a monitor exists for session and registers the client.
// Returns the monitor so the handler can send the initial snapshot.
func (h *Hub) connect(ctx context.Context, session string, c *Client) (*tmux.Monitor, error) {
	h.mu.Lock()
	g, ok := h.sessions[session]
	if !ok {
		g = &sessionGroup{session: session, clients: make(map[*Client]struct{})}
		h.sessions[session] = g
	}
	h.mu.Unlock()

	monitor, err := h.svc.ConnectMonitor(ctx, session)
	if err != nil {
		h.mu.Lock()
		if !ok && len(g.clients) == 0 {
			delete(h.sessions, session)
		}
		h.mu.Unlock()
		return nil, err
	}

	h.mu.Lock()
	g.monitor = monitor
	g.clients[c] = struct{}{}
	h.mu.Unlock()

	// Relay monitor events to this client's write loop.
	go h.relay(session, c)
	return monitor, nil
}

// disconnect removes a client and stops the monitor when it was the last one.
func (h *Hub) disconnect(session string, c *Client) {
	h.mu.Lock()
	g, ok := h.sessions[session]
	if !ok {
		h.mu.Unlock()
		return
	}
	delete(g.clients, c)
	last := len(g.clients) == 0
	h.mu.Unlock()

	if last {
		h.svc.DisconnectMonitor(session)
		h.mu.Lock()
		if g, ok := h.sessions[session]; ok && len(g.clients) == 0 {
			delete(h.sessions, session)
		}
		h.mu.Unlock()
	}
}

// relay copies monitor events (output, state, command results) into the client.
func (h *Hub) relay(session string, c *Client) {
	monitor := h.svc.Monitor(session)
	if monitor == nil {
		return
	}
	ch := monitor.Subscribe()
	defer monitor.Unsubscribe(ch)

	for ev := range ch {
		switch ev.Type {
		case tmux.EvOutput:
			c.Send(Outgoing{Type: EvTerminalOutput, PaneID: ev.PaneID, Data: string(ev.Data)})
		case tmux.EvState:
			// Session is set so the frontend can drop state that arrives on a
			// stale connection left over from a previous session (guards
			// against the UI alternating between two sessions' snapshots).
			c.Send(Outgoing{Type: EvStateDelta, Session: session, Seq: seqOf(ev.Snapshot), Snapshot: ev.Snapshot})
		case tmux.EvCommandResult:
			if ev.Err != nil {
				c.Send(Outgoing{Type: EvCommandError, RequestID: ev.RequestID, Message: ev.Err.Error()})
			} else {
				c.Send(Outgoing{Type: EvCommandSuccess, RequestID: ev.RequestID})
			}
		case tmux.EvReconnecting:
			c.Send(Outgoing{Type: EvTmuxReconnecting})
		case tmux.EvDisconnected:
			c.Send(Outgoing{Type: EvTmuxDisconnected})
		}
	}
}

// sendClient is a helper for direct sends from the handler.
func (h *Hub) sendClient(c *Client, msg Outgoing) {
	c.Send(msg)
}

// BroadcastToSession pushes a message to every client of a session.
func (h *Hub) BroadcastToSession(session string, msg Outgoing) {
	h.mu.Lock()
	defer h.mu.Unlock()
	if g, ok := h.sessions[session]; ok {
		for c := range g.clients {
			c.Send(msg)
		}
	}
}

// Shutdown closes every client connection (PRD §49 graceful shutdown step 2).
func (h *Hub) Shutdown() {
	h.mu.Lock()
	defer h.mu.Unlock()
	for _, g := range h.sessions {
		for c := range g.clients {
			_ = c.conn.Close(websocket.StatusGoingAway, "server shutting down")
		}
		g.closed = true
	}
}
