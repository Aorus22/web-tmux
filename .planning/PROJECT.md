# Tmux GUI (web-tmux)

## What This Is

A graphical control surface for tmux so users never need to memorize tmux commands or
shortcuts. Tmux stays the source of truth for sessions, windows, panes, layouts, and
terminal processes — the app only adds a GUI layer on top. Ships as web (single Go
binary), Electron desktop, and native GTK4 desktop, with a mobile frontend in progress.

## Core Value

Full visual control over tmux (sessions, windows, panes, terminal I/O) while tmux
remains the single source of truth — no state duplication, no terminal backend of its own.

## Current Milestone: v1.0 Desktop GPUI

**Goal:** Add a native GPUI (Rust) desktop frontend that replicates the Electron UI
exactly, porting the proven desktop-gpui architecture from web-term.

**Target features:**
- `desktop-gpui/` Rust workspace (web-term pattern: supervisor + settings + backend-client + terminal + app crates)
- Go backend sidecar spawn (`tmux-gui-server --port 0`) + existing REST/WS protocol consumption — no backend changes expected
- Full UI parity with Electron: title bar, collapsible session→window→pane sidebar, multi-session tabs, window toolbar + layout presets, pane grid (split/zoom/resize/context menus), terminal rendering with scrollback/selection/clipboard
- Create Session dialog, command palette (Ctrl+Shift+P), Settings page (UI themes + terminal themes + tmux binary), empty/error/select states
- Theme system ported from Electron presets; embedded lucide SVG icons; Windows + Linux

## Requirements

### Validated

<!-- Shipped and confirmed valuable. Inferred from the existing codebase (brownfield bootstrap, pre-GSD history). -->

- ✓ Go backend: tmux control mode (`tmux -CC`), REST API, per-session WebSocket, embedded FE — pre-GSD
- ✓ Web frontend (React + Vite + Tailwind + shadcn/ui + xterm.js): sidebar tree, multi-session tabs, pane grid, terminal streaming — pre-GSD
- ✓ Electron desktop shell: spawns Go backend sidecar on a dynamic port, serves FE — pre-GSD
- ✓ Native GTK4 desktop frontend (Go + libvterm terminal) — pre-GSD
- ✓ Session/window/pane management: create/rename/kill, split/zoom/resize, layout presets, stable IDs (`@N`/`%N`) — pre-GSD
- ✓ UI theme system (web-term pattern presets) + terminal themes — pre-GSD
- ✓ Command palette (Ctrl+Shift+P) — pre-GSD
- ✓ Windows support: backend, GTK packaging, pipe-pane streaming, winget/MSYS2 tmux — pre-GSD

### Active

<!-- Current scope. Building toward these. -->

- [ ] Desktop GPUI frontend with UI identical to the Electron version (v1.0)

### Out of Scope

<!-- Explicit boundaries. Includes reasoning to prevent re-adding. -->

- macOS support — no macOS machine for testing; web-term's proven pattern covers Windows + Linux
- Replacing tmux or adding an app-owned terminal backend — PRD principle: tmux owns all state
- Duplicating tmux state in a database — PRD §5: no session/window/pane tables
- Backend protocol changes — frontend-only milestone; the existing REST/WS surface already serves three frontends

## Context

- Repo layout: `be/` (Go backend), `fe/` (React FE), `desktop/` (Electron shell),
  `desktop-gtk/` (Go GTK4 FE), `mobile/`, `scripts/`, `Makefile` (Windows + Linux aware)
- Protocol surface (consumed by all frontends):
  - REST: `/api/health`, `/api/tmux/info`, `/api/tmux/binary`, `/api/sessions` (GET tree, POST create), `/api/sessions/{name}/snapshot`
  - WS (one per open session tab): `hello`, `terminal.input/resize/capture`, `pane.select/split/resize/kill/rename/zoom/break/swap`, `window.select/create/rename/kill/layout/move/break-active`, `session.create/rename/kill`, `state.resync`; events `connection.ready`, `state.snapshot/delta`, `terminal.snapshot/output`, `command.success/error`, `tmux.disconnected/reconnecting`, `server.error`
- Reference implementation: `E:\Coding Stuff\web-term\desktop-gpui` — Rust workspace with
  crates `supervisor` (spawns Go backend), `settings`, `backend-client` (REST + WS),
  `terminal` (alacritty_terminal-based GPUI rendering), `webterm` (app/views/theme).
  Proven 1:1 with its web version on Windows + Linux.
- UI themes in `fe/src/features/settings/data/ui-themes.ts` (generated presets, dark +
  light counterparts) and `terminal-themes.ts` — the GPUI app must port these palettes.

## Constraints

- **Tech stack**: GPUI stack pinned exactly as web-term (`gpui-pre =0.3.3`,
  `gpui-component =0.6.0`, `alacritty_terminal =0.25.1`, tokio/reqwest(rustls)/tokio-tungstenite)
  — gpui is pre-1.0 with breaking-change churn; commit Cargo.lock
- **Compatibility**: UI must match the Electron version 1:1 — same layout, embedded
  lucide SVG icons, same theme palettes and interactions
- **Architecture**: desktop-gpui consumes the existing REST/WS protocol; backend changes
  only if a gap is discovered (log it, don't silently fork the protocol)
- **Platforms**: Windows + Linux (like web-term)

## Key Decisions

<!-- Decisions that constrain future work. Add throughout project lifecycle. -->

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Port web-term's desktop-gpui architecture (5-crate workspace, supervisor spawns backend) | Proven 1:1 GPUI port of a web UI on the exact same stack | ✓ Phase 1 — workspace + sidecar verified (16/16 tests) |
| Reuse the existing Go backend protocol unchanged | Frontend-only milestone; backend already serves Electron/GTK/web | — Pending (REST/WS consumption starts Phase 2) |
| Embed lucide SVG icons as GPUI assets | Icon parity with the Electron UI | — Pending (constants scaffolded Phase 1) |
| Env-only sidecar spawn: empty argv + `TMUXGUI_PORT=0` env config (matches Go os.Getenv contract), TMUX/TMUX_PANE stripped | Go backend reads config exclusively via os.Getenv; avoids argv contract drift | ✓ Phase 1 |
| Settings persisted atomically (NamedTempFile) with `.bak` corrupt recovery, isolated to `tmux-gui-desktop` dir | Prevents partial-write corruption and cross-frontend config collisions | ✓ Phase 1 |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-09-06 after Phase 1 (Workspace Foundation & Backend Sidecar)*
