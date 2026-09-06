# Phase 2: REST Client, Sidebar & Session Management - Research

**Researched:** 2026-09-06  
**Domain:** Typed REST client (`webtmux-backend-client`), hand-rolled collapsible sidebar tree (`views/sidebar.rs`), modal session creation dialog (`gpui-component` Dialog + Input/InputState), native directory picker (`App::prompt_for_paths`), 1.5s background polling with generation guards, and zero/error state workspace pages (`STATE-02`).  
**Confidence:** HIGH — all Go backend handlers (`be/internal/server/health.go`, `router.go`, `model.go`, `command.go`), frontend consumer contracts (`fe/src/lib/api.ts`, `tmux-types.ts`, `CreateSessionDialog.tsx`, `AppSidebar.tsx`), and vendored crate sources (`gpui-component-0.6.0`, `gpui-pre-0.3.3`) were read and line-verified this session.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### REST client crate
- Lift the Phase-1 `backend-client` stub to a full crate mirroring web-term's `backend-client` (`rest.rs` pattern): reqwest (rustls) with a `RestClient { base_url: String, http }`; typed structs `TmuxSession {name, windows, attached, created_at, width, height}`, `TmuxTree { SessionNode { session, windows: Vec<WindowNode { window, panes }> } }` matching the Go JSON (`be/` handlers) with `#[serde(default)]` and null→empty normalization (`api.ts` parity: `sessions ?? []`).
- Endpoints exactly as `fe/src/lib/api.ts`: `GET /api/health`, `GET /api/tmux/info`, `POST /api/tmux/binary` (Phase 6 uses it — NOT this phase), `GET /api/sessions` → `TmuxTree`, `POST /api/sessions {name, cwd?, initialCommand?}` → `{name}`; NO snapshot fetch this phase (Phase 4 owns capture replay; tree only).

#### Sidebar tree + polling
- Hand-rolled sidebar (web-term `nav.rs` pattern; `gpui-component` TitleBar/Tree is out for 1:1): `views/sidebar.rs` — w-60 (240px) column, border-r, bg `#1e1e1e`, right of nothing-left (title bar persists).
- Header row h-9: "Sessions" label (12px, `#808080`) + two 3px-icon ghost buttons (`RefreshCw` lucide + `Plus`) — sha specifics per fe `AppSidebar.tsx`.
- Rows: session row (`ChevronRight` 3.5px pivot + name 14px/500 + windows-count badge muted 10px), expand → window rows (index: name, 12px), expand → pane rows (id, current command); expanded state: `HashSet<String>` per node id; active-session row bg `#2d2d2d` (FE bg-accent-equivalent token) — clicking a row = `SelectSession` (workspace body gets a placeholder "Select a session" page — `STATE-02`).
- Poll: tokio interval 1500ms refetch tree from REST (TanStack parity) + Refresh button triggers same path immediately; new (CLI-created) sessions appear on the next poll — `SESS-06`.
- Sidebar toggle button lands in the Phase-1 title bar bar (`SHELL-03`): binary snap w-60 ↔ w-0, no slide animation (documented deviation A-accepted).

#### Create Session dialog
- `gpui-component` Dialog (`open_dialog`) + Input/InputState (first real text entry — proves the `gpui-component` 0.6 Input path): fields Name (required, validated via tmux session-name rules client-side too), Directory (Input + "Browse…" button → `gpui-pre` `App::prompt_for_paths(PathPromptOptions{directories:true})` built-in picker — stack research verified; no `rfd`), Initial command (optional Input).
- Submit → `POST /api/sessions` → on success close dialog + poll-refresh + open the new session as the active workspace; on error render backend error message (e.g. duplicate session) inline under the form (fe parity).
- Dialog visual tokens from ui-themes default-dark (card `#1e1e1e`, border `#3c3c3c`, radius 8px, primary `#d4d4d4` fill button) — 1:1 with fe `CreateSessionDialog.tsx`.

