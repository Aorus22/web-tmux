#!/usr/bin/env bash
# Desktop build: FE -> embedded into backend binary -> desktop/resources -> electron-builder
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

mkdir -p "$ROOT/desktop/resources"

echo "==> Building frontend"
(
  cd "$ROOT/fe"
  npm run build
)

echo "==> Building Go backend -> desktop/resources/tmux-gui-server"
(
  cd "$ROOT/be"
  go build -o ../desktop/resources/tmux-gui-server ./cmd/server
)

echo "==> Packaging Electron"
(
  cd "$ROOT/desktop"
  npm run build:linux
)

echo "==> Done. Artifacts in desktop/dist"
