# Phase 4: Terminal Engine & Live Pane Rendering - Research

**Researched:** 2026-09-06
**Domain:** alacritty_terminal-backed per-pane terminal engine (`webtmux-terminal` crate port of web-term's `terminal` crate), capture-replay → live-output pipeline (`terminal.capture`/`terminal.snapshot`/`terminal.output` with `replace`/`screenRows`), byte-safe keyboard input over `terminal.input`, wheel scrollback vs TUI paging, GPUI selection/clipboard, and storm-free resize (`terminal.resize` with the Electron debounce dance).
**Confidence:** HIGH — Go WS terminal handlers (`be/internal/realtime/handler.go`, `hub.go`, `be/internal/tmux/service.go`, `monitor.go`, `snapshot.go`, `input_batcher.go`), FE xterm pipeline (`terminalRegistry.ts`, `useTerminal.ts`, `sockets.ts`, `websocket.ts`, `protocol.ts`, `PaneWorkspace.tsx`, `App.tsx`, `settingsStore.ts`), the full web-term reference `terminal` crate (all 6 modules), and the current desktop-gpui seams (`ws.rs`, `app_state.rs` `apply_event`, settings, workspace pins + lockfile) were all read and line-verified this session.

<user_constraints>
## User Constraints (from CONTEXT.md)

> No `04-CONTEXT.md` exists yet (phase directory was created by this research run). Constraints below are copied verbatim from the authoritative upstream sources that bind Phase 4: `.planning/ROADMAP.md` Phase 4 section, `.planning/REQUIREMENTS.md` TERM-01..07, and the out-of-scope carry-forwards of `03-RESEARCH.md`.

### Locked Decisions

- **Phase goal (ROADMAP.md:93-95):** "Every pane renders a live alacritty_terminal-backed terminal through the full capture-replay → live-output pipeline, with byte-safe input, scrollback, selection, and storm-free resize." Depends on Phase 3. Requirements: `TERM-01` through `TERM-07`.
- **Success criteria (ROADMAP.md:99-104, verbatim):**
  1. "A pane's live output renders in the terminal grid and matches the other frontends on the same workload (vim/htop look identical)"
  2. "Opening a pane with history replays captured scrollback exactly once and continues live seamlessly — including reconnect (never doubled history or a blank screen)"
  3. "Typed input including special keys reaches the pane byte-safe through `terminal.input`; wheel scrolls scrollback, and with TUI-scroll on the wheel sends PageUp/PageDown to TUI panes"
  4. "User can select text and copy it to the clipboard, and paste clipboard content into the terminal"
  5. "Window resizes report cols/rows with the Electron debounce dance (no resize storms); inactive hidden session tabs keep ingesting output and catch up when reshown"
- **Transport already exists (Phase 3):** per-session `SessionWsHandle` (`connect_session`, correlated `send_command`, tagged `WsPumpEvent`), `normalize_ws_url`, envelope DTOs `WsIncoming`/`WsOutgoing` with `replace: bool` + `screenRows: Option<i32>` already modeled, and the triple generation guard in `AppState::apply_event`. Phase 4 consumes `terminal.snapshot`/`terminal.output` and sends `terminal.input`/`terminal.resize`/`terminal.capture` — no new transport, no backend changes.
- **Out of scope carried forward:** pane grid geometry/dividers/zoom/presets/context menus (Phase 5 owns them — Phase 4 renders terminals inside the existing Phase-3 placeholder body, one terminal per pane of the active window); theme system + terminal prefs page (Phase 6 — Phase 4 uses FE-default constants); reconnect banners/toasts (Phase 7); backend protocol changes (frontend-only milestone — log gaps, don't fork).
- **Prior verification seam (STATE.md Blockers):** "Phase 4 — alacritty embedding + capture-replay contract tests" and "Phase 3 — verify `terminal.input` byte↔string mapping against the Go WS handler". Both are answered in this document (§Technical Mechanics 3, §Validation Architecture) and must appear as contract tests in the plan.

### the agent's Discretion

Terminal-store placement, renderer defaults (font/scrollback/TUI-switch) ahead of the Phase 6 settings page, reconnect re-capture policy, and stale pane-ID hygiene — drafted as auto-accepted reversible decisions in `## Grey-Area Decisions` below.

### Deferred Ideas (OUT OF SCOPE)

- Pane grid geometry, split/zoom/resize drags, layout presets, pane/window context menus — Phase 5 (`PANE-01..08`, `DLG-02`).
- Settings page, theme tables, command palette, font/icon embedding beyond the Phase-1 JetBrains Mono — Phase 6.
- Reconnect banner/toast UX — Phase 7 (`STATE-03`).
- Session keyboard shortcuts (`EXTRA-02`, v2); tab persistence (`EXTRA-01`, v2).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TERM-01 | Terminal output renders in an alacritty_terminal-backed grid per pane (same engine as web-term) | Port web-term `crates/terminal` (6 modules, `alacritty_terminal =0.25.1` lockfile-verified) into `webtmux-terminal`: `Terminal` (`Term<GpuiEventProxy>` + `Processor`), `TerminalRenderer` (cell measure, `layout_row`, `paint`), `TerminalView` (canvas + focus + input/mouse/scroll handlers). One `Terminal` per pane ID, fed by `process_bytes`. |
| TERM-02 | Opening a pane replays history: capture blob ingests the scrollback delta then rebuilds the positioned screen (`applyCapture` semantics with `ingestedHistory` counter) | `terminal.capture` on pane-mount → `terminal.snapshot{replace:true, screenRows}` → port of FE `applyCapture` (history-delta bytes, then `ESC[2J ESC[H` + per-row `ESC[{r};1H` repaint) fed into the alacritty `Processor`; per-pane `{snapshotWritten, ingestedHistory}` guards give exactly-once replay incl. reconnect. |
| TERM-03 | Keyboard input reaches the pane byte-safe through the WS `terminal.input` path | Port `keystroke_to_bytes` verbatim; `Vec<u8>` → `String` via UTF-8 (all table outputs are ASCII/`key_char` UTF-8; Go does `[]byte(in.Data)` — exact round-trip); send `{type:"terminal.input", paneId, data}` on the owning session's socket. |
| TERM-04 | User can scroll scrollback with the wheel; with the TUI-scroll switch on, the wheel sends PageUp/PageDown to TUI panes | Wheel handler port: SGR 64/65 when the app enabled mouse reporting; TUI-switch ON (per-pane, default true) → notch accumulator (100px/notch, burst clamp 3) emitting `\x1b[5~`/`\x1b[6~` repeats over `terminal.input`; OFF → `scroll_display(delta)`. FE behavior wins over the reference's arrow-key variant. |
| TERM-05 | User can select text and copy/paste via the clipboard | Port `Terminal::selection_*` + `TerminalView::copy_selection` (`selection_to_string` → `cx.write_to_clipboard`) / `paste_clipboard` (`cx.read_from_clipboard` → `terminal.input`); Ctrl+Shift+C / Ctrl+Shift+V (+ Cmd variants); Ctrl+C with selection copies, without passes through as interrupt. |
| TERM-06 | Terminals report cols/rows on container resize with the Electron debounce dance (~100ms resize, 150/325ms capture resync, layout-key invalidation) — no resize storms | Canvas measures cols/rows per paint but only arms a pending size; AppState-level 100ms debounce sends window-level `terminal.resize` via `actualViewport` scaling; layout-key change → 150ms resize + 325ms invalidate+recapture; skipped on first mount. Never send from paint; never send `hello` (dead in FE). |
| TERM-07 | Inactive (hidden) session workspaces keep ingesting terminal output while not rendered | Terminal state lives in `AppState` (pane-id-keyed store), NOT in views; the Phase-3 forward pump already delivers every session's events through `apply_event` regardless of visibility; extend the `terminal.snapshot/output` arms from parse-and-ignore to store commits. Views render only the active session. |
</phase_requirements>

## Summary

Phase 4 is the second half of the web-term port: where Phases 1–3 ported the shell, protocol metadata, and tab lifecycle, Phase 4 ports the `terminal` crate (all six modules: `terminal.rs`, `input.rs`, `view.rs`, `mouse.rs`, `render.rs`, `event.rs`, plus `colors.rs` with a fixed default palette until Phase 6) and wires it to the already-live WebSocket terminal events. No new crates, no new packages, no backend changes — the wire shapes (`replace`, `screenRows`, `paneId`, `data`) are already modeled in `ws.rs`, and the only transport additions are three fire-and-forget sends reusing `SessionWsHandle::send_command`.

The critical intellectual content is the **capture-replay contract**: the backend's capture blob is `scrollback-history lines + visible screen rows` with `screenRows = pane.Height` telling the client where the split is. The FE's `applyCapture` (history-delta append via the `ingestedHistory` counter, then clear+home+explicitly-positioned screen repaint) ports almost line-for-line, with one simplification — alacritty's `Processor::advance` is synchronous while xterm's `write` is async, so the FE's `pendingScreen`/`writingScreen` frame-queue has no GPUI equivalent and is replaced by in-order processing on the GPUI thread. The `snapshotWritten` exactly-once guard and the `ingestedHistory` delta counter both survive the port unchanged; they are what criteria 2 ("never doubled/blank") rests on.

The two deliberate divergences from the web-term reference are both FE-parity wins: (1) TUI wheel sends PageUp/PageDown repeats (FE `useTerminal.ts`), not the reference's arrow-key fallback (`mouse.rs::scroll_to_arrow_keys`); (2) sizing rides `terminal.resize` only — FE's `hello()` is defined but never called, and Phase 3 already established that `hello` is a viewport resize, not a handshake.

**Primary recommendation:** Fill `webtmux-terminal` by porting the six reference modules with the documented deltas (FE-default renderer constants, no settings wiring yet), add a pane-id-keyed `TerminalStore` on `AppState` holding `{Terminal, snapshotWritten, ingestedHistory, tuiScroll}` per pane, extend `apply_event`'s terminal arms to commit into the store (all sessions, visible or not), send `terminal.capture` on pane-terminal creation and on reconnect/layout-resync, route `keystroke_to_bytes` output through `terminal.input` on the owning session's socket, and implement the 100ms/150ms/325ms resize dance at the workspace level — with capture-split, byte-mapping, and exactly-once replay locked by headless contract tests first (the STATE.md blocker).

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| VTE emulation per pane (`Term` + `Processor`, resize, scrollback buffer) | `webtmux-terminal` crate (`terminal.rs` port, pure Rust + alacritty) | — | GPUI-free like supervisor/settings: headless-testable grid assertions with zero window. |
| Keystroke → ANSI bytes | `webtmux-terminal` (`input.rs` verbatim port) | — | Pure function of `(Keystroke, TermMode)`; unit tests port with it. |
| Mouse geometry + SGR reports + selection types | `webtmux-terminal` (`mouse.rs` verbatim port) | — | Pure functions; same headless-testability. |
| Cell layout + text-run batching + quad painting | `webtmux-terminal` (`render.rs` + `colors.rs` port) | GPUI `Window::paint_quad` / text system | `layout_row`/`merge_backgrounds` are pure and headless-testable; only `paint` needs a window. |
| Focus/input/mouse/scroll event wiring + canvas + clipboard | `webtmux-terminal` (`view.rs` port, `TerminalView`) | `AppState` callbacks (`input` → socket send, `resize` → pending-size arm) | View stays dumb: bytes out via `InputCallback`, sizes out via `ResizeCallback`; AppState owns all WS contact. |
| Per-pane terminal store (`{Terminal, snapshotWritten, ingestedHistory, tuiScroll}` keyed by pane ID) + capture/reconnect/resize orchestration | `AppState` (`webtmux` app crate) | Background forward pump (already exists) | Mirrors FE `terminalRegistry` + `useTerminal` + `PaneWorkspace` effects; GPUI thread owns all `Term` mutation so frame order is total. |
| WS terminal sends (`terminal.input/resize/capture`) | Existing `SessionWsHandle::send_command` (no new transport) | `AppState` helpers resolving the owning session's socket | `terminal.input` is uncorrelated by server design (no reply); `capture` correlates only via its `terminal.snapshot` event, not `command.success`. |

---

## Standard Stack

### Core (all already pinned — NO new packages)

| Library | Pinned Version | Purpose | Why Standard / Verification |
|---------|----------------|---------|-----------------------------|
| `alacritty_terminal` | `=0.25.1` | VTE grid, `Term`, `Processor`, selection, `TermMode` | [VERIFIED: `desktop-gpui/Cargo.toml:24`] workspace pin; [VERIFIED: `desktop-gpui/Cargo.lock`] `alacritty_terminal 0.25.1` resolved. Same engine as the reference crate and the TERM-01 requirement text. |
| `gpui` (`gpui-pre`) | `=0.3.3` | `canvas`, `track_focus`, `on_key_down`, mouse/scroll handlers, `cx.write_to_clipboard` / `cx.read_from_clipboard` | [VERIFIED: `desktop-gpui/Cargo.toml:33`] + lockfile `gpui-pre 0.3.3`. Reference `view.rs` uses only stable GPUI APIs present in this pin (`canvas`, `ClipboardItem`, `ScrollWheelEvent`/`ScrollDelta`). |
| `tokio` / `flume` / `serde_json` | `=1.53.1` / `=0.12.0` / `=1.0.151` | Unchanged (forward pump, event channels, envelope parsing) | Already consumed via `ws.rs`; Phase 4 adds no async surface — terminal commits run synchronously on the GPUI thread inside `apply_event`. |

**Installation:** none — `crates/terminal/Cargo.toml:9-16` [VERIFIED] already depends on `gpui`, `alacritty_terminal`, `flume`, `parking_lot`, `anyhow`, `serde`, `serde_json`. Phase 4 adds modules, not dependencies. (`webtmux-terminal` is currently a 1-line stub — [VERIFIED: `crates/terminal/src/lib.rs:1`].)

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Port `web-term/.../terminal` | Wrap xterm.js via webview / port `fe/` xterm pipeline | Rejected: GPUI has no DOM; the milestone mandates the alacritty engine (TERM-01 names it); the reference crate is proven on this exact pin. |
| Per-pane `Terminal` in `AppState` store | `Entity<TerminalView>` per pane held by the workspace view | Rejected: views for hidden sessions unmount (or never mount) — TERM-07 requires ingestion without rendering. State in `AppState`, views render the active session only (FE `App.tsx` invisible-mounted parity). |
| Synchronous in-order commit on GPUI thread | Per-pane background thread feeding `Term` + `cx.notify` | Rejected: alacritty `Term` is `Mutex`-wrapped but frame order across snapshot/output/resize must be total per pane; the FE achieves this by writing one xterm instance from one handler chain. GPUI-thread commits inherit the pump's FIFO order for free. |

## Package Legitimacy Audit

> No external packages are installed in Phase 4 — the VTE engine, clipboard, and canvas primitives are already workspace-pinned and lockfile-committed.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| *(none — no new installs)* | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.
*`alacritty_terminal 0.25.1` was legitimacy-gated in Phase 1 (OK: 6 yrs, ~60k/wk, alacritty/alacritty) and its resolved version was re-confirmed in `desktop-gpui/Cargo.lock` this session. The six ported modules are local-file copies, not registry fetches.*

---

## Terminal Protocol Contracts & Source Truth

### 1. `terminal.input` — byte-safe by construction, `String` is a safe carrier

[VERIFIED: `be/internal/realtime/handler.go:112-120`]

```go
case MsgTerminalInput:
    if in.PaneID == "" {
        fail(errString("paneId required"))
        return
    }
    if err := h.svc.SendInput(session, in.PaneID, []byte(in.Data)); err != nil {
```

[VERIFIED: `be/internal/tmux/service.go:279-287`] `SendInput` forwards to the session monitor's batcher; [VERIFIED: `be/internal/tmux/input_batcher.go:8-16,33-48`] the batcher accumulates raw `[]byte` per pane and flushes every ~6ms or at 512 bytes — it never interprets the bytes (hex-encoding happens only at the `send-keys -H` fallback site, which handles UTF-8/control/ESC/mouse escapes transparently).

**Byte↔string mapping answer (closes the STATE.md Phase-3 seam):** Go's `[]byte(string)` ↔ Rust's `String::from_utf8(bytes)` is an exact round-trip for all input this phase produces — the entire `keystroke_to_bytes` table emits ASCII escapes plus `key_char` UTF-8 text ([VERIFIED: `web-term/.../terminal/src/input.rs:9-155`]), and pasted clipboard content is a Rust `String` already. No `String::from_utf8_lossy` (lossy would silently corrupt on the one unexpected case instead of failing loudly in the contract test). `terminal.input` is intentionally uncorrelated — the server sends no `command.success/error` for it ([VERIFIED: `handler.go:117-119`] no `ok()` call) — so the send path is fire-and-forget over `send_command` with the receiver dropped (same shape as Phase 3's `send_window_select`).

### 2. `terminal.resize` — window-level viewport, bounds `cols>=2, rows>=1`

[VERIFIED: `be/internal/realtime/handler.go:121-128`] rejects `cols<=0 || rows<=0`; [VERIFIED: `be/internal/tmux/service.go:295-304`] enforces `cols < 2 || rows < 1` → `"invalid terminal size %dx%d"`. Unix maps to `refresh-client -C`, Windows to `resize-window` ([VERIFIED: `be/internal/tmux/monitor.go:217-233`]).

Contract consequences: clamp before sending (`cols.max(2)`, `rows.max(1)` — the FE's `pxToColsRows` already floors this way, [VERIFIED: `fe/src/lib/geometry.ts:28-32`]); report the **window** viewport, not per-pane cells (FE `actualViewport` scales one visible pane's measured xterm size to the whole window via tmux cell geometry, [VERIFIED: `fe/src/features/panes/PaneWorkspace.tsx:41-57`]); send on the session's **own** socket (all resize/capture traffic is per-session — the rename-quirk lesson from Phase 3 generalizes).

> Drift note: `service.go:289-294` comments that "the realtime hub aggregates client viewports (minimum across the session, debounced) and is the only caller of this method" — the actual `hub.go` in this repo ([VERIFIED: read in full, 149 lines]) contains **no viewport aggregation or `UpdateViewport`**; `dispatch` calls `ResizeTerminal` directly. Treat the comment as aspirational: single-client direct resize is the real contract. Log as a backend-comment nit only if the planner wants it; no behavior impact.

### 3. `terminal.capture` → `terminal.snapshot{replace:true, screenRows, paneId}` — the replay source

[VERIFIED: `be/internal/realtime/handler.go:130-140`] `terminal.capture{paneId}` → `CapturePane(ctx, session, paneId)` → replies `terminal.snapshot` with `Replace: true, ScreenRows: screenRows`:

```go
c.Send(Outgoing{Type: EvTerminalSnapshot, PaneID: in.PaneID, Data: data, Replace: true, ScreenRows: screenRows})
```

`screenRows` is the pane's current height from the monitor snapshot ([VERIFIED: `be/internal/tmux/service.go:309-322`], nil-snapshot panic guard included). The blob itself: Unix `capture-pane -p -e -J -t <pane> -S -<scrollback>` (joined/wrapped lines, with escapes), Windows `capture-pane -p -e -N -t <pane> -S -<scrollback>` (screen-oriented, [VERIFIED: `be/internal/tmux/snapshot.go:167-177`]). Semantics: **everything above the last `screenRows` lines is scrollback history, the tail is the live screen** ([VERIFIED: `snapshot.go:172-174`], [VERIFIED: `service.go:306-308`]). When the pane is missing from the snapshot, `screenRows` is 0 — the FE treats non-positive `screenRows` as "legacy absolute write" ([VERIFIED: `terminalRegistry.ts:47`]), so the port must keep that fallback branch.

### 4. `terminal.output` — two shapes, one routing rule

Broadcast sites ([VERIFIED: `be/internal/tmux/monitor.go:737,545,481-488`], relay [VERIFIED: `be/internal/realtime/hub.go:102-103`]):

| Producer | `replace` | `screenRows` | Meaning |
|----------|-----------|--------------|---------|
| Unix control-mode `%output` lines (`handleLine`) | `false` (absent) | absent | Incremental byte stream — feed straight to the `Processor`. |
| Windows pipe-pane chunks (`armStream` chunk callback) | `false` | absent | Same incremental shape ("the shape the frontends already consume by default", `monitor.go:542-545`). |
| Windows capture poll / stream-bootstrap integrity frames (`captureVisiblePane`) | `true` | `pane.Height` | Complete screen frames — full-screen replacement via the capture path (`monitor.go:450-489`). |

FE routing (the rule to port, [VERIFIED: `fe/src/lib/sockets.ts:36-44`]): `terminal.output` with `replace` → `replaceScreen` (capture semantics), else `write` (raw append); `terminal.snapshot` → `writeSnapshot` (idempotent exactly-once per instance). The Rust DTOs already carry both fields with the correct serde shape (`replace: bool` default-false, `screenRows: Option<i32>` — [VERIFIED: `crates/backend-client/src/ws.rs:96-123`]).

### 5. Envelope session tagging + hidden-tab delivery (already proven)

[VERIFIED: `be/internal/realtime/hub.go:104-108`] every `state.delta` carries `Session`; terminal events carry `paneId` (globally unique per tmux server — [VERIFIED: `fe/src/lib/sockets.ts:1-9`] comment, routing stays unambiguous across sessions). The Phase-3 forward pump forwards **all** sessions' pump events into `apply_event` regardless of which tab is active ([VERIFIED: `crates/webtmux/src/app_state.rs:997-1018`]); `apply_event` currently **parse-and-ignores** terminal events ([VERIFIED: `app_state.rs:459`] `EV_TERMINAL_SNAPSHOT | EV_TERMINAL_OUTPUT => true, // parse-and-ignore`). Phase 4 replaces that arm with store commits — TERM-07 then holds by construction, since commits never consult `active_session`.

---

## Technical Mechanics & Implementations

### 1. Port the reference `terminal` crate module-by-module (deltas enumerated)

Source: `E:\Coding Stuff\web-term\desktop-gpui\crates\terminal\src\` (all modules read this session). Target: `webtmux-terminal` (same module split, same public names where possible so Phase 5/6 plans can cite them).

| Module | Port action | Deltas from reference |
|--------|-------------|----------------------|
| `terminal.rs` (`Terminal`, `TermDimensions`, `TerminalConfig`) | Near-verbatim | `scrollback_limit` default **2000** (FE `settingsStore.ts:40`, not the reference's 10_000 — 1:1 scrollback depth with Electron; Phase 6 wires the real pref). Keep `process_bytes` / `resize` / `scroll_display` / selection API / `with_term*` verbatim ([VERIFIED: reference `terminal.rs:97-207`]). |
| `input.rs` (`keystroke_to_bytes`) | **Verbatim, including tests** | None — the table (enter `\r`, backspace `\x7f`, tab/S-tab, arrows + `APP_CURSOR` variants, modified arrows `ESC[1;<mod>`, home/end/pageup/pagedown/insert/delete, F1–F12, Ctrl-letter math, Alt-ESC-prefix, `key_char` passthrough) is engine behavior, identical for both apps ([VERIFIED: reference `input.rs:9-155,157-271`]). |
| `mouse.rs` (`pixel_to_cell`, `selection_type_from_clicks`, `modifiers_to_mouse_code`, `mouse_button_report`, `scroll_report`) | Verbatim | None at the function level — but the **caller** uses FE wheel policy (see §4), not the reference `on_scroll` fallthrough. |
| `render.rs` + `colors.rs` (`TerminalRenderer`, `ColorPalette`, `layout_row`, `paint`) | Port with fixed defaults | Renderer default **14px / line-height 1.35** (FE `settingsStore.ts:38-39`, vs reference 14px/1.2 in `view.rs:45-50`); palette = fixed dark default (reference `colors.rs:37-72`) until Phase 6 ports the 78 terminal presets. Keep `measure_cell` ("M"-advance), `layout_row` batching (skip `WIDE_CHAR_SPACER`, INVERSE/DIM/HIDDEN handling), `paint` (bg quads → selection quads → text runs, whitespace-run skip) verbatim ([VERIFIED: reference `render.rs:96-126,128-253,255-399`]). Font family stays `"JetBrains Mono"` — embedded since Phase 1. |
| `event.rs` (`GpuiEventProxy`, `TerminalEvent`) | Verbatim | None ([VERIFIED: reference `event.rs:7-53`]). Bell/title callbacks: title stored per pane for Phase-5 headers (D8); bell ignored. |
| `view.rs` (`TerminalView`) | Port structure, rewire callbacks | `InputCallback(bytes)` → `terminal.input` send on owning session socket; `ResizeCallback(cols,rows)` → **arm pending-size only** (never send from paint — §5); keep focus/clipboard/mouse/selection handlers; clipboard via `cx.write_to_clipboard` / `cx.read_from_clipboard` ([VERIFIED: reference `view.rs:211-300`]). |

### 2. Capture-replay → alacritty mapping (the TERM-02 core)

Port FE `applyCapture` ([VERIFIED: `terminalRegistry.ts:31-65`]) as a byte-oriented function in `webtmux-terminal`, with per-pane `{snapshotWritten: bool, ingestedHistory: usize}` stored in the `AppState` terminal store:

```
on terminal.snapshot(paneId, data, screenRows):
    store_entry(paneId) or drop if pane unknown-retired (D7)
    if entry.snapshotWritten: drop                    // exactly-once (StrictMode-double-capture analog:
                                                      // GPUI view remounts re-request; second reply dies here)
    entry.snapshotWritten = true
    feed apply_capture(entry, data, screenRows) bytes to Processor

apply_capture(entry, data, screenRows): Vec<u8>
    lines = data split on \n (normalize \r\n? → \n FIRST; split on char boundaries — never byte-slice a String)
    positioned(rows) = rows.map ESC[{i+1};1H + row      // explicit origin per row: full-width rows would
                                                      // auto-wrap before CRLF is consumed (registry.ts:41-45)
    rows invalid (None/<=0): return ESC[2J ESC[H + positioned(all lines)     // legacy fallback, byte-for-byte
    split = max(0, len - rows); history = lines[..split]; screen = lines[split..]
    if history.len < ingestedHistory: ingestedHistory = 0   // shrank (rare): accept cosmetic edge,
                                                            // xterm/alacritty have no scrollback-trim API
    else: delta = history[ingestedHistory..]; payload = delta.join(\r\n) + \r\n; ingestedHistory = history.len
    return payload + ESC[2J ESC[H + positioned(screen)
```

Why each piece survives the engine change:
- **History delta as raw bytes:** both xterm and alacritty push received lines into scrollback naturally; feeding `delta + \r\n` through `Processor::advance` is the exact analog of `term.write(delta)`.
- **Clear + positioned repaint:** `ESC[2J ESC[H` + `ESC[{r};1H` are plain VTE sequences the alacritty parser consumes identically — this is also what makes vim/htop frames render identically (TERM-01 criterion 1: same bytes, same grid semantics).
- **`ingestedHistory`:** the backend re-sends overlapping history on every capture (stream-bootstrap + integrity frames, `monitor.go:529-545`); without the delta counter each re-capture duplicates scrollback. Counter lives per pane ID and resets only when the entry is retired (D7).
- **No `pendingScreen` queue:** xterm's `write` is async (hence `writingScreen` + microtask flush in `replaceScreen`, [VERIFIED: `terminalRegistry.ts:90-115`]); `Processor::advance` is synchronous — in-order synchronous commits on the GPUI thread give frame atomicity for free. Do NOT port the queue; do preserve its ordering invariant (one commit at a time, pump order).

`terminal.output` with `replace=true` (Windows frames) runs the **same** `apply_capture` path but **without** the `snapshotWritten` gate (FE `replaceScreen` is not idempotent-gated — [VERIFIED: `terminalRegistry.ts:90-115`]); `replace=false` output feeds `data` bytes directly to the `Processor`.

### 3. Input path (TERM-03): focus → bytes → owning socket

```
TerminalView::on_key_down (port of reference view.rs:264-300):
    copy check: (Ctrl+Shift+C | Cmd+C) && selection non-empty → copy_selection, consume
    paste check: (Ctrl+Shift+V | Cmd+V) → paste_clipboard, consume
    Ctrl+C with EMPTY selection → fall through (interrupt passthrough — FE useTerminal.ts:183-191)
    scroll_to_bottom (any keypress resets viewport)
    keystroke_to_bytes(keystroke, term.mode()) → Some(bytes):
        String::from_utf8(bytes) → expect-ok (contract test locks the table: all outputs ASCII/UTF-8)
        send {type:"terminal.input", paneId, data} on the OWNING session's socket
        (pane → session resolved from the latest snapshot; never the active-session proxy — Phase-3 D5 lesson)
```

Paste of large clipboard text: the server batcher (6ms/512B) already chunks arbitrarily large writes ([VERIFIED: `input_batcher.go:76-87`]); the client sends it as one message — no client-side chunking needed. Mouse-reporting apps (vim): `mouse_button_report` SGR passthrough when `TermMode` has mouse bits ([VERIFIED: reference `mouse.rs:53-79`]) — keep verbatim so vim/htop mouse clicks work like Electron.

### 4. Wheel policy (TERM-04): FE wins over the reference

FE behavior ([VERIFIED: `useTerminal.ts:124-170`]): capture-phase wheel listener; TUI-switch OFF → native scrollback; ON → consume always, accumulate pixel deltas (line×16, page×100), 100px = one notch, clamp bursts to 3 pages, emit PageUp (`\x1b[5~`) / PageDown (`\x1b[6~`) repeats over `terminal.input`. Per-pane switch defaults **true** ([VERIFIED: `useTerminal.ts:89`, `PaneHeader.tsx:38`] `?? true`).

The reference instead falls back to arrow keys in alt-screen ([VERIFIED: `mouse.rs:98-125`] `scroll_to_arrow_keys`, max 5) and the GPUI `on_scroll` converts pixels via `cell_height` ([VERIFIED: `view.rs:410-463`]). The planner must implement the FE policy: SGR 64/65 when mouse-reporting is on (both agree here — keep `scroll_report`'s first branch), else TUI-switch check (AppState per-pane map, D3) → PageUp/PageDown repeats, else `scroll_display(delta_lines)` with the reference's pixel→lines conversion. The success criterion literally names PageUp/PageDown — arrow keys would fail criterion 3's TUI clause against vim (which pages on PageUp/PageDown but cursor-moves on arrows).

### 5. Resize dance (TERM-06): measure in paint, send on debounce, resync on layout-key

FE dance ([VERIFIED: `PaneWorkspace.tsx:131-200`]): viewport `terminal.resize` debounced **~100ms** on container size; layout-key (`activeWindow|WxH|layout|pane-id:cells,zoom…`) change → **150ms** resize + **325ms** invalidate-all-visible + force-`terminal.capture`; skipped on first mount (each TerminalView already captured).

GPUI port:
- `TerminalView` canvas keeps the reference shape: `measure_cell` per paint, `cols/rows = floor(avail/cell)` clamped ≥1, `term.resize(cols,rows)` locally + `resize_cb(cols,rows)` **only on change** ([VERIFIED: reference `view.rs:500-525`]).
- The callback **arms** `AppState.pending_viewport`, it never sends: a 100ms `tokio::time::sleep` debounce task (generation-tagged like the poll pump — a second arm supersedes the first) recomputes `actualViewport` at fire time from settled sizes and sends one `terminal.resize{cols.max(2), rows.max(1)}` on the session socket.
- Layout-key computed from the active snapshot (same fields as FE `layoutKey`); on change (and not first mount): 150ms timer → resize; 325ms timer → `invalidateSnapshot(paneId)` (i.e. `snapshotWritten = false`) + `terminal.capture` per visible registered pane.
- `hello` is never sent: FE defines `hello()` but **no call site exists** (grep-verified: only the definition at `websocket.ts:189`), and Phase 3 (D7) already established it is a viewport resize, not a handshake. Real dimensions ride `terminal.resize` only.

### 6. Hidden tabs + reconnect (TERM-07 + criterion 2's reconnect clause)

- **Hidden ingest:** terminal store keyed by pane ID lives on `AppState`; `apply_event`'s new terminal arms commit for every open session (the pump already forwards all sessions — §Contracts 5). Views render only the active session's panes (FE `App.tsx:229-245` invisible-mounted parity: hidden workspaces keep their sockets AND their terminals; GPUI keeps sockets + store, and lazily (re)creates views on activation without touching the store).
- **Reconnect re-capture (D6):** FE does not recapture on reconnect (terminals stay mounted, new output appends) — but a GPUI client whose socket dropped also lost its server monitor (`hub.go:70-89` last-client-leave teardown), so the screen may have advanced while away. On transport→Connected (or `connection.ready`) for a session: invalidate + `terminal.capture` every registered pane of that session. Idempotent (`snapshotWritten` re-arms, `ingestedHistory` continues from its counter — shrank-history branch absorbs resets), cheap (only on reconnect), and it is what makes criterion 2's "including reconnect (never … blank)" hold.
- **Stale pane hygiene (D7):** on every committed `state.snapshot`/`state.delta`, retire store entries whose pane IDs are absent (killed panes, closed windows). Prevents unbounded growth and wrong-pane replay if tmux ever reuses a `%N`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| VTE parsing / grid / scrollback / alt-screen | Custom ANSI parser or regex-based renderer | `alacritty_terminal 0.25.1` `Term` + `Processor` (reference port) | vim/htop emit the full VTE surface (alt-screen, mouse modes, bracketed paste, SGR); a partial parser renders them wrong — criterion 1 fails. |
| Keystroke → escape mapping | New key table from GPUI docs | `keystroke_to_bytes` verbatim (with its tests) | 40+ sequences with `APP_CURSOR`/modifier subtleties; the table is engine behavior, already proven. |
| Capture-split + exactly-once replay | Ad-hoc "write blob to terminal" | Ported `applyCapture` + `{snapshotWritten, ingestedHistory}` | The doubled-history and blank-screen failures are named, previously-debugged bugs (registry comments); re-deriving reintroduces them. |
| Wheel-to-TUI paging | Custom notion of what TUIs want | FE policy: PageUp/PageDown repeats, 100px/notch, burst-3 | Criterion 3 names the sequences; vim pages on them, cursor-moves on arrows (the reference fallback). |
| Resize source of truth | Per-pane `terminal.resize` per paint | Window-level debounced `terminal.resize` via `actualViewport` | tmux has ONE viewport per session; per-pane-per-frame resizes storm the control socket and rewrap every other frontend. |
| Clipboard | `arboard`/`clipboard` crate or OSC-52 plumbing | `cx.write_to_clipboard` / `cx.read_from_clipboard` (reference pattern) | GPUI-builtin, zero new deps, already proven in the reference view. |
| Frame ordering across snapshot/output | Locks + background threads per pane | Synchronous commits on the GPUI thread in pump order | Total order per pane for free; no torn frames, no queue to drain. |

**Key insight:** the FE team already debugged every ordering bug this phase can hit (double-capture under StrictMode remounts, history-replay cursor shift, async-write frame interleave, layout-change stale pixels) and left the fixes as code comments. The GPUI port's job is to translate each fix, not rediscover each bug — the one fix that does NOT translate is the async-write queue (alacritty is synchronous), and mistaking that for a required port is the phase's most likely over-engineering trap.

---

## Common Pitfalls

### Pitfall 1: Re-requested captures double the screen (criterion-2 killer)
**What goes wrong:** GPUI view remounts (layout change, tab switch, settings placeholder toggles) re-send `terminal.capture`; each reply's blob replays history + screen on top of the live grid → doubled lines, shifted cursor.
**Why it happens:** Treating every `terminal.snapshot` as fresh content instead of a replayable frame.
**How to avoid:** `snapshotWritten` per pane-ID entry: first snapshot replaces, later ones drop — until explicitly re-armed (`invalidateSnapshot` on layout-key resync / reconnect). The store entry must outlive the view (it lives on `AppState`, not in the view).
**Warning signs:** doubled prompt lines after zoom/window-switch; scrollback growing 2× per tab switch.

### Pitfall 2: `ingestedHistory` reset wipes the exactly-once guarantee
**What goes wrong:** Resetting the counter on every capture (or keying it per-view instead of per-pane) re-appends the full scrollback each frame → unbounded scrollback growth and cursor displacement.
**Why it happens:** Misreading the counter as "lines in this frame" rather than "lines already pushed to this terminal instance".
**How to avoid:** Counter persists across captures; only the delta above it is fed; the shrank-history branch resets to 0 AND still paints the screen precisely. Retire the whole entry (counter included) only when the pane leaves the snapshot (D7).
**Warning signs:** scrollback length jumps by the full history size on every layout resync.

### Pitfall 3: Byte-slicing the capture blob panics on UTF-8 boundaries
**What goes wrong:** `&data[..n]` on a blob containing CJK/emoji panics (`byte index is not a char boundary`), killing the forward pump's GPUI update.
**Why it happens:** Capture blobs carry full UTF-8 terminal content; tmux pads rows with spaces but never with partial runes.
**How to avoid:** Split with `str::lines()`/char-boundary APIs only; carry lines as `String`s; join with `\r\n`. The contract test feeds a CJK + emoji blob through `apply_capture` (Validation Architecture).
**Warning signs:** pump stops committing after `htop` shows a fan emoji; `commit` task panics in tests with non-ASCII fixtures.

### Pitfall 4: Sending resizes from the paint path storms the control socket
**What goes wrong:** `canvas` paint runs every frame during a window drag; a synchronous `terminal.resize` per paint floods tmux with resizes, rewraps all panes, and lags every other frontend.
**Why it happens:** Copying the reference's direct `resize_cb(cols, rows)` call ([VERIFIED: `view.rs:515-520`]) without the FE's debounce layer.
**How to avoid:** Callback arms pending-size only; one generation-tagged 100ms timer sends at fire time from settled sizes; local `term.resize` still applies immediately (grid must match the painted bounds — only the *report* is debounced).
**Warning signs:** other frontends rewrap during GPUI window drags; `resize-window` lag in logs.

### Pitfall 5: `hello` with container-zero dimensions shrinks everyone's viewport
**What goes wrong:** Sending `hello{cols,rows}` before first measure (or with a 1×1 fallback) resizes the shared tmux viewport to postage-stamp size for all frontends.
**Why it happens:** Assuming a handshake is required (it isn't — snapshot is unsolicited) or porting `hello()` as "init".
**How to avoid:** Never send `hello` (dead in FE, banned since Phase-3 D7). First `terminal.resize` fires only after a real measure (size>0) through the same 100ms path.
**Warning signs:** Electron panes collapse to 2×1 on GPUI tab open.

### Pitfall 6: Committing terminal events into views breaks hidden tabs
**What goes wrong:** Storing `Terminal`s inside `Entity<TerminalView>` means hidden sessions (no mounted view) drop output; reshown tabs render blank-then-jumpy as they recapture from scratch.
**Why it happens:** Following the reference `TerminalTab { view: Option<Entity<TerminalView>> }` shape ([VERIFIED: `session.rs:19-29`]) where the view OWNS the terminal.
**How to avoid:** Invert ownership for web-tmux: `AppState` terminal store owns every pane's `Terminal`; views borrow/render the active session's entries. (Reference owns-in-view because each web-term tab is one PTY; web-tmux has N panes × M sessions sharing M sockets.)
**Warning signs:** reshown tabs blank; output visible only in the active tab while `tmux capture-pane` shows data arrived.

### Pitfall 7: TUI-switch default OFF (or missing) breaks criterion 3 by default
**What goes wrong:** Wheel over vim scrolls GPUI scrollback instead of paging vim; UAT reads it as "TUI-scroll broken".
**Why it happens:** Forgetting the FE's `?? true` default when the per-pane map has no entry (fresh pane, fresh settings file).
**How to avoid:** `tui_scroll(pane_id) -> bool { map.get(paneId).copied().unwrap_or(true) }` — absent means ON, matching both `useTerminal.ts:89` and `PaneHeader.tsx:38`.
**Warning signs:** wheel works in shell scrollback but does nothing useful in vim on a fresh install.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) |
| Config file | `desktop-gpui/Cargo.toml` (workspace) |
| Quick run command | `cargo test -p webtmux-terminal` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TERM-03 | `keystroke_to_bytes` table (enter/backspace/tab/arrows/app-cursor/modified-arrows/nav/F-keys/Ctrl/Alt/`key_char`) | unit (port reference tests) | `cargo test -p webtmux-terminal input::tests` | ❌ Wave 0 (`crates/terminal/tests/input_test.rs` or inline port) |
| TERM-03 | `terminal.input` bytes survive `Vec<u8>` → `String` → `[]byte` for the full table + CJK/emoji paste (STATE.md byte-mapping seam) | unit | `cargo test -p webtmux-terminal test_input_byte_roundtrip` | ❌ Wave 0 (`crates/terminal/tests/capture_test.rs`) |
| TERM-02 | `apply_capture` splits blob at `screenRows`: history delta above `ingestedHistory` + clear/home/positioned screen; legacy branch when `screenRows<=0`; shrank-history reset; CRLF normalization; CJK/emoji-safe | unit | `cargo test -p webtmux-terminal test_apply_capture_*` | ❌ Wave 0 (`crates/terminal/tests/capture_test.rs`) |
| TERM-02 | Exactly-once: second identical `terminal.snapshot` dropped; post-invalidate snapshot applied; `ingestedHistory` accumulates across frames (vim + htop fixtures) | unit | `cargo test -p webtmux test_snapshot_exactly_once` | ❌ Wave 0 (`crates/webtmux/tests/terminal_test.rs`) |
| TERM-01 | Live-output parity: scripted vim/htop byte stream through `Processor` yields identical grid text vs reference fixture (provider: recorded `terminal.output` frames) | unit | `cargo test -p webtmux-terminal test_live_output_parity` | ❌ Wave 0 (`crates/terminal/tests/capture_test.rs` + fixtures) |
| TERM-04 | Wheel policy: mouse-mode → SGR 64/65; TUI-on → PageUp/PageDown repeats with notch/burst math; TUI-off → scrollback delta; default-on when absent | unit | `cargo test -p webtmux test_wheel_policy` | ❌ Wave 0 (`crates/webtmux/tests/terminal_test.rs`) |
| TERM-05 | Selection round-trip: programmatic selection → `selection_to_string` non-empty; clipboard copy/paste helpers (headless where GPUI clipboard unavailable: test `selection_text` + byte path) | unit | `cargo test -p webtmux-terminal test_selection_text` | ❌ Wave 0 |
| TERM-06 | Debounce: rapid pending-size arms collapse to one `terminal.resize` with settled values + `max(2)/max(1)` clamp; layout-key change schedules 150/325 pair; first mount skips resync | unit | `cargo test -p webtmux test_resize_dance` | ❌ Wave 0 (`crates/webtmux/tests/terminal_test.rs`) |
| TERM-07 | Hidden ingest: terminal events for a non-active session commit to the store; guard layers (generation/session-match/liveness) still drop stale terminal frames | unit | `cargo test -p webtmux test_hidden_session_ingest` | ❌ Wave 0 (`crates/webtmux/tests/terminal_test.rs`) |
| TERM-02/06 | Interop: mock WS server sends `terminal.snapshot{replace,screenRows}` + `terminal.output` frames; client captures on pane-register and commits grid content | integration | `cargo test -p webtmux-backend-client test_terminal_frames` | ❌ Wave 0 (extend `ws_test.rs` mock) |

### Mock WS Server Strategy (no new crates)
Reuse the Phase-3 in-test server (`TcpListener` + `accept_async`): script `connection.ready` + `state.snapshot`, then on `terminal.capture` reply `terminal.snapshot{replace:true, screenRows:<h>, paneId, data:<scrollback+screen fixture>}`, then stream `terminal.output{replace:false}` chunks and one `replace:true` frame; assert the client sent `terminal.capture` for the pane and (for resize tests) debounced `terminal.resize{cols>=2, rows>=1}`. Fixtures: `terminal_capture_vim.json` (history + alt-screen app bytes), `terminal_capture_unicode.json` (CJK/emoji, CRLF mix), `terminal_output_frames.json`.

### Sampling Rate
- **Per task commit:** `cargo test -p webtmux-terminal` (and `-p webtmux` when touching `app_state.rs`)
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `desktop-gpui/crates/terminal/tests/capture_test.rs` — `apply_capture` splits, legacy branch, shrank-history, CRLF/CJK safety, byte round-trip, live-output parity fixtures
- [ ] `desktop-gpui/crates/terminal/tests/input_test.rs` — ported `keystroke_to_bytes` table tests (or inline `#[cfg(test)]` port; file preferred for parity with reference layout)
- [ ] `desktop-gpui/crates/webtmux/tests/terminal_test.rs` — exactly-once, wheel policy, resize dance, hidden ingest, stale-pane retirement
- [ ] `desktop-gpui/crates/backend-client/tests/ws_test.rs` (extend) — terminal frame interop over the mock server

---

## Grey-Area Decisions (draft for CONTEXT.md — all auto-accepted, reversible)

> Planner locks these into `04-CONTEXT.md`. Each is FE-parity or reference-derived; none needs the user.

- **D1 — Scrollback depth 2000 constant until Phase 6.** `TerminalConfig { scrollback_limit: 2000 }` hardcoded (FE `settingsStore.ts:40` default). Rationale: `DesktopSettings` has no terminal prefs yet ([VERIFIED: `crates/settings/src/lib.rs:48-75`]); Phase 6 (`SET-02`) wires font/scrollback/TUI defaults properly. Revisit: replace constant with settings fields, no API break (config struct already exists).
- **D2 — Renderer defaults 14px / 1.35 line-height / dark palette.** FE settings defaults ([VERIFIED: `settingsStore.ts:38-39`]) win over the reference's 1.2 multiplier ([VERIFIED: reference `view.rs:45-50`]). Palette = reference `dark_default` until Phase 6 ports the 78 terminal presets (`THEME-01/02`).
- **D3 — Per-pane TUI-scroll map on `AppState`, default ON.** `HashMap<String, bool>` + `unwrap_or(true)` ([VERIFIED FE defaults: `useTerminal.ts:89`, `PaneHeader.tsx:38`]). No pane-header switch UI in Phase 4 (headers are Phase 5, `PANE-07`); persistence arrives with Phase 6 settings. Wheel behavior is still fully testable and correct by default.
- **D4 — `hello` is never sent.** FE defines it but never calls it (grep-verified, sole hit `websocket.ts:189`); Phase-3 D7 already bans it. All sizing rides debounced `terminal.resize`. Revisit only if a backend gap is proven (log it, don't fork).
- **D5 — TUI wheel = PageUp/PageDown repeats (FE), not arrow keys (reference).** Criterion 3 names the sequences; `scroll_report`'s mouse-mode branch is kept, its `ALT_SCREEN → arrows` branch is NOT used by the wheel path (kept as dead-ported code with a comment, or omitted — planner's call; recommend omit to avoid a second caller reaching for it).
- **D6 — Reconnect re-captures every registered pane of the session.** On transport→Connected / `connection.ready`: `snapshotWritten=false` + `terminal.capture` per pane of that session. Rationale: our reconnect starts a fresh server monitor (old screen may have advanced); FE's no-recapture is safe only because its terminals never unmount AND its server monitor often survives via other clients — neither holds for a single-client GPUI app. Idempotent and cheap.
- **D7 — Retire store entries absent from the latest snapshot.** On each committed `state.snapshot`/`state.delta`: drop pane entries (and their `ingestedHistory`) not present in `snapshot.panes`. Prevents unbounded growth and cross-talk if tmux reuses `%N`. Views for retired panes are gone with the layout anyway (grid is Phase 5).
- **D8 — Titles stored per pane, bell ignored.** `TerminalEvent::Title` → `HashMap<String, String>` last-title (feeds Phase-5 pane headers); `Bell` → no-op (no FE bell surface found; revisit in Phase 7 only if UAT asks).

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | tmux never reuses a live-visible `%N` within one monitor lifetime, so D7 retirement (rather than content comparison) is sufficient hygiene | §6 / D7 | LOW — worst case is a stale entry dropped too early → one extra `terminal.capture` on next sighting; self-healing. |
| A2 | `String::from_utf8(keystroke_to_bytes(...))` never fails for real GPUI `Keystroke`s (all table outputs ASCII; `key_char` is valid UTF-8 by construction) | Contracts §1 | LOW — locked by the round-trip contract test; failure would be loud at plan time, and the fallback (drop + log) is specified in the plan. |
| A3 | GPUI `canvas` paint cadence + synchronous `Processor` commits keep 60fps viable for full-screen vim/htop frames (reference proves the shape on this pin, but web-tmux panes are smaller and more numerous) | Mechanics §1 | MEDIUM — if profiling shows paint-bound frames, the mitigation (dirty-region repaint inside `paint`, batching `cx.notify`) stays inside `render.rs` with no protocol impact. Flag for Phase 7 parity audit. |
| A4 | `Terminal::scroll_display` against alt-screen content is a safe no-op-ish path for wheel-off over vim (FE leaves it to xterm native; alacritty clamps internally) | Mechanics §4 | LOW — worst case is a no-op wheel over alt-screen with the switch off; TUI-on (the default) never touches this path. |

---

## Open Questions

1. **Should `terminal.capture` be re-requested when a hidden tab is reshown (not just on reconnect/layout)?**
   - What we know: hidden tabs keep ingesting (TERM-07), so the grid is current; FE never recaptures on tab switch (`App.tsx` keeps workspaces mounted). GPUI views re-create on activation but the store survives, so no capture is needed in theory.
   - What's unclear: whether a long-hidden pane's grid can desync (e.g. missed `replace:true` frame during a transient pump stall — the flume channel is unbounded so no drops are expected, but a panic-skip in `apply_event` could).
   - Recommendation: no recapture on switch (FE parity); add a manual resync hook (invalidate + capture one pane) the Phase-5 pane menu can expose later if UAT shows drift. Mark reversible.

2. **Where does per-pane `terminal.resize` fit, if anywhere?**
   - What we know: the protocol has only window-level `terminal.resize` (cols/rows, no paneId — [VERIFIED: `protocol.go:11-13`]); per-pane geometry reaches tmux implicitly through the window viewport + layout. FE never sends per-pane sizes.
   - Recommendation: window-level only (FE parity). If Phase 5 divider drags need mid-drag reflow, they reuse the same debounced window path. No action in Phase 4.

---

## Environment Availability

Step 2.6: SKIPPED in the tool-probe sense — no new external dependencies (no new crates, no CLIs, no services beyond the Phase-1 sidecar). The three new sends ride the Phase-3 sockets; `alacritty_terminal 0.25.1` compiles from the committed lockfile (it already builds as a dependency of the stub crate's closure — the stub compiles today).

## Security Domain

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | No | Localhost-only sidecar, no auth surface (unchanged from Phase 1–3) |
| V3 Session Management | No | tmux sessions are server-side tmux objects, not app sessions |
| V4 Access Control | No | Single-user local app; WS origin permissive by backend design (Phase-3 verified) |
| V5 Input Validation | **Yes** | Clamp resize (`cols≥2, rows≥1`) client-side before sending (server rejects anyway); `String::from_utf8` (not lossy) on input bytes; char-boundary-only splitting of capture blobs; serde `default` discipline already on terminal DTO fields; never `unwrap` in the commit path — malformed frames drop-and-continue (FE `websocket.ts:107-111` parity) |
| V6 Cryptography | No | Plain `ws://127.0.0.1`, no TLS in scope (localhost loopback) |

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed `terminal.output` frame panics the commit path | Tampering / DoS | `serde_json::from_str` → drop + continue (already in the read pump, [VERIFIED: `ws.rs:449-452`]); `apply_capture` panics only on logic bugs — fuzz via the unicode/CRLF fixtures |
| Paste of hostile clipboard (OSC sequences, huge blobs) into `terminal.input` | Injection | Bytes pass through to tmux, same as Electron paste-as-input parity (`useTerminal.ts:193-196`); no app-side interpretation, so no app-side injection surface. Bracketed-paste is negotiated by the app inside tmux, not by us |
| Stale reconnect capture clobbers live grid | Tampering | Generation check applies to terminal frames too (existing `apply_event` layer (a)); recapture replies arriving after a re-resolution die on generation mismatch |

---

## Sources

### Primary (HIGH confidence)
- `be/internal/realtime/protocol.go:6-97` — `Incoming`/`Outgoing` structs + all message/event constants (verbatim DTO source)
- `be/internal/realtime/handler.go:26-62` — handshake + unsolicited `connection.ready`/`state.snapshot`; `:112-140` terminal dispatch (`input`/`resize`/`capture`); `:184-193` rename/kill binding
- `be/internal/realtime/hub.go:38-121` — monitor lifecycle, last-client-leave teardown, `Session`-tagged relay with `Replace`/`ScreenRows`
- `be/internal/tmux/service.go:279-329` — `SendInput` / `ResizeTerminal` bounds / `CapturePane` screenRows semantics
- `be/internal/tmux/monitor.go:16-41` `MonitorEvent` shape; `:235-249` batched `SendInput`; `:450-489` capture-poll frames; `:529-545` pipe-stream bootstrap; `:730-737` Unix incremental output
- `be/internal/tmux/snapshot.go:167-177` — capture flags (`-p -e -J` Unix vs `-p -e -N` Windows)
- `be/internal/tmux/input_batcher.go:8-16,33-48,76-87` — 6ms/512B batching, raw-byte passthrough
- `fe/src/features/terminal/terminalRegistry.ts:10-154` — registry, `applyCapture`, `replaceScreen` frame queue, `writeSnapshot` idempotence, `invalidateSnapshot`
- `fe/src/features/terminal/useTerminal.ts:93-250` — xterm opts, `onData→terminalInput`, capture-phase TUI wheel, clipboard keys, `onResize`, initial `terminalCapture`, 80ms fit debounce
- `fe/src/lib/sockets.ts:23-44` — `replace→replaceScreen` routing rule
- `fe/src/lib/websocket.ts:134-204` — terminal dispatch + typed sends (`hello` defined-never-called `:189`)
- `fe/src/lib/protocol.ts:8-42` + `fe/src/lib/geometry.ts:20-32` — envelope types, `CELL_W/H`, `pxToColsRows` floors
- `fe/src/features/panes/PaneWorkspace.tsx:37-57,131-200` — `actualViewport` scaling, 100ms resize debounce, layout-key + 150/325ms resync
- `fe/src/App.tsx:148-175,226-250` — per-session socket manager + hidden-mounted workspaces (TERM-07 FE shape)
- `fe/src/stores/settingsStore.ts:33-45` — FE defaults (fontSize 14, lineHeight 1.35, scrollback 2000, tuiScroll `{}`, confirms true)
- `web-term/desktop-gpui/crates/terminal/src/{terminal,input,view,mouse,render,event,colors}.rs` — full port source (line refs inline above)
- `web-term/desktop-gpui/crates/webterm/src/session.rs:19-109` — `TerminalTab` view-owns-terminal shape (diverged from, §Pitfall 6)
- `desktop-gpui/crates/backend-client/src/ws.rs:59-123` (envelopes), `:234-243` (tagged pump events), `:300-507` (handle + split pumps)
- `desktop-gpui/crates/webtmux/src/app_state.rs:36-65` (`OpenSession`), `:436-474` (`apply_event` incl. terminal parse-and-ignore `:459`), `:917-1050` (`ensure_session_socket` + forward pump)
- `desktop-gpui/crates/settings/src/lib.rs:48-91` — no terminal prefs yet (D1/D3 premise)
- `desktop-gpui/Cargo.toml:24-28,33` + `Cargo.lock` (`alacritty_terminal 0.25.1`, `gpui-pre 0.3.3`) — pins resolved

### Secondary (MEDIUM confidence)
- `02-RESEARCH.md` + `03-RESEARCH.md` — generation guard, correlated-send, target-socket, and no-`hello` rules reused here
- `03-02-SUMMARY.md` — carried deviations affecting Phase 4 (formatted stable IDs, single-filter test invocation, `overflow_x_scroll`-needs-`.id()`)

### Tertiary (LOW confidence)
- None — every Phase-4 claim is grounded in files read this session except A1–A4, which are logged with mitigations.

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — zero new packages; pins re-verified in `Cargo.toml` + `Cargo.lock`; port source is local and fully read.
- Architecture: HIGH — every pipeline stage (capture→replay→live→input→resize→hidden-ingest) has 1:1 Go + FE + reference anchors.
- Pitfalls: HIGH — five of seven are named bugs with cited fixes in FE/reference comments; two are structural (paint-path storms, view-vs-store ownership).

**Research date:** 2026-09-06
**Valid until:** 2026-10-06 (stable domain — backend protocol frozen, pins locked; re-verify only if `be/internal/realtime/`, `be/internal/tmux/`, or `fe/src/features/terminal/` changes)
