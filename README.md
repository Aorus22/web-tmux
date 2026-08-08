# Tmux GUI

A graphical control surface for [tmux](https://github.com/tmux/tmux/wiki). Tmux stays the
source of truth for sessions, windows, panes, layouts and terminal processes — this app only
adds a GUI layer on top, for desktop (Electron) and web.

## Architecture

```
tmux-gui/
├── be/         Go backend — tmux control mode, REST API, WebSocket, embedded FE
├── fe/         React + TypeScript + Vite + Tailwind + shadcn/ui + xterm.js
├── desktop/    Electron shell — spawns the Go backend, opens the window
├── scripts/    Dev/build helper scripts
└── Makefile    Shortcuts
```

- Tmux is the single source of truth. No database duplicates tmux state.
- Mutating commands flow through the control-mode connection (`tmux -CC`).
- Read-only queries use one-shot `tmux` commands.
- All window/pane operations use stable IDs (`@N` / `%N`), never raw indexes.

## Prerequisites

- Linux
- Node.js 22+ (24 recommended)
- Go 1.22+ (current stable recommended)
- tmux >= 3.2 (tested against 3.3 – 3.7)

## Development (web)

```bash
make dev-web
```

- Go backend: http://127.0.0.1:14101
- Vite dev server (proxies `/api`): http://127.0.0.1:14102

## Development (desktop)

```bash
make dev-desktop
```

## Production web build

```bash
make build
./dist/tmux-gui-server
# -> http://127.0.0.1:14101
```

The React build is embedded into the Go binary; a single executable serves everything.

## Desktop build

```bash
make build-desktop
```

Produces AppImage / .deb via electron-builder (Linux).

## Configuration (backend)

| Env | Default | Purpose |
|---|---|---|
| `TMUXGUI_HOST` | `127.0.0.1` | Bind host |
| `TMUXGUI_PORT` | `14101` | Bind port (`0` = dynamic, used by Electron) |
| `TMUXGUI_TMUX_SOCKET` | *(user default)* | tmux socket name (`-L`) or path (`-S`) |
| `TMUXGUI_LOG_LEVEL` | `info` | `debug` / `info` / `warn` / `error` |
| `TMUXGUI_SCROLLBACK_LINES` | `2000` | `capture-pane` history lines |

## Tests

```bash
make test
```

## Security

- Backend binds `127.0.0.1` by default — never `0.0.0.0`.
- WebSocket validates the `Origin` header.
- No CORS needed: prod FE is served by the backend, dev FE goes through the Vite proxy.
