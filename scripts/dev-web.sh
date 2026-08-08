#!/usr/bin/env bash
# Development: Go backend on :14101 + Vite dev server on :14102 (proxies /api)
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cleanup() { kill "${BE_PID:-}" "${FE_PID:-}" 2>/dev/null || true; }
trap cleanup EXIT INT TERM

# Make sure the embedded dist placeholder exists (go:embed needs at least one file)
mkdir -p "$ROOT/be/internal/web/dist"
touch "$ROOT/be/internal/web/dist/.gitkeep"

(
  cd "$ROOT/be"
  go run ./cmd/server &
)
BE_PID=$!

(
  cd "$ROOT/fe"
  npm run dev -- --port 14102
) &
FE_PID=$!

wait -n "$BE_PID" "$FE_PID"
