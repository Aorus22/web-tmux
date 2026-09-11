# Phase 3: WebSocket Client & Multi-Session Tabs - Research

**Researched:** 2026-09-06
**Domain:** Per-session WebSocket lifecycle (`webtmux-backend-client` WS layer, `tokio-tungstenite =0.26.2`), always-connected multi-session tab state (`open_sessions` + per-session snapshot/transport maps in `AppState`), title-bar window tabs + Settings gear (`SHELL-01`), session rename/kill context-menu flows over correlated WS mutations, and the two-layer WS generation guard (`STATE-04`).
**Confidence:** HIGH — Go WS server (`be/internal/realtime/*.go`, `be/internal/tmux/service.go`, `monitor.go`), FE socket stack (`fe/src/lib/websocket.ts`, `sockets.ts`, `socket.ts`, `protocol.ts`, `commands.ts`, stores), web-term reference (`terminal_ws.rs` connect/attach + pump pattern, `session.rs` tab manager), current desktop-gpui code (`app_state.rs`, `tab_strip.rs`, `sidebar.rs`, `session_states.rs`, `create_session_dialog.rs`), workspace pins (`Cargo.toml`, `Cargo.lock`), and vendored `gpui-component-0.6.0` menu source were all read and line-verified this session.

<user_constraints>
## User Constraints (from CONTEXT.md)

> No `03-CONTEXT.md` exists yet (phase directory did not exist at research time). Constraints below are copied verbatim from the authoritative upstream sources that bind Phase 3: `.planning/ROADMAP.md` Phase 3 section and the deferred items of `.planning/phases/02-rest-client-sidebar-session-management/02-CONTEXT.md`.

### Locked Decisions

- **Phase goal (ROADMAP.md:77-81):** "The user can hold several tmux sessions open as always-connected workspace tabs with race-free, correlated mutations." Depends on Phase 2. Requirements: `SHELL-01`, `SESS-02`, `SESS-04`, `SESS-05`, `STATE-04`.
- **Success criteria (ROADMAP.md:83-88, verbatim):**
  1. "The user opens multiple sessions as tabs; all stay connected simultaneously and switching between them never disconnects or re-handshakes a live session"
  2. "The title bar renders drag region, app identity, the active session's tmux-window tabs, and the Settings gear — visually identical to the Electron title bar"
  3. "Renaming a session via its context menu opens a dialog with a real text input and the new name propagates to sidebar and tabs — including mid-session socket re-resolution"
  4. "Killing a session via context menu confirms (when the stored kill-confirm setting says so) and closes its tab cleanly"
  5. "Rapid tab switching while a session churns never applies another session's snapshot events to the open tab (WS generation guard contract)"
