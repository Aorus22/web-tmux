#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

EXE_NAME="tmux-gui-server"
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
    EXE_NAME="tmux-gui-server.exe"
fi

if [[ -f "$EXE_NAME" ]]; then
    echo "[build-test-backend] $EXE_NAME already exists at repo root."
    exit 0
fi

echo "[build-test-backend] Building be/cmd/server to $EXE_NAME..."
cd be
go build -trimpath -ldflags "-s -w" -o "../$EXE_NAME" ./cmd/server
echo "[build-test-backend] Build succeeded: $EXE_NAME"
