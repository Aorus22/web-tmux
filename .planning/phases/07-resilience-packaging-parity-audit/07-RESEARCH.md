# Phase 7: Resilience, Packaging & Parity Audit - Research

**Researched:** 2026-09-07
**Domain:** GPUI desktop resilience UX (reconnect/toast surfaces), Windows+Linux distribution packaging, Electron 1:1 parity audit
**Confidence:** HIGH (all load-bearing claims verified against in-repo source read this session; see Sources)

## User Constraints (from Phase Brief)

> No CONTEXT.md exists for Phase 7 (discussion skipped — autonomous lane). The constraints below are the phase brief verbatim; the planner MUST honor them.

### Locked Scope
- Goal: App survives tmux/backend turbulence gracefully, ships as packaged bundles for Windows + Linux, passes 1:1 parity checklist closing milestone.
- Requirements: STATE-03, PKG-02, PKG-03. Nothing else. (Phase 6 deferred chrome-threading is audit input, not a new requirement — see § Deferred Chrome Item.)
- Prior Phases 1–6 complete (116/116 green). Deferred HV advisories from Phases 2/4/5/6 MUST be folded into the parity checklist here.
- Research lane only, no code.

### Success Criteria (planner must map every plan to these)
1. tmux disconnect/reconnect shows reconnect banner + toasts with strict taxonomy (WS-drop ≠ tmux disconnected/reconnecting); command errors surface as toasts.
2. Package scripts produce Windows + Linux dist bundles with Go backend sidecar adjacent to app exe; packaged app launches and reaches session on both OSes.
3. Makefile targets build/dev/package desktop-gpui as part of repo workflow on both OSes.
4. Side-by-side parity audit vs Electron over daily-driver checklist (vim/htop, rapid switching, resize flood, rename mid-session, theme swap mid-session) shows 1:1 with only documented deviation (sidebar binary snap).

### Deferred Ideas (OUT OF SCOPE)
- EXTRA-01 session-tab persistence, EXTRA-02 keyboard shortcuts, EXTRA-03 Linux window-control polish (all v2, STATE.md).
- macOS support, backend protocol changes, installers (MSI/deb/AppImage), mobile changes.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| STATE-03 | User sees reconnect banner + toasts on `tmux.disconnected`/`tmux.reconnecting` and command errors | § Architecture Patterns (4-way taxonomy, toast queue, auto-reconnect), § Code Examples, § Pitfall 1 (WS-drop conflation — the central design constraint) |
| PKG-02 | Package scripts produce Windows + Linux dist bundles with Go sidecar adjacent to app exe | § Standard Stack (web-term 3-step script precedent), § Code Examples (bundle layout + resolve contract), § Pitfalls 5–7 |
| PKG-03 | Makefile targets integrate desktop-gpui build/dev into repo workflow | § Standard Stack (Makefile OS-dispatch pattern), § Code Examples (target recipes) |

## Summary

Phase 7 has three independent workstreams with one shared property: almost everything is a **port of a proven precedent**, not novel engineering. The resilience UX (STATE-03) ports the Electron/FE transport semantics (`fe/src/lib/websocket.ts`, `sockets.ts`, `PaneWorkspace.tsx`) and the web-term GPUI notification-banner pattern into the existing `AppState`/`apply_event` machinery. The packaging (PKG-02) ports `web-term/desktop-gpui/scripts/package-windows.ps1` + `package-linux.sh` verbatim modulo binary names. The Makefile work (PKG-03) extends the existing OS-dispatched Makefile following the `desktop-gtk` target pattern. The parity audit (SC4) is a manual-UAT consolidation of all deferred HV advisories plus the five daily-driver scenarios.

The single genuinely novel design decision is the **strict event taxonomy** (SC1): the current GPUI read pump **conflates WS transport drops with tmux-health events** — `ws.rs:437-443,485-491` synthesizes `EV_TMUX_DISCONNECTED` on *any* socket close/error, and `apply_event` (`app_state.rs:925-931`) maps both that synthesis and the real server-sent `tmux.disconnected` to the same `TransportState::Disconnected`. The planner must introduce a distinct local transport-lost signal so the banner/toast copy can distinguish "connection to backend dropped (retrying)" from "tmux server lost (waiting for tmux)". Everything downstream (banner copy, toast copy, auto-reconnect trigger, manual Reconnect button) keys off this split.

No new crates or npm/Go packages are needed. Toolchain is present (cargo 1.96, go 1.23.4, MSYS2 tmux). Linux bundle build + Linux smoke launch cannot be verified on this Windows box and must be human/CI items.

**Primary recommendation:** Port, don't invent — FE `BACKOFF` array verbatim for auto-reconnect, web-term 3-step scripts verbatim for packaging, hand-rolled bounded toast overlay (not gpui-component `Root` adoption), and split the conflated disconnect signal as the first STATE-03 task since all banner/toast copy depends on it.

## Auto-Accepted Decisions (planner treats as locked)

> Accepted autonomously (user sleeping); each is the lowest-risk option grounded in a verified precedent.

