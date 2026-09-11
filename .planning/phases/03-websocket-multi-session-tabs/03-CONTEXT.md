# Phase 3: WebSocket Client & Multi-Session Tabs - Context

**Gathered:** 2026-09-06
**Status:** Ready for planning
**Mode:** Auto-accepted autonomous (user sleeping — D1–D8 from 03-RESEARCH.md locked as decided; flagged for post-hoc audit)

<domain>
## Phase Boundary

Stateful half of the protocol client inside the Phase-2 shell: a `webtmux-backend-client`
WS layer (`ws.rs` — envelope DTOs, `connect_session()` + flume-based `SessionWsHandle`,
`normalize_ws_url()`, `request_id()`), per-session connection maps on `AppState`
(`open_sessions: Vec<String>` + `sessions: HashMap<String, OpenSession>` with generation,
transport, snapshot, handle, pending correlations), title-bar window tabs + Settings gear
(SHELL-01), sidebar session context menu + rename dialog + kill-confirm flow (SESS-04,
SESS-05), and the two-layer WS generation guard (STATE-04). Tab switching never touches
sockets. Delivers SHELL-01, SESS-02, SESS-04, SESS-05, STATE-04. Out: terminal
output/snapshot ingestion (Phase 4 owns `terminal.snapshot`/`terminal.output` — the Phase-3
pump parses-and-ignores them), reconnect backoff/banners/toasts (Phase 7, STATE-03),
window/pane menus + split/zoom/resize (Phase 5), Settings page + themes + palette
(Phase 6), session shortcuts (EXTRA-02, v2), tab persistence (EXTRA-01, v2).
Backend protocol unchanged — log gaps, don't fork.

</domain>

<decisions>
## Implementation Decisions

