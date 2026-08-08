package server

import (
	"context"
	"encoding/json"
	"log/slog"
	"net/http"
	"time"

	"tmux-gui/be/internal/tmux"
)

// HealthHandler serves the read-only REST endpoints (PRD §9).
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