| # | Decision | Rationale |
|---|----------|-----------|
| D1 | Toasts = hand-rolled bounded overlay queue in `AppState` render (`VecDeque`, cap 5, autohide timer), NOT gpui-component `Root`/`push_notification` adoption | gpui-component 0.6.0 *has* a full `Notification` system (`Notification::info/success/warning/error`, `Root::push_notification`), but webtmux renders its own root `div`, not `Root` — adoption means re-plumbing the whole render tree mid-milestone. Hand-rolled overlay matches the existing hand-rolled-divs convention (shadcn porting is an explicit anti-feature) and the web-term `notification: Option<String>` banner precedent, and is headless-testable as a pure queue model. |
| D2 | Strict 4-way taxonomy with fixed copy: (a) WS-drop → "Connection lost — retrying… (attempt N)" + auto-retry; (b) `tmux.reconnecting` → "Reconnecting to tmux…" (amber, transient); (c) `tmux.disconnected` → "tmux disconnected — waiting for tmux" + manual Reconnect; (d) `command.error`/timeout/`server.error` → error toast with backend-verbatim message | SC1 demands WS-drop ≠ tmux events. Copy mirrors FE `PaneWorkspace.tsx:217-250` states (Connection lost / Reconnecting to {session} / Connecting) extended with the tmux- prefix for server-originated states so a human can tell which layer failed. |
| D3 | Auto-reconnect backoff ports FE `BACKOFF = [250,500,1000,2000,5000,10_000]ms` verbatim (`websocket.ts:27`), guarded by the existing per-tab generation counter | Same timing the Electron app exhibits → parity by construction; generation guard reuses the proven STATE-04 discipline so stale retry timers die on rename/close/re-resolve. Backend monitor uses the same ladder (`monitor.go:332-333`), so both layers agree. |
| D4 | Dist layout `dist/tmux-gui-windows-x64/` (`webtmux.exe` + `tmux-gui-server.exe` + README) and `dist/tmux-gui-linux-x64/` (`webtmux` + `tmux-gui-server` + README), plain zip/tarball; NO installer (MSI/deb/AppImage) | Verbatim port of the web-term GPUI precedent (`package-windows.ps1`, `package-linux.sh`). `bundle.rs` already resolves exactly this layout (adjacent-to-exe precedence). No fonts/assets copy needed — JetBrains Mono is `include_bytes!` compiled in (`main.rs:43`). |
| D5 | Makefile target names: `desktop-gpui` (release build), `dev-gpui` (cargo run), `package-gpui-windows`, `package-gpui-linux`, wired into `help` | Follows the existing `desktop-gtk` / `package-gtk-windows` naming + OS-dispatch (`HOST_OS`/`EXE_EXT`) pattern. Linux packaging documents "run on Linux" (no cross-build claim). |
| D6 | Parity audit output = checklist table (pass/fail + screenshot ref + deviation link) covering the 5 daily-driver scenarios × all folded HV advisories; single allowed pre-existing deviation = sidebar binary snap | SC4 + brief requirement. All other historical deviations (SelectSessionView trigger override, pane.join D8, DWM dark frame, ctrl-only palette) are recorded as accepted context, not re-argued. |
| D7 | Phase 6 deferred chrome-threading (preset tokens into pane grid/headers/menus/dialogs) is audit-gated fix work: audit first under default-dark + 2 non-default themes; thread tokens only where the audit shows visible divergence, using the established 55-read per-render pattern | Keeps Phase 7 requirements-pure (STATE-03/PKG-02/PKG-03 only) while honoring the 06-VERIFICATION §10 deferral honestly. |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Reconnect banner + toast overlay rendering | GPUI client (`webtmux` views) | — | Pure view of `AppState` transport/error state; no backend change (milestone rule) |
| Transport health detection (WS-drop vs tmux events) | GPUI client (`backend-client` pump + `AppState::apply_event`) | Go backend (event origin) | Backend already emits the right events (`hub.go:115-118`); the split happens client-side |
| Auto-reconnect scheduling | GPUI client (`AppState`, TOKIO_RT timers) | — | FE precedent (`websocket.ts`) is client-side; generation guard lives in `AppState` |
| Dist bundle assembly | Build scripts (`desktop-gpui/scripts/`) | Makefile (entry points) | No runtime component; sidecar discovery via existing `bundle.rs` |
| Parity audit | Human UAT (checklist) | Headless tests (regression guard only) | GPUI is not headless-renderable (Phases 2/4/5 precedent) — pixels need eyes |

## Standard Stack

### Core (no new packages — all pins already in `desktop-gpui/Cargo.toml`)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `gpui` (`gpui-pre`) | `=0.3.3` | App render loop, overlay toast stack, banner views | Milestone-locked pin (PROJECT.md); all chrome is hand-rolled divs on it [VERIFIED: desktop-gpui/Cargo.toml:33] |
| `webtmux-backend-client` | workspace | `TransportState`, `apply_event` input events, `connect_session_with_pending` for retries | Existing pump + guard machinery is the reconnect foundation [VERIFIED: crates/backend-client/src/ws.rs:146-154,367-372] |
| `tokio` (rt-multi-thread, time) | `=1.53.1` | Backoff timers for auto-reconnect, connect tasks off the GPUI thread | Already powers `ensure_session_socket` via `TOKIO_RT` [VERIFIED: Cargo.toml:35-42; app_state.rs:3205-3215] |
| `webtmux-supervisor` | workspace | Sidecar spawn for packaged-app smoke test | Adjacent-backend layout is what the smoke test validates [VERIFIED: crates/supervisor/src/lib.rs:25-39] |

### Supporting (scripts / OS surface)

| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| `package-gpui-windows.ps1` (new, port of web-term `package-windows.ps1`) | n/a | 3-step Windows bundle: Go backend → `cargo build --release` (fxc bootstrap) → assemble `dist/tmux-gui-windows-x64` | PKG-02 Windows |
| `package-gpui-linux.sh` (new, port of web-term `package-linux.sh`) | n/a | Same 3 steps → `dist/tmux-gui-linux-x64` | PKG-02 Linux (run on Linux) |
| Makefile `HOST_OS`/`EXE_EXT` dispatch | existing | `desktop-gpui` / `dev-gpui` / `package-gpui-*` targets | PKG-03 [VERIFIED: Makefile:12-43] |
| MSYS2/winget tmux + `tmux -V` probe | present | Live backend for smoke + audit | Smoke/audit need a real tmux [VERIFIED: `C:\msys64\usr\bin\tmux.exe` exists] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled toast overlay (D1) | `gpui_component::notification` (`Notification::error(..)` + `Root::push_notification`) | Richer (autohide/dedupe/actions built in [VERIFIED: registry `gpui-component-0.6.0/src/notification.rs` + `root.rs` `push_notification`]) but requires adopting gpui-component `Root` as the window root view — a render-tree replumb with milestone-closing risk. Revisit post-v1.0. |
| Plain dist zip (D4) | MSI / deb / AppImage / `linuxdeploy` | GTK precedent uses `linuxdeploy`+deb (`desktop-gtk/scripts/build-linux.sh:21-43`), but that serves GTK's `.so`/schema payload. GPUI precedent (web-term) is plain dist dir; installers add signing/repo overhead with zero SC requirement. |
| Client auto-reconnect (D3) | Rely on manual Reconnect button only | FE has BOTH (`scheduleReconnect` auto + `reconnectSession` manual, `websocket.ts:126-132` + `PaneWorkspace.tsx:31-35`). Manual-only would be a visible parity regression. |

