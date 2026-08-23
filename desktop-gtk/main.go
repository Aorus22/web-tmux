package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"path/filepath"
	"runtime"
	"syscall"

	"tmux-gui/desktop-gtk/internal/backend"
	"tmux-gui/desktop-gtk/internal/ui"
)

const (
	appID      = "dev.tmuxgui.gtk"
	appName    = "Tmux GUI"
	appVersion = "0.1.0"
)

func main() {
	preparePortableRuntime()
	backendPath := flag.String("backend-path", defaultBackendPath(), "path to tmux-gui-server")
	userDataDir := flag.String("user-data-dir", backend.UserDataDir(), "per-user configuration and logs")
	noBackend := flag.Bool("no-backend", false, "connect to an already running backend")
	port := flag.Int("port", 0, "backend port when --no-backend is used")
	version := flag.Bool("version", false, "print version and exit")
	flag.Parse()

	if *version {
		fmt.Printf("%s %s\n", appName, appVersion)
		return
	}

	log.SetFlags(log.LstdFlags | log.Lmicroseconds)
	ctx, cancel := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer cancel()

	var manager *backend.Manager
	var ports <-chan int
	if *noBackend {
		if *port <= 0 {
			log.Fatal("--port must be greater than zero with --no-backend")
		}
		ports = immediatePort(*port)
	} else {
		manager = backend.NewManager(*backendPath, *userDataDir)
		if err := manager.Start(ctx); err != nil {
			log.Fatalf("start backend: %v", err)
		}
		ports = manager.Port()
	}

	app, err := ui.NewApp(ui.AppConfig{
		ID:          appID,
		Name:        appName,
		Version:     appVersion,
		UserDataDir: *userDataDir,
		Manager:     manager,
		PortChannel: ports,
	})
	if err != nil {
		log.Fatal(err)
	}
	os.Exit(app.Run(ctx))
}

func preparePortableRuntime() {
	if runtime.GOOS != "windows" {
		return
	}
	exe, err := os.Executable()
	if err != nil {
		return
	}
	root := filepath.Dir(exe)
	_ = os.Setenv("PATH", root+string(os.PathListSeparator)+os.Getenv("PATH"))
	share := filepath.Join(root, "share")
	if _, err := os.Stat(share); err == nil {
		_ = os.Setenv("GTK_DATA_PREFIX", root)
		_ = os.Setenv("XDG_DATA_DIRS", share)
		_ = os.Setenv("GSETTINGS_SCHEMA_DIR", filepath.Join(share, "glib-2.0", "schemas"))
	}
	loaders := filepath.Join(root, "lib", "gdk-pixbuf-2.0", "2.10.0", "loaders")
	if _, err := os.Stat(loaders); err == nil {
		_ = os.Setenv("GDK_PIXBUF_MODULEDIR", loaders)
	}
}

func defaultBackendPath() string {
	suffix := ""
	if runtime.GOOS == "windows" {
		suffix = ".exe"
	}
	if exe, err := os.Executable(); err == nil {
		return filepath.Join(filepath.Dir(exe), "tmux-gui-server"+suffix)
	}
	return "tmux-gui-server" + suffix
}

func immediatePort(port int) <-chan int {
	ch := make(chan int, 1)
	ch <- port
	close(ch)
	return ch
}
