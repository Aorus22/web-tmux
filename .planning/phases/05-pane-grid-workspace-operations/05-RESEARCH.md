# Phase 5: Pane Grid & Workspace Operations - Research

**Researched:** 2026-09-07
**Domain:** GPUI pane-grid geometry, divider drags, tmux pane/window mutations over WS, context menus + rename dialogs
**Confidence:** HIGH (protocol + geometry + FE parity all Read-verified this session; GPUI drag/viewport specifics MEDIUM)

## Summary

Phase 5 replaces the Phase-4 stacked terminal placeholder with a faithful tmux-geometry workspace: panes absolutely positioned from snapshot cell geometry via a verbatim port of `fe/src/lib/geometry.ts` (`pixelRect` + `closePaneGaps`), draggable dividers sending throttled incremental `pane.resize` steps with negative-flip, zoom-as-server-toggle, six layout presets from a window toolbar, full pane/window context menus, and pane/window rename dialogs cloning the proven Phase-3 DLG1 pattern (`gpui-component` Dialog + `InputState`). Every mutation rides the existing correlated `SessionWsHandle::send_command` path — no new crates, no backend changes, no protocol fork.

The single biggest scope finding: **PANE-05's "join" has no backend WS route and no Electron surface** (`pane.join` absent from `protocol.go`, `handler.go`, `websocket.ts`, and all of `fe/src` — zero grep hits). It is descoped to a logged backend gap (D8); everything else in PANE-05 exists 1:1. The second finding: **Phase 3 deliberately left Plus-button + chip context menu out of the title-bar tabs** ("No Plus new-window button and no chip context menu (D8 — Phase 5 owns window ops)" [VERIFIED: desktop-gpui/crates/webtmux/src/views/tab_strip.rs:1-9]), so PANE-08 extends `tab_strip.rs` in place. Third: `DesktopSettings.confirm_kill_pane/window` already exist with `default_true` (struct parity landed early) — only the AppState gates + dialogs are missing.

**Primary recommendation:** Port `geometry.ts` verbatim into a pure `pane_geometry` module with headless unit tests (closing the STATE.md window_state coverage gap in the same wave), render panes absolutely positioned inside the workspace div, implement divider drags on the proven `terminal_view.rs` mouse-move pattern with 40ms throttle + FLIP, and clone the rename/kill dialog + context-menu patterns exactly — all sends correlated, no optimistic flips, server snapshot as sole truth.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Pane pixel layout (cell → px mapping, gap closing, divider placement) | GPUI client (desktop-gpui) | — | Pure view derivation from snapshot geometry; tmux owns cells, client owns pixels |
| Pane/window mutations (split/zoom/resize/kill/rename/swap/break/layout/move) | Go backend (tmux CLI) | GPUI client (thin correlated send) | tmux is source of truth; client never applies optimistic geometry |
| Zoom state | Backend snapshot (`zoomed` flag) | — | No client-side zoom boolean; toggle command + snapshot truth avoids desync |
| Rename/kill dialogs, context menus, toolbar, headers | GPUI client | — | Chrome only; validated against FE item lists verbatim |
| Terminal viewport (`terminal.resize`) | GPUI client (existing Phase-4 dance) | — | Unchanged; divider drags must NOT send it directly (layout-key timers do) |

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PANE-01 | Panes render from tmux cell geometry (pixelRect + closePaneGaps), matching Electron | Geometry port (D1), absolute layout pattern, divider adjacency in cell space |
| PANE-02 | Split pane right/down via header buttons or context menu, stable pane ID | Split direction mapping + correlated send pattern (D5) |
| PANE-03 | Zoom fills workspace; next action restores | Zoom-as-server-toggle, zoomed-filter render (D4) |
| PANE-04 | Divider drags: incremental cell steps, ~40ms throttle, negative flip, no storms | Drag state machine + FLIP + throttle (D3), resize-dance interplay pitfall |
| PANE-05 | Pane menu kills/renames/swaps/breaks/joins | Menu item list verbatim; **join descoped** (D8, no WS route) |
| PANE-06 | Layout presets (even-h, even-v, main-h, main-v, tiled, next) from window toolbar | Preset values + toolbar shape verbatim (D7) |
| PANE-07 | Pane headers: path, pane ID, TUI-scroll switch, quick actions + tooltips | Header contents verbatim; TUI map already on AppState (D7) |
| PANE-08 | Title-bar tab menu renames/moves/breaks/kills windows | tab_strip extension (D8 decision in Phase 3 left this to Phase 5) |
| DLG-02 | Every rename flow opens a dialog with a real text input | DLG1 clone pattern for pane + window (D6) |

(No CONTEXT.md exists for Phase 5 yet — no discuss-phase constraints to honor. D1–D10 below are auto-accepted autonomous decisions, flagged for post-hoc audit per the Phase-4 precedent.)

## Locked Decisions (auto-accepted, autonomous mode)