### WS transport crate (D5, D6, D7 carry)
- New `crates/backend-client/src/ws.rs` on the already-pinned stack (tokio-tungstenite
  =0.26.2, futures-util =0.3.32, flume =0.12.0 — NO new deps): `WsIncoming` / `WsOutgoing`
  envelope DTOs verbatim from `be/internal/realtime/protocol.go` (serde `default`
  discipline; `replace: bool` plain not Option per pitfall 4), `SessionSnapshot` reusing
  Phase-2 `TmuxSession`/`TmuxWindow`/`TmuxPane` DTOs with `deserialize_null_default`,
  `normalize_ws_url()` (`http(s)://host:port` → `ws(s)://host:port/api/ws?session=<query-encoded>`,
  NOT web-term's append-`/ws` rule), `request_id()` via workspace-pinned `rand =0.10.2`,
  `connect_session()` returning flume-based `SessionWsHandle` (reference
  `terminal_ws.rs` split-pump shape). No `hello` in Phase 3 (D7 — `hello` is a viewport
  resize, not a handshake; fake 80×24 shrinks other frontends); send `state.resync` on
  `connection.ready` (FE `sockets.ts` parity). Correlated send: `pending:
  HashMap<requestId, oneshot::Sender>` per session + 10s forget-timeout (FE `commands.ts` parity).
- **D5 — Rename rides the target's own socket (FE-quirk correction, LOCKED):** Go binds
  `session.rename` to the URL session (`handler.go:184-185`); the FE facade's
  active-socket proxy renames the wrong session. GPUI resolves the target handle at submit
  time; if the target tab isn't open, `ensure_session_socket(target)` first.
- **D6 — Kill transport order (LOCKED):** victim socket → any live socket + explicit
  `session` field (Go honors it, `handler.go:186-193`) → ephemeral one-shot WS
  (connect → kill → close, commits NOTHING — guard tab-liveness rule drops its bootstrap).
- **D7 — No `hello`; `state.resync` on ready (LOCKED).**

### Tab state + generation guard (D3, D4 carry)
- `AppState`: `open_sessions: Vec<String>` (ordered tab list, FE `appStore.openSessions`),
  `sessions: HashMap<String, OpenSession>` (`generation`, `transport`, `snapshot`,
  `handle`, `pending`, `last_error`), `active_session: Option<String>` UNCHANGED seam
  (view pointer only). `open_session` (push + ensure-socket if new, then activate),
  `set_active_session` (assign + notify — NO socket calls, ever), `close_session`
  (remove + drop entry + neighbor `min(idx, len-1)`, FE `appStore.ts:61-73` parity).
  Workspace body for an open session renders a Phase-3 placeholder panel (snapshot summary:
  session name + window list — real WS data proving the path; terminals are Phase 4).
- **D3 — No visible session-tab strip (LOCKED, FE 1:1):** "tabs" = `open_sessions` state +
  sidebar highlight + title-bar window tabs. FE has no session strip (grep-verified
  `App.tsx:229-245` hidden workspaces). Do not invent a strip row.
- **D4 — Reconnect policy = connect-on-open + teardown-on-close ONLY (LOCKED):** no backoff
  loop, no banner, no toasts in Phase 3; per-tab `transport` + `last_error` stored so
  Phase 7 (STATE-03) has renderable state. Dropped sockets sit `Disconnected`.
- Guard (STATE-04, all three layers): (a) per-connection monotonic generation — pump
  captures `(session, generation)` at spawn, apply path drops `generation != current`;
  (b) envelope session match — drop snapshot/delta whose `msg.session` ≠ connection
  session; (c) tab-liveness — drop events for sessions no longer in the map.
  Rename re-resolution bumps generation (old→new entry migration); stale old-generation
  events die on layer (a).

### Title bar + context menus + dialogs (D1, D2, D8 carry)
- **SHELL-01:** extend `views/tab_strip.rs` (S1 `h(px(44.0))` drag bar kept): divider +
  `WindowTabs` middle (flex-1, ONLY when `active_session` set; chips `h-7 max-w-44`,
  `{index}: {name}` truncated, active bg `#2d2d2d`/text `#d4d4d4`, idle `#808080`,
  hover `#262626`) fed by active snapshot's `windows` + `active_window`; chip click =
  fire-and-forget correlated `window.select` (FE `WindowTabs.tsx:120` parity, no await).
  Settings gear (`Settings` lucide ghost): `active_session = None` + `showing_settings`
  placeholder page ("available in Phase 6" honest stub, tabs stay open underneath).
  **D8 — Plus (new-window) button OMITTED (LOCKED):** window ops are Phase 5; no dead
  controls. Window-chip context menu is Phase 5, not here.
- Sidebar rows: wrap in `gpui-component` `ContextMenuExt::context_menu` with STABLE ids
  `.id(("session-row", name))` (poll-tick state loss pitfall); menu = `Rename` +
  separator + `Kill Session` destructive only (no future-item dead entries).
  Left-click select stays `MouseButton::Left`-only (no right-click conflict by construction).
- **D1 — Rename dialog tokens = DLG1 reuse (LOCKED):** 440px, `#1e1e1e`, 1px `#3c3c3c`,
  radius 8, padding 20, title "Rename Session", label "New name", autofocus + Enter-submit,
  `Rename` disabled while empty/busy, inline `#7F1D1D` error. `RenameSessionForm` Entity
  on `AppState` (DLG1 lifetime). Submit: `validate_session_name` pre-flight → send on
  target socket → 10s correlated await → success re-resolves socket old→new (generation+1,
  move `open_sessions`/`active_session`/`expanded_sessions`, `trigger_poll`, notify);
  error/timeout → inline error, dialog stays open.
- **D2 — Kill-confirm source = `DesktopSettings` (LOCKED):** add `confirm_kill_session` (+
  `confirm_kill_pane`, `confirm_kill_window` for FE-struct parity, honored Phase 5/6),
  all default `true` with `#[serde(default)]` so legacy Phase-1/2 files keep confirming.
  Confirm dialog reuses DLG1 tokens with destructive Kill (`#7F1D1D` fill, white text).
  Success: drop entry + `close_session` neighbor activation + `trigger_poll`.
  Errors: inline destructive (toasts are Phase 7).

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets (from Phases 1–2)
- `webtmux-backend-client` (`models.rs` DTOs + `deserialize_null_default`, `validation.rs`
  `validate_session_name`, `rest.rs` `RestClient::new(&base_url)` + `RestError`) — ws.rs
  extends the same crate, same disciplines; workspace pins already include
  tokio-tungstenite/futures-util/flume (`crates/backend-client/Cargo.toml:10-17`).
- `AppState` entity (`app_state.rs`): `TOKIO_RT` global runtime (all WS I/O spawned there —
  never block the GPUI thread), `trigger_poll` + `poll_generation` guard pattern to mirror
  for the WS guard, `create_session_form` Entity lifetime = template for
  `RenameSessionForm`, `workspace_state()` routing (kill-last-tab → `None` reuses it).
- `views/tab_strip.rs` S1 bar (drag center filler, identity, 3 window controls, `#7F1D1D`
  close hover = kill-button token source), `views/sidebar.rs` SB1 rows (stable-id + menu
  anchor point; left-click select at `sidebar.rs:154`), `views/session_states.rs` ST1
  pages + `render_workspace_body`, `views/create_session_dialog.rs` DLG1 (Dialog +
  `InputState` + `is_submitting` guard + inline error).
- `webtmux-settings` atomic-write + `.bak` recovery (`settings/src/lib.rs:128-153`) —
  kill-confirm flags ride the same path; `icons.rs` lucide set (+ `SETTINGS_SVG` already).
- Pins: `rand =0.10.2` for `request_id()`; `tokio` net/time/sync in workspace features.

### Established Patterns
- Generation/epoch guard on async commits (poll pump) → WS guard layers (a)+(c).
- DLG1 form-entity lifetime (fresh Entity per open, `None` on dismiss) → rename + kill-confirm.
- In-process mock servers in tests (`tokio::net::TcpListener`) → mock WS server via
  `accept_async` (no new crates).
- `RestClient::new`-inside-runtime rule → `connect_async` likewise (needs reactor).

### Integration Points
- `GET /api/ws?session=<name>` on the supervisor `base_url` (learned at
  `SupervisorEvent::Ready`); plain `connect_async`, no auth/subprotocol (Go
  `handler.go:26-63`; permissive origin).
- Unsolicited bootstrap: `connection.ready` + `state.snapshot` arrive with no client
  message; client answers ready with `state.resync`.
- `fe/src/lib/{websocket,sockets,protocol,commands}.ts` + `appStore.ts`/`tmuxStore.ts` +
  `SessionContextMenu.tsx` + `AppTitleBar.tsx`/`WindowTabs.tsx` are the parity anchors;
  `web-term/.../terminal_ws.rs` + `session.rs` are the Rust pump/lifecycle anchors.
- Backend protocol unchanged: rename URL-binding, kill explicit-target, delta `Session`
  tagging, `command.success/error` per requestId, victim-socket NOT closed server-side
  (client closes locally) — any deviation found is logged as a backend issue.
</code_context>

<specifics>
## Specific Ideas

- Fire-and-forget `window.select` on chip click (mirror FE); optimistic flip only if UAT flags lag (reversible, RESEARCH open question 1).
- Ephemeral kill socket commits nothing — document in plan so stray snapshots can't create phantom tabs (RESEARCH open question 2).
- Sidebar badges stay on the REST tree (not snapshots); snapshot counts display-only.
- `window.select` on chip click is the read-path proof of the correlated channel before Phase 5.
</specifics>

<deferred>
## Specific Ideas / Deferred Ideas

- Terminal `output`/`snapshot` ingestion + capture replay — Phase 4 (pump parses-and-ignores).
- Reconnect backoff loop + banner + command-error toasts — Phase 7 (STATE-03).
- Window/pane context menus, layout presets, split/zoom/resize — Phase 5.
- Settings page, theme system, command palette — Phase 6 (gear renders honest placeholder).
- Session keyboard shortcuts — EXTRA-02, v2.
- Tab persistence on restart — EXTRA-01, v2.
</deferred>
