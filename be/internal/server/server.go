package server

import (
	"context"
	"log/slog"
	"net"
	"net/http"
	"time"
)

// Server wraps the HTTP server lifecycle (PRD §49: graceful shutdown on
// SIGINT/SIGTERM: close HTTP, cancel WebSocket clients, terminate control-mode
// children, wait, exit — never kill the tmux server).
type Server struct {
	http    *http.Server
	handler http.Handler
	log     *slog.Logger

	// onShutdown is invoked before the server closes (realtime hub shutdown).
	onShutdown func()
}

func New(addr string, handler http.Handler, log *slog.Logger) *Server {
	return &Server{
		http: &http.Server{
			Addr:              addr,
			Handler:           handler,
			ReadHeaderTimeout: 10 * time.Second,
			IdleTimeout:       60 * time.Second,
		},
		handler: handler,
		log:     log.With("component", "server"),
	}
}

// SetShutdownHook registers a callback run on graceful shutdown.
func (s *Server) SetShutdownHook(fn func()) {
	s.onShutdown = fn
}

// ListenAndServe starts the HTTP server, blocking until it exits.
func (s *Server) ListenAndServe() error {
	s.log.Info("listening", "addr", s.http.Addr)
	return s.http.ListenAndServe()
}

// Serve starts the HTTP server on an already-bound listener (dynamic port).
func (s *Server) Serve(ln net.Listener) error {
	s.log.Info("listening", "addr", ln.Addr().String())
	return s.http.Serve(ln)
}

// Shutdown performs the graceful shutdown sequence.
func (s *Server) Shutdown(ctx context.Context) error {
	s.log.Info("shutting down")
	if s.onShutdown != nil {
		s.onShutdown()
	}
	return s.http.Shutdown(ctx)
}
