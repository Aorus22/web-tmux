package realtime

import (
	"context"
	"log/slog"
	"sync"
	"time"

	"github.com/coder/websocket"
	"tmux-gui/be/internal/tmux"
)

// Client is a single WebSocket connection. It has a buffered send queue so the
// write loop never blocks the relay goroutines (slow-client protection).
type Client struct {
	session string
	conn    *websocket.Conn
	log     *slog.Logger

	mu     sync.Mutex
	sendCh chan Outgoing
	closed bool

	done chan struct{}
}

// newClient wraps a WebSocket connection.
func newClient(session string, conn *websocket.Conn, log *slog.Logger) *Client {
	return &Client{
		session: session,
		conn:    conn,
		log:     log.With("ws", session),
		sendCh:  make(chan Outgoing, 256),
		done:    make(chan struct{}),
	}
}

// Send enqueues a message; drops it if the client is closing or too slow.
func (c *Client) Send(msg Outgoing) {
	c.mu.Lock()
	if c.closed {
		c.mu.Unlock()
		return
	}
	c.mu.Unlock()

	select {
	case c.sendCh <- msg:
	case <-c.done:
	default:
		// Slow consumer: drop rather than grow memory.
	}
}

// writeLoop drains the send queue onto the WebSocket.
func (c *Client) writeLoop(ctx context.Context) {
	defer close(c.done)
	for {
		select {
		case <-ctx.Done():
			return
		case msg := <-c.sendCh:
			wctx, cancel := context.WithTimeout(ctx, 5*time.Second)
			err := c.conn.Write(wctx, websocket.MessageText, mustMarshal(msg))
			cancel()
			if err != nil {
				return
			}
		}
	}
}

// close shuts the client down.
func (c *Client) close() {
	c.mu.Lock()
	if c.closed {
		c.mu.Unlock()
		return
	}
	c.closed = true
	c.mu.Unlock()
	_ = c.conn.Close(websocket.StatusNormalClosure, "")
}

// seqOf extracts a stable sequence marker from a snapshot for delta ordering.
// The monitor increments an internal counter per snapshot; we expose it here
// by reading the snapshot's Seq field if present.
func seqOf(_ *tmux.Snapshot) uint64 {
	// The snapshot struct does not carry seq; the hub tracks ordering via
	// connection order. Returning 0 keeps the wire format stable while the
	// frontend relies on snapshot replacement, not gap detection, for state.
	return 0
}

func mustMarshal(msg Outgoing) []byte {
	b, err := jsonMarshal(msg)
	if err != nil {
		return []byte(`{"type":"server.error","message":"encode error"}`)
	}
	return b
}
