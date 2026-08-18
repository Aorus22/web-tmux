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

echo "==> Copying FE dist -> desktop/resources/fe-dist (served by Electron itself)"
rm -rf "$ROOT/desktop/resources/fe-dist"
cp -r "$ROOT/be/internal/web/dist" "$ROOT/desktop/resources/fe-dist"

echo "==> Packaging Electron"
(
  cd "$ROOT/desktop"
  npm run build
)

echo "==> Done. Artifacts in desktop/dist"