- **Deferred from Phase 2 (02-CONTEXT.md:77-79, verbatim):** "Rename/kill session flows + context menus — Phase 3 (mutating commands ride the WebSocket there)".
- **Out of scope carried forward:** session keyboard shortcuts (`ctrl-tab`/`alt-1`… — `EXTRA-02`, v2); tab persistence on restart (`EXTRA-01`, v2); terminal rendering/output ingestion (Phase 4 owns `terminal.snapshot`/`terminal.output` consumption); reconnect banners/toasts (`STATE-03`, Phase 7); backend protocol changes (frontend-only milestone — log gaps, don't fork).
- **Active-session seam (prior state):** `AppState.active_session: Option<String>` + placeholder workspace — Phase 3 replaces the placeholder with real tabs. Sidebar polling (`trigger_poll`, `poll_generation`) and DLG1 dialog Entity pattern are established and must be extended, not reinvented.

### the agent's Discretion

Rename dialog tokens, kill-confirm default source, tab-strip geometry, and WS reconnect policy within the Phase 3 vs Phase 7 boundary — drafted as auto-accepted reversible decisions in `## Grey-Area Decisions (draft for CONTEXT.md)` below.

### Deferred Ideas (OUT OF SCOPE)

- Session keyboard shortcuts — `EXTRA-02`, v2.
- Tab persistence on restart — `EXTRA-01`, v2.
- Terminal output ingestion/rendering (`terminal.output`, `terminal.snapshot`, capture replay) — Phase 4.
- Auto-reconnect backoff loop + reconnect banner + command-error toasts — Phase 7 (`STATE-03`).
- Window/pane context menus, layout presets, split/zoom/resize — Phase 5.
- Settings page, theme system, command palette — Phase 6.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SHELL-01 | Frameless title bar with drag region, app identity, tmux-window tabs of the active session, Settings gear — visually identical to Electron | Extend `views/tab_strip.rs` (S1, `h(px(44.0))` drag bar already built): insert `WindowTabs` middle (`h-7` chips, `max-w-44`, active `bg-secondary`) fed by the active session's WS snapshot, plus Settings gear (`Settings` lucide, opens Settings placeholder per FE `setActiveSession(null)` + `sidebarPage='settings'` — page itself is Phase 6). Geometry/tokens in §Technical Mechanics 7. |
| SESS-02 | Open a session as a workspace; multiple sessions stay connected simultaneously; switching never tears down a connection | `open_sessions: Vec<String>` + `HashMap<String, SessionConn>` (one `tokio-tungstenite` connection per open session, FE `sockets.ts` parity) in `AppState`; `set_active_session` only retargets the view index, never touches sockets (§2, §4). |
| SESS-04 | Rename a session via context menu (dialog with real text input) | `session.rename` (`newName`) sent on the **target session's own socket** (Go binds rename to the URL session, `handler.go:184-185`); `RenameSessionForm` Entity reusing the DLG1 Dialog+`InputState` pattern; post-success socket re-resolution old→new (§5). Client-side pre-flight reuses `validate_session_name`. |
| SESS-05 | Kill a session via context menu with confirmation (respecting kill-confirm setting) | `session.kill` with optional explicit `session` field (Go honors cross-session kill, `handler.go:186-193`); confirm dialog gated by new `DesktopSettings.confirm_kill_session` (default `true`, FE `settingsStore.ts` parity); on success close socket + tab with neighbor activation (§6). |
| STATE-04 | WS generation guard: stale session events never render into the wrong tab | Two layers: (a) per-connection monotonic generation — superseded sockets never reconnect/deliver (FE `websocket.ts:37-43,67-124`); (b) per-event session match + apply-time generation check before committing snapshots (§3). |
</phase_requirements>

## Summary

Phase 3 adds the stateful half of the protocol client: where Phase 2 polls metadata over REST, Phase 3 holds one live WebSocket per open session tab (`GET /api/ws?session=<name>`, `coder/websocket` server side, `tokio-tungstenite =0.26.2` client side) and routes correlated mutations (`session.rename`, `session.kill`, `state.resync`, window selects later) with `requestId → command.success/error` correlation and a 10s forget-timeout.

The Go server was read in full and imposes three non-obvious contracts the plan must honor: (1) the initial `state.snapshot` + `connection.ready` arrive **unsolicited** immediately after connect (`handler.go:57-62`) — no handshake message is required to get state, and `hello` only resizes the tmux viewport (so Phase 3 must NOT send `hello`; sizing belongs to Phase 4 with real terminal dimensions); (2) `session.rename` binds to the **URL session** (`handler.go:184-185`) and ignores any `session` field — renaming a non-active session through the active tab's socket (which is what the FE facade accidentally does) renames the wrong session; (3) `session.kill` DOES honor an explicit `session` field (`handler.go:186-193`), so kills can ride any live socket, while `state.delta` events always carry `Session` (`hub.go:103-108`) precisely so clients can drop cross-session leakage.

The FE has **no visible session-tab strip** (verified by grep: `openSessions` only drives hidden mounted workspaces in `App.tsx:229-245`; switching = `setActiveSession`, sidebar click = `openSession`). "Tabs" in every success criterion therefore means: entries in `openSessions` with live sockets + live snapshots, the sidebar active highlight, and the title-bar **window** tabs of the active session. The planner must not invent a session strip row — that decision is recorded in the grey-area list as auto-accepted (FE 1:1 parity wins).

**Primary recommendation:** Add `crates/backend-client/src/ws.rs` (`WsClient` envelope DTOs + `connect_session()` returning a flume-based `SessionWsHandle` + `normalize_ws_url()` + `request_id()`), store `open_sessions: Vec<String>` + `sessions: HashMap<String, OpenSession>` (`generation`, `transport`, `snapshot`, `handle`, `pending: HashMap<requestId, oneshot::Sender>`) on `AppState`, extend `views/tab_strip.rs` with `WindowTabs` + Settings gear, wrap sidebar rows in `gpui-component` `ContextMenuExt::context_menu` (verified present in `menu/context_menu.rs`), add `RenameSessionForm` + kill-confirm dialogs reusing DLG1 tokens, and gate every snapshot commit behind the (session, generation) guard.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| WS envelope DTOs + connect + correlated send | `webtmux-backend-client` crate (`ws.rs`, pure tokio) | — | Headless, GPUI-independent; unit-testable against an in-process `tokio-tungstenite::accept_async` mock with zero new deps. |
| Per-session connection map + generations + snapshots + pending correlations | `AppState` entity (`webtmux` app crate) | Background tokio pumps (one read pump per session) | GPUI main thread owns canonical tab state; pumps only forward tagged `(session, generation, WsEvent)` messages. Mirrors FE `sockets.ts` map + `tmuxStore` per-session records. |
| Session tab lifecycle (open/switch/close/neighbor activation) | `AppState` methods (`open_session`, `set_active_session`, `close_session`) | `views/sidebar.rs`, `views/session_states.rs` | FE `appStore.ts:52-80` parity: `openSession` appends + activates; `closeSession` activates neighbor at `min(idx, len-1)`; `setActiveSession` never touches sockets. |
| Title-bar window tabs + Settings gear (`SHELL-01`) | `views/tab_strip.rs` | Active session's `Snapshot` (windows + `activeWindow`) | FE `AppTitleBar.tsx` + `WindowTabs.tsx` parity; read-only in Phase 3 except `window.select` on click (cheap, same correlated path). |
| Session context menu + rename dialog + kill confirm | `views/session_context_menu.rs` (new) + dialog entities on `AppState` | `gpui-component` `menu::ContextMenuExt` / `Dialog` / `InputState` | Reuses proven 02 patterns (DLG1 form-entity lifetime, `is_submitting` guard, inline destructive error). |
| Kill-confirm setting | `webtmux-settings` crate (`DesktopSettings.confirm_kill_session`, default `true`) | `views/session_context_menu.rs` gate | FE `settingsStore.ts:24,44` parity (`confirmKillSession: true`); `#[serde(default=true)]` so existing settings files keep confirming. |
| Terminal event ingestion (`terminal.output/snapshot`) | Phase 4 — NOT this phase | — | WS read pump must parse-and-ignore (or buffer) terminal events in Phase 3; consuming them is Phase 4 scope. |

---

## Standard Stack

### Core (all already pinned — NO new packages)

| Library | Pinned Version | Purpose | Why Standard / Verification |
|---------|----------------|---------|-----------------------------|
| `tokio-tungstenite` | `=0.26.2` (`connect`, `handshake` features, no TLS) | WS client transport | [VERIFIED: `desktop-gpui/Cargo.toml:27`] workspace pin; [VERIFIED: `desktop-gpui/Cargo.lock:6950-6953`] resolves to `0.26.2`. Identical pin in reference `web-term/desktop-gpui/Cargo.toml` (verified by grep this session). Plain `ws://127.0.0.1` needs no TLS features. |
| `futures-util` | `=0.3.32` | `SinkExt`/`StreamExt` split pumps | [VERIFIED: `desktop-gpui/Cargo.toml:28`]; [VERIFIED: `desktop-gpui/Cargo.lock:2096-2098`] `0.3.32`. Reference `terminal_ws.rs:4,318` uses the same split + `connect_async` pattern to port. |
| `flume` | `=0.12.0` | Unbounded outbound queue + inbound event channels | [VERIFIED: `desktop-gpui/Cargo.toml:25`]. Reference `terminal_ws.rs:230-234,442-444` proves the `flume::unbounded` outbound/inbound pump shape on this exact version. |
| `gpui-component` | `=0.6.0` | `menu::ContextMenuExt`/`PopupMenu`, `Dialog`, `Input`/`InputState` | [VERIFIED: `C:\Users\alyza\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\gpui-component-0.6.0\src\lib.rs:53`] `pub mod menu`; [VERIFIED: `.../menu/mod.rs:9-12`] exports `ContextMenu, ContextMenuExt, ContextMenuState`, `PopupMenu, PopupMenuItem`; [VERIFIED: `.../menu/context_menu.rs:13-39`] `context_menu()` builder + stable-ID requirement. `alert` module also exists ([VERIFIED: `lib.rs:26`] `pub mod alert`) but the kill confirm should reuse the DLG1 `Dialog` pattern for token control. |
| `tokio` | `=1.53.1` | Runtime, `time::sleep` backoff (Phase 7), `sync::oneshot` correlation | [VERIFIED: `desktop-gpui/Cargo.toml:35-42`]. |
| `serde` / `serde_json` | `=1.0.229` / `=1.0.151` | WS envelope DTOs | [VERIFIED: `desktop-gpui/Cargo.toml:43-44`]. Same null-slice discipline as Phase 2 (`deserialize_null_default`). |

**Installation:** none — `crates/backend-client/Cargo.toml:10-17` [VERIFIED] already depends on `tokio`, `reqwest`, `serde`, `serde_json`, `thiserror`, `tokio-tungstenite`, `futures-util`, `flume`. Phase 3 adds a module, not a dependency. (`crates/webtmux/Cargo.toml:35-36` also carries `tokio-tungstenite` + `futures-util` in dev-deps for integration tests.)

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `tokio-tungstenite` client | `coder/websocket` Rust port / `async-tungstenite` | Rejected: server uses `coder/websocket` (Go) but the wire is RFC6455 — `tokio-tungstenite` is the workspace-pinned, web-term-proven client. No interop issue (both are plain WS transports). |
| `flume` channels for pump→GPUI bridge | `tokio::sync::mpsc` directly into `cx.spawn` | Either works; `flume` matches the reference `TerminalWsHandle` shape and supports sync `send` from GPUI event handlers (e.g. kill-by-name from a context menu without an async context). |
| `gpui-component` `ContextMenuExt` | Hand-rolled right-click div + absolute popup | Rejected per Don't Hand-Roll: element-state management, dismissal, and anchoring are already solved in `menu/context_menu.rs`. |
| Visible session-tab strip row | Nothing (FE parity) | Rejected: FE has no session strip (grep-verified); adding one breaks 1:1. See grey-area D3. |

## Package Legitimacy Audit

> No external packages are installed in Phase 3 — all transport, channel, and UI primitives are already workspace-pinned and lockfile-committed.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| *(none — no new installs)* | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.
*Registry re-verification (`npm view`-class check) is N/A: the Rust pins were confirmed in `Cargo.lock` (`tokio-tungstenite 0.26.2`, `futures-util 0.3.32`) rather than re-queried from crates.io — versions are frozen by the committed lockfile per the Phase 1 exact-pin policy.*

---

## WS Contracts & Go Backend Source Truth

### 1. Endpoint: `GET /api/ws?session=<name>` — handshake + unsolicited bootstrap

[VERIFIED: `be/internal/server/router.go:34-35`] `mux.Handle("GET /api/ws", ws)`; [VERIFIED: `be/internal/realtime/handler.go:26-63`]

```go
session := r.URL.Query().Get("session")          // handler.go:27
if session == "" { http.Error(w, "missing session query parameter", 400); return }
conn, err := websocket.Accept(w, r, &websocket.AcceptOptions{OriginPatterns: []string{"*"}}) // :33-35
monitor, err := h.hub.connect(ctx, session, client)  // :48 — fails "session %q does not exist" if dead
go client.writeLoop(ctx)                             // :55
client.Send(Outgoing{Type: EvConnectionReady, Session: session})          // :58
if snap := monitor.Snapshot(); snap != nil {
    client.Send(Outgoing{Type: EvStateSnapshot, Session: session, Snapshot: snap}) // :60-62
}
```

**Contract consequences for the Rust client:**
- URL: `ws://127.0.0.1:<port>/api/ws?session=<urlencoded-name>` (http→ws scheme swap of the supervisor `base_url`; FE parity `websocket.ts:70-73`). No auth headers, no subprotocols, permissive origin — plain `connect_async` works [ASSUMED interop note: `tokio-tungstenite` client vs `coder/websocket` server is standard RFC6455; the web-term reference runs the same combination — LOW risk, proven by integration test in Validation Architecture].
- Connecting to a nonexistent session yields `server.error` + close `PolicyViolation` (`handler.go:48-52`) — the client must surface this as a per-tab transport error (tab stays open, shows error on retry via `state.resync`; full banner UX is Phase 7).
- `connection.ready` + full `state.snapshot` arrive **without any client message**. `hello` is ONLY a viewport resize (`handler.go:103-110` → `ResizeTerminal`, bounds `cols>=2, rows>=1` per `service.go:295-298`). **Phase 3 sends no `hello`** (no real dimensions exist before Phase 4; a fake 80×24 would shrink the user's tmux viewport).
- FE `onReady` sends `state.resync` (`sockets.ts:23-29`) as a belt-and-braces full refresh — cheap, idempotent, copy it.

### 2. Envelopes: `Incoming` (client→server) / `Outgoing` (server→client)

[VERIFIED: `be/internal/realtime/protocol.go:6-56`] field names verbatim; [VERIFIED: `fe/src/lib/protocol.ts:8-68`] `MSG`/`EV` string constants match Go consts 1:1 (`protocol.go:58-97`).

```rust
// crates/backend-client/src/ws.rs (sketch — planner expands)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsIncoming {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "requestId", skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub cols: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub rows: Option<i32>,
    #[serde(rename = "paneId", skip_serializing_if = "Option::is_none")] pub pane_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub amount: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "command")] pub initial_command: Option<String>,
    #[serde(rename = "newName", skip_serializing_if = "Option::is_none")] pub new_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub title: Option<String>,
    #[serde(rename = "otherPaneId", skip_serializing_if = "Option::is_none")] pub other_pane_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub layout: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsOutgoing {
    #[serde(rename = "type")] pub msg_type: String,
    #[serde(rename = "requestId", default)] pub request_id: Option<String>,
    #[serde(default)] pub session: Option<String>,
    #[serde(rename = "paneId", default)] pub pane_id: Option<String>,
    #[serde(default)] pub data: Option<String>,
    #[serde(default)] pub replace: bool,                       // NOT Option — Go omits false, serde default=false
    #[serde(rename = "screenRows", default)] pub screen_rows: Option<i32>,
    #[serde(default)] pub message: Option<String>,
    #[serde(default)] pub seq: u64,                            // always 0 today (client.go:85-93) — keep for forward compat
    #[serde(default)] pub snapshot: Option<SessionSnapshot>,
}
```

Message-type constants (verbatim, [VERIFIED: `protocol.go:58-97`]): `hello`, `terminal.input`, `terminal.resize`, `terminal.capture`, `pane.select/split/resize/kill/rename/zoom/break/swap`, `window.select/create/rename/kill/layout/move/break-active`, `session.create/rename/kill`, `state.resync` → events `connection.ready`, `state.snapshot`, `state.delta`, `terminal.snapshot`, `terminal.output`, `command.success`, `command.error`, `tmux.disconnected`, `tmux.reconnecting`, `server.error`.

### 3. `SessionSnapshot` DTO (state.snapshot / state.delta payload)

[VERIFIED: `be/internal/tmux/model.go:45-52`] struct + JSON tags verbatim:

```go
type Snapshot struct {
    Session      Session  `json:"session"`
    Windows      []Window `json:"windows"`
    Panes        []Pane   `json:"panes"`
    ActiveWindow string   `json:"activeWindow"`
    ActivePane   string   `json:"activePane"`
}
```

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub session: TmuxSession,                       // reuse Phase-2 models.rs DTOs verbatim
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub windows: Vec<TmuxWindow>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub panes: Vec<TmuxPane>,
    #[serde(default)] pub active_window: String,    // "@N" stable ID — Index is display-only
    #[serde(default)] pub active_pane: String,      // "%N" stable ID
}
```

Null-slice discipline from Phase 2 applies (`deserialize_null_default`, `models.rs:5-12` [VERIFIED]) — Go `Tree()` already guards panes (`snapshot.go:150-152`) but `Snapshot()` one-shot queries do not guarantee non-nil `windows`/`panes` on all paths.

### 4. `session.rename` — URL-session binding (the FE-quirk correction)

[VERIFIED: `be/internal/realtime/handler.go:184-185`] `failOr(fail, h.svc.RenameSession(ctx, session, in.NewName), ok)` where `session := c.session` (`handler.go:88`) — **the `session` field of the message is ignored for rename**. [VERIFIED: `be/internal/tmux/service.go:94-102`] rename validates `newName` with the same `ValidateSessionName` rules (client pre-flight reuses `validate_session_name`) and routes via the session's control monitor with one-shot fallback.

FE quirk (documented so the planner does NOT copy it): `SessionContextMenu.doRename` calls the `tmuxSocket` facade, which proxies to the **active** session's socket (`socket.ts:17-28`). Right-clicking a non-active sidebar row therefore renames the wrong session. The GPUI client must send `session.rename` on the **target session's own socket** (`ensure tab socket → send {type:"session.rename", newName, requestId}`).

### 5. `session.kill` — explicit-target support

[VERIFIED: `be/internal/realtime/handler.go:186-193`]: `target := session; if in.Session != "" { target = in.Session }` then `KillSession(ctx, target)`. [VERIFIED: `be/internal/tmux/service.go:106-126`]: control-mode kill with one-shot fallback + `takeMonitor(name)` + `m.Stop()` — the server tears down its own monitor, so the victim socket goes quiet (no more `state.delta`); the WS TCP connection itself is NOT proactively closed by the server.

FE parity: `sessionKillByName(name)` (`websocket.ts:320-324`) exists exactly for "the context menu can target a session that is not the active tab". Transport selection order for GPUI: (1) victim's own socket if open; (2) any live socket + explicit `session` field; (3) ephemeral one-shot WS (connect → kill → close) when zero tabs are open. All three share one send path.

### 6. `state.delta` — server-side session tagging (why the guard is duality, not paranoia)

[VERIFIED: `be/internal/realtime/hub.go:104-108`]: every `EvState` relay sends `Outgoing{Type: EvStateDelta, Session: session, …}` with the comment "the frontend can drop state that arrives on a stale connection left over from a previous session". The server architects already hit this bug class. FE defense is twofold — per-socket `generation` (`websocket.ts:37-43, 67-124`: stale `onmessage`/`onclose` early-return) PLUS per-event `msg.session !== this.session` drop (`websocket.ts:140-148`) PLUS view mirroring only for `viewSession` (`tmuxStore.ts:59-63, 76-86`). The GPUI client ports all three (§Technical Mechanics 3).

### 7. Command correlation + timeout

[VERIFIED: `be/internal/realtime/handler.go:87-101`] every action with `requestId` gets exactly one of `command.success`/`command.error` echoing the ID; untracked input (`terminal.input`) gets no reply by design. [VERIFIED: `be/internal/tmux/monitor.go:758-797`] FIFO `%begin/%end/%error` correlation — responses arrive in command order per session. FE: `runCommand` 10s timeout + `forgetCommand` so late responses are ignored and busy flags never stick (`commands.ts:17-36`, `tmuxStore.ts:128-135`). GPUI: `pending: HashMap<String, oneshot::Sender<CommandResult>>` per session + `tokio::time::timeout(10s)` at the await site; timeout drops the entry (late `command.success` then finds no entry → ignored).

---

## Technical Mechanics & Implementations

### 1. Per-session WS map (FE `sockets.ts` → Rust)

[VERIFIED: `fe/src/lib/sockets.ts:21-101`] `Map<string, {socket, connected}>` with `ensureSocket` (create-once + `connect()` if not connected) and `closeSocket` (close + delete). [VERIFIED: `fe/src/App.tsx:152-169`] the `ensuredRef` effect: new `openSessions` entries get `clearSession + setTransport(connecting) + ensureSocket`; removed entries get `closeSocket + clearSession`.

```rust
pub struct OpenSession {
    pub generation: u64,                 // bumped per (re)connect — STATE-04 layer (a)
    pub transport: TransportState,       // Connecting | Connected | Reconnecting | Disconnected
    pub snapshot: Option<SessionSnapshot>,
    pub handle: Option<SessionWsHandle>, // flume-based, reference terminal_ws.rs:228-299 pattern
    pub pending: HashMap<String, oneshot::Sender<CommandResult>>,
    pub last_error: Option<String>,      // server.error / connect failure for the tab
}
// AppState: pub open_sessions: Vec<String>,  // ORDERED tab list (FE appStore.openSessions)
//           pub sessions: HashMap<String, OpenSession>,
//           pub active_session: Option<String>,  // UNCHANGED seam — view pointer only
```

Reference pump shape to port: `TerminalWsClient::spawn_pumps` ([VERIFIED: `web-term/.../terminal_ws.rs:424-525`]) — split `connect_async` stream, unbounded outbound queue task + inbound read task, `AtomicU8` status. Differences for web-tmux: text JSON envelopes (not binary PTY bytes), first-two-messages bootstrap (`connection.ready` → send `state.resync`; `state.snapshot` → commit), per-message `(session, generation)` tagging into the GPUI-bound channel.

`normalize_ws_url`: does NOT follow web-term's append-`/ws` rule ([VERIFIED: `terminal_ws.rs:30-45`]) — web-tmux rule is `http(s)://host:port` → `ws(s)://host:port/api/ws?session=<percent-encoded>` (FE `websocket.ts:70-73` parity). Percent-encode with the same set as `encodeURIComponent` (session names allow `/` per `ValidateSessionName`, so path-segment encoding is wrong — encode as a query value).

### 2. Tab switching without re-handshake (success criterion 1)

FE proof: `setActiveSession` only flips `activeSession` (`appStore.ts:75-80`); `setViewSession` only re-mirrors the already-stored snapshot (`tmuxStore.ts:78-86`); the `ensuredRef` effect never closes a socket whose name is still in `openSessions` (`App.tsx:152-169`); inactive workspaces stay mounted (`App.tsx:229-245`, `invisible pointer-events-none`).

GPUI translation (prescriptive):
- `set_active_session(name)` = assign `active_session` + `cx.notify()`. No socket calls. Ever. (Plan-checker red flag: any `connect`/`close` inside the switch path.)
- Sidebar click on a known tree session = `open_session(name)`: if not in `open_sessions`, push + `ensure_session_socket(name)` (creates entry with `generation+1`, spawns connect task); then `set_active_session`. If already open, only `set_active_session`.
- `close_session(name)` (tab X / kill flow) = FE `closeSession` parity (`appStore.ts:61-73`): remove from `open_sessions`, drop entry (which disconnects the handle — reference `TerminalTab::disconnect`, [VERIFIED: `web-term/.../session.rs:102-108`]), clear snapshot, neighbor activation `open[idx.min(len-1)]`, `cx.notify()`. The tmux session itself keeps running (monitor torn down server-side on last-client-leave, `hub.go:70-89`).
- Phase 3 has no tab-X button on a visible strip (no strip exists); "closes its tab cleanly" = the kill flow + sidebar state update. Workspace body for an open session renders a Phase-3 placeholder panel (snapshot summary: session name + window list — real data, proving the WS path) since terminal rendering is Phase 4.

### 3. Generation guard contract (success criterion 5 / STATE-04)

Three checks, all required (FE runs all three — §6 above):

1. **Connection generation** (stale-socket suppression): each `OpenSession.generation` increments on every `ensure/connect`. The read pump captures `(session: String, generation: u64)` at spawn; every inbound event is forwarded as `WsPumpEvent { session, generation, event }`. The GPUI apply path drops events whose `generation != sessions[session].generation` (covers: reconnect races, rename re-resolution overlap, close-then-reopen within one frame).
2. **Envelope session match**: drop `state.snapshot`/`state.delta` whose `msg.session` is present and ≠ the connection's session (FE `websocket.ts:141-148` verbatim rule). Belt-and-braces against server-side misrouting.
3. **Tab-liveness**: drop events for sessions no longer in `sessions` map (tab closed while pump drains). Pump tasks observe handle-drop via flume disconnect and exit (reference: output-pump `break` on `recv_async` error, `terminal_ws.rs:486-488, 515`).

Unit-testable headlessly: generation counter + pure `apply_event(session, generation, event)` function (§Validation Architecture).

### 4. Rename flow with mid-session socket re-resolution (success criterion 3)

FE dialog parity (`SessionContextMenu.tsx:43-62,108-134`): `Rename` menu item → dialog prefilled with current name (`setNewName(sessionName)`, autofocus, Enter-submits, `Rename` disabled while empty/trim-empty or busy) → `runCommand(sessionRename)` → success toast + `onCreated(newName)`.

GPUI steps (prescriptive):
1. `RenameSessionForm` Entity on `AppState` (DLG1 lifetime pattern, `create_session_dialog.rs:30-43,46-131` [VERIFIED]): `{ name_input: Entity<InputState>, is_submitting: bool, error_message: Option<String>, target: String }`. Tokens = DLG1 (440px, `#1e1e1e`, 1px `#3c3c3c`, radius 8, padding 20, title "Rename Session", label "New name", footer Cancel/Rename).
2. Submit: client-side `validate_session_name` pre-flight → send on **target's own socket** `{type:"session.rename", newName, requestId}` (§4 contract) → await correlated result with 10s timeout.
3. On success — **re-resolution** (FE has no equivalent; this is the GPUI-correct addition the success criterion demands): close old entry's handle WITHOUT dropping the tab position; insert new entry under `newName` with `generation = old+1`; connect; move `open_sessions` item, `active_session` (if it pointed at old), `expanded_sessions`, and any snapshot in place (snapshot re-arrives via bootstrap; keep stale marked until then); `trigger_poll` (tree shows new name); `cx.notify()`. In-flight events tagged with the old generation are dropped by guard layer 1 — this is exactly the "rapid switching while churning" scenario the criterion names.
4. On `command.error`/timeout: inline destructive error (DLG1 `#7F1D1D` pattern), dialog stays open, tab untouched.

### 5. Kill flow with kill-confirm setting (success criterion 4)

FE parity (`SessionContextMenu.tsx:64-83,136-155`; `commands.ts:40-50`): `Kill Session` (destructive variant, separator above) → `shouldConfirm('session')` ? AlertDialog (`Kill session "X"?` + "This terminates the tmux session and all processes inside it." + Cancel/Kill-destructive) : direct kill → `sessionKillByName` → toast + `closeSession`.

GPUI steps: add `confirm_kill_session: bool` (default `true`) to `DesktopSettings` with `#[serde(default = "default_true")]` so existing Phase-1/2 settings files (which lack the key) keep confirming — FE default is `true` ([VERIFIED: `fe/src/stores/settingsStore.ts:44`]). Also add `confirm_kill_pane` + `confirm_kill_window` (both default `true`) now so the struct matches FE `Settings` shape ([VERIFIED: `settingsStore.ts:22-24`]) — Phase 3 honors only the session flag; pane/window flags are read in Phase 5/6 (`DLG-03`). Confirm dialog reuses DLG1 tokens with a destructive Kill button (`#7F1D1D` fill, white text — matches S1 close-button hover token in `tab_strip.rs:157`). On correlated success: `closeSocket` equivalent (drop entry) + `close_session` neighbor activation + `trigger_poll`. On error: destructive inline error or toast-equivalent — toasts are Phase 7, so inline error in the confirm dialog (consistent with assumption A6 from Phase 2).

### 6. `gpui-component` context-menu wiring (sidebar rows)

[VERIFIED: `menu/context_menu.rs:13-39`]: any `InteractiveElement + ParentElement + Styled` gets `.context_menu(|menu, _window, _cx| menu …)`; **the element ID must be stable across renders** (lines 26-34) or open-state is lost. Sidebar session rows therefore need `.id(("session-row", session_name.clone()))` (stable per session) before `.context_menu(...)`. Menu items via `PopupMenu` (`menu/popup_menu.rs`, re-exported [VERIFIED: `menu/mod.rs:12`]): items `New Window` (Phase 5 action — include as disabled/placeholder? NO: out-of-scope items must not ship as dead menu entries; Phase-3 menu = `Rename` + separator + `Kill Session` destructive only, matching FE structure minus the future item), `Rename`, separator, `Kill Session` (destructive styling).

Right-click must not also trigger the row's left-click select: the FE uses `ContextMenuTrigger` wrapping with `stopPropagation` on the chevron (`AppSidebar.tsx:80-110`); GPUI rows use `on_mouse_down(MouseButton::Left, …)` for select (existing `sidebar.rs:154`) and the context menu handles `MouseButton::Right` internally — no conflict by construction, but the plan must keep selection-on-left-click only.

### 7. Title bar: WindowTabs + Settings gear (SHELL-01)

FE anatomy ([VERIFIED: `AppTitleBar.tsx:36-121`]): `h-11` (44px — S1 already `h(px(44.0))` [VERIFIED: `tab_strip.rs:23`]) flex row, drag region whole bar, `no-drag` on interactive children (GPUI: interactive divs are never drag areas except the explicit `window_control_area(Drag)` center filler [VERIFIED: `tab_strip.rs:76-81`] — keep that structure, extend the middle). Order: sidebar toggle + `TerminalSquare` + "Tmux GUI" (built) → divider (`ml-1 h-5 w-px border-l`, `AppTitleBar.tsx:63-66`) → `WindowTabs` flex-1 middle (ONLY when `activeSession` is set, `AppTitleBar.tsx:61`) → Settings gear (`Settings size-4` ghost `icon-sm`, `AppTitleBar.tsx:70-83`) → window controls (built).

`WindowTabs` chip contract ([VERIFIED: `WindowTabs.tsx:110-190`]): container flex-1 horizontal scroll, no-scrollbar; chip `h-7 max-w-44 rounded-md px-2.5 text-sm` (`{index}: {name}` truncated), active `bg-secondary text-secondary-foreground` else `text-muted-foreground hover:bg-muted/70` → GPUI tokens: active bg `#2d2d2d` + text `#d4d4d4`, idle text `#808080`, hover `#262626` (02-UI-SPEC derived blends); close-X (`size-3.5`) visible on hover only (`group-hover`) → GPUI: render X only when chip hovered (hover state per chip id) or always-on-active; trailing `Plus size-3.5` new-window button (Phase 5 action — render as visual anchor WITHOUT wiring? Decision: render disabled-look anchor? NO dead controls — OMIT the Plus in Phase 3; window ops are Phase 5. Record in grey-area D8). Click chip = correlated `window.select` on the active session's socket (cheap, same `requestId` path — proves correlation before Phase 5). Tab context menu (rename/move/kill windows, `PANE-08`) is Phase 5 — NOT this phase. Data source: active session's `SessionSnapshot.windows` + `active_window`; empty-windows edge → render empty middle (no crash).

Settings gear action (FE parity `AppTitleBar.tsx:75-79`): `setActiveSession(null)` + `sidebarPage='settings'`. GPUI Phase 3: `active_session = None` + a `showing_settings: bool` (or `sidebar_page` enum with `Sessions/Settings`) rendering a placeholder Settings page ("available in Phase 6" — honest stub, tabs stay open underneath per FE `SettingsPage` comment). Full page is Phase 6.

### 8. RequestId generation + correlation plumbing

FE `nextRequestId`: `Math.random().toString(36).slice(2,10)` (`websocket.ts:185-187`). Rust: `rand =0.10.2` is already workspace-pinned ([VERIFIED: `Cargo.toml:53`]) — use `rand::random::<u64>().to_string()` base36-ish or `format!("{:x}", rand::random::<u64>())`; uniqueness per session only needs to avoid collision within the 10s pending window. Send path: serialize `WsIncoming` → outbound flume → write pump `Message::Text` (reference write pump [VERIFIED: `terminal_ws.rs:442-479`]). Correlated send helper on the session entry: `send_command(msg) -> Result<oneshot::Receiver<CommandResult>>` registering `pending[request_id]` BEFORE enqueueing (no lost-wakeup race).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| WS client transport, split read/write pumps, reconnect-safe handles | Raw `TcpStream` + manual HTTP upgrade + manual ping/pong | `tokio-tungstenite =0.26.2` `connect_async` + `split()` + `flume` queues | Reference `terminal_ws.rs:304-339,424-525` proves the exact shape; manual upgrade mishandles `coder/websocket`'s strict handshake and mask rules. |
| Right-click menus, dismissal, anchoring | Absolute-positioned div popup with manual outside-click tracking | `gpui-component::menu::{ContextMenuExt, PopupMenu}` | Element-state open tracking + stable-ID contract already solved (`context_menu.rs:73-88`); hand-rolled versions leak open menus across re-renders. |
| Single-line rename input | Custom key-event text buffer | `gpui-component::input::{Input, InputState}` (DLG1 pattern) | Selection, cursor, clipboard, IME, Enter-subscribe (`InputEvent::PressEnter`) already proven in `create_session_dialog.rs:78-87`. |
| Command→response matching | Ad-hoc "last command wins" flag | `requestId` + per-session `pending` map + 10s forget-timeout | Server correlates FIFO per session (`monitor.go:758-797`); without requestIds, concurrent rename+kill responses are indistinguishable and busy flags stick (FE `commands.ts:10-16` postmortem). |
| Session-name validation for rename | New regex | Existing `validate_session_name` (`backend-client/src/validation.rs`) | Same Go `ValidateSessionName` rules govern rename (`service.go:94-97`); duplicating the regex guarantees drift. |
| Reconnect backoff loop | Custom retry timer in Phase 3 | Defer to Phase 7 (`STATE-03` owns it) | FE `BACKOFF = [250,500,1000,2000,5000,10000]` (`websocket.ts:27`) + banner/toast UX ship together; a Phase-3 half-retry without surfaces creates zombie sockets. Phase 3 stores `transport` state so Phase 7 has something to render. |

**Key insight:** the FE's socket layer is three small files with two subtle correctness mechanisms (connection generation + session match) that exist because the team already debugged cross-session snapshot leakage (the `hub.go:105-108` comment says so explicitly). Port the mechanisms, not just the happy path — and fix the one real FE bug (rename-via-active-socket) instead of cloning it.

---

## Common Pitfalls

### Pitfall 1: Sending `hello` with fake dimensions shrinks the user's tmux viewport
**What goes wrong:** `hello{cols:80,rows:24}` resizes the control client's viewport (`handler.go:103-110` → `ResizeTerminal`), visibly shrinking tmux windows for every other frontend.
**Why it happens:** Assuming a handshake message is required to get state — it isn't (snapshot is unsolicited).
**How to avoid:** Phase 3 sends only `state.resync` on `connection.ready`. `hello`/`terminal.resize` arrive with Phase 4's measured sizes.
**Warning signs:** other frontends' panes rewrap on GPUI tab open.

### Pitfall 2: Rename sent on the wrong socket renames the wrong session
**What goes wrong:** Right-click session B while tab A is active → session A gets renamed.
**Why it happens:** Copying the FE `tmuxSocket` facade (`socket.ts:17-28`), which always targets the active session.
**How to avoid:** Resolve the target session's own handle at submit time; if its tab isn't open, `ensure_session_socket(target)` first (server will connect a monitor for it — `hub.go:40-67`).
**Warning signs:** rename success toast but the clicked row didn't change.

### Pitfall 3: Stale pump delivers a dead session's snapshot into the reused tab
**What goes wrong:** Kill session A, recreate A, old pump's buffered `state.delta` commits into the new tab showing ghost windows.
**Why it happens:** Pump task outlives the tab; generation not checked at apply time.
**How to avoid:** Generation check at BOTH forward time (pump tags) and apply time (`sessions[session].generation == event.generation`), plus entry-existence check. Re-resolution (rename) and close-reopen both bump generation.
**Warning signs:** flickering window lists immediately after rename/kill-recreate.

### Pitfall 4: `replace: false` vs missing `replace` in `WsOutgoing`
**What goes wrong:** `#[serde(default)] bool` handles both, but an `Option<bool>` sketch would treat missing as `None` and branch wrong in Phase 4.
**Why it happens:** Go omits `false` (`json:"replace,omitempty"`); some payloads carry no `replace` at all.
**How to avoid:** Model as plain `bool` with `#[serde(default)]` (see DTO sketch). Same for `seq` (`u64` default 0).
**Warning signs:** Phase-4 terminal frames misclassified as full replacements.

### Pitfall 5: Context menu loses open state on every poll tick
**What goes wrong:** Right-click menu flashes and closes whenever the 1.5s REST poll triggers `cx.notify()`.
**Why it happens:** Unstable element ID — `context_menu.rs:26-34` derives state identity from the element ID; a fresh ID per render resets state.
**How to avoid:** `.id(("session-row", name))` + `.id(("session-ctx", name))` stable per session; never include poll counters or indices in IDs.
**Warning signs:** menu usable only between poll ticks.

### Pitfall 6: Blocking the GPUI thread on WS connect
**What goes wrong:** UI freezes for seconds when opening a tab while tmux is slow (Windows bootstrap can take seconds).
**Why it happens:** `connect_async` awaited inside a GPUI handler or `cx.update`.
**How to avoid:** All WS I/O lives in `TOKIO_RT`-spawned tasks (established `app_state.rs` supervisor pattern); GPUI thread only flips `transport` enum + `cx.notify()`. `RestClient::new`-inside-runtime rule from Phase 2 applies equally to `connect_async` (needs a reactor).

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) |
| Config file | `desktop-gpui/Cargo.toml` (workspace) |
| Quick run command | `cargo test -p webtmux-backend-client` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| STATE-04 | WS envelopes round-trip: `WsIncoming` serializes `requestId`/`newName`/explicit `session`; `WsOutgoing` parses `state.snapshot` + `state.delta` with `session` tag, `command.success/error` with `requestId` | unit | `cargo test -p webtmux-backend-client test_ws_envelope_roundtrip` | ❌ Wave 0 (`crates/backend-client/tests/ws_test.rs`) |
| STATE-04 | `SessionSnapshot` parses full + null-slice payloads (`windows:null`, `panes:null` → empty) | unit | `cargo test -p webtmux-backend-client test_snapshot_null_normalization` | ❌ Wave 0 (`crates/backend-client/tests/ws_test.rs`) |
| STATE-04 | Generation guard: stale `(session, old_generation)` events dropped; current applied; unknown-session dropped | unit | `cargo test -p webtmux test_ws_generation_guard` | ❌ Wave 0 (`crates/webtmux/tests/ws_guard_test.rs`) |
| SESS-02 | Tab manager: open appends + activates; switch never touches sockets (pure state fn); close activates neighbor `min(idx,len-1)`; closing last clears active | unit | `cargo test -p webtmux test_tab_lifecycle` | ❌ Wave 0 (`crates/webtmux/tests/tabs_test.rs`) |
| SESS-02 | Live interop: mock WS server sends `connection.ready` + `state.snapshot`; client connects `?session=` URL, auto-sends `state.resync`, commits snapshot | integration | `cargo test -p webtmux-backend-client test_ws_bootstrap_flow` | ❌ Wave 0 (`crates/backend-client/tests/ws_test.rs` + in-test `accept_async` mock) |
| SESS-04 | Rename re-resolution: success on target socket migrates entry old→new with bumped generation; stale old-generation events dropped post-migration | unit | `cargo test -p webtmux test_rename_reresolution` | ❌ Wave 0 (`crates/webtmux/tests/tabs_test.rs`) |
| SESS-04 | Rename validation pre-flight rejects empty/colon/dot names without network | unit | `cargo test -p webtmux-backend-client test_rename_validation_reuse` | ❌ Wave 0 (extends `validation_test.rs`) |
| SESS-05 | Kill flow: `session.kill` serializes explicit `session` field for cross-session kill; confirm default `true` for fresh + legacy settings JSON | unit | `cargo test -p webtmux-settings test_kill_confirm_defaults` + `cargo test -p webtmux-backend-client test_kill_by_name_serialization` | ❌ Wave 0 (`crates/settings/tests/store_test.rs` extend + `ws_test.rs`) |
| SHELL-01 | Title-bar tab model: active window derived from snapshot (`active_window` ∈ windows); no-snapshot → empty middle, no panic | unit | `cargo test -p webtmux test_window_tabs_model` | ❌ Wave 0 (`crates/webtmux/tests/tabs_test.rs`) |

