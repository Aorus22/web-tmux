package web

import (
	"embed"
	"io/fs"
	"net/http"
	"strings"
)

//go:embed all:dist
var distFS embed.FS

// distSub is the embedded frontend build rooted at dist/.
var distSub fs.FS

func init() {
	sub, err := fs.Sub(distFS, "dist")
	if err != nil {
		panic("embed dist: " + err.Error())
	}
	distSub = sub
}

// Handler serves the embedded frontend with SPA fallback (PRD §62).
// /api/* paths are never served here — the router handles them separately.
type Handler struct {
	fileServer http.Handler
}

func NewHandler() *Handler {
	return &Handler{
		fileServer: http.FileServer(http.FS(distSub)),
	}
}

func (h *Handler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if strings.HasPrefix(r.URL.Path, "/api/") {
		http.NotFound(w, r)
		return
	}

	path := strings.TrimPrefix(r.URL.Path, "/")
	if path == "" {
		path = "index.html"
	}

	// Try the real asset first; fall back to index.html for SPA routes.
	if _, err := fs.Stat(distSub, path); err == nil {
		h.fileServer.ServeHTTP(w, r)
		return
	}

	serveIndex(w, r)
}

// serveIndex writes the SPA shell.
func serveIndex(w http.ResponseWriter, r *http.Request) {
	data, err := fs.ReadFile(distSub, "index.html")
	if err != nil {
		http.Error(w, "frontend not built", http.StatusNotFound)
		return
	}
	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	w.Write(data)
}
