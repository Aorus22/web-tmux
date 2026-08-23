package ws

import (
	"context"
	"encoding/json"
	"fmt"
	"net/url"
	"sync"
	"sync/atomic"
	"time"

	"github.com/gorilla/websocket"
	"tmux-gui/desktop-gtk/internal/protocol"
)

type Client struct {
	port      int
	session   string
	onMessage func(protocol.Outgoing)
	onState   func(string)
	writeMu   sync.Mutex
	mu        sync.Mutex
	conn      *websocket.Conn
	queue     []protocol.Incoming
	closed    atomic.Bool
	cancel    context.CancelFunc
}

func New(port int, session string, onMessage func(protocol.Outgoing), onState func(string)) *Client {
	return &Client{port: port, session: session, onMessage: onMessage, onState: onState}
}

func (c *Client) Start(parent context.Context) {
	ctx, cancel := context.WithCancel(parent)
	c.cancel = cancel
	go c.loop(ctx)
}

func (c *Client) loop(ctx context.Context) {
	backoff := []time.Duration{250 * time.Millisecond, 500 * time.Millisecond, time.Second, 2 * time.Second, 5 * time.Second, 10 * time.Second}
	attempt := 0
	for ctx.Err() == nil && !c.closed.Load() {
		c.state(map[bool]string{true: "connecting", false: "reconnecting"}[attempt == 0])
		u := url.URL{Scheme: "ws", Host: fmt.Sprintf("127.0.0.1:%d", c.port), Path: "/api/ws", RawQuery: url.Values{"session": []string{c.session}}.Encode()}
		conn, _, err := websocket.DefaultDialer.DialContext(ctx, u.String(), nil)
		if err != nil {
			c.state("reconnecting")
			d := backoff[min(attempt, len(backoff)-1)]
			attempt++
			select {
			case <-time.After(d):
			case <-ctx.Done():
				return
			}
			continue
		}
		attempt = 0
		c.mu.Lock()
		c.conn = conn
		queued := c.queue
		c.queue = nil
		c.mu.Unlock()
		c.state("connected")
		for _, m := range queued {
			_ = c.write(m)
		}
		for {
			var msg protocol.Outgoing
			if err := conn.ReadJSON(&msg); err != nil {
				break
			}
			if c.onMessage != nil {
				c.onMessage(msg)
			}
		}
		c.mu.Lock()
		if c.conn == conn {
			c.conn = nil
		}
		c.mu.Unlock()
		_ = conn.Close()
		if ctx.Err() == nil {
			c.state("reconnecting")
		}
	}
	c.state("disconnected")
}

func (c *Client) Send(msg protocol.Incoming) error {
	c.mu.Lock()
	connected := c.conn != nil
	if !connected {
		c.queue = append(c.queue, msg)
		c.mu.Unlock()
		return nil
	}
	c.mu.Unlock()
	return c.write(msg)
}

func (c *Client) write(msg protocol.Incoming) error {
	c.writeMu.Lock()
	defer c.writeMu.Unlock()
	c.mu.Lock()
	conn := c.conn
	c.mu.Unlock()
	if conn == nil {
		return nil
	}
	b, err := json.Marshal(msg)
	if err != nil {
		return err
	}
	return conn.WriteMessage(websocket.TextMessage, b)
}

func (c *Client) Close() {
	if c.closed.Swap(true) {
		return
	}
	if c.cancel != nil {
		c.cancel()
	}
	c.mu.Lock()
	if c.conn != nil {
		_ = c.conn.Close()
	}
	c.mu.Unlock()
}
func (c *Client) state(s string) {
	if c.onState != nil {
		c.onState(s)
	}
}
func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}
