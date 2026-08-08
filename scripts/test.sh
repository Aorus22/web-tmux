#!/usr/bin/env bash
# Run all tests
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "==> Backend tests"
(
  cd "$ROOT/be"
  go test ./...
)

echo "==> Frontend tests"
(
  cd "$ROOT/fe"
  npm run test
)
