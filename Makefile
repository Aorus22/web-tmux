.PHONY: install dev-web dev-desktop dev-gtk build-fe build-be build build-desktop build-gtk build-gtk-windows test test-be test-fe test-gtk clean

install:
	cd fe && npm install
	cd desktop && npm install
	cd be && go mod download

# Development: Go backend (:14101) + Vite dev server (:14102)
dev-web:
	./scripts/dev-web.sh

# Desktop development: Electron spawns backend, loads Vite dev server
dev-desktop:
	./scripts/dev-desktop.sh

dev-gtk:
	./desktop-gtk/scripts/dev-linux.sh

build-fe:
	cd fe && npm run build

build-be:
	cd be && go build -o ../dist/tmux-gui-server ./cmd/server

# Full production web build: embedded FE + single Go binary
build: build-fe build-be

# Production desktop build: FE -> embedded in backend -> electron-builder
build-desktop:
	./scripts/build-desktop.sh

build-gtk:
	./desktop-gtk/scripts/build-linux.sh

build-gtk-windows:
	powershell -ExecutionPolicy Bypass -File desktop-gtk/scripts/build-windows.ps1

test: test-be test-fe

test-be:
	cd be && go test ./...

test-fe:
	cd fe && npm run test

test-gtk:
	cd desktop-gtk && go test ./...

clean:
	rm -rf dist
	rm -rf desktop/out desktop/dist
	rm -rf be/internal/web/dist/*
	touch be/internal/web/dist/.gitkeep
	cd fe && npm run clean 2>/dev/null || true