**Installation:** none — zero new packages. Packaging scripts use only `go`, `cargo`, `rustc` (all present).

**Version verification:** n/a (no new packages). Existing pins confirmed in `desktop-gpui/Cargo.toml` (gpui `=0.3.3`, gpui-component `=0.6.0`, tokio `=1.53.1`) [VERIFIED: desktop-gpui/Cargo.toml:24-55].

## Package Legitimacy Audit

Not required — this phase installs **zero** external packages. All Rust crates are pre-pinned workspace deps (lockfile committed since Phase 1); packaging scripts invoke only first-party code + system toolchains. `Cargo.toml`/`Cargo.lock` must remain untouched (planner: add a gate asserting empty `git diff Cargo.toml Cargo.lock`, per the Phase 5/6 precedent).

**Packages removed due to SLOP:** none. **Flagged SUS:** none.

## Architecture Patterns

### System Architecture Diagram (STATE-03 event flow)

```
tmux control-mode loss                    backend process death / net drop
        │                                                    │
        ▼                                                    ▼
Go monitor backoff ──► hub relay ──► WS frame         TCP close / connect fail
250ms..10s ladder        hub.go:115-118                    (transport layer)
(monitor.go:623-673)            │                                │
        │                       │                                │
        ▼                       ▼                                ▼
 tmux.reconnecting      tmux.disconnected              GPUI read pump synthesizes
 server event            server event                  LOCAL transport-lost signal  ← D2 split (new)
        │                       │                                │
        └───────────┬───────────┘                                │
                    ▼                                            ▼
        AppState::apply_event ──► transport=Reconnecting/  transport=Disconnected
        (guard layers a/b/c)      last_error untouched     + auto-reconnect timer
                    │                       (FE BACKOFF ladder, generation-guarded)
                    ▼                                            │
        banner (persistent state) ◄──────────────────────────────┘
        toast (transitions: reconnected ✓ / command.error ✗ / server.error ✗)
```

A reader traces the primary use case: kill `tmux` server → monitor broadcasts `EvReconnecting` → hub sends `tmux.reconnecting` → pump forwards → `apply_event` sets `Reconnecting` → amber banner; monitor re-establishes → `EvReady` → `refreshSnapshot` → `state.delta` → `apply_event` sets `Connected` + `recapture_session` (existing D6) → banner clears + "Reconnected" toast. Kill the *backend process* instead → socket closes → pump emits local transport-lost (NOT `tmux.disconnected`) → "Connection lost — retrying" banner + BACKOFF retry loop + manual Reconnect button.

### Recommended Project Structure (new files only; everything else exists)

```
desktop-gpui/
├── scripts/
│   ├── package-gpui-windows.ps1   # NEW: port of web-term package-windows.ps1
│   └── package-gpui-linux.sh      # NEW: port of web-term package-linux.sh
├── crates/webtmux/src/
│   ├──app_state.rs                # EXTEND: toast queue, reconnect timers, transport-lost signal
│   └── views/
│       ├── reconnect_banner.rs    # NEW: banner view (4-way taxonomy copy)
│       └── toasts.rs              # NEW: bounded toast overlay view
└── crates/webtmux/tests/
    ├── reconnect_test.rs          # NEW: transport mapping + taxonomy unit contracts
    └── toast_test.rs              # NEW: queue cap/dedupe/autohide-model contracts
dist/                              # NEW (gitignored): tmux-gui-windows-x64/, tmux-gui-linux-x64/
```

### Pattern 1: Split transport-lost from tmux-health at the pump boundary
**What:** The read pump's close/error arms (`ws.rs:437-443,485-491`) currently forward `WsOutgoing { msg_type: EV_TMUX_DISCONNECTED }` [VERIFIED]. Introduce a distinct local signal (e.g. a new `WsPumpEvent` variant or an internal `EV_TRANSPORT_LOST` handled only in `apply_event`, never serialized) so `TransportState::Disconnected` from a dead socket is distinguishable from server-sent `tmux.disconnected`.
**When to use:** Always — this is the prerequisite for every SC1 copy string.
**Example:**
```rust
// Current conflation (desktop-gpui/crates/backend-client/src/ws.rs:437-443):
forward(WsOutgoing {
    msg_type: EV_TMUX_DISCONNECTED.to_string(), // ← also used by the real server event (hub.go:118)
    session: Some(read_session.clone()),
    ..Default::default()
});
// Target: a pump-local variant the server can never send, e.g.
// forward(WsOutgoing { msg_type: EV_TRANSPORT_LOST /* "transport.lost" */, ... })
// with apply_event mapping it to Disconnected + arming the BACKOFF retry,
// while EV_TMUX_DISCONNECTED maps to Disconnected WITHOUT auto-retry (manual Reconnect).
```

### Pattern 2: FE-parity auto-reconnect with generation guard
**What:** Port `BACKOFF = [250, 500, 1000, 2000, 5000, 10000]` (`fe/src/lib/websocket.ts:27` [VERIFIED]) as a `cx.spawn` timer chain on `TOKIO_RT`; capture `(session, generation)` at arm time and drop the retry when `entry.generation != captured` (rename/close/re-resolve bumps generation — `app_state.rs:829,3196` [VERIFIED]). On success the existing `ensure_session_socket` outcome path runs (which already re-captures panes via D6 `recapture_session`).
**When to use:** Transport-lost only (never for server-sent `tmux.disconnected` — the *backend monitor* owns that retry, `monitor.go:623-673`).

### Pattern 3: Web-term 3-step bundle assembly
**What:** Port verbatim: `[1/3]` build Go backend into dist (`go build -ldflags "-s -w"`), `[2/3]` `cargo build --release` (Windows: bootstrap `tools/fxc` + `GPUI_FXC_PATH` first), `[3/3]` copy app exe adjacent + write README + list contents [VERIFIED against `E:\Coding Stuff\web-term\desktop-gpui\scripts\package-windows.ps1` + `package-linux.sh` read this session]. Binary names swap `webterm→webtmux`, `backend→tmux-gui-server` (bundle.rs contract [VERIFIED: crates/webtmux/src/bundle.rs]).
**When to use:** Both OS scripts; keep the `[1/3]` skip-if-exists flag (`-SkipBackend`) for fast iteration.

