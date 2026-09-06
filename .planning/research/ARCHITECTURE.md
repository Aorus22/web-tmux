# Architecture Research: Desktop GPUI Frontend for web-tmux

**Domain:** Native GPUI (Rust) desktop frontend integrating with an existing Go + tmux backend protocol (brownfield port of web-term's proven desktop-gpui architecture)
**Researched:** 2026-09-06
**Confidence:** HIGH — derived from first-hand reading of both codebases (web-term reference + web-tmux target); GPUI API claims corroborated by docs (MEDIUM per provider seam) and by the working reference implementation

> Reads behind this document: `.planning/PROJECT.md`; web-tmux `fe/src/lib/{protocol,tmux-types,websocket,sockets,commands,api,geometry}.ts`, `fe/src/stores/{tmuxStore,appStore,settingsStore}.ts`, `fe/src/features/panes/PaneWorkspace.tsx`, `fe/src/features/terminal/terminalRegistry.ts`, `fe/src/features/settings/data/{ui-themes,terminal-themes}.ts`, `fe/src/App.tsx`; web-tmux `be/cmd/server/main.go`, `be/internal/config/config.go`, `desktop/main.js`; web-term `desktop-gpui/crates/{supervisor,settings,backend-client,terminal,webterm}` (full pattern read: main.rs, app_state.rs, session.rs, theme.rs, window_state.rs, bundle.rs, supervisor/lib.rs, settings/lib.rs, terminal/terminal.rs, views/nav.rs, views/tab_strip.rs, icons.rs).

---

## Standard Architecture

### System Overview

The GPUI app is a **frontend only**. tmux remains the source of truth; the Go backend (`tmux-gui-server`) is the only process that talks to tmux (control mode). The app ports web-term's 5-crate workspace but swaps the SSH-PTY backend client for web-tmux's per-session JSON WS protocol.

```
┌──────────────────────────────────────────────────────────────────────┐
│  desktop-gpui/  (Rust workspace, pinned: gpui-pre 0.3.3,             │
│                         gpui-component 0.6.0, alacritty_terminal     │
│                         0.25.1, tokio + reqwest(rustls) +            │
│                         tokio-tungstenite)                           │
│                                                                      │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │ App crate (webtmux) — one GPUI window                       │    │
│  │                                                             │    │
│  │  main.rs          headless bootstrap → Application::run     │    │
│  │  app_state/       root Entity<AppState>                     │    │
│  │    sessions.rs      SessionTab, SessionManager              │    │
│  │    ws_pump.rs       output → Terminal events pump           │    │
│  │    actions.rs       GPUI action bindings (Ctrl+Shift+P …)   │    │
│  │  views/           pure render fns: titlebar/tab strip,      │    │
│  │    sidebar tree, pane workspace, modals, settings, palette  │    │
│  │  theme.rs         ported preset tables (new palettes!)      │    │
│  │  icons.rs         lucide SVG const bytes (svg().data())     │    │
│  │  window_state.rs  restore/observe (port verbatim)           │    │
│  │  bundle.rs        tmux-gui-server path resolution            │    │
│  └─────────────────────────────────────────────────────────────┘    │
│  ┌──────────────┐ ┌───────────┐ ┌──────────────┐ ┌──────────────┐   │
│  │ supervisor   │ │ settings  │ │ backend-     │ │ terminal     │   │
│  │ (tokio only, │ │ (JSON at  │ │ client       │ │ (gpui +      │   │
│  │ process +    │ │ appdata)  │ │ REST + JSON  │ │ alacritty_   │   │
│  │ handshake)   │ │           │ │ ws per tab   │ │ terminal)    │   │
│  └──────────────┘ └───────────┘ └──────────────┘ └──────────────┘   │
└══════════════════════════════════════════════════════════════════════┘
                       │ spawn/env/handshake        │ REST + WS (loopback JSON)
                       ▼                            ▼
┌─────────────────────────────────────────────────────────────────────┐
│ tmux-gui-server (Go, be/ — UNCHANGED)                               │
│   BACKEND_PORT:<n> handshake ▸ REST endpoints ▸ /api/ws?session=X   │
│   tmux -CC control client per WS (spawned/killed by backend)        │
└─────────────────────────────────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────────────┐
│ tmux server (source of truth — sessions/windows/panes/processes)    │
└─────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Ported From / How |
|-----------|----------------|-------------------|
| **supervisor crate** | Spawn `tmux-gui-server` with `TMUXGUI_HOST=127.0.0.1`, `TMUXGUI_PORT=0`, parse `BACKEND_PORT:<n>` stdout handshake, poll readiness on `/api/health`, watch child exit, stderr tail for diagnostics, graceful stop (tmux sessions never die) | web-term `crates/supervisor` nearly verbatim — swap env vars, drop db/encryption-key, change probe endpoint |
| **settings crate** | `DesktopSettings` JSON in platform app-data dir; corrupt-file `.bak` recovery; `paths` module; window state; open-session-tab persistence; tmux binary choice | web-term `crates/settings` trimmed: no encryption key, no DB path (web-tmux backend is stateless) |
| **backend-client crate** | Typed DTOs mirroring `fe/src/lib/tmux-types.ts`; REST client (`/api/health`, `/api/tmux/info`, `/api/tmux/binary` POST, `/api/sessions` GET tree + POST create, `/api/sessions/{name}/snapshot`); **new** `tmux_ws.rs` JSON WS client per session tab (generation guard, backoff 250ms→10s, send queue, requestId correlation) | web-term `BackendClient`/`types.rs` reused in shape; `terminal_ws.rs` (binary PTY frames) is **replaced** — web-tmux WS speaks JSON text frames with per-pane addressing |
 | **terminal crate** | alacritty_terminal 0.25.1 `Term` + `Processor` inside `Terminal`, GPUI `TerminalView`/`TerminalRenderer` (JetBrains Mono, batched text runs, ColorPalette), `keystroke_to_bytes`, mouse cell math, `GpuiEventProxy` (flume) | web-term `crates/terminal` **ported as-is** (this is the rendering engine — all 7 modules: terminal/view/render/input/mouse/colors/event) |
| **app crate (webtmux)** | Root `Entity<AppState>`: supervisor orchestration, `SessionManager` (tabs = tmux session names), per-tab snapshot model, per-pane terminal slots, all views (sidebar tree, tab strip/titlebar, pane workspace, toolbars, modals, palette, settings, status) | web-term `crates/webterm` pattern — but the flat 3,837-line `app_state.rs` should be **split into per-domain modules** (web-tmux has more domains than web-term) |
| **theme.rs (in app crate)** | UI theme presets (17 colors incl. `input`/`ring`/`destructiveForeground`) + terminal ANSI presets, both **generated from web-tmux's `ui-themes.ts` / `terminal-themes.ts`**, terminal preset linked to UI preset (`resolvedTerminalTheme`) | web-term `theme.rs` scaffolding (ThemePreset struct, `find_theme_preset`, `terminal_palette_for_preset`) but with **web-tmux palette values — not web-term's** |

---

## Model Mapping: The Core Architectural Adaptation

This is the central question of the port. **web-term: one WS = one terminal = one tab. web-tmux: one WS = one session = N windows, each with a pane grid; each visible pane is its own terminal instance.**

| web-term concept | web-tmux GPUI equivalent | Key change |
|------------------|--------------------------|------------|
| `TerminalSessionManager { tabs: Vec<TerminalTab>, active_tab_index }` | `SessionManager { tabs: Vec<SessionTab>, active_index }` | Same skeleton; tab id = session **name** (protocol keys by name; FE does exactly this) |
| `TerminalTab { view: Option<Entity<TerminalView>> (1 terminal), ws_handle, session_id, status }` | `SessionTab { name, status (Connecting/Connected/Reconnecting/Disconnected), snapshot: Option<TmuxSnapshot>, ws_handle, panes: HashMap<String /*%N*/, PaneSlot> }` | One tab holds a **snapshot + pane map**, not one terminal |
 | `WsConnectRequest { type: "connect", … }` binary PTY handshake | URL `/api/ws?session=<name>`; `hello(cols, rows)` JSON → `connection.ready` → `state.resync` request → `state.snapshot` | JSON text frames; per-pane addressing on every terminal message |
 | `terminalWsHandle.send_input(bytes)` binary | `terminal.input { paneId, data: <string> }` — keystrokes → `keystroke_to_bytes` → feed bytes through the same string path the FE uses (JS keeps the byte buffer in a plain string; mirror whatever the Go decoder expects) | Not binary; verify the exact byte↔string mapping against Go's WS handler during the WS phase |
| Backend PTY bytes → one `Terminal.process_bytes` | `terminal.output { paneId, data, replace, screenRows }` routed to `panes[paneId].terminal.process_bytes` | Output routed by paneId |
| — | `terminal.snapshot { paneId, data, screenRows }` + `terminal.capture` requests | New: capture/replace semantics (registry logic below) |
| `state.snapshot` does not exist | `state.snapshot` / `state.delta { session, snapshot }` replaces the tab's whole window/pane model | New topology handler; **defensive session check** (`msg.session !== name` → drop) |

### Recommended app-state model

```rust
// app_state/sessions.rs
pub struct SessionManager { tabs: Vec<SessionTab>, active_index: usize }

pub struct SessionTab {
    pub name: String,                        // tmux session name (stable id)
    pub status: SessionStatus,               // mirrors tmuxStore transports
    pub ws: Option<TmuxWsHandle>,            // one WS per open tab
    pub snapshot: Option<TmuxSnapshot>,      // message-driven full replace
    pub focused_pane: Option<String>,        // terminal.input target ("%N")
    pub panes: BTreeMap<String, PaneSlot>,   // key = pane id "%N"
}

pub struct PaneSlot {
    pub meta: TmuxPane,                      // left/top/width/height cells, zoomed, active
    pub view: Option<Entity<TerminalView>>,  // alacritty engine + renderer (terminal crate)
}

// screen/scrollback state INSIDE the alacritty Terminal (scrollback_limit from settings,
// default 2000 lines to match TMUXGUI_SCROLLBACK_LINES feeding capture-pane history)
```

**Command correlation:** `pending: HashMap<String /*requestId*/, oneshot::Sender<…>>` on `AppState` — `command.success`/`command.error` resolve the oneshot; a 10 s timeout forgets the entry (port of `fe/src/lib/commands.ts::runCommand`, so a dead socket can never leave UI busy-flags stuck). Create-session must go over **REST** (it must work with zero sessions — no socket exists yet; the FE does exactly this).

**View routing:** open tabs stay "alive" while hidden (GPUI: entities persist; render only the active tab's workspace). All panes of all open tabs keep their Term buffers current from WS events even when not displayed — this is what the React FE does by keeping inactive tabs mounted (`invisible pointer-events-none`). This is free in GPUI: output pumps write into `Terminal` entities that simply aren't rendered when hidden.

### Session/window/pane roles

- **Sidebar tree** (sessions → windows → panes): from **`GET /api/sessions` polled every 1.5 s** (tokio task → app update → `cx.notify()`). tmux owns the truth; the poll only refreshes metadata. Session rename/kill, window create/rename/kill, layout presets, context menus all hang off this tree and route over the tab's WS (or REST for session-create with zero sessions).
- **Workspace** (one open tab per session): driven by the tab's **WS snapshot** (`state.snapshot`/`state.delta` — FE treats both identically as full-replace). Active window = `snapshot.activeWindow`; visible panes = that window's panes (`zoomed` pane alone fills the workspace).
- **Tabs strip / titlebar**: open `SessionTab`s + `active_index`; web-term's `views/tab_strip.rs` (frameless titlebar, sidebar toggle, custom min/max/close controls, DWM dark via `DwmSetWindowAttribute`, `ShowWindowAsync` maximize toggle, `window_state::observe`) ports as-is.

---

## Recommended Project Structure

```
desktop-gpui/
├── Cargo.toml               # workspace; pinned versions; COMMIT Cargo.lock
├── crates/
│   ├── supervisor/          # framework-free (tokio only) — spawns tmux-gui-server
│   │   └── src/lib.rs       # SpawnOptions, parse_handshake_line, BackendStatus watch,
│   │                        # readiness probe (/api/health), stderr tail, exit monitor
│   ├── settings/
│   │   └── src/lib.rs       # DesktopSettings + paths (appdata); NO encryption key
│   ├── backend-client/
│   │   └── src/
│   │       ├── lib.rs       # re-exports
│   │       ├── types.rs     # TmuxSession/Window/Pane/Snapshot/Tree (mirror tmux-types.ts)
│   │       ├── rest.rs      # BackendClient: health, tmux_info, set_tmux_binary, tree,
│   │       │                #   create_session, snapshot
│   │       └── tmux_ws.rs   # TmuxSocket: JSON frames, generation guard, backoff,
│   │                        #   send queue, requestId helpers, event handlers
│   ├── terminal/            # PORTED VERBATIM from web-term
│   │   └── src/{terminal,view,render,input,mouse,colors,event}.rs
│   └── webtmux/             # app crate (gpui)
│       └── src/
│           ├── main.rs              # TOKIO_RT.enter; settings; bundle; window restore;
│           │                        # fonts; gpui_component::init; theme; DWM; open_window
│           ├── app_state.rs         # Entity<AppState>: backend_status, client, settings,
│           │                        # managers, view routing
│           ├── sessions.rs          # SessionManager/SessionTab/PaneSlot (see above)
│           ├── ws_pump.rs           # WS event → panes/snapshot/pending resolution
│           ├── tree_poll.rs         # 1.5 s REST tree task
│           ├── actions.rs           # GPUI actions: palette, tab nav, session ops
│           ├── theme.rs             # THEME_PRESETS + TERMINAL_THEMES (generated from fe/ data)
│           ├── icons.rs             # lucide SVG consts (port: zoom, split, palette, …)
│           ├── window_state.rs / bundle.rs   # verbatim ports (bin name tmux-gui-server)
│           └── views/
│                ├── mod.rs            # nav shell (titlebar + sidebar + workspace)
│                ├── tab_strip.rs      # session tabs + window controls (from web-term)
│                ├── sidebar.rs        # collapsible tree + context menus
│                ├── pane_workspace.rs # geometry → positioned TerminalViews + dividers
│                ├── pane_view.rs      # per-pane frame/context menu wrapper
│                ├── create_session.rs / palette.rs / settings.rs / states.rs
│                └── status.rs         # status bar: transport state, tmux version
└── test-support/backend     # tmux-gui-server build output (bundle path candidate)
```

### Structure Rationale

- **Same 5-crate split as web-term** (PROJECT.md constraint): supervisor and terminal deliberately GPUI-free where possible so spawn/reader logic stays CI-testable (web-term calls this out explicitly).
- **`backend-client` gains one new module** (`tmux_ws.rs`) instead of reusing web-term's `terminal_ws.rs`: binary PTY frames cannot express `paneId`-addressed JSON; the file is the only full rewrite among the ported crates.
- **App crate split per domain** — web-term's flat 3,837-line `app_state.rs`/148 KB of state proved workable but painful; web-tmux adds command palette, settings page, sidebar tree, create dialog. Keep the single root `Entity<AppState>` (GPUI idiom, matches web-term main.rs) but factor `sessions.rs`, `tree_poll.rs`, `ws_pump.rs` modules.

---

## Architectural Patterns

### Pattern 1: Sidecar Supervisor (port web-term `supervisor` crate nearly verbatim)

**What:** GPUI app spawns the Go backend as a child process on `TMUXGUI_PORT=0`, learns the loopback port from the backend's `BACKEND_PORT:<n>` stdout handshake, probes readiness, and supervises the child's lifetime until app exit (kill backend; **tmux sessions survive**).

**Verified compatibility:** web-tmux's `be/cmd/server/main.go` already emits `BACKEND_PORT:%d` on listen — the same handshake web-term's supervisor parses byte-for-byte.

**Deltas vs web-term (all required):**

| Aspect | web-term | web-tmux |
|--------|----------|----------|
| Env vars | `WEBTERM_HOST/_PORT/_DB_PATH/_ENCRYPTION_KEY/_ALLOWED_ORIGINS` | `TMUXGUI_HOST=127.0.0.1`, `TMUXGUI_PORT=0`, `TMUXGUI_TMUX_BIN=<resolved>`, optionally `TMUXGUI_LOG_LEVEL`, `TMUXGUI_SCROLLBACK_LINES` |
| Secrets | 64-hex encryption key validated, env-only | None — remove validation |
| Readiness probe | `GET /api/settings` | `GET /api/health` (success ⇒ ready) |
| Binary name | `backend[.exe]` | `tmux-gui-server[.exe]` (update `bundle.rs` candidates: settings override → adjacent-to-exe → `desktop-gpui/test-support` chain) |
| Env sanitization | — | **Spawn the child with `TMUX` / `TMUX_PANE` removed from its environment** (Electron does this; if the GPUI app is launched from inside a tmux session, every backend `tmux` call would otherwise attach to the launcher's own session instead of the user's default socket) |
| tmux binary resolution | — | Electron resolves `TMUXGUI_TMUX_BIN` from `%LOCALAPPDATA%\Microsoft\WinGet\Links\tmux.exe` / PATH — the supervisor must replicate this fallback and honor a `DesktopSettings` override; also set the persisted choice via `POST /api/tmux/binary` once ready |

Example (child env vars):
```rust
// supervisor/src/lib.rs — SpawnOptions::env_vars() equivalent
("TMUXGUI_HOST", "127.0.0.1".into()),
("TMUXGUI_PORT", "0".into()),
("TMUXGUI_TMUX_BIN", Self::resolve_tmux_binary().into()),
```

**Trade-offs:** reuses a battle-tested lifecycle (starting / ready / failed{reason, stderr_tail} / crashed watch states, stderr ring buffer, exit monitor) incl. Windows-specific gotchas — no reason to improvise. The real change surface is ~50 lines.

### Pattern 2: One `TmuxWsClient` per open session tab (JSON protocol client)

**What:** For each open session tab the app owns **exactly one** WebSocket at `ws://127.0.0.1:<port>/api/ws?session=<name>`. Closing the tab closes the socket (the backend stops a session's control-mode monitor when its last client leaves — documented in `lib/sockets.ts`). The client is a tokio task owning one `tokio_tungstenite` connection and publishing parsed events onto channels.

**What to port from `fe/src/lib/websocket.ts` (all load-bearing, keep 1:1):**
- **Generation guard**: every `open()` bumps a generation counter; a superseded socket's `onclose` must neither reconnect nor deliver messages (stale socket otherwise blinks the UI between sessions).
- **Backoff ladder** 250 ms → 10 s (6 steps), reset on open.
- **Send queue** while connecting (initial `terminal.capture` on a tab's workspace must not be lost).
- **requestId correlation** for every GUI action (`pane.*`, `window.*`, `session.*`), resolved by `command.success`/`command.error`; forget after 10 s (`runCommand` port).
- `hello(cols, rows)` on open; `connection.ready` → `state.resync` request; **defensive cross-session snapshot rejection**.
- Terminal events carry `paneId`, `data`, optional `replace`, optional `screenRows` (visible-row count inside a capture blob — used to split history vs screen).

**What changes from web-term's `terminal_ws.rs`:** frames are **JSON text** (web-term sends raw binary keystroke bytes and receives binary PTY output). Input must be text as `terminal.input { paneId, data }`.

**Trade-offs:** slightly larger client than web-term's, but the message surface is fixed by the unchanged backend (PROJECT constraint: no protocol changes; gaps get logged, not forked).

### Pattern 3: Snapshot-owned window model + local per-pane algebra (the pane grid)

**What:** The workspace renders the active window's panes **proportionally from tmux-provided cell geometry** — no translated layouts, no app-owned grid. Port these exact functions from `fe/src/lib/geometry.ts` (keep the formulas bit-identical for UI parity):
- `pixelRect`: map pane cells to container px (`scaleY = containerHeight / windowHeight`, `scaleX = containerWidth / windowWidth`)
- `closePaneGaps`: absorb tmux's 1-cell border strips so panes tile edge-to-edge (order-independent neighbor absorption)
- `pxToColsRows`: nominal CELL_W=8/CELL_H=18 fallback. All three are pure functions — port them and add unit tests mirroring the FE's edge cases (T-layout adjacency, zoom fill).

**Viewport & resync discipline (port `PaneWorkspace` behavior exactly):**
- Report **actual** window viewport: scale the visible pane's real terminal size (`Terminal` cols/rows — analog of `terminalRegistry.size`) to the whole window by that pane's tmux cell ratio; debounced ~100 ms → `terminal.resize` on **this session's own socket** (never the active-tab proxy).
- Track a `layoutKey` (`activeWindowId|WxH|layout|visible[paneId:geometry:zoom]`); when it changes (zoom / window switch / tmux layout rebalance): debounced `terminalResize`, then forced `terminal.capture` of every visible pane (invalidate-before-write) — incremental output can leave stale grid pixels after topology changes.
- Dividers: adjacency detected in cell space between immediate neighbors sharing an edge (T-layouts included); a drag converts pixel delta ÷ `cellPx` into a `paneResize { direction, amount }`.

**GPUI shape:** a `PaneWorkspace` render function computes `pixelRect` for every visible pane and lays out the pane's `TerminalView` entity at computed absolute bounds (GPUI supports absolutely-positioned children, which is a direct translation of the FE's absolutely-positioned divs); an empty/zoomed state replaces the grid.

**Trade-offs:** less code — and closer to pixel parity with the FE — than translating tmux geometry into a native grid component, because tmux already owns layout. The only genuinely new GPUI code is drag math (divider → `paneResize`) and child positioning; the formulas are copied from a working file.

### Pattern 4: Capture/snapshot ingestion into alacritty (`terminalRegistry` port)

**What:** FE routes every WS terminal frame into the pane's terminal through a tiny registry that (a) writes a capture blob exactly once per pane instance (`snapshotWritten` guard) and (b) tracks an `ingestedHistory` counter so only the history **delta** above the visible screen is re-fed on every capture. `applyCapture` splits the blob by `screenRows`: un-ingested history lines first, then an explicit-origin redraw (`\x1b[i;1H...`) of the visible grid.

**GPUI adaptation:** no registry map needed — the WS pump already knows the tab and pane (`SessionTab.panes`). Port `applyCapture` as a free function feeding alacritty via `Terminal::process_bytes` (same ANSI escape stream works — VTE is fully byte-level). Keep the two guards (`first write idempotent`, `history Δ`) bit-identical; they are the difference between "capture appends duplicate screens" (#24's doubled-line class of bug) and clean resyncs.

### Pattern 5: Theme as generated Rust tables + gpui-component bridge

**What:** web-term proves the scaffold — but web-tmux's palettes are its own: `ui-themes.ts` (2,515 lines, 17 UI colors incl. `input`/`ring`/`destructiveForeground`, plus a linked `terminalTheme` per preset) and `terminal-themes.ts` (2,019 lines, fg/bg/cursor + 16 ANSI entries). Both files say "Generated by scripts — do not hand-edit", so the GPUI `theme.rs` ships as generated const tables produced by a small script consuming both TS files (not web-term's values: e.g. web-tmux `default-dark` primary is neutral `#d4d4d4`; `destructiveForeground`, `input`, `ring` must be added to web-term's ThemePreset struct).

**Theme data flow:** settings store both `ui_theme` preset name and optional terminal-theme override (FE: `terminalTheme: string | null`; null ⇒ follow UI theme). `resolvedTerminalTheme` semantic ports directly. Apply at startup and live-repaint tab terminals on change (`set_palette` iterates open tabs — web-term `AppState::set_theme_preset` pattern).

**Bridge to gpui-component 0.6:** `gpui_component::init(cx)` + `Theme::change(ThemeMode::Dark|Light, None, cx)` at apply time (web-term pattern), and additionally push the preset's primaries/border/radius into the gpui-component global theme tokens (`Theme::global_mut` → `tokens.colors`) so stock widgets (modals, menus, popovers) match the Electron look without per-widget overrides.

### Pattern 6: Single root `Entity<AppState>` + pure view functions + explicit async pumps

**What:** web-term chooses one big root entity, function-style views (`render_nav_shell(app, cx) -> AnyElement`), and explicit pump wiring: tokio task → `mpsc` channel → `cx.spawn`/`AsyncApp` pumping `entity.update(cx, ...)` + `cx.notify()` back onto the GPUI main thread. GPUI's `Context::spawn` / `AsyncApp` APIs exist precisely for this.

**Details to keep:** a `LazyLock<tokio::runtime::Runtime>` (`TOKIO_RT`) entered with `.enter()` guard at the top of `main` before `Application::run`; WS/REST/supervisor futures spawn via `TOKIO_RT.spawn`; their results surface in the UI through the `AsyncApp` update path. This is the proven pattern for the pinned gpui version.

**Trade-offs:** one root entity means `AppState` grows large — that's the accepted trade of the reference (which is exactly why web-tmux's app_state should be split into `sessions.rs` / `ws_pump.rs` / `tree_poll.rs` while keeping the single entity). A finer-grained entity graph (per-tab entities with subscriptions) is viable but adds no value here: the pump discipline and `WeakEntity` upgrade pattern already isolate async results cleanly.

---

## Data Flow

### Startup Flow

```
main()
  ├─ TOKIO_RT.enter()                    // tokio primitives usable inside GPUI thread
  ├─ DesktopSettings::load()             // appdata JSON, .bak recovery
  ├─ bundle::resolve_backend_path        // settings override → beside exe → dev/test-support
  ├─ window_state::restore (bounds)      // + maximized flag
  ├─ Application::run(closure)
  ├─   fonts + gpui_component::init + theme
  ├─   open_window (TitlebarOptions transparent, min size, DWM dark Win attr)
  │     └─ cx.new(AppState) → start_supervisor
  ├─     Supervisor::spawn(SpawnOptions)         [tokio]
  ├─       stdin null, stdout/stderr piped, kill_on_drop
  ├─     BACKEND_PORT:<n> parse (handshake watch, 15s timeout)
  ├─     GET /api/health probe loop (success ⇒ Ready)
  └─   status → AppState.backend_status (watch → mpsc → GPUI)
       ├─ on Ready: BackendClient(base_url)
       ├─   tokio task: GET /api/sessions every 1.5s → update sidebar tree (metadata only)
       ├─   App applies persisted tmux binary: POST /api/tmux/binary (once; App.tsx parity)
       ├─   GET /api/tmux/info → status bar "tmux 3.x"
       └─   optional: re-open last open-session tabs (saved names) → per-tab WS open
```

Note: the Electron FE does *not* persist open tabs (it restores nothing across restarts); web-term's desktop app does (`SavedSessionTab`). Re-opening named tmux sessions after a restart is safe (tmux sessions survive) and restores the desktop workflow — decide it explicitly in planning as an accepted desktop-native extra.

Failure surfaces exactly like the FE: status "Starting backend…" while no port; a 10 s no-port timeout flips to an error page; `Failed{reason, stderr_tail}` renders the stderr tail (Electron parity: "Starting backend…" → error page).

### Session Tab Open Flow

```
User: session.create (REST POST /api/sessions — works with zero sessions)
  or: sidebar click on existing session
      ↓
SessionManager::open_tab(name)       → ensure one WS per tab, Connecting
      ↓ ws open → hello(cols, rows)
      ↓ connection.ready → Connected → state.resync
      ↓ state.snapshot { session, windows[], panes[], activeWindow, activePane }
      ↓ build the workspace model for the active window
      ↓   terminal.capture(paneId) per visible pane (send queue flushes once connected)
      ↓ terminal.snapshot { paneId, data, screenRows }
      ↓   applyCapture → Term grid → first paint
      ↓ terminal.output { paneId, data, replace }   … live stream, per pane
```

Snapshot updates are **full replaces** (both `state.snapshot` and `state.delta` are applied identically, like the FE). On replacement, reconcile pane slots: evict terminal instances for panes gone from the active window, instantiate + capture terminals for newly visible panes, keep otherwise.

### Terminal Output / Input Flow

```
WS event terminal.output(paneId, data)
     ↓ [ws_pump]
SessionTab.panes[paneId].view.terminal.process_bytes(ansi_replay_prefix? + data)
     → alacritty Term grid updated (even if tab hidden — cheap; notify only if rendered)
     ↓ [input]
TerminalView keystroke → keystroke_to_bytes → terminal.input { paneId: focused_pane, data: <string payload> }
     (click on pane first: mouse handler → pane.select(requestId) + focus that pane → then keys route)
```

Selection/copy uses alacritty Selection (web-term `mouse.rs`: `pixel_to_cell`, `selection_type_from_clicks`); clipboard default via GPUI clipboard.

### Command Flow (every GUI mutation)

```
UI action → TmuxWs::pane_split(dir) etc. → { type, paneId, requestId }
     ↓ pending[requestId] = oneshot
command.success / command.error → resolve (message on error)
     ↓ callers await like runCommand (toasts, busy flags)
10 s timeout → forget (dead queue can never stick the UI)
```

`state.delta`/`state.snapshot` arriving after commands confirms the new geometry — the workspace never invents layout; it renders what tmux reports.

### Window/Session Resize Flow

```
workspace size change (window resize, sidebar toggle, layout change)
     ↓ real viewport = visible pane's cols/rows scaled by tmux cell ratio (debounce 100ms)
terminal.resize { cols, rows } → tmux reflows window
     ↓ state.delta arrives (new pane cells) → re-render pane rects
layoutKey changed? → further debounce (150ms) resize, then (325ms) forced captures
```

---

## Integration Points

### External Boundary: tmux-gui-server (Go, unchanged)

| Surface | Integration Pattern | Gotchas |
|---------|--------------------|---------|
| Process spawn | supervisor crate: spawn with env-only knobs, `stdin(null)`, `stdout/stderr` piped, `kill_on_drop`; graceful stop with grace period | clear `TMUX`/`TMUX_PANE` from child env; resolve `TMUXGUI_TMUX_BIN` (winget/PATH); `TMUXGUI_PORT=0` lets the OS assign an ephemeral port, so there is no busy-port contention path |
| `BACKEND_PORT:<n>` | stdout line handshake (same parser as web-term) | after the handshake resolves, keep a background drain on stdout so the child can never SIGPIPE on a late write (web-tmux logs go to stderr, but drain anyway — web-term's supervisor stops reading stdout once the handshake future returns) |
| REST `/api/health` | supervisor readiness + FE error state | — |
| REST `/api/tmux/info` | status bar version | cached (staleTime 60 s in FE) |
| REST `/api/tmux/binary` | POST `{ path }`; applied once after ready when a persisted choice exists | backend validates with `tmux -V`; failures surface in Settings page |
| REST `/api/sessions` GET | sidebar tree poll every 1.5 s (tokio) | metadata only; null → `[]` normalization |
| REST `/api/sessions` POST | create dialog; `{name, cwd, initialCommand}`; REST — not WS | error body shape `{error}` → friendly toast |
| REST `/api/sessions/{name}/snapshot` | optional one-shot seed | WS snapshot arrives anyway after `state.resync`; not load-bearing |
| WS `/api/ws?session=<name>` | 1 per open tab; JSON text; see Patterns 2–4 | generation guard; server closes ⇒ stop control monitoring; pane ids unique across sessions |
| Desktop settings JSON | GPUI-authored at platform config dir | replaces React localStorage: uiTheme, terminalTheme override, font family/size, cursor style/blink, scrollback, tmuxBinary, confirm-kill flags, window_state, open_sessions (Vec<String>) |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|--------------|-------|
| app ↔ supervisor | `tokio::sync::watch` (status) + `info()` result | supervisor keeps GPUI-free (headless-testable) |
| app ↔ backend-client | async trait-like calls + WS event channel (flume/mpsc) | backend-client has no gpui dependency |
| app ↔ terminal crate | `TerminalView` entities; input/resize callbacks; `ColorPalette` | terminal crate only touches gpui, not the rest of the app |
| ws_pump ↔ sessions | direct mutation via `WeakEntity<AppState>` + `cx.notify()` on tabs/snapshots | single-write discipline; no shared locks on app state |
| theme ↔ gpui-component | `Theme::change(mode)` + Global token overrides | custom view colors come from `AppState` preset accessor fns like web-term |

---

## New vs Modified vs Reused (explicit)

| Component | Status | Detail |
|-----------|--------|--------|
| crates/terminal (all 7 modules) | **REUSE** (verbatim) | rendering engine copied; add settings-driven `scrollback_limit` and CellDimensions updates feeding resize |
| window_state.rs, bundle.rs, icons.rs scaffold | **REUSE + light edits** | bin name `tmux-gui-server`; web-tmux-specific lucide set (title bar, palette, split, zoom, refresh, alert, settings, +, …) |
| supervisor crate | **REUSE + modify** | env vars, probe endpoint, no key/db validation, child env hygiene |
| settings crate | **REUSE + modify** | schema: tmuxBinary, ui_theme/terminal_theme override, font/cursor/scrollback, confirm-kill, open_sessions: Vec<String>, window_state; drop encryption key |
| backend-client/rest.rs, types.rs | **REWRITE (same shape)** | new endpoint set + tmux types (mirror tmux-types.ts); error-body `{error}` handling |
| backend-client/tmux_ws.rs | **NEW** | replaces terminal_ws.rs' binary protocol with web-tmux JSON protocol (generation guard/backoff/correlation logic ported from `websocket.ts`) |
| web-term webterm app crate | **ADAPT** (pattern, not wholesale copy) | new SessionTab/PaneSlot model, pane workspace geometry, sidebar tree, palette, settings page, status bar, empty/error states; nav shell / tab strip / theme scaffold ported; **split app_state.rs into modules** |
| theme presets | **NEW data** | generated from web-tmux's `ui-themes.ts`/`terminal-themes.ts`; extend ThemePreset struct (input/ring/destructiveForeground); link terminal theme per UI theme |
| Go backend (`be/`) | **UNCHANGED** | frontend-only milestone (PROJECT.md Out of Scope) |

---

## Suggested Build Order (dependency-respecting)

1. **Workspace skeleton + supervisor + settings**: desktop-gpui workspace pinned to web-term versions (commit Cargo.lock), crate `supervisor` (web-tmux env/probe), settings crate (schema above), minimal window shell with backend status page (Starting→Ready/Failed). *Gate: app opens, spawns tmux-gui-server, port handshake resolves, /api/health green; stderr tail surfaces on failure.*
2. **backend-client REST + app scaffolding**: types + rest; sidebar tree (1.5 s poll), status bar (tmux version), tmux-binary apply, empty/error/select states, Create Session dialog via REST POST. *Depends on: supervisor Ready → BackendClient.*
3. **backend-client WS client (tmux_ws) + SessionTab lifecycle**: JSON protocol, generation guard/backoff/queue, requestId correlation; open/close per tab; optional restore-tabs-after-restart (desktop-native extra — see Startup note); snapshot model + window tab strip + window toolbar without live terminals (placeholder rects). *Proves the multi-tab socket discipline before terminal complexity lands.*
4. **terminal crate port + single-pane rendering**: verbatim port (alacritty Term → TerminalView); wire per-pane output pump incl. capture/history-delta ingestion, focus/select, keystrokes→terminal.input, scrollback/selection/clipboard. *Biggest new-code chunk; do it before the grid so one visible pane renders.*
5. **Pane grid**: geometry port (pixelRect/closePaneGaps/pxToColsRows), multi-pane layouts, zoom, dividers→paneResize, layout presets (`window.layout`), pane/window context menus (select/split/kill/rename/zoom/break/swap, window move/break-active/create/rename/kill), per-tab viewport resize + layout-change resync.
6. **Shell parity + theme system**: theme.rs generated from web-tmux's TS palettes (UI + terminal, live re-skin incl. hidden tabs), settings page (ui themes grid, terminal themes, font/cursor/scrollback, tmux binary path w/ validation), command palette (Ctrl+Shift+P), embedded lucide icons, titlebar/window-state polish (DWM dark, maximized restore).
7. **Hardening & packaging**: tmux.disconnected/reconnecting banners, ErrorState retry, Windows+Linux runs, bundle packaging (backend adjacent to exe), Makefile targets, Cargo.lock-locked builds.

Ordering rationale: **supervisor → REST → WS → terminal engine → pane grid → shell → packaging** mirrors hard dependencies (nothing renders without the spawn + snapshot client; terminal engine is the long pole and must be de-risked on a single pane before grid math is layered; theme/palette are isolated polish). Phases 1–3 are high-confidence ports (web-term proven + FE algorithmic fidelity); phases 4–5 carry the real engineering risk (alacritty embedding, drag math, capture resync) and deserve the phase-specific attention.

---

## Scaling Considerations

| Scale | Consideration |
|-------|---------------|
| 1–3 open tabs, ≤ ~9 panes | Every open tab keeps a live WS and Term buffers current even while hidden — a few 80×24 grids + byte feeds, negligible. Matches the FE's keep-mounted tabs exactly |
| 10+ sessions / many panes | Terminals only exist for the active window's panes (window switches create/capture fresh ones, like the FE's unmount/mount); snapshot updates are small JSON full-replaces; keep the 1.5 s tree poll as-is |
| Chatty output (builds/logs) | Output never touches the view layer per frame: bytes → `Term` grid → single `cx.notify()` (only when the tab is rendered) → batched GPUI draw; alacritty's grid diffing (web-term `render.rs`) is the hot path |

**First bottleneck (long-run):** pane grids with many open tabs updating simultaneously — solved by rendering only the active tab while hidden tabs' Term entities just ingest bytes (FE-parity).
**Second:** WS reconnect storms on tmux death — the FE's backoff ladder + generation guard prevent pile-ups; keep both.

---

## Anti-Patterns

### Anti-Pattern 1: Modeling "one terminal per tab" (web-term's shape) to save effort

**Why wrong:** web-tmux's snapshot model has N panes under one session WS; a per-tab terminal breaks split, zoom, layouts, resize reporting.
**Do instead:** the tab owns `snapshot + panes: BTreeMap<paneId, PaneSlot>` (Pattern 3).

### Anti-Pattern 2: A global per-pane registry like xterm's

**Why worse in GPUI:** the WS pump always knows the SessionTab; a global map complicates eviction per snapshot and loosens ownership.
**Do instead:** pane slots live on the tab; a helper applies the capture ANSI (Pattern 4). *(Note: keep the registry's two hard guards — snapshot-once and history-delta counter — as pure functions.)*

### Anti-Pattern 3: Maintaining derived pane layout in the app

**Why wrong:** tmux owns layout; recomputed pixel spillage and stale topology follows.
**Do instead:** render what `state.snapshot` reports; run commands and await the next snapshot.

### Anti-Pattern 4: Binary-framed WS reuse from web-term

**Why wrong:** pane addressing and JSON `terminal.input` shaping are central; binary frames can't carry `paneId`.
**Do instead:** text JSON client mirroring `websocket.ts` incl. generation guard + command correlation.

### Anti-Pattern 5: Duplicating tmux state in local structures or a DB

**Why wrong:** the app would drift from the real server (splits from a second client, manual tmux usage, etc.) and violate the PRD's core principle.
**Do instead:** tmux owns everything; GPUI renders snapshots. All app-side structures are derived views of the latest `state.snapshot`, rebuildable via `state.resync` at any time. Matches PROJECT.md Out of Scope directly.

---

## Sources

- web-term reference implementation (proven Windows+Linux GPUI port): `E:\Coding Stuff\web-term\desktop-gpui\crates\{supervisor,settings,backend-client,terminal,webterm}` (PRIMARY/high-confidence code-level reference)
- web-tmux FE (behavioral ground truth): `fe/src/lib/{protocol,tmux-types,websocket,sockets,commands,api,geometry}.ts`, `stores/{tmuxStore,appStore,settingsStore}.ts`, `features/{panes,terminal,settings/data}`; `App.tsx`
- web-tmux backend/Electron (integration ground truth): `be/cmd/server/main.go` (BACKEND_PORT), `be/internal/config/config.go` (env), `desktop/main.js` (sidecar spawn, env hygiene, dev-mode)
- tmux control mode: official tmux wiki Control-Mode page (MEDIUM after cross-verification against shipped backend behavior)
- gpui/gpui-component API: Context7 `/websites/rs_gpui`, `/longbridge/gpui-component` (MEDIUM per confidence seam), corroborated by working web-term usage
- Cached digests in the GSD research store (keys `09c2545…` gpui entity/async pattern · `74313c4…` tmux control mode · `0b995eb…` alacritty embedding as read from web-term code)

---
*Architecture research for: web-tmux desktop-gpui integration (v1.0 milestone)*
*Researched: 2026-09-06*