- **D1 — Verbatim geometry port into a pure module.** New `pane_geometry` module in the `webtmux` crate ports `pixelRect`, `closePaneGaps`, `pxToCells`, `resizeDragStep` (and divider adjacency) as pure functions over `i64` cell coordinates, with headless `cargo test` coverage. No GPUI types inside — the STATE.md window_state coverage gap is closed in the same test wave (D9).
- **D2 — Absolute positioning, not flex tiling.** Panes render as absolutely positioned children of the workspace div at their `pixelRect` rects (FE `PaneWorkspace.tsx:353-360` parity). Zoomed window renders only the zoomed pane full-size with dividers hidden.
- **D3 — Divider drag state machine with 40ms throttle + FLIP.** `mouse_down` records `(pane_id, direction, start_px, last_cells=0, last_sent)`; `mouse_move` (proven `terminal_view.rs:345-379` pattern) computes the incremental step, drops `step==0`, drops sends inside the 40ms window **without** advancing `last_cells`; negative steps flip direction with positive amount. Steps ride correlated `pane.resize` with the receiver dropped (no per-step await/task spam); truth follows via snapshot deltas.
- **D4 — Zoom is a server toggle; no client zoom state.** Header button, header double-click, and menu Zoom all send `pane.zoom`; render filters `panes.find(zoomed)` → `[zoomed]`. Zoom button hidden for single-pane windows (`canZoom = otherPanes.len() > 0`).
- **D5 — Correlated sends for everything except select + drag steps.** All pane/window mutations use `SessionWsHandle::send_command` + 10s correlated await with inline error (the `submit_rename`/`await_rename_result` shape); `pane.select`/`window.select` and drag steps are fire-and-forget correlated sends (receiver dropped — pending entry cleans itself on reply). Never send `hello`. Never optimistic-flip UI.
- **D6 — Rename dialogs clone DLG1 exactly.** One form entity per dialog on `AppState`, `has_active_dialog` double-open guard, `InputState` prefill + autofocus, Enter submits, `is_submitting` double-submit guard, reset-on-dismiss, inline error line. Prefill: pane → `title || current_command`; window → `name`. Gate is trim + non-empty only (no session-name validation — tmux titles accept anything).
- **D7 — Menus/toolbar/headers copy FE item lists verbatim.** Pane menu (9 rows incl. separators), window tab menu (6 rows), toolbar (5 presets + Next), header (path + id + TUI switch + 4 actions). Kill entries honor `confirm_kill_pane`/`confirm_kill_window` via new AppState gates mirroring `kill_requires_confirm`. TUI switch reads/writes the existing `AppState::tui_scroll` map (default ON); no persistence in Phase 5 (Phase 6 owns it).
- **D8 — "Join" descoped; window ops extend `tab_strip.rs`.** No `pane.join` WS route and no FE join surface exist, and backend changes are out of scope — join is logged as a backend gap, not invented. PANE-08 adds the Plus button (`window.create`, no args) and chip context menus to the existing title-bar tabs.
- **D9 — Coverage lands with the geometry wave.** New headless tests: geometry module (pixelRect scale, gap absorption, drag-step incrementality, FLIP), `window_state.rs` clamp-guard (the STATE.md-flagged missing `window_state.rs` test: 800..3840 / 500..2160 clamps, -10000 sentinel, maximized preservation), pane/window kill-gate tests.
- **D10 — Nine lucide icons appended to `icons.rs`.** SplitSquareHorizontal, SplitSquareVertical, Maximize2, Columns2, Rows2, SquareSplitHorizontal, SquareSplitVertical, LayoutGrid, ArrowRightLeft (stroke-width 1.5 to match). Tooltips via gpui-component tooltip if present in 0.6.0, else hover-reveal label — planner verifies against the vendored source [ASSUMED].

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `gpui` (`gpui-pre`) | `=0.3.3` [VERIFIED: desktop-gpui/Cargo.toml:33] | Pane grid views, mouse drag events, dialogs | Milestone-pinned; pre-1.0 churn is why the pin + committed Cargo.lock exist |
| `gpui-component` | `=0.6.0` [VERIFIED: desktop-gpui/Cargo.toml:34] | Dialog + InputState (rename), ContextMenuExt + PopupMenuItem (menus) | Already proven by Phase-3 rename/kill dialogs and sidebar menus — same imports reused |
| `webtmux-backend-client` | workspace path | `MSG_PANE_*`/`MSG_WINDOW_*` consts, `WsIncoming`, `SessionWsHandle::send_command` | Envelope shapes mirror `protocol.go` verbatim; correlation + forget-timeout built in |
| `webtmux-terminal` | workspace path | `TerminalView` instances rendered inside each positioned pane | Phase-4 engine; Phase 5 only re-hosts views in geometry, never touches emulation |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `webtmux-settings` | workspace path | `confirm_kill_pane/window` flags (already `default_true`) | Kill-confirm gates for pane/window kills |
| `tokio` (time) | `=1.53.1` | 40ms drag throttle clock (`Instant::now`), 10s correlated await | Drag state machine + dialog awaits |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Absolute positioning | Flex/box tiling computed client-side | Rejected: tmux geometry is irregular (T-layouts, uneven splits); only absolute rects reproduce it exactly |
| Per-step drag await | Fire-and-forget drag steps (chosen, D3) | Awaiting every 40ms step serializes the drag behind 10s timeouts on a dead socket; dropped receivers self-clean via the pending map |
| Client zoom boolean | Server-toggle + snapshot truth (chosen, D4) | A client flag desyncs on external tmux changes (CLI zoom); snapshot is the only truth |

**Installation:**

```bash
# No new packages. Pins unchanged; lockfile committed.
cargo build -p webtmux
```

**Version verification:** No new dependencies are introduced (all four crates + `gpui-pre =0.3.3`, `gpui-component =0.6.0` already in `desktop-gpui/Cargo.toml:24-55` [VERIFIED] and `Cargo.lock` committed per Phase 1). Nothing to `cargo search`.

## Package Legitimacy Audit