### Anti-Patterns to Avoid
- **Reusing `EV_TMUX_DISCONNECTED` for socket close:** the exact conflation SC1 forbids; a reviewer must be able to `grep` zero server-event constants in the pump's close arms.
- **Retrying server-sent `tmux.disconnected` client-side:** double-retry (client socket churn + monitor reconnect) — the monitor already retries with the same ladder.
- **Unbounded toast growth:** reconnect flaps (250ms ladder) can enqueue dozens of toasts; cap (5) + dedupe-by-key + autohide, else the overlay becomes the bug.
- **Blocking the GPUI thread on connect:** `connect_async` must stay on `TOKIO_RT` (existing `ensure_session_socket` pattern, `app_state.rs:3205-3215`) — never `await` it in a `cx` closure (Pitfall 6 class).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Backend discovery in packaged app | New path probing / registry keys / installer-set env | Existing `resolve_backend_path` (settings override → adjacent-to-exe → dev candidates) | Already implements the packaged layout; it's pure and headless-testable (`resolve_backend_path_with_current_exe`) [VERIFIED: crates/webtmux/src/bundle.rs] |
| Reconnect timing | New backoff constants | FE `BACKOFF` ladder verbatim (both layers already agree: FE `websocket.ts:27`, monitor `monitor.go:332-333`) | Parity by construction; divergent ladders cause flap-vs-patience mismatches |
| Command correlation/timeout | New timeout plumbing | Existing 10s correlated `send_command` + `note_session_error` (matches FE `COMMAND_TIMEOUT_MS=10_000`, `commands.ts:17`) | Only the *surface* (toast) is new, not the mechanism |
| Installer/packaging framework | MSI/WiX/deb/AppImage/electron-builder-for-GPUI | Plain dist dir + zip (web-term precedent) | Zero SC requirement; signing/repo overhead; reversible later |
| Toast primitives | Custom animation engine / `Root` adoption | Hand-rolled static overlay queue (D1) | GPUI has no CSS transitions (sidebar-snap precedent); `Root` adoption replumbs the render tree |

**Key insight:** Phase 7 is an integration-and-surface phase: every mechanism it needs already exists (events, guards, backoff ladders, bundle resolver, packaging precedent). The only new state is *presentation* (banner/toast views + a transport-lost bit) plus *scripts*. Plans that invent mechanisms will collide with proven ones.

## Backend Source Truth (for planner + executor — all read this session)

- Event constants: `command.error`, `tmux.disconnected`, `tmux.reconnecting`, `server.error` [VERIFIED: be/internal/realtime/protocol.go:86-97].
- Relay: `EvReconnecting→tmux.reconnecting`, `EvDisconnected→tmux.disconnected`, `EvCommandResult(err)→command.error{requestId,message}` [VERIFIED: be/internal/realtime/hub.go:100-121]. Note: relayed tmux events carry **no `session` tag** — the GPUI triple guard treats absent-tag as belonging (`app_state.rs:865-869`), which is correct for own-socket delivery.
- Monitor retry: `backoff = 250ms,500ms,1s,2s,5s,10s`, broadcasts `EvReconnecting` per attempt, `EvReady` + full snapshot refresh on success [VERIFIED: be/internal/tmux/monitor.go:332-341,623-680].
- FE transport model: per-session `TransportState = connecting|connected|reconnecting|disconnected` (`tmuxStore.ts:13`); WS-close → `reconnecting` + auto-retry (`websocket.ts:126-132`); `tmux.reconnecting/disconnected` server frames → same two states (`websocket.ts:161-166` — FE itself conflates; GPUI must do *better* per SC1, hence D2). FE reconnect UX = auto-retry + manual `reconnectSession` (close+ensure) button in the `disconnected` empty state (`PaneWorkspace.tsx:31-35,217-233`).
- FE error surface: `runCommand` 10s timeout (`commands.ts:17-19`) + `toast.error` at ~13 call sites (palette, panes, windows, sessions, binary); success toasts (`Session created/renamed/killed`, `Window created/renamed/closed`, `Pane renamed`) are the toast-copy precedent.
- GPUI current state: `TransportState::{Connecting,Connected,Reconnecting,Disconnected}` stored per tab since Phase 3 (`ws.rs:146-154`); `apply_event` handles `EV_TMUX_DISCONNECTED/RECONNECTING/SERVER_ERROR` (`app_state.rs:925-936`) but **no view reads `transport` for a banner** (only `session_states.rs` workspace routing; grep shows zero banner/toast views) and **no auto-retry exists** (`ensure_session_socket` Err arm sets `Disconnected + last_error`, no timer — `app_state.rs:3288-3294`). Command errors go to `last_error` inline (toolbar line `window_toolbar.rs:71-75,163-164`, dialog inline errors), never toasts.

## Deferred Chrome Item (Phase 6 → Phase 7 audit gate)

Pane grid/headers, context menus, kill/rename dialogs still read hardcoded `rgb(0x…)` literals (e.g. `pane_grid.rs:72`, `pane_context_menu.rs:238`) instead of preset tokens [VERIFIED: 06-VERIFICATION.md §4/§10]. The fix pattern is established (55 per-render preset reads across 5 chrome files). Planner: make the parity audit run theme-swap scenarios under **default-dark + 2 non-default themes** (one light, one saturated dark); file threading tasks only for surfaces the audit flags as visibly divergent. Do NOT pre-schedule a blind re-theme of all files.

## Parity Audit Checklist (SC4 — folds ALL deferred HV advisories)

Notation: source phase-HV → checklist row. Accepted deviations are recorded once in § Accepted Deviation Registry and are NOT re-audited.

### A. Daily-driver scenarios (SC4 verbatim)

