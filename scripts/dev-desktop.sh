#!/usr/bin/env bash
# Desktop development: Electron spawns Go backend (:14101) and loads Vite (:14102)
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cleanup() { kill "${FE_PID:-}" 2>/dev/null || true; }
trap cleanup EXIT INT TERM

mkdir -p "$ROOT/be/internal/web/dist"
touch "$ROOT/be/internal/web/dist/.gitkeep"

(
  cd "$ROOT/fe"
  npm run dev -- --port 14102
) &
FE_PID=$!

(
  cd "$ROOT/desktop"
  NODE_ENV=development npm start
)
