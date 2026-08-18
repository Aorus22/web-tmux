package server

import (
	"context"
	"encoding/json"
	"log/slog"
	"net/http"
	"strings"
	"time"

	"tmux-gui/be/internal/tmux"
)

// HealthHandler serves the REST endpoints (PRD §9: reads; §10 bootstrap
// create). All mutating actions go through the per-session WebSocket except
// create-session, which must work with zero sessions (no WS to send it on).
type HealthHandler struct {
	svc *tmux.Service
	log *slog.Logger
}

func (h *HealthHandler) writeJSON(w http.ResponseWriter, status int, v interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

// Handle serves GET /api/health — reports tmux presence and version.
func (h *HealthHandler) Handle(w http.ResponseWriter, r *http.Request) {
	ctx, cancel := context.WithTimeout(r.Context(), 3*time.Second)
	defer cancel()

	version, err := h.svc.TmuxVersion(ctx)
	tmuxOK := err == nil

	h.writeJSON(w, http.StatusOK, map[string]interface{}{
		"status": "ok",
		"tmux": map[string]interface{}{
			"installed": tmuxOK,
			"version":   version,
		},
	})
}

// HandleInfo serves GET /api/tmux/info.
func (h *HealthHandler) HandleInfo(w http.ResponseWriter, r *http.Request) {
	ctx, cancel := context.WithTimeout(r.Context(), 3*time.Second)
	defer cancel()

	version, err := h.svc.TmuxVersion(ctx)
	h.writeJSON(w, http.StatusOK, map[string]interface{}{
		"version": version,
		"ok":      err == nil,
	})
}

// HandleTree serves GET /api/sessions — full sidebar tree.
func (h *HealthHandler) HandleTree(w http.ResponseWriter, r *http.Request) {
	ctx, cancel := context.WithTimeout(r.Context(), 5*time.Second)
	defer cancel()

	tree, err := h.svc.Tree(ctx)
	if err != nil {
		// No tmux server / not installed: return an empty tree, not a 500.
		h.writeJSON(w, http.StatusOK, map[string]interface{}{"sessions": []interface{}{}})
		return
	}
	h.writeJSON(w, http.StatusOK, tree)
}

// HandleSessionSnapshot serves GET /api/sessions/{name}/snapshot.
func (h *HealthHandler) HandleSessionSnapshot(w http.ResponseWriter, r *http.Request) {
	name := r.PathValue("name")
	ctx, cancel := context.WithTimeout(r.Context(), 5*time.Second)
	defer cancel()

	snap, err := h.svc.Snapshot(ctx, name)
	if err != nil {
		h.writeJSON(w, http.StatusNotFound, map[string]string{"error": err.Error()})
		return
	}
	h.writeJSON(w, http.StatusOK, snap)
}

// HandleSessionCreate serves POST /api/sessions — creates a tmux session.
// This is the only mutating REST endpoint: it must work when no session
// exists yet (no control-mode client, PRD §10 bootstrap path), which the
// per-session WebSocket cannot do. The service falls back to a one-shot
// `tmux new-session` when no monitor is connected.
type createSessionRequest struct {
	Name           string `json:"name"`
	Cwd            string `json:"cwd"`
	InitialCommand string `json:"initialCommand"`
}

func (h *HealthHandler) HandleSessionCreate(w http.ResponseWriter, r *http.Request) {
	var req createSessionRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.writeJSON(w, http.StatusBadRequest, map[string]string{"error": "invalid JSON body: " + err.Error()})
		return
	}
	if err := tmux.ValidateSessionName(req.Name); err != nil {
		h.writeJSON(w, http.StatusBadRequest, map[string]string{"error": err.Error()})
		return
	}

	ctx, cancel := context.WithTimeout(r.Context(), 5*time.Second)
	defer cancel()
	if err := h.svc.CreateSession(ctx, req.Name, req.Cwd, req.InitialCommand); err != nil {
		if strings.Contains(err.Error(), "duplicate") {
			h.writeJSON(w, http.StatusConflict, map[string]string{"error": err.Error()})
			return
		}
		h.log.Warn("create session failed", "name", req.Name, "err", err)
		h.writeJSON(w, http.StatusInternalServerError, map[string]string{"error": err.Error()})
		return
	}
	h.writeJSON(w, http.StatusCreated, map[string]string{"name": req.Name})
}
