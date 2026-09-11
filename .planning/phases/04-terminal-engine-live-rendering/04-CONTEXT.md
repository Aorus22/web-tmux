# Phase 4: Terminal Engine & Live Pane Rendering - Context

**Gathered:** 2026-09-06
**Status:** Ready for planning
**Mode:** Auto-accepted autonomous (user sleeping — D1–D8 from 04-RESEARCH.md locked as decided; flagged for post-hoc audit)

<domain>
## Phase Boundary

The second half of the web-term port inside the Phase-3 shell: fill the
`webtmux-terminal` stub crate by porting the six reference modules
(`terminal.rs`, `input.rs`, `view.rs`, `mouse.rs`, `render.rs`, `event.rs`,
plus `colors.rs` with a fixed dark default until Phase 6) with the documented
FE-parity deltas, add a pane-id-keyed `TerminalStore` on `AppState`
(`{Terminal, snapshotWritten, ingestedHistory, tuiScroll}` per pane), extend
`apply_event`'s terminal arms from parse-and-ignore to store commits (all
sessions, visible or not), send `terminal.capture` on pane-terminal creation
and on reconnect/layout-resync, route `keystroke_to_bytes` output through
`terminal.input` on the owning session's socket, and implement the
100ms/150ms/325ms resize dance at the workspace level. Views render only the
active session's panes inside the existing Phase-3 placeholder body (one
terminal per pane of the active window). Delivers TERM-01..TERM-07. Out: pane
grid geometry/dividers/zoom/presets/context menus (Phase 5 — Phase 4 renders
terminals inside the existing body, no geometry work), theme system +
terminal prefs page (Phase 6 — Phase 4 uses FE-default constants D1/D2),
reconnect banners/toasts (Phase 7 — Phase 4 only re-captures silently per
D6), backend protocol changes (frontend-only milestone — log gaps, don't
fork), session shortcuts (EXTRA-02, v2), tab persistence (EXTRA-01, v2).

</domain>

<decisions>
## Implementation Decisions

### Terminal crate port (D1, D2, D5 carry)
- Target `webtmux-terminal` keeps the reference module split and public names
  (`Terminal`, `TermDimensions`, `TerminalConfig`, `keystroke_to_bytes`,
  `TerminalRenderer`, `ColorPalette`, `GpuiEventProxy`, `TerminalView`) so
  Phase 5/6 plans can cite them. No new packages — `alacritty_terminal
  =0.25.1`, `gpui-pre =0.3.3`, tokio/flume/serde_json all already pinned and
  lockfile-committed; `crates/terminal/Cargo.toml:9-16` already declares them.