> No external packages are installed by this phase. All imports resolve to the milestone-pinned workspace (`gpui-pre =0.3.3`, `gpui-component =0.6.0`, `alacritty_terminal =0.25.1`, tokio/serde/reqwest/rustls) verified in `desktop-gpui/Cargo.toml:24-55` [VERIFIED]. The `gsd_run query package-legitimacy` seam is not on PATH in this environment, but the gate is vacuous: zero new package names appear anywhere in this research.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| *(none — no new packages)* | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```text
tmux (source of truth)
  │  control-mode + one-shot queries
  ▼
Go backend ──state.snapshot/delta──▶ per-session WS socket ──▶ SessionWsHandle
  │                                        │ split-pump + generation guard (Ph3)
  │                                        ▼
  │                                   AppState::apply_event
  │                                    (snapshot → windows/panes;
  │                                     terminal arms → TerminalStore — Ph4)
  │                                        │
  │                                        ▼
  │                              WorkspaceView (active session)
  │                                ├── WindowToolbar (layout presets)
  │                                ├── PaneGrid ◀── pane_geometry::pixel_rect
  │                                │     ├── PaneView × N (header + TerminalView)
  │                                │     └── DividerHandle × M (40ms drag → pane.resize)
  │                                └── TitleBar tabs (+ Plus, chip menus — Ph3 D8)
  │
  └── Mutations (all correlated, no optimistic flip):
      pane.select/split/resize/kill/rename/zoom/break/swap
      window.select/create/rename/kill/layout/move/break-active
```

A reader traces the primary use case: snapshot arrives → filter `panes` by `activeWindow` → `close_pane_gaps` → `pixel_rect` per pane → absolute children + divider handles → drag sends `pane.resize` → tmux reflows → new delta re-renders.

### Recommended Project Structure

```text
desktop-gpui/crates/webtmux/src/
├── pane_geometry.rs      # NEW (D1): pixel_rect, close_pane_gaps, px_to_cells,
│                         #   resize_drag_step, divider_layout — pure, headless-tested
├── views/
│   ├── pane_grid.rs          # NEW: workspace grid (zoom filter, positioned panes, dividers)
│   ├── pane_view.rs          # NEW: header (path/id/TUI/actions) + TerminalView host
│   ├── pane_context_menu.rs  # NEW: 9-row menu + swap picker + rename/kill dialogs
│   ├── rename_pane_dialog.rs # NEW: DLG1 clone (D6)
│   ├── rename_window_dialog.rs # NEW: DLG1 clone (D6)
│   ├── window_toolbar.rs     # NEW: 6 preset buttons (D7)
│   ├── tab_strip.rs          # EXTEND: Plus button + chip context menus (Ph3 D8)
│   ├── terminal_view.rs      # REUSE: untouched engine; mouse-move pattern reused for drags
│   ├── rename_session_dialog.rs  # REUSE: pattern source for D6
│   └── session_context_menu.rs   # REUSE: pattern source for menus + kill confirms
└── app_state.rs              # EXTEND: pane/window correlated sends + kill gates (D5/D7)
```

### Pattern 1: Verbatim geometry port (pure functions, i64 cells)

**What:** Port `fe/src/lib/geometry.ts` line-for-line into `pane_geometry.rs`, operating on `i64` internally (Rust `TmuxPane.left/top/width/height` are `usize` [VERIFIED: desktop-gpui/crates/backend-client/src/models.rs:94-97] — subtraction like `p.left -= 1` and `pos - start` underflows `usize`; convert at the boundary).
**When to use:** All layout math, divider adjacency, drag-step computation.
**Example:**

```rust
// Source: fe/src/lib/geometry.ts:48-55 ("scaleX = containerWidth / windowWidth ...")
// and geometry.ts:88-114 (closePaneGaps), PaneWorkspace.tsx:283-345 (dividers)
pub struct CellPane { pub id: String, pub left: i64, pub top: i64, pub w: i64, pub h: i64 }
pub struct PxRect { pub left: f32, pub top: f32, pub w: f32, pub h: f32 }

pub fn pixel_rect(p: &CellPane, cw: f32, ch: f32, ww: i64, wh: i64) -> PxRect {
    if ww <= 0 || wh <= 0 { return PxRect { left: 0., top: 0., w: 0., h: 0. }; }
    let (sx, sy) = (cw / ww as f32, ch / wh as f32);
    PxRect { left: (p.left as f32 * sx).round(), top: (p.top as f32 * sy).round(),
             w: (p.w as f32 * sx).round(), h: (p.h as f32 * sy).round() }
}
// close_pane_gaps: absorb 1-cell strips — `q.left + q.width + 1 == p.left`
// (+ vertical overlap) → `p.left -= 1; p.width += 1`; same for top.
// Dividers: vertical iff `b.left == a.left + a.width` + vertical overlap
// (direction 'R', handle `left = a.rect.left + a.rect.width - 2`, w=4,
// cursor col-resize); horizontal iff `b.top == a.top + a.height` +
// horizontal overlap (direction 'D', `top = ... - 2`, h=4, cursor row-resize).
// Drag step: `px_to_cells(pos - start, cell) - last_cells`; throttle 40ms;
// `step < 0 → (FLIP[dir], abs(step))`.
```

### Pattern 2: Correlated mutation sends (copy `send_window_select` + `submit_rename`)

**What:** Fire-and-forget selects vs. awaited mutating commands, both via `handle.send_command` on the owning session's socket.
**When to use:** Every pane/window action (D5).
**Example:**