| # | Scenario | Procedure | Pass bar | Folded HV items |
|---|----------|-----------|----------|-----------------|
| A1 | vim/htop visual parity | Same workload side-by-side Electron vs GPUI; enter/exit alt-screen, syntax colors, cursor | Indistinguishable grids; no doubled history after tab switch/zoom/layout change; no blank flash | Ph4 HV-1; Ph5 HV-1 (T-layouts + zoom fill) |
| A2 | Rapid session switching under churn | 2+ sessions open; churn tmux windows in background; fast-switch tabs; kill last tab; kill unopened session | Never shows wrong-session windows; neighbor activation; last-kill routes to workspace pages | Ph3 HV-4; Ph2 HV-5 (ghost-row/poll race); Ph6 HV-8 (empty-tree palette guards) |
| A3 | Resize flood | Drag window edges continuously; fling dividers fast past the 4px handle toward OS edge; watch tmux logs | One resize per settle (~100ms); no rewrap mid-drag; layout-key timers settle 150/325ms; no capture storms | Ph4 HV-5; Ph5 HV-2 |
| A4 | Rename mid-session | Rename session/window/pane via menus while session streams; empty-name gate; duplicate-name error; unopened-target rename | Prefill/focus/Enter correct; success migrates sidebar+tabs, no ghost windows; errors inline, tab untouched | Ph3 HV-3; Ph5 HV-4 (dialog half) |
| A5 | Theme swap mid-session | Click several Appearance cards incl. light↔dark flips while panes stream; restart with non-default theme | Chrome + all terminals converge per pick, no mixed-preset flash; restart restores preset + geometry | Ph6 HV-1, HV-2, HV-3 |

### B. Folded HV residuals (grouped; each becomes a checklist row)

- **Sidebar/tree (Ph2 HV-1, HV-5; Ph3 HV-2):** 240px header/buttons/chevrons/indent/`{n}w` badges; toggle snaps 240↔0 with no torn widths under rapid clicks; menus stay open across 1.5s poll ticks; CLI-created sessions land ≤~1.5s; no ghost rows.
- **Dialogs (Ph2 HV-2, HV-3; Ph5 HV-4):** Create dialog autofocus/typing/Browse-cancel-keeps-cwd/pick-populates/submit-selects/duplicate-inline-error; double-submit creates exactly one session; all rename dialogs prefill + Enter-submit + empty-gated.
- **State pages (Ph2 HV-4; Ph1 HV-1):** Empty/Error/SelectSession copy/layout match Electron; Retry re-fires poll and recovers; startup shows Starting, never flashes Error.
- **Terminal I/O (Ph4 HV-2, HV-3, HV-4):** Typing/special-keys echo; Ctrl+C interrupt vs copy routing; wheel pages TUI / scrolls shell (fresh-install default ON); burst ≤~3 pages; select→copy→paste round-trip incl. CJK/emoji; JetBrains Mono glyphs, no fallback.
- **Workspace ops (Ph5 HV-3, HV-5; Ph6 HV-7):** Every pane/tab menu row acts (split/zoom/swap/break/kill/layout/move/create/Plus); kill gates honor switches (off→direct, on→verbatim dialog, no restart); TUI switch flips wheel behavior immediately; header hover affordance (tooltip fallback).
- **Settings/palette/font (Ph6 HV-3..HV-8; Ph1 HV-3, HV-4):** Card grid + filter counts; pref apply without wrap drift; binary bad-path verbatim error / good-path `Using {binary} ({version})` / stored-path apply-once with `Checking…`; palette opens from terminal focus, filters both groups, Esc clears-then-closes, focus sane after; title-bar controls + drag; geometry persists across restart.
- **Failed-backend loop (Ph1 HV-2):** Kill backend → Failed page with reason + working Retry/Quit (also exercises the STATE-03 transport-lost path at the supervisor layer).

### Accepted Deviation Registry (record, do not re-argue)

1. Sidebar collapse is a binary snap 240px↔0px, no slide animation (v1.0 deviation, STATE.md; CSS transitions have no GPUI equivalent).
2. SelectSessionView has NO Create-dialog trigger (user-accepted override 2026-09-06, 02-VERIFICATION.md frontmatter; triggers = Plus + EmptyState CTA).
3. `pane.join` absent — no backend WS route, no Electron surface (D8 backend gap, 05-VERIFICATION.md §9; needs server-side route, outside milestone).
4. Light chrome under force-dark DWM frame (Windows `DwmSetWindowAttribute` in `main.rs:63-84`; Ph6 Pitfall 6).
5. Palette binding ctrl-only; macOS Cmd+Shift+P explicitly non-goal (06-02-SUMMARY.md D-decision).
6. EXTRA-01/02/03 deferred to v2.

## Common Pitfalls

### Pitfall 1: WS-drop vs tmux-event conflation (THE Phase 7 trap)
**What goes wrong:** Banner says "tmux disconnected" when actually the local socket died (or vice versa); auto-retry fights the backend monitor.
**Why it happens:** `ws.rs:437-443,485-491` synthesizes `EV_TMUX_DISCONNECTED` for transport failures — the same constant the server sends for tmux loss [VERIFIED]. FE has the same conflation (`websocket.ts:129-130` vs `:161-166`).
**How to avoid:** D2 split as the FIRST STATE-03 task; gate it with a unit test asserting close-synthesis ≠ server constant; grep-gate "zero `EV_TMUX_DISCONNECTED` in pump close arms" in review.
**Warning signs:** Any plan that renders banner copy directly from `TransportState` without an origin bit.

### Pitfall 2: Retry storms / stuck retries
**What goes wrong:** Reconnect loop spams `connect_async` ( FD / log flood) or a zombie timer reconnects a renamed/closed tab.
**Why it happens:** No backoff cap, or timer not bound to generation.
**How to avoid:** D3 — verbatim BACKOFF ladder + generation capture + drop-on-mismatch; cap attempts messaging (banner shows attempt N, timer keeps ladder at 10s tail like FE `BACKOFF[5]`).
**Warning signs:** `tokio::time::sleep` with a literal not traceable to the BACKOFF const.

### Pitfall 3: Toast flood on flappy connections
**What goes wrong:** 250ms-ladder flaps enqueue unbounded toasts covering the workspace.
**Why it happens:** Toast pushed per transition without dedupe/cap.
**How to avoid:** Queue cap 5 (drop-oldest), dedupe key per (kind+session), autohide 5s errors / 3s success [ASSUMED timings — FE sonner defaults are the reference, tune in UAT].
**Warning signs:** `push_toast` call sites without a corresponding queue-cap test.

