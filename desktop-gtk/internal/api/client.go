package api

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"time"

	"tmux-gui/desktop-gtk/internal/protocol"
)

type Client struct {
	base string
	http *http.Client
}

func New(port int) *Client {
	// Ceiling only: every call binds its own, shorter context deadline. It has
	// to exceed create-session's 35s budget because the first session also
	// boots the tmux server, which is slow on Windows.
	return &Client{base: fmt.Sprintf("http://127.0.0.1:%d", port), http: &http.Client{Timeout: 40 * time.Second}}
}

func (c *Client) get(ctx context.Context, path string, out any) error {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, c.base+path, nil)
	if err != nil {
		return err
	}
	res, err := c.http.Do(req)
	if err != nil {
		return err
	}
	defer res.Body.Close()
	if res.StatusCode/100 != 2 {
		return fmt.Errorf("%s: %s", path, res.Status)
	}
	return json.NewDecoder(res.Body).Decode(out)
}

func (c *Client) Info(ctx context.Context) (protocol.TmuxInfo, error) {
	var v protocol.TmuxInfo
	return v, c.get(ctx, "/api/tmux/info", &v)
}

func (c *Client) SetTmuxBinary(ctx context.Context, path string) (protocol.TmuxInfo, error) {
	b, _ := json.Marshal(map[string]string{"path": path})
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.base+"/api/tmux/binary", bytes.NewReader(b))
	if err != nil {
		return protocol.TmuxInfo{}, err
	}
	req.Header.Set("Content-Type", "application/json")
	res, err := c.http.Do(req)
	if err != nil {
		return protocol.TmuxInfo{}, err
	}
	defer res.Body.Close()
	if res.StatusCode/100 != 2 {
		var e struct {
			Error string `json:"error"`
		}
		_ = json.NewDecoder(res.Body).Decode(&e)
		if e.Error != "" {
			return protocol.TmuxInfo{}, fmt.Errorf("%s", e.Error)
		}
		return protocol.TmuxInfo{}, fmt.Errorf("/api/tmux/binary: %s", res.Status)
	}
	var v protocol.TmuxInfo
	if err := json.NewDecoder(res.Body).Decode(&v); err != nil {
		return protocol.TmuxInfo{}, err
	}
	return v, nil
}
func (c *Client) Tree(ctx context.Context) (protocol.Tree, error) {
	var v protocol.Tree
	err := c.get(ctx, "/api/sessions", &v)
	if v.Sessions == nil {
		v.Sessions = []protocol.SessionTreeNode{}
	}
	return v, err
}
func (c *Client) Snapshot(ctx context.Context, session string) (protocol.Snapshot, error) {
	var v protocol.Snapshot
	return v, c.get(ctx, "/api/sessions/"+url.PathEscape(session)+"/snapshot", &v)
}

func (c *Client) CreateSession(ctx context.Context, name, cwd, command string) error {
	b, _ := json.Marshal(map[string]string{"name": name, "cwd": cwd, "initialCommand": command})
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.base+"/api/sessions", bytes.NewReader(b))
	if err != nil {
		return err
	}
	req.Header.Set("Content-Type", "application/json")
	res, err := c.http.Do(req)
	if err != nil {
		return err
	}
	defer res.Body.Close()
	if res.StatusCode/100 != 2 {
		var e struct {
			Error string `json:"error"`
		}
		_ = json.NewDecoder(res.Body).Decode(&e)
		if e.Error != "" {
			return fmt.Errorf("%s", e.Error)
		}
		return fmt.Errorf("create session: %s", res.Status)
	}
	return nil
}
