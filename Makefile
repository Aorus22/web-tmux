# Tmux GUI — perintah build (Windows dan Linux)
# Jalankan dari root repo:
#   make install          pasang dependency FE + Go backend + desktop GTK
#   make install-electron pasang dependency Electron ke desktop/node_modules
#   make fe               build frontend -> be/internal/web/dist
#   make be               build backend -> tmux-gui-server[.exe] di root
#   make desktop-gpui      build GPUI desktop (release) ke desktop-gpui/target
#   make dev-gpui           jalankan GPUI desktop dari source (cargo run)
#   make package-gpui-windows  bundel dist Windows (tmux-gui-windows-x64)
#   make package-gpui-linux    bundel dist Linux - jalankan di Linux

.DEFAULT_GOAL := help

ifeq ($(OS),Windows_NT)
    HOST_OS := windows
else
    UNAME_S := $(shell uname -s 2>/dev/null)
    ifeq ($(UNAME_S),Darwin)
        HOST_OS := darwin
    else
        HOST_OS := linux
    endif
endif

EXE_EXT :=
NPM_INSTALL_FE = cd fe && npm install
NPM_INSTALL_DESKTOP = cd desktop && npm install
NPM_BUILD_FE = cd fe && npm run build
NPM_BUILD_ELECTRON = cd desktop && npm run build:linux

ifeq ($(HOST_OS),windows)
    SHELL := cmd.exe
    .SHELLFLAGS := /D /C
    EXE_EXT := .exe
    NPM_INSTALL_FE = cd /D fe && npm install
    NPM_INSTALL_DESKTOP = cd /D desktop && npm install
    NPM_BUILD_FE = cd /D fe && npm run build
    NPM_BUILD_ELECTRON = cd /D desktop && npm run build:win
    COPY_ELECTRON_ASSETS = powershell -NoProfile -ExecutionPolicy Bypass -Command "New-Item -ItemType Directory -Force -Path 'desktop\resources' | Out-Null; if (Test-Path -LiteralPath 'desktop\resources\fe-dist') { Remove-Item -Recurse -Force -LiteralPath 'desktop\resources\fe-dist' }; Copy-Item -Recurse -Force -LiteralPath 'be\internal\web\dist' -Destination 'desktop\resources\fe-dist'; Copy-Item -Force -LiteralPath 'tmux-gui-server.exe' -Destination 'desktop\resources\tmux-gui-server.exe'"
else ifeq ($(HOST_OS),darwin)
    NPM_BUILD_ELECTRON = cd desktop && npm run build
    COPY_ELECTRON_ASSETS = mkdir -p desktop/resources && rm -rf desktop/resources/fe-dist && cp -r be/internal/web/dist desktop/resources/fe-dist && cp -f tmux-gui-server desktop/resources/tmux-gui-server
else
    COPY_ELECTRON_ASSETS = mkdir -p desktop/resources && rm -rf desktop/resources/fe-dist && cp -r be/internal/web/dist desktop/resources/fe-dist && cp -f tmux-gui-server desktop/resources/tmux-gui-server
endif

.PHONY: help install install-electron fe be desktop-gtk desktop-electron desktop-gpui dev-gpui package-gpui-windows package-gpui-linux build-fe build-be build build-desktop build-gtk build-gtk-windows package-gtk-windows dev-web dev-desktop dev-gtk test test-be test-fe test-gtk clean

help:
	@echo Target yang tersedia:
	@echo   make install          - install dependency FE, Electron, backend, dan desktop GTK
	@echo   make install-electron - install dependency ke desktop/node_modules
	@echo   make fe               - build frontend ke be/internal/web/dist
	@echo   make be               - build backend menjadi tmux-gui-server$(EXE_EXT)
	@echo   make desktop-gtk      - build binary GTK ke compiled
	@echo   make desktop-electron - build aplikasi Electron
	@echo   make desktop-gpui     - build GPUI desktop (release)
	@echo   make dev-gpui         - jalankan GPUI desktop dari source
	@echo   make package-gpui-windows - bundel dist Windows tmux-gui-windows-x64
	@echo   make package-gpui-linux   - bundel dist Linux (jalankan di Linux)
	@echo   make dev-web          - jalankan backend + Vite
	@echo   make dev-desktop      - jalankan Vite + Electron

