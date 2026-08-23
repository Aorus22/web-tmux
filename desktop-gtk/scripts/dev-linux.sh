#!/usr/bin/env bash
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
mkdir -p "$repo/dist/gtk/dev"
(cd "$repo/be" && go build -o "$repo/dist/gtk/dev/tmux-gui-server" ./cmd/server)
(cd "$repo/desktop-gtk" && go run . --backend-path "$repo/dist/gtk/dev/tmux-gui-server")