#### Zero/error states
- EmptyState (no sessions): exact fe copy + layout (div.p-8 flex column centered, muted copy) — `STATE-02`.
- ErrorState (tmux missing — tree GET fails with backend-alive): fe `ErrorState.tsx` structure: heading + explanation + Retry (primary) wired to the poll path.
- SelectSessionView: muted centered copy over the bare body when a session tab is open but no session selected (parity fe `SelectSessionView.tsx`).
- The Phase-1 Starting/Failed pages remain untouched; ErrorState wiring = tree fetch mapped to *error branch*.

### the agent's Discretion
None declared beyond the locked decisions; phase boundary is fully pinned by `02-CONTEXT.md` + approved `02-UI-SPEC.md` (visuals locked — do not re-research).

### Deferred Ideas (OUT OF SCOPE)
- Session keyboard shortcuts (`ctrl-tab`/`alt-1`…) — `EXTRA-02`, v2
- Rename/kill session flows + context menus — Phase 3 (mutating commands ride the WebSocket there)
- Tab persistence on restart — `EXTRA-01`, v2

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SHELL-03 | User can toggle the sidebar collapsed/expanded (binary snap; no slide animation — accepted deviation, documented) | Title bar sidebar toggle button (`PanelLeft` icon) toggling boolean `sidebar_open` in `AppState`; `views/sidebar.rs` binary switch between `w(px(240.0))` and `w(px(0.0))` with `overflow_hidden`. |
| SESS-01 | User sees a collapsible session→window→pane tree in the sidebar, refreshed by 1.5s polling plus a manual refresh button | Typed `TmuxTree` fetched via `webtmux-backend-client::RestClient::tree()` matching Go structs verbatim; recursive div composition with `expanded: HashSet<String>`; background 1500ms tokio ticker + generation counter; manual refresh trigger. |
| SESS-03 | User can create a session (name required, optional cwd + initial command) via the Create Session dialog with a native directory picker | `gpui-component` Dialog + `Entity<InputState>` for Name/CWD/Command; native directory selection via `cx.prompt_for_paths(PathPromptOptions { directories: true, .. })`; client-side + server-side validation; `POST /api/sessions`. |
| SESS-06 | Sessions created from the tmux CLI appear in the sidebar without an app restart | 1.5s polling loop in `AppState` continuously updates `TmuxTree` state entity; `cx.notify()` updates GPUI view tree reactively. |
| STATE-02 | User sees Empty (no sessions), Error (tmux missing), and SelectSession states matching Electron | Three distinct workspace body views: `EmptyState` (`TerminalSquare` icon + "Create Session" CTA), `ErrorState` (`AlertTriangle` icon + "Retry" button), `SelectSessionView` (`SquareTerminal` icon + 2-col session grid). |
</phase_requirements>

## Summary

Phase 2 transitions the web-tmux desktop client from a supervisor shell to a functional session manager. It implements typed REST communication with the Go backend, renders a hand-rolled collapsible sidebar tree with non-blocking 1.5s background polling, introduces interactive modal forms via `gpui-component` Dialog and `InputState`, enables native OS directory selection through GPUI's built-in `prompt_for_paths`, and establishes clear zero/error/selection workspace states.

All REST JSON contracts were verified against Go backend source (`be/internal/server/health.go`, `model.go`, `command.go`) and Electron frontend clients (`fe/src/lib/api.ts`, `tmux-types.ts`). The data structures use serde defaults and camelCase mapping to prevent deserialization failures on empty or omitted backend slices (`sessions ?? []`).

A generation/epoch counter guard is established for all asynchronous REST polling tasks to prevent stale responses from overwriting newer tree snapshots—a critical safety mechanism that becomes load-bearing in Phase 3.