### Pitfall 4: Release build without `GPUI_FXC_PATH`
**What goes wrong:** Opaque shader-compile failure on Windows `--release`; dev works, package script fails.
**Why it happens:** `gpui-pre-windows` compiles HLSL only in release [VERIFIED: BUILDING.md:44-45,55-67].
**How to avoid:** Port the fxc-bootstrap block verbatim (compile `tools/fxc/main.rs` via `rustc` if `GPUI_FXC_PATH` unset); fail fast with a clear message when `rustc` itself is missing.
**Warning signs:** Package script invoking `cargo build --release` without a preceding fxc check.

### Pitfall 5: Linux bundle built-but-dead (missing runtime `.so`)
**What goes wrong:** `webtmux` builds on the dev box (dev headers present) but exits on a clean target (no `libwayland-client`, `libxkbcommon`, GL).
**Why it happens:** BUILDING.md lists *build-time* headers; runtime closure is undeclared [VERIFIED: BUILDING.md:21-38].
**How to avoid:** Script prints `ldd` closure into the bundle README (or a `RUNTIME-DEPS.txt`); audit checklist includes "launch on clean Ubuntu/VM" row; document `apt install` runtime line. No `linuxdeploy` (D4).
**Warning signs:** A Linux smoke test that only runs on the build machine.

### Pitfall 6: PowerShell/bash quoting in Makefile + scripts
**What goes wrong:** Recipes that work in one shell break in the other (`2>/dev/null`, `;` chaining under `cmd.exe` — the 06-02 SUMMARY hit exactly this: `E:\dev\null` created).
**Why it happens:** Windows targets run under `cmd.exe` (`Makefile:30-31` [VERIFIED]).
**How to avoid:** Windows recipes delegate to `powershell -NoProfile -ExecutionPolicy Bypass -File ...` (existing `desktop-gtk`/`package-gtk-windows` pattern, `Makefile:82,99-100`); keep logic in `.ps1`/`.sh`, keep Makefile to one-line invocations.
**Warning signs:** Multi-line shell logic inline in a Makefile recipe.

### Pitfall 7: Cross-OS packaging hubris
**What goes wrong:** Plan claims "build both bundles on Windows CI" with no cross toolchain.
**Why it happens:** Assuming `cargo build` targets Linux from Windows by default.
**How to avoid:** D5 documents Linux packaging runs on Linux; planner adds an Open Question (cross via `--target x86_64-unknown-linux-gnu` + zig/musl is explicitly OUT — [ASSUMED] out of scope unless user asks).
**Warning signs:** A `package-gpui-linux` recipe without a host-OS guard/message.

## Code Examples

### Reconnect banner view shape (FE copy precedent)
```tsx
// FE source of truth for banner copy — fe/src/features/panes/PaneWorkspace.tsx:217-250:
transport === 'disconnected' → "Connection lost" / "The connection to {session} dropped. Reconnect to keep working." + [Reconnect]
transport === 'reconnecting' → "Reconnecting to {session}..."
// GPUI port (D2): same strings for transport-lost; tmux-originated states add the tmux- prefix:
//   tmux.reconnecting → "Reconnecting to tmux…"  |  tmux.disconnected → "tmux disconnected — waiting for tmux"
```

### Correlated-error → toast (FE call-site pattern, 13 sites)
```ts
// fe/src/lib/commands.ts:17-19 + FE call-site shape (e.g. WindowTabs.tsx:59-61):
const COMMAND_TIMEOUT_MS = 10_000;                       // GPUI: Duration::from_secs(10) — already parity
try { await runCommand(() => sock.paneKill(id)); toast.success('…'); }
catch (e) { toast.error((e as Error).message); }         // ← GPUI toast.push(Error, backend-verbatim msg)
// GPUI equivalent source: CommandResult.message via note_session_error paths (app_state.rs:2563-2785)
```

### Bundle layout + resolver contract (testable headless)
```rust
// desktop-gpui/crates/webtmux/src/bundle.rs — precedence:
// 1. settings.backend_path override → 2. <current_exe-dir>/tmux-gui-server[.exe] if exists → 3. dev candidates
// => dist layout MUST be:  dist/tmux-gui-windows-x64/{webtmux.exe, tmux-gui-server.exe, README.txt}
//                          dist/tmux-gui-linux-x64/{webtmux, tmux-gui-server, README.txt}
// Headless contract test: resolve_backend_path_with_current_exe(&settings, Some(fake_dir)) with a
// tempdir containing a fake tmux-gui-server[.exe] → returns the adjacent path (no display, no build).
```

