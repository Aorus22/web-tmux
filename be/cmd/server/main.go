package main

import (
	"context"
	"fmt"
	"log/slog"
	"net"
	"os"
	"os/signal"
	"runtime"
	"syscall"
	"time"

	"tmux-gui/be/internal/config"
	"tmux-gui/be/internal/realtime"
	"tmux-gui/be/internal/server"
	"tmux-gui/be/internal/tmux"
)

func main() {
	if runtime.GOOS != "linux" && runtime.GOOS != "windows" {
		fmt.Fprintln(os.Stderr, "Unsupported operating system.")
		fmt.Fprintln(os.Stderr, "Tmux GUI currently supports Linux and Windows.")
		os.Exit(1)
	}

	cfg, err := config.Load()
	if err != nil {
		fmt.Fprintln(os.Stderr, "config:", err)
		os.Exit(1)
	}

	log := newLogger(cfg.LogLevel)

	// Verify tmux is installed (PRD §4.2).
	exec := tmux.NewExecutor(tmux.ResolveSocket(cfg.TmuxSocket))
	if _, err := exec.Run(context.Background(), "-V"); err != nil {
		fmt.Fprintln(os.Stderr, "Tmux is not installed.")
		os.Exit(1)
	}

	svc := tmux.NewService(tmux.ResolveSocket(cfg.TmuxSocket), log, cfg.ScrollbackLines)
	hub := realtime.NewHub(svc, log)
	router := server.NewRouter(cfg, svc, hub, log)
	srv := server.New(cfg.Addr(), router, log)

	// Bind explicitly so the actual port is always known: Electron needs
	// BACKEND_PORT:<n> to load the URL (PRD §50). The desktop app pins a fixed
	// port so the renderer origin stays stable and localStorage (settings)
	// survives restarts; if that port is busy (e.g. the web version is
	// running), fall back to a dynamic port rather than failing to start.
	ln, err := net.Listen("tcp", cfg.Addr())
	if err != nil && cfg.Port != 0 {
		log.Warn("port busy, falling back to dynamic port", "addr", cfg.Addr(), "err", err)
		ln, err = net.Listen("tcp", net.JoinHostPort(cfg.Host, "0"))
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, "listen:", err)
		os.Exit(1)
	}
	fmt.Printf("BACKEND_PORT:%d\n", ln.Addr().(*net.TCPAddr).Port)
	go func() { _ = srv.Serve(ln) }()
	// Graceful shutdown (PRD §49): close HTTP, cancel WS clients, stop
	// control-mode children. tmux sessions are never killed.
	stop := make(chan os.Signal, 1)
	signal.Notify(stop, syscall.SIGINT, syscall.SIGTERM)
	<-stop

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := srv.Shutdown(ctx); err != nil {
		log.Warn("shutdown error", "err", err)
	}
	log.Info("bye")
}

func newLogger(level string) *slog.Logger {
	var lv slog.Level
	switch level {
	case "debug":
		lv = slog.LevelDebug
	case "warn":
		lv = slog.LevelWarn
	case "error":
		lv = slog.LevelError
	default:
		lv = slog.LevelInfo
	}
	return slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: lv}))
}