**Primary recommendation:** Implement `webtmux-backend-client` with typed DTOs and `reqwest` client, add a generation-guarded polling pump in `AppState`, construct `views/sidebar.rs` and `views/session_states.rs` using raw GPUI div composition matching `02-UI-SPEC.md`, and wire `CreateSessionDialog` using `gpui-component` `open_dialog` + `Input` + `App::prompt_for_paths`.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| HTTP REST Client (`GET/POST /api/*`) | `webtmux-backend-client` crate (pure async tokio/reqwest) | — | Headless, GPUI-independent crate for deterministic network communication and easy unit/mock testing. |
| Polling & Generation Guard Pump | `AppState` entity (`webtmux` app crate) | Background tokio task | GPUI main thread owns the canonical `TmuxTree` state; tokio ticker triggers async fetches tagged with generation IDs. |
| Collapsible Sidebar Tree (`SB1`) | `views/sidebar.rs` (app crate) | `AppState` (`expanded: HashSet<String>`) | Hand-rolled GPUI div layout matching Electron geometry (240px width, 36px header, nested indent, badge styling). |
| Create Session Dialog (`DLG1`) | `views/create_session_dialog.rs` (app crate) | `gpui-component` Dialog / Input | Modal lifecycle, focus management, backdrop dismiss, and form state binding. |
| Directory Picker Integration | GPUI Platform API (`App::prompt_for_paths`) | `CreateSessionDialog` callback | Native OS dialog invocation via GPUI built-in platform bindings without third-party crate overhead. |
| Workspace State Pages (`ST1`) | `views/session_states.rs` (app crate) | `AppState` active session router | Renders EmptyState, ErrorState, or SelectSessionView in the main workspace body based on `TmuxTree` and selection status. |

---

## Standard Stack

### Core Workspace Dependencies (Phase 2)

| Library | Pinned Version | Purpose | Why Standard / Verification |
|---------|----------------|---------|-----------------------------|
| `reqwest` | `=0.12.28` | HTTP client (rustls-tls, json) | [VERIFIED: `desktop-gpui/Cargo.toml:45-49`] Pinned workspace dependency; rustls prevents OpenSSL linking issues on Linux. |
| `serde` & `serde_json` | `=1.0.229` & `=1.0.151` | DTO serialization/deserialization | [VERIFIED: `desktop-gpui/Cargo.toml:43-44`] Handles camelCase tagging and null-safe defaults. |
| `gpui-component` | `=0.6.0` | Dialog, Input, Button, Theme | [VERIFIED: `gpui-component-0.6.0/src/lib.rs`] Provides `open_dialog`, `DialogContent`, `Input`, and `InputState`. |
| `gpui` (`gpui-pre`) | `=0.3.3` | UI framework, window rendering, platform API | [VERIFIED: `gpui-pre-0.3.3/src/app.rs:1582`] Provides `prompt_for_paths` for native directory picker. |
| `tokio` | `=1.53.1` | Multi-threaded async runtime & time ticker | [VERIFIED: `desktop-gpui/Cargo.toml:35-42`] Drives background interval polling. |

---

## REST API Contracts & Go Backend Source Truth

### 1. Endpoint: `GET /api/health`
[VERIFIED: `be/internal/server/health.go:28-43`]

**Go Implementation:**
```go
func (h *HealthHandler) Handle(w http.ResponseWriter, r *http.Request) {
    ctx, cancel := context.WithTimeout(r.Context(), 3*time.Second)
    defer cancel()

    version, err := h.svc.TmuxVersion(ctx)
    tmuxOK := err == nil

    h.writeJSON(w, http.StatusOK, map[string]interface{}{
        "status": "ok",
        "tmux": map[string]interface{}{
            "installed": tmuxOK,
            "version":   version,
        },
    })
}
```

**Rust DTO:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub status: String,
    pub tmux: HealthTmux,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthTmux {
    pub installed: bool,
    pub version: String,
}
```

---

### 2. Endpoint: `GET /api/tmux/info`
[VERIFIED: `be/internal/server/health.go:45-56`]

**Go Implementation:**
```go
func (h *HealthHandler) HandleInfo(w http.ResponseWriter, r *http.Request) {
    ctx, cancel := context.WithTimeout(r.Context(), 3*time.Second)
    defer cancel()

    version, err := h.svc.TmuxVersion(ctx)
    h.writeJSON(w, http.StatusOK, map[string]interface{}{
        "version": version,
        "ok":      err == nil,
        "binary":  tmux.BinaryPath(),
    })
}
```

**Rust DTO:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TmuxInfo {
    pub version: String,
    pub ok: bool,
    pub binary: String,
}
```