```rust
// Source: crates/webtmux/src/app_state.rs:1447-1460 (fire-and-forget select)
// and app_state.rs:1510-1529 + 1535-1559 (correlated rename + 10s await)
let msg = WsIncoming { msg_type: MSG_PANE_SPLIT.to_string(), pane_id: Some(p.clone()),
    direction: Some("horizontal".to_string()), ..Default::default() };
let rx = handle.send_command(msg).map_err(|e| e.to_string())?; // await w/ timeout,
// or `let _ = handle.send_command(msg);` for select + drag steps.
```

### Pattern 3: DLG1 dialog clone + stable-id context menus

**What:** `window.has_active_dialog` guard → fresh form entity on `AppState` → `open_dialog` with `DialogTitle` + `Input::new(&input_state)` + footer → `on_close` clears the form; menus via `row.context_menu(...)` with stable ids.
**When to use:** Pane/window rename dialogs, kill confirms, swap picker, all context menus (D6/D7).
**Example:**

```rust
// Source: views/rename_session_dialog.rs:41-126 (open guard + prefill + Enter via
// InputEvent::PressEnter) and views/session_context_menu.rs:33-66
// (row.context_menu with PopupMenuItem::new("Rename") / destructive red Kill).
```

### Anti-Patterns to Avoid

- **Client-side zoom/layout state:** never store `zoomed` or assume a layout result — send the toggle and render the next snapshot. External tmux clients invalidate any local copy instantly.
- **Cumulative drag amounts:** never send total `pos - start` displacement — tmux applies each resize on top of current size, so cumulative sends overshoot and stick at limits (FE `geometry.ts:133-146` documents this; always subtract `last_cells`).
- **`usize` geometry arithmetic:** `p.left - 1`, `pos - start` panic on underflow in debug — convert snapshot `usize` cells to `i64` at the module boundary.
- **Per-step drag awaits:** never `await` each 40ms step (task spam + 10s-timeout serialization on dead sockets) — drop the receiver; the pending map self-cleans.
- **`terminal.resize` from drag code:** divider drags send only `pane.resize`; the Phase-4 layout-key timers (150/325ms) own viewport resync. Sending both double-resizes tmux mid-drag.
- **Unstable menu ids:** never embed indices/counters in context-menu element ids — the 1.5s poll re-render drops open menus (Phase-3 lesson; use `pane-menu/%N`, `window-tab/@N`).
- **Inventing `pane.join`:** no WS route exists — do not craft raw tmux or a new message type; log the backend gap (D8).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Dialog + text input | Custom modal/input | `gpui-component` Dialog + `InputState`/`InputEvent::PressEnter` (DLG1 clone) | Focus, Enter handling, double-submit guard, dismiss-reset already proven in `rename_session_dialog.rs` |
| Context menus | Custom popup positioning | `ContextMenuExt::context_menu` + `PopupMenuItem` (+ `.separator()`) | Proven in `session_context_menu.rs`; stable-id discipline included |
| WS correlation | Custom requestId bookkeeping | `SessionWsHandle::send_command` + 10s await | Pending-map insert-before-enqueue (no lost wakeup), forget-timeout, late-reply ignore all built in |
| Cell geometry math | New layout algorithm | Verbatim `geometry.ts` port | 1:1-with-Electron is the milestone claim; any deviation is a parity bug by definition |
| Kill confirmation | Ad-hoc confirms | `confirm_kill_pane/window` + DLG1-token confirm dialogs | FE defaults are `true` [VERIFIED: fe/src/stores/settingsStore.ts:42-43] and Rust already carries `default_true` [VERIFIED: crates/settings/src/lib.rs:66-71] |
| Raw tmux access | Any client-side tmux string | Typed WS dispatch allowlist | Server rejects unknown types (`"unknown message type"` [VERIFIED: be/internal/realtime/handler.go:203-205]) — injection is structurally impossible |

**Key insight:** Phase 5 is a port, not an invention task. Every behavior — formulas, throttle, item lists, prefill sources, confirm gates — already exists verbatim in `fe/` and every transport pattern already exists in `desktop-gpui/`. Novel code is confined to GPUI view composition and the drag state machine.

## Common Pitfalls

### Pitfall 1: Split-direction naming trap (`horizontal` = side-by-side)

**What goes wrong:** Sending `"vertical"` for a "Split right" button (intuition: a vertical divider line).
**Why it happens:** tmux `-h` (FE `"horizontal"`) splits left/right; `-v` (`"vertical"`) splits top/bottom — the flag names the *new layout axis*, not the divider.
**How to avoid:** Copy the mapping verbatim — Split right → `"horizontal"` [VERIFIED: fe/src/features/panes/PaneHeader.tsx:100], Split down → `"vertical"` (default param) [VERIFIED: PaneHeader.tsx:41]; backend: `"vertical" → -v, else -h` [VERIFIED: be/internal/tmux/command.go:184-196] with the source comment `"horizontal" = split left/right (tmux -h), "vertical" = top/bottom (-v)`.
**Warning signs:** Splits appear rotated 90° vs Electron in the parity audit.

### Pitfall 2: Window targets ride the `paneId` field