### Mock WS Server Strategy (no new crates)
In-test server with `tokio::net::TcpListener` + `tokio_tungstenite::accept_async` (both already in the dependency closure: `tokio` net feature [VERIFIED: `Cargo.toml:35-42`], `tokio-tungstenite` in backend-client deps [VERIFIED: `crates/backend-client/Cargo.toml:15`]). Script: accept → read connect (ignore body) → send `{"type":"connection.ready","session":N}` + `{"type":"state.snapshot","session":N,"snapshot":{…realistic…}}` → optionally assert the client sent `{"type":"state.resync"}` within 2s → for mutation tests, read `session.rename`/`session.kill`, reply `{"type":"command.success","requestId":…}` and assert URL `?session=` targeted the right session. Fixtures: `ws_snapshot_full.json`, `ws_snapshot_null_slices.json`, `ws_delta.json`.

### Sampling Rate
- **Per task commit:** `cargo test -p webtmux-backend-client`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `desktop-gpui/crates/backend-client/tests/ws_test.rs` — envelope round-trip, snapshot null-normalization, bootstrap interop, kill-by-name serialization
- [ ] `desktop-gpui/crates/backend-client/tests/validation_test.rs` (extend) — rename pre-flight reuse
- [ ] `desktop-gpui/crates/webtmux/tests/tabs_test.rs` — pure tab lifecycle, rename re-resolution, window-tabs model
- [ ] `desktop-gpui/crates/webtmux/tests/ws_guard_test.rs` — generation-guard apply function
- [ ] `desktop-gpui/crates/settings/tests/store_test.rs` (extend) — kill-confirm defaults + legacy-JSON migration