- **D1 — Scrollback depth 2000 constant until Phase 6 (LOCKED):**
  `TerminalConfig { scrollback_limit: 2000 }` hardcoded (FE
  `settingsStore.ts:40` default, not the reference's 10_000).
  `DesktopSettings` has no terminal prefs yet; Phase 6 (`SET-02`) wires them.
- **D2 — Renderer defaults 14px / 1.35 line-height / fixed dark palette
  (LOCKED):** FE settings defaults win over the reference's 1.2 multiplier;
  palette = reference `dark_default` until Phase 6 ports the 78 terminal
  presets. Font stays `"JetBrains Mono"` (embedded since Phase 1).
- `input.rs` (`keystroke_to_bytes`) ports **verbatim, including its tests** —
  the table is engine behavior, identical for both apps. `mouse.rs` ports
  verbatim at the function level, but the wheel caller uses FE policy (D5),
  not the reference `on_scroll` fallthrough.
- **D5 — TUI wheel = PageUp/PageDown repeats, NOT arrow keys (LOCKED):**
  criterion 3 names the sequences; keep `scroll_report`'s mouse-mode branch,
  do NOT use its `ALT_SCREEN → arrows` branch from the wheel path (omit it —
  no second caller should reach for it).

### Capture-replay pipeline (TERM-02 core)
- Port FE `applyCapture` (`terminalRegistry.ts:31-65`) as a byte-oriented
  `apply_capture` function in `webtmux-terminal`: split blob on `\n`
  (normalize `\r\n` → `\n` FIRST, char-boundary APIs only — never byte-slice,
  Pitfall 3), non-positive `screenRows` → legacy branch (`ESC[2J ESC[H` +
  positioned all-lines), else history/screen split at `len - rows` with the
  `ingestedHistory` delta counter (shrank-history resets to 0), payload =
  `delta.join(\r\n) + \r\n + ESC[2J ESC[H + positioned(screen)` where each
  screen row is prefixed `ESC[{r};1H` (explicit origin — full-width rows
  would auto-wrap before CRLF is consumed).
- Per-pane `{snapshotWritten: bool, ingestedHistory: usize}` lives in the
  `AppState` terminal store (outlives views — Pitfall 1). First
  `terminal.snapshot` per pane replaces; later ones drop until explicitly
  re-armed. `terminal.output` with `replace=true` runs the same
  `apply_capture` path WITHOUT the `snapshotWritten` gate (FE `replaceScreen`
  is not idempotent-gated); `replace=false` output feeds bytes straight to
  the `Processor`.
- Do NOT port the `pendingScreen`/`writingScreen` frame queue — alacritty's
  `Processor::advance` is synchronous; in-order commits on the GPUI thread
  give frame atomicity for free. Preserve its ordering invariant (one commit
  at a time, pump order).

### Store + input + hidden ingest (D3, D6, D7, D8 carry)
- `AppState` terminal store keyed by pane ID (owns every pane's `Terminal`;
  views borrow/render the active session's entries — inverted from the
  reference's view-owns-terminal shape because hidden sessions unmount views
  but must keep ingesting, Pitfall 6). `apply_event`'s terminal arms commit
  for every open session; commits never consult `active_session` (TERM-07 by
  construction). Synchronous commits on the GPUI thread in pump order (total
  order per pane, no locks/threads).
- Input path: copy check (Ctrl+Shift+C / Cmd+C with non-empty selection →
  copy, consume) → paste check (Ctrl+Shift+V / Cmd+V → paste, consume) →
  Ctrl+C with EMPTY selection falls through (interrupt passthrough) →
  `scroll_to_bottom` → `keystroke_to_bytes(keystroke, term.mode())` →
  `String::from_utf8` (expect-ok, contract-tested; never lossy) → fire-and-
  forget `terminal.input{paneId, data}` on the OWNING session's socket
  (pane → session resolved from the latest snapshot; never the active-session
  proxy — Phase-3 D5 lesson). Large pastes go as one message (server 6ms/512B
  batcher chunks them). Mouse-reporting apps get verbatim SGR passthrough.
- **D3 — Per-pane TUI-scroll map on `AppState`, default ON (LOCKED):**
  `HashMap<String, bool>` + `unwrap_or(true)` (FE `useTerminal.ts:89` /
  `PaneHeader.tsx:38` parity). No header switch UI in Phase 4 (headers are
  Phase 5); persistence arrives with Phase 6.
- **D6 — Reconnect re-captures every registered pane of the session
  (LOCKED):** on transport→Connected / `connection.ready`: `snapshotWritten
  = false` + `terminal.capture` per pane of that session. Idempotent, cheap,
  and what makes criterion 2's "including reconnect" hold for a single-client
  app whose server monitor dies on last-client-leave.
- **D7 — Retire store entries absent from the latest snapshot (LOCKED):** on
  each committed `state.snapshot`/`state.delta`, drop pane entries (and their
  `ingestedHistory`) not present in `snapshot.panes`. Prevents unbounded
  growth and cross-talk if tmux reuses `%N`.
- **D8 — Titles stored per pane, bell ignored (LOCKED):**
  `TerminalEvent::Title` → `HashMap<String, String>` last-title (feeds
  Phase-5 headers); `Bell` → no-op.
- **D4 — `hello` is never sent (LOCKED):** FE defines it but never calls it;
  Phase-3 D7 already bans it. First `terminal.resize` fires only after a real
  measure (size > 0) through the 100ms path.

### Resize dance (TERM-06)
- `TerminalView` canvas keeps the reference shape: `measure_cell` per paint,
  `cols/rows = floor(avail/cell)` clamped ≥ 1, local `term.resize` applied
  immediately + `resize_cb(cols,rows)` ONLY on change. The callback ARMS
  `AppState.pending_viewport` — it never sends (Pitfall 4).
- One generation-tagged 100ms timer (second arm supersedes the first, poll-
  pump pattern) recomputes `actualViewport` at fire time from settled sizes
  and sends ONE window-level `terminal.resize{cols.max(2), rows.max(1)}` on
  the session socket (FE `PaneWorkspace.tsx:41-57` scaling; tmux has ONE
  viewport per session — never per-pane-per-frame).
- Layout-key (`activeWindow|WxH|layout|pane-id:cells,zoom…`, FE `layoutKey`
  fields) change, not on first mount: 150ms timer → resize; 325ms timer →
  `snapshotWritten = false` + `terminal.capture` per visible registered pane.
- No recapture on plain tab switch (FE parity — store survives, views
  re-create against a current grid); reconnect and layout-key are the only
  re-arm paths.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets (from Phases 1–3)
- `webtmux-terminal` stub (`crates/terminal/src/lib.rs:1`, deps already in
  `crates/terminal/Cargo.toml:9-16`) — Phase 4 fills it; zero new packages.
- `ws.rs` envelope DTOs (`WsIncoming`/`WsOutgoing` with `replace: bool` +
  `screenRows: Option<i32>`, `MSG_TERMINAL_INPUT/RESIZE/CAPTURE`,
  `EV_TERMINAL_SNAPSHOT/OUTPUT`) and `SessionWsHandle::send_command`
  (correlated send + forget-timeout drop; `terminal.input` is uncorrelated by
  server design — no `ok()` in `handler.go:117-119` — so sends are
  fire-and-forget with the receiver dropped, same shape as Phase 3's
  `send_window_select`).
- `AppState::apply_event` (`app_state.rs:436-474`) triple guard ((a)
  generation, (b) envelope session match, (c) tab-liveness) + terminal
  parse-and-ignore arm (`:459`) that Phase 4 replaces with store commits;
  forward pump (`:997-1018`) already delivers every session's events
  regardless of visibility — TERM-07 rides it.
- `OpenSession` map + `ensure_session_socket` + `TOKIO_RT` (never block the
  GPUI thread); `TmuxPane` DTOs with null-slice discipline; `validate_`
  patterns from Phase 2; correlated-mutation + 10s-timeout + inline-error
  shape from 03-02 (reused only where correlation exists — terminal sends
  don't correlate).
- Reference port source (fully read in research):
  `E:\Coding Stuff\web-term\desktop-gpui\crates\terminal\src\` (`lib.rs:1-17`
  re-export list is the module checklist; `terminal.rs:97-207`
  process/resize/scroll/selection API; `input.rs:9-155` table + `:157-271`
  tests; `mouse.rs` geometry/SGR; `render.rs:96-126,128-253,255-399`
  measure/layout/paint; `event.rs:7-53` proxy; `view.rs:211-300,410-525`
  handlers + canvas + measure/resize_cb).

### Established Patterns
- Generation/epoch guard on async commits (poll pump → WS triple guard) →
  terminal frames pass the same three layers + `snapshotWritten` gate.
- In-process mock servers in tests (`TcpListener` + `accept_async`) → extend
  `ws_test.rs` mock with scripted terminal frames (no new crates).
- Pure `AppState` methods (no GPUI Context) stay headless-testable → store
  commit/wheel/debounce logic is pure; only `paint` needs a window.
- Cargo 1.96 accepts a SINGLE test-name filter per invocation (03-01/03-02
  deviation) — plan verify commands run one filter per command.
- Stable element ids as formatted `String`s (03-02 deviation — no
  `(&str, String)` `ElementId` impl); `.id()` before `overflow_x_scroll()`.

### Integration Points
- `terminal.capture{paneId}` → `terminal.snapshot{replace:true, screenRows,
  paneId, data}` (`handler.go:130-140`); `screenRows` = pane height, 0 when
  the pane is missing from the snapshot (keep the legacy fallback branch).
- `terminal.output` routing: `replace` → capture path, else raw append
  (FE `sockets.ts:36-44`); producers: Unix `%output` lines + Windows
  pipe-pane chunks (`replace:false`), Windows capture-poll/integrity frames
  (`replace:true` + `pane.Height`).
- `terminal.resize{cols,rows}` window-level on the session's OWN socket;
  server rejects `cols<=0||rows<=0`, enforces `cols≥2, rows≥1` — clamp
  client-side first. Unix `refresh-client -C`, Windows `resize-window`.
- `fe/src/features/terminal/{terminalRegistry.ts,useTerminal.ts}` +
  `PaneWorkspace.tsx:131-200` + `App.tsx:229-245` + `settingsStore.ts:33-45`
  are the parity anchors; backend protocol unchanged.
</code>

<specifics>
## Specific Ideas

- Fire the FIRST `terminal.capture` for a pane at TerminalView/pane-terminal
  creation (FE `useTerminal` initial-capture parity) — this is what turns a
  blank grid into replayed history on open.
- Manual single-pane resync hook (invalidate + capture one pane) is a
  nice-to-have ONLY if it falls out naturally; the Phase-5 pane menu can
  expose it later (RESEARCH open question 1). Do not gold-plate.
- Per-pane `terminal.resize` does not exist in the protocol (RESEARCH open
  question 2) — window-level only, even during future Phase-5 drags.
- User is away: decisions auto-accepted; context kept minimal (research +
  planner files fill detail).
</specifics>

<deferred>
## Specific Ideas / Deferred Ideas

- Pane grid geometry, split/zoom/resize drags, layout presets, pane/window
  context menus — Phase 5 (PANE-01..08, DLG-02).
- Settings page, theme tables (102 UI + 78 terminal), command palette,
  font/icon embedding beyond Phase-1 JetBrains Mono — Phase 6.
- Reconnect banner/toast UX — Phase 7 (STATE-03); Phase 4 recaptures silently.
- Session keyboard shortcuts — EXTRA-02, v2.
- Tab persistence on restart — EXTRA-01, v2.
</deferred>