**What goes wrong:** Adding a `window_id` JSON field the server ignores, so window ops silently target nothing.
**Why it happens:** `Incoming` has no `windowId` — every window command reads `in.PaneID` for the `@N` id.
**How to avoid:** Always set `pane_id: Some("@N")` for `window.select/create(n/a)/rename/kill/layout/move/break-active` [VERIFIED: be/internal/realtime/handler.go:167-180]; `WsIncoming.pane_id` already exists [VERIFIED: crates/backend-client/src/ws.rs:70-71].
**Warning signs:** `command.error` / no-op on every window action.

### Pitfall 3: Drag storms via layout-key feedback

**What goes wrong:** Each `pane.resize` step changes geometry → layout-key effect re-fires → extra `terminal.resize`/capture spam mid-drag.
**Why it happens:** FE semantics: layout-key change arms 150ms resize + 325ms recapture; during an active drag the timers keep resetting and only fire 150/325ms after the *last* step — self-debouncing — but only if the planner replicates the reset-on-change structure instead of firing unconditionally per snapshot.
**How to avoid:** Replicate the effect shape (`prevLayoutKey` compare, clear-and-rearm timers) from `PaneWorkspace.tsx:172-200`; skip resync on first mount (each TerminalView already captures initially).
**Warning signs:** tmux `refresh-client` floods in logs while dragging; capture tearing.

### Pitfall 4: `hello` is a resize, not a handshake — never send it

**What goes wrong:** Sending `hello` on socket open or layout change unexpectedly resizes the tmux session.
**Why it happens:** Name suggests handshake; server treats `cols/rows > 0` as `ResizeTerminal` [VERIFIED: be/internal/realtime/handler.go:104-110].
**How to avoid:** Phase-3 D7 / Phase-4 D4 ban stands — answer `connection.ready` with `state.resync` only [VERIFIED: crates/backend-client/src/ws.rs:454-464].
**Warning signs:** Session geometry jumps on tab switch.

### Pitfall 5: `next-layout` is opaque — pass through verbatim

**What goes wrong:** "Validating" the preset list down to tmux's five documented layout names and dropping `next-layout`.
**Why it happens:** tmux docs list five `select-layout` names; `next-layout` looks like a different command.
**How to avoid:** FE sends all six strings (`even-horizontal`, `even-vertical`, `main-horizontal`, `main-vertical`, `tiled`, `next-layout` [VERIFIED: fe/src/features/windows/WindowToolbar.tsx:19-25,68]) through `window.layout` [VERIFIED: websocket.ts:282-286] and the backend passes `in.Layout` straight into `select-layout` [VERIFIED: be/internal/tmux/command.go:178-180]. Replicate the bytes; surface any `command.error` via toast. tmux-side acceptance is backend domain [ASSUMED].
**Warning signs:** "Next Layout" button missing or filtered in review.

### Pitfall 6: Swap/break need same-window pane lists, not global

**What goes wrong:** Swap picker lists panes from other windows; `swap-pane` across windows misbehaves.
**Why it happens:** `snapshot.panes` is flat across the session — filtering by `windowId == activeWindowId` is the view's job [VERIFIED: fe/src/features/panes/PaneWorkspace.tsx:116; PaneView.tsx:30-32].
**How to avoid:** Filter `otherPanes` to the same window; disable Swap when empty [VERIFIED: fe/src/features/panes/PaneContextMenu.tsx:139].
**Warning signs:** Swap dialog shows panes from background windows.

## Code Examples

Verified patterns from in-repo sources:

### Correlated pane split (stable `%N` target, direction vocabulary)

```rust
// Source: be/internal/realtime/handler.go:144-149 + be/internal/tmux/command.go:184-196
// + fe/src/lib/websocket.ts:216-220
// direction: "horizontal" → split-window -t %N -h (side-by-side)
//            "vertical"   → split-window -t %N -v (top/bottom); "" defaults to "horizontal"
let msg = WsIncoming {
    msg_type: MSG_PANE_SPLIT.to_string(),       // "pane.split" [ws.rs:19]
    pane_id: Some("%3".to_string()),
    direction: Some("horizontal".to_string()),
    ..Default::default()
};
```

### Throttled incremental resize with flip (exact FE semantics)

```rust
// Source: fe/src/features/panes/PaneResizeHandle.tsx:30-57,80-94
// + fe/src/lib/geometry.ts:139-146
const FLIP: Record<Dir, Dir> = { L: 'R', R: 'L', U: 'D', D: 'U' };
// per move: step = px_to_cells(pos - start, cell) - last_cells;
//   if step == 0 → return; if now - last_sent < 40 → return (do NOT advance last_cells);
//   last_sent = now; last_cells += step;
//   (dir, amount) = step < 0 ? (FLIP[direction], -step) : (direction, step);
//   → pane.resize { paneId, direction: 'L'|'R'|'U'|'D', amount }
```

### Zoom toggle + zoomed-only render

```rust
// Source: be/internal/tmux/command.go:211-213 (`resize-pane -Z -t <target>` toggles)
// + fe/src/features/panes/PaneWorkspace.tsx:119-120 + 361
// zoomed = panes.find(p => p.zoomed); visible = zoomed ? [zoomed] : panes;
// dividers render only when !zoomed. Zoom triggers: header button, header
// double-click (PaneHeader.tsx:67), context-menu Zoom (PaneContextMenu.tsx:138).
// Backend zoom truth: Zoomed = window_zoomed_flag=="1" && pane_active=="1"
// [be/internal/tmux/snapshot.go:244-247].
```

### Rename prefill + Enter-submit (DLG-02)