---

### 3. Endpoint: `GET /api/sessions` (Sidebar Tree)
[VERIFIED: `be/internal/server/health.go:85-97`, `be/internal/tmux/model.go:3-68`]

**Go Data Model:**
```go
// Session is the top-level tmux object. Name is the stable identifier.
type Session struct {
    Name      string `json:"name"`
    Windows   int    `json:"windows"`
    Attached  int    `json:"attached"`
    CreatedAt int64  `json:"createdAt"`
    Width     int    `json:"width"`
    Height    int    `json:"height"`
}

// Window uses its tmux stable ID (@N) as identifier; Index is display-only.
type Window struct {
    ID     string `json:"id"`
    Index  int    `json:"index"`
    Name   string `json:"name"`
    Active bool   `json:"active"`
    Panes  int    `json:"panes"`
    Width  int    `json:"width"`
    Height int    `json:"height"`
    Layout string `json:"layout"`
}

// Pane uses its tmux stable ID (%N) as identifier; Index is display-only.
type Pane struct {
    ID             string `json:"id"`
    Index          int    `json:"index"`
    WindowID       string `json:"windowId"`
    Active         bool   `json:"active"`
    Zoomed         bool   `json:"zoomed"`
    Left           int    `json:"left"`
    Top            int    `json:"top"`
    Width          int    `json:"width"`
    Height         int    `json:"height"`
    PID            int    `json:"pid"`
    CurrentCommand string `json:"currentCommand"`
    CurrentPath    string `json:"currentPath"`
    Title          string `json:"title"`
}

type Tree struct {
    Sessions []SessionTreeNode `json:"sessions"`
}

type SessionTreeNode struct {
    Session Session          `json:"session"`
    Windows []WindowTreeNode `json:"windows"`
}

type WindowTreeNode struct {
    Window Window `json:"window"`
    Panes  []Pane `json:"panes"`
}
```

**Null vs Empty Normalization:**
[VERIFIED: `fe/src/lib/api.ts:59-64`]
Frontend parity requires `sessions: tree?.sessions ?? []` and `panes: wn.panes ?? []`.
In Rust serde, use `#[serde(default)]` on collection vectors so `null` or missing JSON fields deserialize to `vec![]` without error.