---

## Grey-Area Decisions (draft for CONTEXT.md — all auto-accepted, reversible)

> User is sleeping — nothing here blocks. Each is numbered for post-hoc audit; the planner locks them into `03-CONTEXT.md`.

- **D1 — Rename dialog tokens = DLG1 reuse.** 440px, `#1e1e1e`, 1px `#3c3c3c`, radius 8, padding 20, title "Rename Session", label "New name", autofocus + Enter-submit, `Rename` disabled while empty/busy, inline `#7F1D1D` error. Rationale: FE rename dialog is structurally identical to the create dialog (`SessionContextMenu.tsx:108-134` vs `CreateSessionDialog.tsx:90-152`); no new UI-SPEC needed.
- **D2 — Kill-confirm default source = `DesktopSettings`.** Add `confirm_kill_session` (+ `confirm_kill_pane`, `confirm_kill_window` for struct parity, honored later) defaulting `true` with `#[serde(default)]` so legacy settings files keep confirming. Rationale: FE `settingsStore.ts:33-45` defaults all three to `true`; settings persist via the proven atomic-write + `.bak` path (`settings/src/lib.rs:128-153` [VERIFIED]).
- **D3 — No visible session-tab strip (FE 1:1 parity).** "Tabs" = `open_sessions` state + sidebar highlight + title-bar window tabs. Rationale: grep-verified FE has no session strip (`openSessions` only drives hidden workspaces, `App.tsx:229-245`); inventing one breaks the milestone's 1:1 claim. Revisit only with an explicit deviation + UI-SPEC amendment.
- **D4 — Phase 3 reconnect policy = connect-on-open + teardown-on-close ONLY.** No backoff loop, no banner, no toasts; per-tab `transport` + `last_error` are stored so Phase 7 (`STATE-03`) has renderable state. A dropped socket in Phase 3 sits in `Disconnected` with resync-on-next-open. Rationale: FE backoff (`BACKOFF`, `websocket.ts:27`) is inseparable from its reconnect UX, which belongs to Phase 7; half a retry loop creates zombies.
- **D5 — Rename rides the target's own socket (FE-quirk correction).** Documented in §4; if the target tab isn't open, `ensure_session_socket(target)` first. Rationale: Go binds rename to the URL session — anything else renames the wrong session.
- **D6 — Kill transport order: victim socket → any live socket + explicit `session` → ephemeral one-shot.** Rationale: covers sidebar kills with zero tabs open without stranding a connection.
- **D7 — No `hello` in Phase 3; `state.resync` on `connection.ready`.** Rationale: `hello` is a viewport resize, not a handshake (§1); fake dimensions harm other frontends. Real sizing ships with Phase 4 terminals.
- **D8 — Title-bar Plus (new-window) button OMITTED in Phase 3.** Rationale: window creation is Phase 5; shipping an unwired affordance (or secretly wiring a Phase-5 mutation early) violates phase boundaries. Window-chip click (`window.select`) IS wired — it is read-path selection over the already-required correlated channel.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `tokio-tungstenite 0.26.2` client interoperates with Go `coder/websocket` server (both plain RFC6455, no subprotocols) | WS Contracts §1 | LOW — same combination as web-term reference; covered by the Wave-0 bootstrap interop test, which fails loudly at plan time if wrong. |
| A2 | `msg.session` on `state.snapshot` is always populated (Go sends `Session: session` on the initial snapshot, `handler.go:61`; FE defensively tolerates absence, `websocket.ts:142`) | Generation guard | LOW — guard treats absent as "belongs to this connection" (FE parity); worst case equals current FE behavior. |
| A3 | Killing the last/only tab leaves `active_session=None` → workspace shows `SelectSessionView` (sessions remain in tree) or `EmptyState` (tree empty) via existing `workspace_state()` routing | Tab lifecycle | Minimal — routing already tested (`state_view_test.rs`); Phase 3 only changes what sets `active_session`. |
| A4 | `gpui-component` `PopupMenu` item API (labels, destructive variant, separators) mirrors the `ContextMenu` shadcn structure closely enough that the FE 3-item menu ports without custom popup code | Context menus | LOW — `popup_menu.rs` + `menu_item.rs` exist in the vendored source; planner verifies signatures at plan time (they were listed, not line-read). |
| A5 | Server does NOT close the victim WS on kill (no close frame in `KillSession` path, `service.go:106-126`) — client must close locally on correlated success | Kill flow | LOW — even if a close frame arrives, the read pump already handles `Message::Close` → `Disconnected`, and idempotent local close is harmless. |
| A6 | `SessionSnapshot.session.windows/attached` counts stay consistent enough with `windows.len()` for badges (sidebar badge keeps using tree data, not snapshot) | Title bar/tabs | None — sidebar badges intentionally stay on the REST tree (unchanged Phase-2 path); snapshot counts are display-only. |

