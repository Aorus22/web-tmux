---
phase: "7"
slug: "resilience-packaging-parity-audit"
status: draft
shadcn_initialized: false
preset: none
created: "2026-09-07"
---

# Phase 7 — UI Design Contract (minimal)

> Resilience-surface contract for the reconnect banner + toast overlay.
> Minimal spec — executors follow 07-RESEARCH.md D1–D3 and the FE sources verbatim.

**Parity ground truth:**
- `fe/src/features/panes/PaneWorkspace.tsx:28-35,217-250` (banner copy + manual reconnect)
- `fe/src/lib/websocket.ts:27,126-132` (BACKOFF ladder + auto-retry)
- `fe/src/lib/commands.ts:17-19` (10s timeout; ~13 `toast.error` call sites for copy tone)
- Web-term `notification: Option<String>` banner precedent (overlay stacking shape)

---

## Design System

| Property | Value |
|----------|-------|
| Component library | Hand-rolled GPUI divs on `gpui-pre =0.3.3` (D1 — no `Root`/notification adoption) |
| Icon library | Embedded lucide consts (`icons.rs`, `stroke-width="1.5"`); kind icons prefix every toast (Error vs Info vs Reconnect) |
| Font | JetBrains Mono embedded (no new text furniture) |
| Hand-rolled rule | Banner + toast stack render as raw divs via the existing `.when()` overlay pattern (`app_state.rs:3379-3381`); colors resolve through the active `UiThemePreset` row per render (no caching) |

---

## Surfaces

### BANNER-1: Reconnect banner (STATE-03, D2)
- Full-width bar docked directly above the workspace (below title bar / tab strip), persistent
  while its state holds (never autohides).
- 4-way copy (fixed strings, tmux- prefix marks server-originated states):
  - WS-drop (transport.lost): `"Connection lost — retrying… (attempt N)"` + `[Reconnect]` button.
  - `tmux.reconnecting`: `"Reconnecting to tmux…"` (amber, transient, no button).
  - `tmux.disconnected`: `"tmux disconnected — waiting for tmux"` + `[Reconnect]` button.
  - Connected/recovered: banner unmounts + `"Reconnected"` success toast fires.
- `[Reconnect]` = manual close+ensure path (FE `reconnectSession` parity); attempt count
  increments per BACKOFF ladder step. Banner NEVER renders from bare `TransportState` —
  always (transport, origin).

### TOAST-1: Toast overlay stack (STATE-03, D1/D2)
- Bottom-right overlay stack (above workspace, below palette modal in z-order), max 5
  visible (drop-oldest), dedupe key per (kind+session).
- Kinds: error (command.error / timeout / `server.error`, backend-verbatim message as plain
  text, autohide 5s), success (`Session/Window created|renamed|killed`, `Pane renamed`,
  `Reconnected`, autohide 3s), info (transient reconnect notes, autohide 3s).
- Timings are UAT-tunable guesses (A1) — the 07-03 audit may adjust them.

---

## Tokens

Banner/toast chrome resolves through the active `UiThemePreset` row via `theme.rs` helpers
(error/red, warning/amber, success/green, surface/border tokens); backend-verbatim strings
render as plain text elements only, never markup (T-07-01).

## Assumptions

- A1: `.when()` overlay stacking composes banner + toast + palette without re-plumbing
  (palette overlay precedent `app_state.rs:3379-3381`).
- A2: Bottom-right stack does not collide with pane-header quick actions at small window
  sizes (audit row A3 watches this; move only on eyes-on evidence).