### Makefile target shape (existing OS-dispatch pattern)
```make
# Follow desktop-gtk (Makefile:80-88) + package-gtk-windows (:99-100):
ifeq ($(HOST_OS),windows)
desktop-gpui:
	powershell -NoProfile -ExecutionPolicy Bypass -File "desktop-gpui/scripts/build-gpui.ps1"
package-gpui-windows:
	powershell -NoProfile -ExecutionPolicy Bypass -File "desktop-gpui/scripts/package-gpui-windows.ps1"
else
desktop-gpui:
	cargo build --release --manifest-path desktop-gpui/Cargo.toml --package webtmux
package-gpui-linux:
	./desktop-gpui/scripts/package-gpui-linux.sh
endif
dev-gpui:
	cargo run --manifest-path desktop-gpui/Cargo.toml -p webtmux
# + help lines (Makefile:47-56 pattern). Linux packaging on Windows prints "run on Linux" (Pitfall 7).
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| FE conflates WS-close and `tmux.reconnecting` into one `reconnecting` state (`websocket.ts:129-130` + `:161-166`) | Phase 7 mandates strict split (D2) — GPUI must exceed FE here | This phase (SC1) | Banner/toast copy can name the failing layer; no retry-fighting |
| Errors inline-only in GPUI (`last_error` toolbar line, dialog error lines) | Errors ALSO surface as toasts (SC1); inline lines stay (dialogs keep them per Ph2/Ph5 UAT) | This phase | Matches FE's 13-site `toast.error` behavior |
| web-term GPUI `notification: Option<String>` single banner | Bounded multi-toast queue (cap 5, dedupe, autohide) | This phase (D1) | Flap-safe; single-banner would thrash on 250ms-ladder flaps |
| Manual-only `cargo build` + ad-hoc scripts | `make desktop-gpui / dev-gpui / package-gpui-*` in repo workflow | This phase (PKG-03) | Discoverable, CI-ready entry points on both OSes |

**Deprecated/outdated:**
- `desktop/resources/` sidecar staging for GPUI: that path serves Electron (`COPY_ELECTRON_ASSETS`, Makefile:37-43). GPUI uses adjacent-to-exe (`bundle.rs`); do not copy GPUI binaries into `desktop/resources`.
- `linuxdeploy`/AppImage/deb for the GPUI app: GTK-specific precedent, not applicable (D4).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Toast autohide timings (5s errors / 3s success) are UAT-tunable guesses, not FE-verified values | Pitfall 3 | Low — cosmetic; audit adjusts |
| A2 | Plain zip/tarball distribution is acceptable (no installer, no signing, no repo metadata) | D4 | Medium — if user expects MSI/deb, packaging plans under-deliver; confirm at discuss-gate if one runs |
| A3 | Linux cross-build from Windows is out of scope; Linux packaging runs on Linux | Pitfall 7 / D5 | Medium — if no Linux box/CI exists, PKG-02 Linux half is unverifiable; needs a human with Linux |
| A4 | `Root`-less hand-rolled overlay composes correctly with the existing root `div` + palette overlay stacking | D1 | Low — palette overlay precedent (`app_state.rs:3379-3381`) shows `.when()` stacking works |
| A5 | `ldd`-closure note + runtime `apt` line suffices for Linux launch (no bundled `.so`) | Pitfall 5 | Medium — clean-distro smoke test in audit either confirms or escalates to bundling `.so` |
| A6 | Backend emits no NEW event types for Phase 7 (milestone no-backend-change rule holds) | Backend Truth | Low — all four events verified in `protocol.go` + `hub.go`; monitor retry verified |

## Open Questions

1. **Where does the Linux bundle get built/smoked?**
   - What we know: This box is Windows-only; no WSL/Linux CI observed in repo (no `.github` workflows checked — planner should confirm).
   - What's unclear: Whether the user has a Linux machine/VM or expects CI to cover it.
   - Recommendation: Planner marks Linux build + Linux smoke as `human_needed` items with the exact commands; do not block Windows-side plans on them.
2. **Should `server.error` be a toast, a banner, both, or log-only?**
   - What we know: FE only `console.error`s it (`sockets.ts:54-56`); `apply_event` stores it in `last_error`.
   - What's unclear: No FE visible-surface precedent.
   - Recommendation: Error toast (consistent with SC1 "command errors surface as toasts"; `server.error` is command-adjacent). Cheap, reversible.
3. **Reconnect attempt cap?**
   - What we know: FE retries forever on the 10s tail (`BACKOFF[5]`); backend monitor retries forever too.
   - What's unclear: Whether infinite retry is desired for a desktop app (battery/log noise) vs cap + manual Reconnect.
   - Recommendation: Match FE (infinite, 10s tail) for parity; banner always offers manual Reconnect + shows attempt count. Revisit post-v1.0.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo/rustc | Build + tests | ✓ | 1.96.0 | — |
| go | Backend sidecar build | ✓ | go1.23.4 | — |
| tmux (MSYS2) | Smoke + audit live backend | ✓ | `C:\msys64\usr\bin\tmux.exe` | — |
| `rustc`-built `tools/fxc` path | Windows `--release` packaging | ✓ (source present; build in script) | — | Debug bundle for iteration |
| Linux host / VM | PKG-02 Linux build + smoke | ✗ | — | Human/CI item (Open Q1) |
| `linuxdeploy` | — | ✗ (n/a) | — | Not needed (D4) |
| MSVC toolchain | Build (BUILDING.md says MSVC) | ⚠️ sysroot shows `stable-x86_64-pc-windows-gnu` | 1.96.0-gnu | Prior phases built+tested green on this toolchain — scripts must NOT assume MSVC (`cl.exe`); use installed toolchain as-is |

**Missing dependencies with no fallback:**
- Linux build/smoke environment — planner must route to human (cannot be automated from this box).

**Missing dependencies with fallback:** none.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` workspace (`desktop-gpui/Cargo.toml`) |
| Config file | none — see Wave 0 (no new harness needed) |
| Quick run command | `cargo test -p webtmux --test reconnect_test` / `--test toast_test` (plus `--test ws_guard_test` regression) |
| Full suite command | `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` (expect 116 + new; keep 0 failed) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| STATE-03 | Pump close-synthesis ≠ server `tmux.disconnected` constant; `apply_event` maps each to distinct state+retry-arm | unit | `cargo test -p webtmux --test reconnect_test` | ❌ Wave 0 |
| STATE-03 | Toast queue cap/dedupe/autohide-model; taxonomy copy keys | unit | `cargo test -p webtmux --test toast_test` | ❌ Wave 0 |
| STATE-03 | Banner/toast pixels, kill-tmux vs kill-backend drill, flap behavior | manual-only | Launch checklist §A + §B rows | n/a (GPUI not headless-renderable — Ph2/4/5 precedent) |
| PKG-02 | Adjacent-backend resolution with fake exe dir | unit | `cargo test -p webtmux --test bundle_test` (or extend existing) | ❌ Wave 0 |
| PKG-02 | Bundles launch and reach session (both OSes) | manual-only | `dist/tmux-gui-*/webtmux[.exe]` smoke | n/a (Linux half → human) |
| PKG-03 | `make desktop-gpui/dev-gpui/package-gpui-*` run green | manual-only | `make <target>` per-OS | n/a |

### Sampling Rate
- **Per task commit:** quick run command for the touched test file + `git diff --stat Cargo.toml Cargo.lock` empty (zero-new-package gate).
- **Per wave merge:** full suite command.
- **Phase gate:** Full suite green before `/gsd-verify-work`; parity checklist table fully ticked.

