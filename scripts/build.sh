#!/usr/bin/env bash
# Production web build: FE build -> embedded into Go binary -> dist/tmux-gui-server
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

mkdir -p "$ROOT/dist" "$ROOT/be/internal/web/dist"

echo "==> Building frontend (embedded into be/internal/web/dist)"
(
  cd "$ROOT/fe"
  npm run build
)

echo "==> Building Go backend -> dist/tmux-gui-server"
(
  cd "$ROOT/be"
  go build -o ../dist/tmux-gui-server ./cmd/server
)

echo "==> Done. Run: ./dist/tmux-gui-server"