---

## Open Questions

1. **Should `window.select` on chip click be optimistic (flip `active_window` locally) or await `command.success`?**
   - What we know: FE fires `tmuxSocket.windowSelect(w.id)` WITHOUT awaiting (`WindowTabs.tsx:120` — no `runCommand`); the new active window arrives via the next `state.delta` (control-mode `session-window-changed` → debounced resync, `monitor.go:742-749`). Latency is one poll tick.
   - What's unclear: whether GPUI's `cx.notify()` + snapshot commit cadence makes the lag perceptible vs React.
   - Recommendation: mirror FE (fire-and-forget, state follows via delta); add optimistic flip only if UAT flags lag. Mark reversible.

2. **Where does the ephemeral kill socket's `state.snapshot` bootstrap go?**
   - What we know: connecting a socket to session X auto-starts its monitor and pushes a snapshot (`hub.go:40-67`).
   - Recommendation: ephemeral socket commits NOTHING (no tab entry, generation guard drops by tab-liveness rule); it sends kill and closes. Document in plan so a stray snapshot can't create a phantom tab.

---

## Environment Availability

Step 2.6: SKIPPED in the tool-probe sense — no new external dependencies (no new crates, no CLIs, no services beyond the already-running sidecar from Phase 1). The WS endpoint is served by the same sidecar (`base_url` learned at `SupervisorEvent::Ready`, `app_state.rs:235-241` [VERIFIED]); tmux presence continues to be reported via the Phase-2 tree path.