**Rust DTOs:**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TmuxTree {
    #[serde(default)]
    pub sessions: Vec<SessionTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTreeNode {
    pub session: TmuxSession,
    #[serde(default)]
    pub windows: Vec<WindowTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowTreeNode {
    pub window: TmuxWindow,
    #[serde(default)]
    pub panes: Vec<TmuxPane>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TmuxSession {
    pub name: String,
    pub windows: usize,
    pub attached: usize,
    pub created_at: i64,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TmuxWindow {
    pub id: String,
    pub index: usize,
    pub name: String,
    pub active: bool,
    pub panes: usize,
    pub width: usize,
    pub height: usize,
    pub layout: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TmuxPane {
    pub id: String,
    pub index: usize,
    pub window_id: String,
    pub active: bool,
    pub zoomed: bool,
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
    pub pid: usize,
    pub current_command: String,
    pub current_path: String,
    pub title: String,
}
```

---

### 4. Endpoint: `POST /api/sessions` (Create Session)
[VERIFIED: `be/internal/server/health.go:118-150`, `be/internal/tmux/command.go:17-38`]

**Request Body Structure:**
```json
{
  "name": "dev",
  "cwd": "C:\\Users\\user\\projects",
  "initialCommand": "nvim"
}
```

**Go Validation Rules:**
```go
var validSessionName = regexp.MustCompile(`^[A-Za-z0-9][A-Za-z0-9_./-]*$`)

func ValidateSessionName(name string) error {
    if name == "" {
        return fmt.Errorf("session name is required")
    }
    if len(name) > 200 {
        return fmt.Errorf("session name too long")
    }
    if strings.ContainsAny(name, ":.") {
        return fmt.Errorf("session name must not contain ':' or '.'")
    }
    if strings.HasPrefix(name, "$") {
        return fmt.Errorf("session name must not start with '$'")
    }
    if !validSessionName.MatchString(name) {
        return fmt.Errorf("invalid session name %q", name)
    }
    return nil
}
```

**Go Response Behavior:**
- **Success (201 Created):** `{"name": "dev"}`
- **Validation Failure (400 Bad Request):** `{"error": "session name must not contain ':' or '.'"}`
- **Duplicate Session (409 Conflict):** `{"error": "duplicate session: dev"}`
- **Server Error (500 Internal Server Error):** `{"error": "<internal error details>"}`

**Rust Request / Error DTOs:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_command: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSessionResponse {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiErrorResponse {
    pub error: String,
}
```

---

## Technical Mechanics & Implementations

### 1. `gpui-component` 0.6 Dialog & Input Mechanics

[VERIFIED: `gpui-component-0.6.0/src/dialog/dialog.rs:1-692`, `gpui-component-0.6.0/src/input/input.rs:165-202`, `gpui-component-0.6.0/src/window_ext.rs:29-60`]

`gpui-component` provides `WindowExt::open_dialog` on `Window`, which handles dialog layering, focus entrapment, backdrop rendering, and keyboard dismissal (Escape):

```rust
// Opening dialog on window
window.open_dialog(cx, |dialog, _window, cx| {
    dialog
        .w(px(440.0))
        .close_button(true)
        .overlay(true)
        .overlay_closable(true)
        .keyboard(true)
        .header(
            DialogHeader::new()
                .child(DialogTitle::new().child("Create Session"))
                .child(DialogDescription::new().child("Start a new tmux session."))
        )
        .child(
            // Form body container with Input elements
            render_create_session_form(form_entity, cx)
        )
        .footer(
            DialogFooter::new()
                .child(
                    Button::new("cancel")
                        .label("Cancel")
                        .ghost()
                        .on_click(|_, window, cx| {
                            window.close_dialog(cx);
                        })
                )
                .child(
                    Button::new("create")
                        .label("Create")
                        .primary()
                        .on_click(...)
                )
        )
});
```

**Input / InputState Handling:**
- Each text field is managed via an `Entity<InputState>` created with `cx.new(|cx| InputState::new(cx))`.
- Text is read with `input_state.read(cx).text().to_string()`.
- Text is updated with `input_state.update(cx, |state, cx| state.set_text("new text", window, cx))`.
- Rendered in UI via `Input::new(&self.name_input).placeholder("dev")`.

---

### 2. Native Directory Picker via GPUI Built-in Platform API

[VERIFIED: `gpui-pre-0.3.3/src/app.rs:1582-1587`, `gpui-pre-0.3.3/src/platform.rs:2259-2268`]

`gpui-pre` includes built-in platform file and directory picker integration on `App` (`cx`):

```rust
pub struct PathPromptOptions {
    pub files: bool,
    pub directories: bool,
    pub multiple: bool,
    pub prompt: Option<SharedString>,
}
```

**Usage inside an event handler:**
```rust
let rx = cx.prompt_for_paths(PathPromptOptions {
    files: false,
    directories: true,
    multiple: false,
    prompt: Some("Select Working Directory".into()),
});

// Await in GPUI async context
cx.spawn(|mut cx| async move {
    if let Ok(Ok(Some(paths))) = rx.await {
        if let Some(first_dir) = paths.first() {
            let path_str = first_dir.to_string_lossy().to_string();
            let _ = cx.update(|cx| {
                // Update cwd input state
            });
        }
    }
}).detach();
```

**Platform Behavior & Failure Modes:**
- **Windows:** Uses `IFileOpenDialog` with `FOS_PICKFOLDERS`. Returns `None` if user cancels or closes modal.
- **Linux:** Uses XDG Desktop Portals (`org.freedesktop.portal.FileChooser`). Returns `Err` if portal service is not active. Handled gracefully by checking `if let Ok(Ok(Some(...)))`.

---

### 3. Sidebar Render Pattern & Recursive Div Composition

[VERIFIED: `fe/src/components/layout/AppSidebar.tsx:43-154`]

The sidebar is composed as a raw GPUI div element matching FE geometry:
- Fixed width container: `w(px(240.0))` when expanded, `w(px(0.0))` when collapsed (`overflow_hidden`).
- Header (`h(px(36.0))`): Title "Sessions" (12px, `#808080`) + Action buttons (`RefreshCw`, `Plus`).
- Tree Area: Iterates over `tree.sessions`.
  - Session Row (`px(8.0)`, `py(6.0)`, rounded 6px, cursor pointer):
    - `ChevronRight` icon (14px) rotated 90° if `expanded.contains(&session.name)`.
    - Session name (Body 14px/500, `#d4d4d4`, truncated).
    - Window count badge (`px(4.0)`, 10px, `#808080`, bg `#2d2d2d`).
  - Window Sub-Tree (`ml(px(16.0))`, `border_l_1`, `pl(px(8.0))`):
    - Window row (Label 12px/400, `#aaaaaa`): `"{window.index}: {window.name}"`.
    - Pane row (`pl(px(12.0))`, 11px/400, `#808080`): 4px circular dot + `{pane.current_command || pane.title || pane.id}`.
- Footer: 1px top border + "Settings" button (14px icon + text).

---

### 4. Background Polling & Generation / Epoch Guard

To prevent race conditions where a delayed HTTP response from an older polling cycle overwrites a newer tree update or manual refresh:

```rust
pub struct AppState {
    pub tree: TmuxTree,
    pub poll_generation: u64,
    pub is_polling: bool,
    // ...
}

impl AppState {
    pub fn trigger_poll(&mut self, cx: &mut Context<Self>) {
        self.poll_generation += 1;
        let current_gen = self.poll_generation;
        let base_url = match &self.base_url {
            Some(url) => url.clone(),
            None => return,
        };
        let client = self.rest_client.clone();
        let entity = cx.entity().downgrade();

        cx.spawn(|cx| async move {
            let result = client.tree().await;
            let _ = cx.update(|cx| {
                if let Some(app) = entity.upgrade() {
                    app.update(cx, |this, cx| {
                        // Drop stale responses
                        if this.poll_generation != current_gen {
                            return;
                        }
                        match result {
                            Ok(tree) => {
                                this.tree = tree;
                                this.backend_error = None;
                            }
                            Err(e) => {
                                this.backend_error = Some(e.to_string());
                            }
                        }
                        cx.notify();
                    });
                }
            });
        }).detach();
    }
}
```

---

### 5. Windows Shell & Reqwest Runtime Pitfall

[VERIFIED: `01-RESEARCH.md §spawn-contract`, `desktop-gpui/crates/webtmux/src/app_state.rs:9-15`]

- **Tokio Runtime Enter:** `reqwest::Client` with `rustls` creates internal async connection pools and timers. It MUST be initialized inside the context of `TOKIO_RT.enter()`.
- **Base URL Resolution:** In desktop mode, `base_url` is dynamic (`http://127.0.0.1:<port>`) learned from the supervisor's `BackendInfo.base_url`. `RestClient` accepts `base_url: String` dynamically.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Modal Dialog & Focus Trap | Custom floating div overlay with manual key interception | `gpui-component::dialog::open_dialog` | Handles backdrop occlusion, focus trapping, Escape key dismiss, and theme-styled dialog container. |
| Single-Line Text Input | Custom keyboard event capture / text buffer | `gpui-component::input::{Input, InputState}` | Handles text selection, cursor navigation, backspace, clipboard copy/paste, and IME. |
| Native Directory Dialog | Custom file tree browser UI or `rfd` crate | `cx.prompt_for_paths(PathPromptOptions)` | GPUI built-in platform method uses Windows `IFileOpenDialog` and Linux portal natively with zero extra crates. |
| HTTP Connection Pool | Raw `TcpStream` HTTP parser | `reqwest` (=0.12.28 rustls) | Workspace-pinned standard client with connection pooling, JSON decoding, and timeout management. |

---

## Common Pitfalls

### Pitfall 1: Deserializing `null` slices from Go backend
**What goes wrong:** Go's `json.Marshal` serializes empty/nil slices `[]SessionTreeNode(nil)` as `null`. Standard Rust `Vec<T>` deserializer throws `invalid type: null, expected a sequence`.  
**Why it happens:** When tmux has 0 sessions or a window has 0 panes, the Go backend encodes `nil` slice fields.  
**How to avoid:** Annotate every vector field in Rust DTOs with `#[serde(default)]`.

### Pitfall 2: Stale Poll Responses Overwriting New State
**What goes wrong:** A slow 1.5s background poll returns after a user performs a manual refresh or creates a session, causing the UI to briefly show deleted/stale sessions.  
**Why it happens:** Asynchronous network latency varies across polling ticks.  
**How to avoid:** Increment a `poll_generation: u64` counter on every poll trigger; discard responses if `poll_generation != current_gen`.

### Pitfall 3: Initializing Reqwest Client Outside Tokio Context
**What goes wrong:** Panic `there is no reactor running, must be called from the context of a Tokio 1.x runtime`.  
**Why it happens:** `reqwest::Client::new()` registers reactor hooks with Tokio.  
**How to avoid:** Ensure `RestClient::new()` is called inside `TOKIO_RT.enter()` or within an async task spawned on `TOKIO_RT`.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test framework (`cargo test`) |
| Config file | `desktop-gpui/Cargo.toml` |
| Quick run command | `cargo test -p webtmux-backend-client` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SESS-01 | REST client parses `GET /api/sessions` JSON tree into typed `TmuxTree` with `null` normalization | unit | `cargo test -p webtmux-backend-client test_parse_sessions_tree` | ❌ Wave 0 (`crates/backend-client/tests/rest_test.rs`) |
| SESS-01 | 1.5s interval polling loop updates state and generation guard discards stale tick | unit | `cargo test -p webtmux test_polling_generation_guard` | ❌ Wave 0 (`crates/webtmux/tests/polling_test.rs`) |
| SESS-03 | `POST /api/sessions` serializes request body `{name, cwd, initialCommand}` and handles 201/400/409 errors | unit | `cargo test -p webtmux-backend-client test_create_session_request` | ❌ Wave 0 (`crates/backend-client/tests/rest_test.rs`) |
| SESS-03 | Client-side session name validator rejects empty names, colons, dots, and leading `$` | unit | `cargo test -p webtmux-backend-client test_session_name_validation` | ❌ Wave 0 (`crates/backend-client/tests/validation_test.rs`) |
| SESS-06 | CLI-created sessions appear in `AppState` after polling tick | integration | `cargo test -p webtmux test_cli_session_polling_integration` | ❌ Wave 0 (`crates/webtmux/tests/polling_test.rs`) |
| SHELL-03 | Sidebar toggle snaps width state between 240px and 0px | unit | `cargo test -p webtmux test_sidebar_toggle_snap` | ❌ Wave 0 (`crates/webtmux/tests/sidebar_test.rs`) |
| STATE-02 | Workspace body selects correct view (`EmptyState` vs `ErrorState` vs `SelectSessionView`) | unit | `cargo test -p webtmux test_workspace_state_routing` | ❌ Wave 0 (`crates/webtmux/tests/state_view_test.rs`) |

### Mock Server & Test Fixture Strategy
To test `webtmux-backend-client` without running a live Go backend or requiring external mock crates:
- Use a lightweight in-process mock HTTP server via `tokio::net::TcpListener` responding with pre-recorded JSON payloads from `be/` handlers.
- Verify serialization and deserialization against real Go JSON outputs (fixtures for `health.json`, `tree_full.json`, `tree_empty.json`, `tree_null_slices.json`, `create_error_duplicate.json`).

### Sampling Rate
- **Per task commit:** `cargo test -p webtmux-backend-client`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full workspace test suite green before completing phase.

### Wave 0 Gaps
- [ ] `desktop-gpui/crates/backend-client/tests/rest_test.rs` — covers SESS-01 & SESS-03 REST contracts & DTO parsing.
- [ ] `desktop-gpui/crates/backend-client/tests/validation_test.rs` — covers SESS-03 session name validation rules.
- [ ] `desktop-gpui/crates/webtmux/tests/polling_test.rs` — covers SESS-01 & SESS-06 generation-guarded polling pump.
- [ ] `desktop-gpui/crates/webtmux/tests/sidebar_test.rs` — covers SHELL-03 toggle snap & tree expansion state.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Binary snap without animation for sidebar toggle (`SHELL-03`) | User Constraints / Sidebar | None — locked design decision in PRD and UI-SPEC. |
| A2 | Settings button rendered in sidebar footer as visual parity anchor | UI-SPEC parity | Minimal — action is a no-op placeholder until Phase 6. |
| A3 | Inline error banner in `CreateSessionDialog` rather than toast notification | Create Session Dialog | Minimal — toasts arrive in Phase 7; inline error gives immediate feedback on failure. |
| A4 | Lightweight `tokio::net::TcpListener` mock server for headless REST tests | Validation Architecture | None — avoids adding extra unpinned test dependencies like wiremock. |

---

## Open Questions

1. **How should manual refresh interact with the 1.5s background interval timer?**
   - *What we know:* Clicking `RefreshCw` triggers an immediate REST tree fetch.
   - *Recommendation:* Triggering manual refresh should reset/restart the 1.5s interval timer so an automatic poll does not fire immediately after a manual click (FE TanStack Query parity).

---

## Sources

### Primary (HIGH confidence)
- `be/internal/server/health.go:1-151` — REST handlers for `/api/health`, `/api/tmux/info`, `/api/sessions`, `/api/sessions/{name}/snapshot`.
- `be/internal/tmux/model.go:1-68` — Go structs for `Session`, `Window`, `Pane`, `Tree`, `SessionTreeNode`, `WindowTreeNode`.
- `be/internal/tmux/command.go:17-38` — Go session name validation rules (`ValidateSessionName`).
- `fe/src/lib/api.ts:1-74` — Frontend REST API client calls and `sessions ?? []` normalization.
- `fe/src/lib/tmux-types.ts:1-74` — TypeScript interface definitions for tmux data models.
- `fe/src/components/layout/AppSidebar.tsx:1-156` — Sidebar layout, header, tree items, count badge, expanded state.
- `fe/src/features/sessions/CreateSessionDialog.tsx:1-155` — Dialog fields, validation, directory picker trigger, submission logic.
- `fe/src/features/sessions/EmptyState.tsx:1-30` — Zero sessions state layout and CTA.
- `fe/src/features/sessions/ErrorState.tsx:1-22` — Tmux missing error state layout and Retry button.
- `fe/src/features/sessions/SelectSessionView.tsx:1-62` — Session picker grid and placeholder layout.
- `gpui-component-0.6.0/src/dialog/dialog.rs:1-692` & `window_ext.rs:1-60` — `open_dialog` API, focus trapping, header/content/footer composition.
- `gpui-component-0.6.0/src/input/input.rs:1-1078` — `Input` element and `InputState` binding.
- `gpui-pre-0.3.3/src/app.rs:1582-1587` & `platform.rs:2259-2268` — `App::prompt_for_paths` and `PathPromptOptions`.
