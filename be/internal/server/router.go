package server

import (
	"log/slog"
	"net/http"

	"tmux-gui/be/internal/config"
	"tmux-gui/be/internal/realtime"
	"tmux-gui/be/internal/tmux"
	"tmux-gui/be/internal/web"
)

// Router wires all routes (PRD §54: plain net/http, no framework):
//
//	GET  /api/health       — liveness + tmux presence
//	GET  /api/tmux/info    — tmux version/config info
//	GET  /api/sessions     — full session/window/pane tree (sidebar polling)
//	GET  /api/sessions/{name}/snapshot — one session snapshot
//	POST /api/sessions     — create a session (works with zero sessions)
//	GET  /api/ws           — WebSocket: ?session=<name>
//	GET  /                 — embedded frontend (SPA fallback)
func NewRouter(cfg *config.Config, svc *tmux.Service, hub *realtime.Hub, log *slog.Logger) http.Handler {
	mux := http.NewServeMux()

	health := &HealthHandler{svc: svc, log: log.With("component", "health")}
	mux.HandleFunc("GET /api/health", health.Handle)
	mux.HandleFunc("GET /api/tmux/info", health.HandleInfo)
	mux.HandleFunc("GET /api/sessions", health.HandleTree)
	mux.HandleFunc("GET /api/sessions/{name}/snapshot", health.HandleSessionSnapshot)
	mux.HandleFunc("POST /api/sessions", health.HandleSessionCreate)

	ws := realtime.NewWSHandler(hub, svc, log)
	mux.Handle("GET /api/ws", ws)

	fe := web.NewHandler()
	mux.Handle("/", fe)

	return corsMiddleware(logMiddleware(log, mux))
}

// corsMiddleware allows the Electron desktop app (which serves the FE itself
// over the app:// origin) to call the backend API. The backend binds
// 127.0.0.1 only, so echoing any local origin is safe. The WebSocket handler
// already accepts all origins. Web mode is same-origin and unaffected.
func corsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if origin := r.Header.Get("Origin"); origin != "" {
			w.Header().Set("Access-Control-Allow-Origin", origin)
			w.Header().Set("Vary", "Origin")
			w.Header().Set("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
			w.Header().Set("Access-Control-Allow-Headers", "Content-Type")
		}
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		next.ServeHTTP(w, r)
	})
}

// logMiddleware logs requests at debug level.
func logMiddleware(log *slog.Logger, next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		log.Debug("http", "method", r.Method, "path", r.URL.Path, "remote", r.RemoteAddr)
		next.ServeHTTP(w, r)
	})
}