## Security Domain

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | No | Localhost-only sidecar, no auth surface (unchanged from Phase 1/2) |
| V3 Session Management | No | tmux sessions are server-side tmux objects, not app sessions |
| V4 Access Control | No | Single-user local app; WS origin permissive by backend design (`handler.go:33-35`, localhost bind) |
| V5 Input Validation | **Yes** | `validate_session_name` pre-flight on rename (same as create); percent-encode session names in WS URLs (names may contain `/`); serde `default` discipline on all WS DTOs so malformed frames degrade to `server.error` handling, never panics |
| V6 Cryptography | No | Plain `ws://127.0.0.1`, no TLS in scope (localhost loopback, matches FE desktop which uses `ws://127.0.0.1:<port>`) |

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed WS frame panics the pump | Tampering / DoS | `serde_json::from_str` → drop + continue (FE `websocket.ts:106-111` parity: `try/catch → return`); never `unwrap` in the read loop |
| Stale socket writes after tab close | Tampering | Generation check on send path too: `send_command` verifies entry generation matches the handle's captured generation |

---

## Sources

### Primary (HIGH confidence)
- `be/internal/realtime/protocol.go:1-97` — envelope structs + all message/event type constants (verbatim DTO source)
- `be/internal/realtime/handler.go:26-224` — WS handshake, bootstrap sends, full dispatch switch incl. rename URL-binding (`:184-185`) and kill explicit-target (`:186-193`)
- `be/internal/realtime/hub.go:38-121` — monitor lifecycle, last-client-leave teardown, `Session`-tagged delta relay (`:104-108`)
- `be/internal/realtime/client.go:27-101` — send queue (256, slow-drop), write loop
- `be/internal/tmux/service.go:77-126` — `Create/Rename/KillSession` semantics, monitor teardown on kill
- `be/internal/tmux/monitor.go:34-41,758-797` — event types, FIFO `%begin/%end/%error` correlation
- `be/internal/tmux/model.go:3-67` — `Session/Window/Pane/Snapshot/Tree` JSON tags (verbatim)
- `fe/src/lib/websocket.ts:27-335` — backoff table, generation field + guards, send-queue flush, full typed action list, `sessionKillByName`
- `fe/src/lib/sockets.ts:1-101` — per-session map, `ensureSocket`/`closeSocket`/`getOrCreateSocket`, `onReady → requestState`
- `fe/src/lib/protocol.ts:8-81` + `fe/src/lib/commands.ts:1-50` — `MSG`/`EV` constants, `runCommand` 10s timeout + forget, `shouldConfirm` kill gates
- `fe/src/lib/socket.ts:17-28` — active-session facade (source of the rename quirk)
- `fe/src/stores/appStore.ts:44-91` — `openSession`/`closeSession` neighbor rule / `setActiveSession` purity
- `fe/src/stores/tmuxStore.ts:59-135` — per-session snapshots/transports, view mirroring, `forgetCommand`
- `fe/src/stores/settingsStore.ts:33-45` — kill-confirm defaults (`true × 3`)
- `fe/src/components/layout/AppTitleBar.tsx:36-121` + `fe/src/features/windows/WindowTabs.tsx:48-238` — title-bar anatomy + chip geometry + rename/kill dialog copy
- `fe/src/features/sessions/SessionContextMenu.tsx:1-158` — menu structure, rename/kill flows, dialog copy verbatim
- `fe/src/components/layout/AppSidebar.tsx:42-155` — row geometry, `stopPropagation` chevron, `panes ?? []` guard
- `fe/src/App.tsx:148-175,226-250` — socket-manager effect + hidden-mounted workspaces (no session strip — verified)
- `web-term/desktop-gpui/crates/backend-client/src/terminal_ws.rs:1-45,226-339,424-525` — URL normalization shape, `TerminalWsHandle` flume API, connect/attach + pump reference
- `web-term/desktop-gpui/crates/webterm/src/session.rs:1-121,188-204` — `TerminalTab`/`TerminalSessionManager` lifecycle (add/switch/close/disconnect/neighbor clamp)
- `desktop-gpui/Cargo.toml:24-55` + `Cargo.lock:2096-2098,6950-6953` — pinned WS stack, lockfile-resolved versions
- `desktop-gpui/crates/backend-client/src/{lib,models,rest}.rs`, `crates/backend-client/Cargo.toml:9-17` — existing DTO/validation/client patterns to extend
- `desktop-gpui/crates/webtmux/src/{app_state.rs:27-124,views/tab_strip.rs:1-81,views/sidebar.rs:130-184,views/session_states.rs:19-44,views/create_session_dialog.rs:30-131}` — seams to extend
- `desktop-gpui/crates/settings/src/lib.rs:44-154` — settings struct + atomic write + `.bak` recovery (kill-confirm lands here)
- `gpui-component-0.6.0/src/{lib.rs:26,53,menu/mod.rs:9-12,menu/context_menu.rs:13-39}` — menu module availability + stable-ID contract