### Wave 0 Gaps
- [ ] `crates/webtmux/tests/reconnect_test.rs` — transport-lost vs tmux-event mapping, BACKOFF const, generation-guarded retry-arm model
- [ ] `crates/webtmux/tests/toast_test.rs` — queue cap/dedupe/copy-key model
- [ ] `crates/webtmux/tests/bundle_test.rs` (or fit into existing suite) — `resolve_backend_path_with_current_exe` adjacency with tempdir fake exe
- [ ] Scripts themselves: `desktop-gpui/scripts/package-gpui-windows.ps1`, `package-gpui-linux.sh` (planner Task 0 — port, then Windows dry-run `-SkipBackend` fast path)

## Security Domain

No `security_enforcement` key in `.planning/config.json` → treated as enabled. Assessment is brief: Phase 7 adds no trust-boundary changes.

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Loopback-only sidecar, no auth surface (unchanged) |
| V3 Session Management | no | No session tokens; tmux names only (unchanged) |
| V4 Access Control | no | Localhost single-user (unchanged) |
| V5 Input Validation | yes (unchanged) | Existing `validate_session_name` (Go byte-parity) + opaque `tmux_binary` string (never spawned client-side); toast/banner render backend-verbatim error strings as *text* — executor must not interpret them as markup/commands |
| V6 Cryptography | no | None in scope; `reqwest/rustls` untouched |

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Backend-verbatim error text rendered in toast/banner | Spoofing (a malicious tmux error string mimics app chrome) | Render as plain text element, never HTML/markup; prefix toasts with kind icon (Error vs Info) so origin is visually typed |
| Dist bundle swaps `tmux-gui-server` beside exe (binary planting) | Tampering | `bundle.rs` `settings.backend_path` override lets paranoid users pin an absolute path; README notes expected sibling filenames; no auto-update/download channel added |

## Sources

### Primary (HIGH confidence — source read this session, claim checkable at cited lines)
- `be/internal/realtime/protocol.go:58-97` — WS message/event constants incl. `tmux.disconnected/reconnecting`, `command.error`, `server.error`
- `be/internal/realtime/hub.go:100-121` — monitor→WS relay mapping (no session tag on tmux events)
- `be/internal/tmux/monitor.go:332-341,623-680` — 250ms..10s backoff ladder, EvReconnecting broadcast, EvReady + snapshot refresh
- `fe/src/lib/websocket.ts:27,126-132,161-166` — FE BACKOFF ladder, auto-reconnect, server-event dispatch (and its conflation)
- `fe/src/lib/sockets.ts:23-61` — per-session handler → transport mapping; `server.error` log-only
- `fe/src/lib/commands.ts:17-19` — 10s command timeout precedent
- `fe/src/lib/protocol.ts:70-81` — FE event-name table
- `fe/src/features/panes/PaneWorkspace.tsx:28-35,217-250` — manual reconnect helper + banner copy precedent
- `fe/src/stores/tmuxStore.ts:13` — FE TransportState union
- `desktop-gpui/crates/backend-client/src/ws.rs:46-48,146-154,437-443,485-491` — event consts, TransportState, close-arm conflation (Pitfall 1 evidence)
- `desktop-gpui/crates/webtmux/src/app_state.rs:865-869,925-936,3288-3294,3379-3381` — guard absent-tag rule, transport mapping, no-retry Err arm, overlay stacking precedent
- `desktop-gpui/crates/webtmux/src/bundle.rs` — full adjacent-exe resolution contract (packaging target)
- `desktop-gpui/crates/webtmux/src/main.rs:43` — `include_bytes!` font (no asset-copy step needed)
- `desktop-gpui/crates/webtmux/src/views/window_toolbar.rs:71-75,163-164` — inline `last_error` surface (toast predecessor)
- `desktop-gpui/Cargo.toml:24-55` — exact pins (gpui 0.3.3, gpui-component 0.6.0, tokio 1.53.1)
- `Makefile:12-43,80-100` — HOST_OS dispatch, EXE_EXT, gtk/powershell recipe patterns, help format
- `desktop-gpui/docs/BUILDING.md:21-67` — Linux dev headers, Windows fxc/`GPUI_FXC_PATH` release requirement
- `E:\Coding Stuff\web-term\desktop-gpui\scripts\package-windows.ps1` + `package-linux.sh` — 3-step bundle precedent (full files read)
- Vendored `gpui-component-0.6.0/src/notification.rs` + `src/root.rs` (`push_notification`, `Notification::info/success/warning/error`) — alternative assessed and deferred (D1)
- Toolchain probes this session: `cargo 1.96.0`, `go1.23.4`, `C:\msys64\usr\bin\tmux.exe` present; sysroot `stable-x86_64-pc-windows-gnu`
- All six prior VERIFICATION.md + 02/06 SUMMARYs (HV fold-in source; 116/116 baseline)

### Secondary (MEDIUM confidence)
- None — no web sources were needed; every load-bearing claim is primary.

### Tertiary (LOW confidence — tagged [ASSUMED] inline)
- A1–A6 in Assumptions Log (toast timings, installer acceptability, Linux cross-build scope, overlay stacking, `.so` closure sufficiency, no new backend events).

## Metadata

**Methodology note:** The `gsd-tools` research-plan/classify-confidence seam is not installed in this environment (no `gsd_run` on PATH, no `gsd-core/bin`), so provider routing and tiering were done manually: every HIGH claim cites an in-repo file+line range read this session with values quoted verbatim; anything from training knowledge or unverifiable-here (Linux behavior) is tagged `[ASSUMED]` and mirrored in the Assumptions Log.

**Confidence breakdown:**
- Standard Stack: HIGH — all versions/pins read from `Cargo.toml`; scripts read in full from the web-term reference.
- Architecture: HIGH — event flow traced end-to-end (monitor → hub → pump → apply_event → views) with line citations; the one design gap (conflation) is evidenced, not inferred.
- Pitfalls: HIGH for 1/2/4/6 (sourced), MEDIUM for 5/7 (Linux-unverifiable here — flagged as human items, not facts).

**Research date:** 2026-09-07
**Valid until:** 2026-10-07 (stable domain: pinned pre-1.0 deps, local backend, no fast-moving ecosystem surface)