```rust
// Source: fe/src/features/panes/PaneContextMenu.tsx:129-136,180-206 (pane)
// + fe/src/features/windows/WindowTabs.tsx:151-158,192-218 (window)
// + desktop-gpui/crates/webtmux/src/views/rename_session_dialog.rs:56-85 (GPUI host pattern)
// pane prefill:   pane.title || pane.currentCommand || ''
// window prefill: w.name
// dialog: Title "Rename Pane"/"Rename Window", Label "New name", autofocused Input,
// Enter submits, Cancel + Rename (disabled when empty/busy).
// sends: pane.rename { paneId: %N, title } / window.rename { paneId: @N, name }.
```

### Layout preset apply (toolbar)

```rust
// Source: fe/src/features/windows/WindowToolbar.tsx:19-37,40-77
// + fe/src/lib/websocket.ts:282-286
// LAYOUTS = even-horizontal | even-vertical | main-horizontal | main-vertical | tiled
// (+ Next Layout → 'next-layout'); bar is h-8 with icon buttons + separator.
// apply: window.layout { paneId: activeWindowId(@N), layout } — correlated.
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Phase-4 stacked placeholder body | tmux-geometry absolute grid (this phase) | Phase 5 | Real multi-pane workspace; Phase 4 terminals re-hosted unchanged |
| Session-only context menus / rename | + pane + window-tab menus / renames | Phase 5 | DLG-02 complete for all three rename flows |
| Title-bar tabs without Plus/menu (Ph3 D8) | Tabs + Plus + chip menus | Phase 5 | PANE-08; window.create/rename/move/break-active/kill wired |
| `kill_requires_confirm` reads only session flag | + pane/window gates | Phase 5 | SESS-05 pattern extended; struct flags already exist |

**Deprecated/outdated:**
- Nothing deprecated. Phase-4 resize dance, capture-replay, TUI map, title store all reused untouched.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `next-layout` is accepted tmux-side via the `select-layout` passthrough (same bytes as Electron, so parity holds regardless) | Pitfall 5 | LOW — identical-to-Electron bytes; any failure surfaces as `command.error` toast in both |
| A2 | Divider pointer capture: GPUI `on_mouse_move` streams while the button is held (as `terminal_view.rs` selection-drag relies on), so no explicit pointer-capture API is needed for dividers | Patterns/D3 | MEDIUM — if moves stop outside the 4px handle, add capture; planner adds a verification step dragging fast past the handle |
| A3 | gpui-component 0.6.0 tooltip availability for header/toolbar tooltips; fallback is hover-reveal label | D10 | LOW — cosmetic; planner checks vendored source first |
| A4 | Drag-step sends with dropped receivers are safe (pending map self-cleans on reply; forget-timeout drops late entries) | D3/D5 | LOW — same map Phase 3 stress-tested; consider a bounded smoke test |
| A5 | Reference `web-term/desktop-gpui` grid code not consulted (subagent quota); FE + Phase-3/4 in-repo patterns suffice | Sources | LOW — FE is the parity authority, not web-term |
| A6 | `window.move` offsets are exactly `-1`/`+1` for menu Move Left/Right | PANE-08 | LOW — FE sends `±1` [VERIFIED: WindowTabs.tsx:159-164]; tmux `move-window %+d` is general |

## Open Questions

1. **PANE-05 "join" wording vs. no implementation surface**
   - What we know: zero `join` hits in `fe/src`; no `pane.join` in `protocol.go`/`handler.go`/`websocket.ts`/`protocol.ts` (grep-verified); `cmdJoinPane` exists server-side [VERIFIED: be/internal/tmux/command.go:227-229] but unreachable. Backend changes are out of scope (PROJECT.md).
   - What's unclear: whether the roadmap author intended `join-pane` (pull another pane into this window) or mislabeled `break-pane` (which exists in both menus).
   - Recommendation: D8 descopes join; planner logs a backend-gap issue (`pane.join` WS route) and implements kill/rename/swap/break exactly like Electron. Flag in verification for post-hoc user audit.

2. **Kill-confirm toggles have no Settings UI until Phase 6**
   - What we know: `confirm_kill_pane/window` default `true` in both stores; Phase 6 (SET-04/DLG-03) builds the switches.
   - Recommendation: Phase 5 reads the flags (correct defaults) without any toggle UI — same as Phase 3 did for the session flag.

3. **TUI-scroll switch persistence**
   - What we know: `AppState::tui_scroll` map defaults ON [VERIFIED: app_state.rs:858-861]; FE persists per-pane in localStorage; Rust has no equivalent field yet.
   - Recommendation: in-memory map only in Phase 5 (matches D3 lock); persistence arrives with Phase 6 SET-02.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo + rustc | All builds/tests | ✓ | 1.96.0 | — |
| Go backend sidecar (`tmux-gui-server`) | Runtime snapshots/mutations | ✓ (Phase 1 verified) | — | — |
| tmux (via backend) | Geometry/commands | ✓ (backend domain) | — | — |
| New external tools/packages | — | n/a | — | — |

**Missing dependencies with no fallback:** none
**Missing dependencies with fallback:** none

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (workspace; existing per-crate `tests/` dirs) |
| Config file | none (cargo workspace default) |
| Quick run command | `cargo test -p webtmux pane_geometry` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PANE-01 | pixelRect scaling + zero-dim guard | unit | `cargo test -p webtmux pane_geometry::pixel_rect` | ❌ Wave 0 (new `pane_geometry.rs` + tests) |
| PANE-01 | closePaneGaps 1-cell absorption, order-independence, edge invariance | unit | `cargo test -p webtmux pane_geometry::close_gaps` | ❌ Wave 0 |
| PANE-01 | Divider adjacency incl. T-layout overlap, non-adjacent exclusion | unit | `cargo test -p webtmux pane_geometry::dividers` | ❌ Wave 0 |
| PANE-04 | Incremental drag steps (last_cells subtraction), 40ms throttle accounting, FLIP map | unit | `cargo test -p webtmux pane_geometry::drag_step` | ❌ Wave 0 |
| PANE-04 | No `terminal.resize` emitted from drag path (code review gate) | review | — (planner adds checklist item) | n/a |
| PANE-02/03/05/06/08 | Correlated WS envelopes (`pane.split` direction strings, `paneId`-carried `@N`, layout strings) | unit (serde) | `cargo test -p webtmux-backend-client ws_pane_window` | ❌ Wave 0 (extend `ws_test.rs`) |
| PANE-05/08 | Kill gates read pane/window flags (mirror `kill_confirm_test.rs`) | unit | `cargo test -p webtmux --test kill_confirm_test` (extend) | ✅ extend existing |
| DLG-02 | Rename prefill sources + non-empty gate (pure helpers) | unit | `cargo test -p webtmux rename_prefill` | ❌ Wave 0 |
| STATE (gap) | window_state clamp guards (800..3840, 500..2160, -10000 sentinel, maximized preservation) | unit | `cargo test -p webtmux --test window_state_test` | ❌ Wave 0 (STATE.md-flagged missing file) |
| PANE-01..08, DLG-02 | Pixel parity vs Electron, dialog typing, drag feel, fast-drag capture | manual/HV | Phase-7 parity audit (GPUI not headless-renderable — Ph2/Ph4 precedent) | deferred HV |

### Sampling Rate

- **Per task commit:** `cargo test -p webtmux pane_geometry && cargo test -p webtmux-backend-client`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `crates/webtmux/src/pane_geometry.rs` + `#[cfg(test)]` (or `tests/pane_geometry_test.rs`) — covers PANE-01/PANE-04 math
- [ ] `crates/webtmux/tests/window_state_test.rs` — covers STATE.md clamp-guard gap (pure `extract_window_state`/`restore` paths that don't need a `Window`)
- [ ] `crates/backend-client/tests/ws_pane_window_test.rs` (or extend `ws_test.rs`) — envelope shapes for all 15 pane/window commands
- [ ] Extend `crates/webtmux/tests/kill_confirm_test.rs` — pane/window gates
- [ ] Rename prefill/gate helpers factored pure for headless test (else manual-only with justification)

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Local single-user desktop; no auth surface |
| V3 Session Management | no | No app sessions (tmux names are identifiers, not credentials) |
| V4 Access Control | no | No multi-user boundary; typed WS allowlist is the control (below) |
| V5 Input Validation | yes | Rename inputs: trim + non-empty gate; no session-name rules for titles; typed commands only — client can never send raw tmux (server `default:` → `server.error` [VERIFIED: be/internal/realtime/handler.go:203-205]) |
| V6 Cryptography | no | No crypto in this phase (`ws://` loopback sidecar is Phase-1 architecture) |

### Known Threat Patterns for GPUI pane grid

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Command injection via rename title (e.g. `"; kill-pane"`) | Tampering | Structurally impossible: title travels as JSON `title` field into `select-pane -t %N -T <title>` argv (no shell) — typed dispatch, never string-concatenated |
| Geometry-driven panic (zero/negative dims, `usize` underflow) | Denial of service | `ww<=0\|\|wh<=0 → zero rect` guard + `i64` internals + `cols.max(2)/rows.max(1)` clamps (FE + Ph4 precedent) |
| Drag-command flood | Denial of service | 40ms throttle + `step==0` drop + correlated pending-map self-clean |

## Sources

### Primary (HIGH confidence — Read this session, quotes beside claims)

- `be/internal/realtime/protocol.go:58-97` — all 24 command/event strings: `"pane.select"`, `"pane.split"`, `"pane.resize"`, `"pane.kill"`, `"pane.rename"`, `"pane.zoom"`, `"pane.break"`, `"pane.swap"`, `"window.select"`, `"window.create"`, `"window.rename"`, `"window.kill"`, `"window.layout"`, `"window.move"`, `"window.break-active"`, … (no `pane.join`)
- `be/internal/realtime/handler.go:103-205` — dispatch: split default `"horizontal"`, window targets via `in.PaneID`, `hello` = resize, unknown → `server.error`
- `be/internal/tmux/command.go:174-229` — `move-window -t <t> %+d`; `select-layout -t <t> <layout>` passthrough; `"vertical" → -v, else -h`; `resize-pane -Z -t`; `resize-pane -t <t> -<DIR> <amt>`; `swap-pane -s/-t`; `break-pane -t`
- `be/internal/tmux/snapshot.go:19,244-247` — `PaneFormat` field order; `Zoomed: p[4] == "1" && p[3] == "1"`
- `be/internal/tmux/model.go:14-43` + `fe/src/lib/tmux-types.ts:12-45` — Window/Pane/snapshot shapes (`@N`/`%N`, cells, `layout`, `title`, `currentPath`)
- `fe/src/lib/geometry.ts:23-146` — `CELL_W = 8`, `CELL_H = 18`, `pixelRect` scale formula, `closePaneGaps` ±1 absorption, `pxToCells`, `resizeDragStep` incrementality
- `fe/src/features/panes/PaneWorkspace.tsx:116-200,261-370` — flat-list filter, zoom filter, 100/150/325ms timers, `layoutKey` fields, divider adjacency + 4px/`-2` handles + `cellPx`, `!zoomed` divider gate
- `fe/src/features/panes/PaneResizeHandle.tsx:30-94` — `FLIP = { L:'R', R:'L', U:'D', D:'U' }`, negative→flip+abs, `step==0` drop, `now - lastSent < 40` throttle that doesn't advance `lastCells`
- `fe/src/features/panes/PaneHeader.tsx:41-47,66-167` — h-7 header, `currentPath` + mono `id`, TUI switch (`?? true`), Split right/horizontal + Split down/vertical + Zoom(`canZoom`) + Kill with tooltips, double-click zoom
- `fe/src/features/panes/PaneContextMenu.tsx:123-206` — 9-row menu order, swap picker (`currentCommand || title || currentPath`, disabled when empty), rename prefill `title || currentCommand || ''`, kill confirm
- `fe/src/features/panes/PaneView.tsx:50-72` — absolute positioned, active border, `!pane.active → paneSelect`, `canZoom = otherPanes.length > 0`
- `fe/src/features/windows/WindowToolbar.tsx:19-77` + `LayoutSelector.tsx:32-52` — six preset strings incl. `'next-layout'`, h-8 bar, `window.layout { paneId: @N, layout }`
- `fe/src/features/windows/WindowTabs.tsx:56-218` + `WindowContextMenu.tsx:34-43` — Plus → `windowCreate()`, tab menu rows, move `±1`, rename prefill `w.name`, kill confirm
- `fe/src/lib/websocket.ts:210-298` + `fe/src/lib/protocol.ts` (MSG/EV) + `fe/src/lib/commands.ts:19-50` — correlated `runCommand` + 10s timeout, `shouldConfirm('pane'|'window'|'session')`
- `fe/src/stores/settingsStore.ts:33-45` — `tuiScrollPanes: {}`, all three `confirmKill*: true`
- `desktop-gpui/crates/backend-client/src/ws.rs:13-94` — MSG_/EV_ consts verbatim, `WsIncoming` fields (`pane_id`, `direction`, `amount: Option<i32>`, `title`, `other_pane_id`, `layout`, `name`), `send_command` + split-pump + resync-on-ready
- `desktop-gpui/crates/backend-client/src/models.rs:73-102` — `TmuxWindow`/`TmuxPane` (`usize` cells — motivates D1 `i64`)
- `desktop-gpui/crates/webtmux/src/app_state.rs:1447-1460,1510-1559` — `send_window_select`, correlated rename + 10s await; `:415-462,682-683,858-861` — `tui_scroll` map + `unwrap_or(true)`, session-only kill gate
- `desktop-gpui/crates/webtmux/src/views/rename_session_dialog.rs:41-126` + `session_context_menu.rs:33-66` — DLG1 + stable-id menu patterns to clone
- `desktop-gpui/crates/webtmux/src/views/tab_strip.rs:1-9` — Phase-3 D8 carve-out (no Plus/menu — Phase 5 owns)
- `desktop-gpui/crates/webtmux/src/views/terminal_view.rs:305-379,489-495` — mouse down/move/up + `pressed_button` drag-tracking precedent
- `desktop-gpui/crates/webtmux/src/window_state.rs:100-174` — clamp guards (`800..3840`, `500..2160`, `-10000` sentinel) needing tests (D9)
- `desktop-gpui/crates/settings/src/lib.rs:61-71` — `confirm_kill_session/pane/window` all `default_true`
- `desktop-gpui/crates/webtmux/src/icons.rs` — 13 existing consts; none of the 9 Phase-5 icons present
- `desktop-gpui/Cargo.toml:24-55` — pins (`gpui-pre =0.3.3`, `gpui-component =0.6.0`, `alacritty_terminal =0.25.1`); `Cargo.lock` committed

### Secondary (MEDIUM confidence — subagent digests, cross-checked against Primary reads above)

- Backend protocol digest (all 8 pane + 7 window commands, param overloads, `terminal.input` uncorrelated + 6ms/512B batcher, full-snapshot deltas, 250ms Windows monitor cadence) — command strings and handler lines re-verified by direct Read
- FE geometry/interaction digest (pixelRect, gap absorption, divider T-layout overlap, zoom, menus, presets, headers, timer cascade) — formulas and lists re-verified by direct Read
- Phase-3/4 lock digest (generation guard, hello ban, TerminalStore ownership, TUI-ON, resize dance, capture-replay, title store, owning-socket input, stable menu ids, deferred-to-5 list) — consistent with 04-CONTEXT excerpts Read this session

### Tertiary (LOW confidence — see Assumptions Log)

- A1–A6 (tmux-side `next-layout` acceptance, GPUI pointer-capture sufficiency, tooltip widget availability, dropped-receiver safety, web-term reference gap, move offsets)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — zero new packages; every import resolves to Read-verified pins and proven Phase-3/4 modules
- Architecture: HIGH — geometry, menus, presets, dialogs all copied verbatim from Read-verified FE + backend sources; only GPUI view composition is novel
- Pitfalls: HIGH for protocol/geometry traps (all Read-verified); MEDIUM for drag-capture (A2) and tooltip widget (A3)

**Research date:** 2026-09-07
**Valid until:** 30 days (stable: pinned pre-1.0 deps, frozen FE/backend surfaces, no fast-moving inputs)
