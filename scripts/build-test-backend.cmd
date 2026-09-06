@echo off
setlocal
cd /d "%~dp0\.."

if exist "tmux-gui-server.exe" (
    echo [build-test-backend] tmux-gui-server.exe already exists at repo root.
    exit /b 0
)

echo [build-test-backend] Building be/cmd/server to tmux-gui-server.exe...
cd be
go build -trimpath -ldflags "-s -w" -o ../tmux-gui-server.exe ./cmd/server
if errorlevel 1 (
    echo [build-test-backend] Failed to build tmux-gui-server.exe
    exit /b 1
)

echo [build-test-backend] Build succeeded: tmux-gui-server.exe
exit /b 0