### Secondary (MEDIUM confidence)
- `be/internal/tmux/snapshot.go:88-126,128-165` — snapshot/tree build paths (null-slice risk on `Snapshot()`)
- `be/internal/tmux/command.go:17-38` — rename shares create's validation rules
- `02-RESEARCH.md` + `02-UI-SPEC.md` + `02-CONTEXT.md` + `02-01-SUMMARY.md` + `02-02-SUMMARY.md` — established patterns (generation guard, DLG1 entity lifetime, token tables)

### Tertiary (LOW confidence)
- None — every Phase-3 claim is grounded in files read this session except A1/A4/A5, which are logged with fallback plans.

## Metadata

**Confidence breakdown:**
- Standard Stack: HIGH — pins verified in `Cargo.toml` + `Cargo.lock`; reference-crate usage read line-by-line; zero new packages.
- Architecture: HIGH — per-session map, generation guard, and lifecycle rules each have 1:1 FE + Go + web-term anchors.
- Pitfalls: HIGH — four of six are named bugs with cited fixes (server comment, FE facade quirk, DLG1 precedent); two are structural (stable IDs, reactor context).

**Research date:** 2026-09-06
**Valid until:** 2026-10-06 (stable domain — backend protocol frozen, pins locked; re-verify only if `be/internal/realtime/` changes)