install:
	$(NPM_INSTALL_FE)
	$(MAKE) install-electron
	cd be && go mod download
	cd desktop-gtk && go mod download

install-electron:
	$(NPM_INSTALL_DESKTOP)

fe:
	$(NPM_BUILD_FE)
	@echo Hasil FE siap: be/internal/web/dist

be:
	cd be && go build -trimpath -ldflags "-s -w" -o ../tmux-gui-server$(EXE_EXT) ./cmd/server
	@echo Hasil backend: tmux-gui-server$(EXE_EXT)

desktop-electron: fe be
	$(COPY_ELECTRON_ASSETS)
	$(NPM_BUILD_ELECTRON)
	@echo Hasil Electron siap di desktop/dist

ifeq ($(HOST_OS),windows)
desktop-gtk:
	powershell -NoProfile -ExecutionPolicy Bypass -File "desktop-gtk/scripts/build-gtk.ps1"
else
desktop-gtk:
	mkdir -p compiled
	cd desktop-gtk && go vet ./...
	cd desktop-gtk && go build -trimpath -ldflags "-s -w" -o ../compiled/tmux-gui-desktop .
endif

# Backward-compatible aliases for the previous target names.
build-fe: fe
build-be: be
build: fe be
build-desktop: desktop-electron
build-gtk: desktop-gtk
build-gtk-windows:
	powershell -NoProfile -ExecutionPolicy Bypass -File "desktop-gtk/scripts/build-gtk.ps1"

package-gtk-windows:
	powershell -NoProfile -ExecutionPolicy Bypass -File "desktop-gtk/scripts/build-windows.ps1"

# GPUI desktop (PKG-03): same HOST_OS-dispatch shape as desktop-gtk.
# Windows recipes stay one-line powershell -File delegations (Pitfall 6) -
# no standalone build-gpui.ps1 in scope, so desktop-gpui reuses the tested
# 3-step package path with -SkipBackend (release client build, no Go rebuild).
ifeq ($(HOST_OS),windows)
desktop-gpui:
	powershell -NoProfile -ExecutionPolicy Bypass -File "desktop-gpui/scripts/package-gpui-windows.ps1" -Release -SkipBackend
else
desktop-gpui:
	cargo build --release --manifest-path desktop-gpui/Cargo.toml --package webtmux
endif

dev-gpui:
	cargo run --manifest-path desktop-gpui/Cargo.toml -p webtmux

package-gpui-windows:
	powershell -NoProfile -ExecutionPolicy Bypass -File "desktop-gpui/scripts/package-gpui-windows.ps1" -Release

ifeq ($(HOST_OS),windows)
package-gpui-linux:
	@echo package-gpui-linux: run on Linux - no cross-build from Windows (Pitfall 7)
else
package-gpui-linux:
	./desktop-gpui/scripts/package-gpui-linux.sh
endif

dev-web:
	./scripts/dev-web.sh

dev-desktop:
	./scripts/dev-desktop.sh

ifeq ($(HOST_OS),windows)
dev-gtk:
	powershell -NoProfile -ExecutionPolicy Bypass -File "desktop-gtk/scripts/dev-windows.ps1"
else
dev-gtk:
	./desktop-gtk/scripts/dev-linux.sh
endif

test: test-be test-fe

test-be:
	cd be && go test ./...

test-fe:
	cd fe && npm run test

test-gtk:
	cd desktop-gtk && go test ./...

clean:
	rm -rf dist
	rm -rf desktop/out desktop/dist desktop/resources/fe-dist
	rm -rf be/internal/web/dist/*
	touch be/internal/web/dist/.gitkeep
	cd fe && npm run clean 2>/dev/null || true
