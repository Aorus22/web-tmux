# Phase 1: Workspace Foundation & Backend Sidecar - Research

**Researched:** 2026-09-06
**Domain:** Rust GPUI workspace skeleton + Go sidecar spawn/handshake + startup-path UI + window/settings persistence (brownfield port of the proven `web-term/desktop-gpui` reference)
**Confidence:** HIGH — every mechanic claim below was verified this session by reading the two local codebases and the vendored pinned crate sources; external doc metadata (Context7) is MEDIUM and always corroborated by the working reference.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- `desktop-gpui/` at repo root, 5-crate workspace mirroring web-term (`supervisor`, `settings`, `backend-client`, `terminal`, app crate named `webtmux`)
- All five crates created now; `backend-client`/`terminal` as compilable stubs filled by later phases
- Workspace Cargo.toml copies web-term pins verbatim: gpui-pre =0.3.3, gpui-component =0.6.0, alacritty_terminal =0.25.1, tokio =1.53.1, reqwest 0.12.28 (rustls), tokio-tungstenite 0.26.2, flume, parking_lot, dirs; app crate adds `gpui-platform = { package = "gpui-pre-platform", version = "=0.3.3" }`; Cargo.lock committed
- fxc HLSL tool ported from web-term `tools/fxc` (Windows release builds only; set via GPUI_FXC_PATH)
- Locate `tmux-gui-server(.exe)` adjacent to the running executable; repo-root fallback for dev runs; env override for tests
- Spawn with `--port 0`; parse `BACKEND_PORT:<n>` from stdout (Electron desktop/main.js handshake parity); probe `/api/health` for readiness
- Strip `TMUX`/`TMUX_PANE` from the child environment; drop web-term-only spawn args (encryption key, db path)
- BackendStatus::{Starting, Ready, Failed{reason, stderr_tail}}; 10s port timeout → Failed (Electron parity)
- Starting page mirrors Electron: centered spinner + "Starting backend…" muted text; Failed page shows reason + redacted stderr tail + working Retry (re-spawned supervisor) and Quit
- Phase 1 title bar is minimal: drag region + custom min/max/restore/close; full tab strip/gear arrives Phases 3/6 (staged to keep phase boundaries clean)
- Port web-term `window_state.rs` verbatim (restore at boot, observe + save on move/resize/maximize)
- Port the Windows DWM dark-frame block from web-term `main.rs` verbatim now (LOW cost, avoids later dark-frame bugs)
- Port webterm-settings crate pattern: JSON settings in the platform config dir via `dirs`, corrupt-file `.bak` recovery, defaults on first run
- Phase 1 fields: window geometry + theme placeholder; tmux/terminal preference fields arrive with the Phase 6 data model

### the agent's Discretion
None declared beyond the locked decisions; phase boundary is fully pinned by 01-CONTEXT.md + approved 01-UI-SPEC.md (visuals locked — do not re-research).

### Deferred Ideas (OUT OF SCOPE)
- Session-tab persistence across restarts (Electron restores nothing) — EXTRA-01, v2
- Session-level keyboard shortcuts — EXTRA-02, v2

**Scope fences (from CONTEXT.md `<domain>`):** In: workspace skeleton (5 crates, exact web-term pins, committed Cargo.lock), supervisor spawn pipeline, status UI, window controls, settings/window-state persistence, Windows (incl. release fxc) + Linux build readability. Out: REST/WS clients (Phases 2–3), terminal engine (Phase 4), pane grid (Phase 5), theme/settings content + palette (Phase 6), package bundles (Phase 7).
</user_constraints>

## Project Constraints (from AGENTS.md)

- No AGENTS.md exists at the repo root or in `.opencode/` (verified by filesystem check this session) — no in-repo directives.
- Global editor directive (AGENTS.md at harness level): long-running processes (≥1 minute, e.g. `go build` warmups, full `cargo build`s) should be run under tmux at execution time. Applies to the executor's dev loop, not to app code.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SHELL-02 | User can minimize/maximize/restore/close via the custom title-bar window controls | Exact GPUI wiring verified from web-term `tab_strip.rs:285-349` + vendored gpui-pre 0.3.3 APIs: `window.minimize_window()`, `window_state::toggle_maximize` (`IsZoomed`/`ShowWindowAsync`), `window.remove_window()`, drag via `.window_control_area(WindowControlArea::Drag)` → `HTCAPTION` (native drag + double-click + snap free) |
| SET-05 | Settings persist across restarts (webterm settings-crate pattern) | Full `DesktopSettings`/`paths` API port verified (web-term `crates/settings/src/lib.rs` + `paths.rs`), `.bak` corrupt-file recovery quoted, injectable base-dir test seam verified in `store_test.rs` |
| STATE-01 | "Starting backend…" while the sidecar warms up; failed-backend page with reason + Retry/Quit after the 10s port timeout | Spawn handshake contract fully verified (`BACKEND_PORT:%d` at be/cmd/server/main.go:61; config env names at be/internal/config/config.go:56-61); supervisor lifecycle port deltas enumerated; Starting/Failed/Ready view structure from web-term `views/status.rs` per approved 01-UI-SPEC |
| PKG-01 | `cargo build` works on Windows (debug without fxc; release with the ported fxc tool) and Linux (dev-package prerequisites documented) | Vendored `gpui-pre-windows-0.3.3/build.rs` verified: shader compilation gated at `#[cfg(all(target_os = "windows", not(debug_assertions)))]`, `GPUI_FXC_PATH` read first, SDK fallback, panic without either; web-term `tools/fxc` (302-line zero-crate tool) is the port source; env audited (rustc 1.96.0, go 1.23.4, tmux present) |
</phase_requirements>

## Summary

Phase 1 is a **near-verbatim port of a proven reference**, and every mechanism the planner needs has now been read line-by-line on both sides this session. The web-term `desktop-gpui` workspace (5 crates: supervisor / settings / backend-client / terminal / `webterm` app) already proved, on this exact pinned stack, everything Phase 1 needs: sidecar spawn on a dynamic port with a stdout `BACKEND_PORT:<n>` handshake, readiness probing, a `Starting → Ready / Failed{reason, stderr_tail}` status surface, custom frameless window controls with DWM dark-frame bootstrap, and JSON settings with `.bak` recovery. The port surface concentrates in ~6 files plus the workspace manifest; the crate set adds **zero new dependencies** beyond the mirrored pins.

The single most important research deliverable is a **factual correction to the spawn contract**: `01-CONTEXT.md` says "Spawn with `--port 0`", but the web-tmux Go backend reads configuration **only from environment variables** (`be/internal/config/config.go` uses `os.Getenv` exclusively — there is no `flag` package anywhere in `be/`), and the Electron paragon spawns with **an empty argv** and `TMUXGUI_PORT=0` as an env var. A supervisor that passes `--port 0` as argv would be silently ignored and the backend would bind its default port **4090** (`getenvInt("TMUXGUI_PORT", 4090)`), colliding with dev servers and violating the dynamic-port decision. The plan must spawn with env `TMUXGUI_PORT=0` (+ `TMUXGUI_HOST=127.0.0.1`, resolved `TMUXGUI_TMUX_BIN`), argv empty, and strip `TMUX`/`TMUX_PANE` from the child env.

The second notable adaptation: web-tmux's `BackendStatus` has **no `Crashed` variant** (approved UI-SPEC S3: "`BackendStatus` for web-tmux is `Starting | Ready | Failed { reason, stderr_tail }` (early-exit folds into `reason`, e.g. `backend exited early (code 1)`)"), unlike web-term's 4-variant enum; the supervisor's post-ready exit monitor must map to `Failed` instead of `Crashed`. `BackendStatus` with those exact three variants is a locked CONTEXT decision and has been quoted verbatim from `01-CONTEXT.md` this session.

**Primary recommendation:** fork web-term's workspace manifest and the four Phase-1 load-bearing files (`supervisor/lib.rs`, `settings/{lib,paths}.rs`, app `main.rs`, `window_state.rs`, `views/status.rs`) with the documented deltas (env-var spawn table, `/api/health` probe, 3-variant `BackendStatus`, 10s timeouts, pattern-based stderr redaction, `tmux-gui-server` binary candidates, `tmux-gui-desktop` config dir), commit `Cargo.lock` first, and build the vertical slice in the reference's exact order: workspace skeleton → supervisor → settings → window shell with status pages → fxc tooling + build docs.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Sidecar process lifecycle (spawn, `BACKEND_PORT:<n>` handshake, `/api/health` readiness, exit watch, stop) | Supervisor crate (tokio-only, GPUI-free) | — | web-term keeps this crate gpui-free so it is headless-testable in CI (verified crate doc-header, supervisor/lib.rs:1-4) |
| Backend status → UI propagation | App crate `Entity<AppState>` (watch→mpsc→`cx.spawn` pump) | Supervisor `tokio::sync::watch` | GPUI entities are main-thread-only; the channel-back pump pattern is the established bridge (app_state.rs:1766-1852) |
| Startup-path views (S1 title bar, S2 starting, S3 failed, S4 ready body) | App crate (free-function views) | gpui-component Theme bridge | 01-UI-SPEC is the approved visual contract; web-term `views/status.rs` is the interaction reference |
| Custom window controls + drag + maximize toggle | App crate window APIs + `window_state.rs` | Windows DWM (dark frame) | OS-level hit-testing (HTCAPTION) comes from `WindowControlArea`; buttons are own handlers (verified in vendored platform source) |
| Window/settings persistence (.bak recovery) | Settings crate (JSON via `dirs`) | App crate `on_window_should_close` flush | Pure-Rust, injectable-base, headless-tested (settings/tests/store_test.rs) |
| fxc shader tooling (Windows release) | Build tooling (`tools/fxc`) | packaging script (Phase 7) | Vendored `gpui-pre-windows-0.3.3/build.rs` requires fxc.exe only in release builds; `GPUI_FXC_PATH` is the export seam |

## Standard Stack

### Core
The workspace manifest is **copied verbatim** from `E:\Coding Stuff\web-term\desktop-gpui\Cargo.toml` (read this session; the ecosystem-level verify of the same pin set against live crates.io happened 2026-09-06 in `.planning/research/STACK.md`, and `package-legitimacy check` re-hit the registry today). Exact pins — quote from web-term Cargo.toml:30-56:

```toml
# Exact pins — see .planning/phases/20-desktop-foundation-backend-integration/20-RESEARCH.md
# gpui is pre-1.0 with real breaking-change churn; every core dep is pinned exactly
# and Cargo.lock is committed (plan 20-01 Task 1).
gpui = { package = "gpui-pre", version = "=0.3.3" }
gpui-component = "=0.6.0"
tokio = { version = "=1.53.1", features = ["rt-multi-thread","process","macros","sync","time","net"] }
serde = { version = "=1.0.229", features = ["derive"] }
serde_json = "=1.0.151"
reqwest = { version = "=0.12.28", default-features = false, features = ["json","multipart","rustls-tls"] }
dirs = "=6.0.0"
thiserror = "=2.0.20"
parking_lot = "=0.12.5"
rand = "=0.10.2"
tempfile = "=3.27.0"
libc = "=0.2.189"
flume = "=0.12.0"
alacritty_terminal = "=0.25.1"
tokio-tungstenite = { version = "=0.26.2", default-features = false, features = ["connect", "handshake"] }
futures-util = "=0.3.32"
anyhow = "=1.0.104"
```

App crate delta (webterm/Cargo.toml:18-29, verbatim): `gpui-platform = { package = "gpui-pre-platform", version = "=0.3.3" }` and `raw-window-handle = "=0.6.2"`; dev-deps `tokio-tungstenite`, `futures-util`, `tempfile`.

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `gpui-pre` (aliased `gpui`) | `=0.3.3` | GPU UI framework: window, views, svg, animation | Snapshot fork that `gpui-component 0.6.0` binds to; proven 1:1 on Windows+Linux in web-term [VERIFIED: web-term Cargo.lock + this machine's vendored `gpui-pre-0.3.3`] |
| `gpui-pre-platform` | `=0.3.3` | Platform impl for `Application::with_platform(gpui_platform::current_platform(false))` | Easily missed; REQUIRED for the app crate entry (vendored gpui-pre-platform-0.3.3/src/gpui_platform.rs:57) |
| `gpui-component` | `=0.6.0` | `gpui_component::init(cx)`, `Theme::change`, Spinner | Stock widgets for later phases; Phase 1 uses init + Theme tokens (+ optionally Spinner) |
| `tokio` | `=1.53.1` (features above) | Supervisor runtime + process mgmt | `tokio::process::Command` `kill_on_drop` backstop; `.enter()` guard pattern |
| `reqwest` | `=0.12.28` rustls | `/api/health` readiness probe (+ `adopt_or_clear`) | rustls avoids OpenSSL on Linux |
| `parking_lot`, `dirs`, `thiserror`, `serde*`, `rand`, `tempfile`(dev), `libc`(dev, unix-only) | per pin | supervisor tail buffer, settings paths, typed errors, DTOs, first-run settings, tests | Reference glue set |
| `raw-window-handle` | `=0.6.2` | HWND for DWM attrs + `IsZoomed`/`ShowWindowAsync` | App crate only |

Internals this phase: supervisor and settings are **real**; `backend-client`/`terminal` are compilable stubs (empty lib + minimal Cargo.toml) per locked decision.

Crate naming (discretion — mirror web-term's prefix rule): workspace members `crates/{supervisor,settings,backend-client,terminal,webtmux}` with package names `webtmux-supervisor`, `webtmux-settings`, `webtmux-backend-client`, `webtmux-terminal`, `webtmux`; workspace version `0.1.0`, edition 2021, `resolver = "2"`, `[profile.release] lto = "thin"`. [ASSUMED A2]

### Installation
```bash
# inside web-tmux repo root; Go backend binary already exists (see Environment)
cd desktop-gpui && cargo build                      # debug — no fxc needed
cargo test -p webtmux-supervisor -p webtmux-settings # headless suites
```

### Alternatives Considered
None — all stack decisions are locked in 01-CONTEXT.md ("copies web-term pins verbatim"). The only non-port choices (recommended in this doc): settings dir name, crate package names, `windows_subsystem` adoption timing.

## Package Legitimacy Audit

> Gate run via `gsd_run query package-legitimacy check --ecosystem crates` on 2026-09-06 against the live registry. The names themselves are **not** products of web search or training recall in this session: they were read from the reference implementation's committed `Cargo.lock` and this machine's resolved registry cache (`~/.cargo/registry/src/index.crates.io-*/gpui-pre-0.3.3`, `gpui-component-0.6.0` present and previously compiled).

| Package | Registry | Verdict | Signals | Disposition |
|---------|----------|---------|---------|-------------|
| gpui-pre | crates | **SUS** (too-new 0.3.3 pub 2026-09-03, ~271 dl/wk; repo zed-industries/zed) | exists, not deprecated, no postinstall | **Approved — reference-locked.** SUS signals are expected for a niche fork snapshot; the name came from the proven web-term lockfile + working builds (not search), and the vendored source is on this machine. No checkpoint needed; it is a locked CONTEXT decision. |
| gpui-pre-platform | crates | **SUS** (same profile) | exists | Same disposition as gpui-pre. |
| gpui-component | crates | OK | 7 yrs / 3,687 dl/wk / longbridge/gpui-kit | Approved |
| alacritty_terminal | crates | OK | 6 yrs, 60k/wk, alacritty/alacritty | Approved |
| tokio, reqwest, tokio-tungstenite, flume, parking_lot, dirs, raw-window-handle, serde, serde_json, thiserror, rand, tempfile, libc | crates | OK (each) | mature, official repos | Approved |
| directory-1.x / directory crate — NOT used | — | — | — | Use `dirs =6.0.0` only (locked) |

**Packages removed due to [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** `gpui-pre`, `gpui-pre-platform` — flagged **and accepted with rationale** (reference-locked, exactly-pinned names from the proven implementation, vendored copies verified on this machine). If the planner wants extra assurance, a single confirmation that the built binary launches (first debug build task) economically covers both. No `checkpoint:human-verify` task is required because the CONTEXT decision to copy these pins is already user-ratified; the planner should still record the SUS note in the risk table.

## Architecture Patterns

### System Architecture Diagram

```
 main()  [webtmux app crate]
   │ TOKIO_RT.enter()                      (LazyLock<Runtime>, multi-thread enable_all)
   │ DesktopSettings::load()               (JSON %APPDATA%|~/.config ),
   │ bundle::resolve_backend_path()        (settings override → beside exe → repo-root/dev candidates)
   │ window_state::restore(settings)       (initial WindowBounds incl. Maximized)
   ▼
 Application::with_platform(current_platform(false)).run(closure)
   │ add_fonts(JetBrains Mono TTF)  →  gpui_component::init(cx)  →  theme placeholder apply
   │ open_window(TitlebarOptions{appears_transparent,title:"Tmux GUI"}, DWM-for-HWND)
   │     └─ Entity<AppState> → start_supervisor(cx)
   ▼                                (tokio task)
 Supervisor::spawn(SpawnOptions)
   ├─ spawn child: argv EMPTY, env TMUXGUI_HOST=127.0.0.1 · TMUXGUI_PORT=0 ·
   │               TMUXGUI_TMUX_BIN=<resolved>; TMUX/TMUX_PANE stripped
   │               stdin null, stdout+stderr piped, kill_on_drop(true)
   ├─ read stdout lines → parse "BACKEND_PORT:<n>" ──(10s timeout)──┐
   │                                                                │ timeout /
   │                                                                ▼ stdin EOF w/o token / early exit
   │                                                    BackendStatus::Failed{reason, stderr_tail}
   ├─ poll GET /api/health @250ms (per-req 1s timeout; success⇒ any <500) ──> BackendStatus::Ready
   ├─ stderr ring tail → 2000-char newest-wins buffer (10 lines shown on Failed page)
   └─ exit monitor (100ms tick) ── child gone ──> Failed{"backend exited early (code N)"}
   ▼ watch channel → mpsc → GPUI cx.spawn pump → AppState.backend_status + cx.notify()
   ├─ S2 Starting page ──▶ S3 Failed page (Retry re-runs start_supervisor; Quit → stop+cx.quit)
   └─ Ready → S4 Ready shell (client handle stored as base_url for Phase 2)
```

External service: `tmux-gui-server` (Go) — UNCHANGED; binds `BACKEND_PORT:%d\n` on stdout after listening (be/cmd/server/main.go:61 `[VERIFIED...]`), serves `GET /api/health`.

### Pattern 1: Sidecar spawn + `BACKEND_PORT:<n>` handshake (the corrected contract)

**What:** GPUI app spawns the Go backend as a child with env-only knobs, learns the loopback port from its single stdout line, probes readiness, supervises until exit.

**Spawn contract (CORRECTION of one wording in the locked decision):**
`01-CONTEXT.md` says "Spawn with `--port 0`", but the backend has **no argv parsing at all** — verification: `be/` contains no `flag.`/`os.Args` use (grep this session), and `config.Load()` reads only environment variables. The Electron paragon spawns with empty argv ([VERIFIED: desktop/main.js:142] `backend = spawn(bin, [], { env, stdio: ['ignore', 'pipe', 'pipe'] })`):

| Aspect | web-term (reference) | web-tmux (Phase 1 port) | Provenance |
|--------|----------------------|--------------------------|------------|
| Port binding | `WEBTERM_PORT=":0"` env | **`TMUXGUI_PORT=0` env — NOT argv `--port 0`** | [VERIFIED: be/internal/config/config.go:40,56-61] verbatim: `"TMUXGUI_PORT", 4090` default + `Host: getenv("TMUXGUI_HOST", "127.0.0.1"), Port: port, TmuxSocket: os.Getenv("TMUXGUI_TMUX_SOCKET"), LogLevel: strings.ToLower(getenv("TMUXGUI_LOG_LEVEL", "info")), ScrollbackLines: scrollback`; Electron env block [VERIFIED: desktop/main.js:131-135] `TMUXGUI_HOST: '127.0.0.1', TMUXGUI_PORT: isDev ? '9001' : '0', TMUXGUI_TMUX_BIN: resolveTmuxBinary()` |
| Handshake token | `BACKEND_PORT:<n>` on stdout | same token, same regex `/BACKEND_PORT:(\d+)/` ([VERIFIED: desktop/main.js:147]; print site [VERIFIED: be/cmd/server/main.go:61] `fmt.Printf("BACKEND_PORT:%d\n", ln.Addr().(*net.TCPAddr).Port)`) | web-term supervisor/parser reused byte-for-byte |
| Child env hygiene | — | delete `TMUX` and `TMUX_PANE` from child env ([VERIFIED: desktop/main.js:140-141] `delete env.TMUX` / `delete env.TMUX_PANE`; comment at 136-139 explains: otherwise every backend `tmux` call attaches to the launcher's tmux session) | Electron parity; already in CONTEXT |
| Secrets | 64-hex `WEBTERM_ENCRYPTION_KEY` validated | none — drop `validate_hex_key`, drop `db_path` | [VERIFIED: be/internal/config/config.go:56-62] — config struct has no key/db fields |
| Readiness probe | `GET /api/settings` | **`GET /api/health`** | [VERIFIED: be/internal/server/router.go:27] `mux.HandleFunc("GET /api/health", health.Handle)`; response is JSON `{"status":"ok","tmux":{"installed":bool,"version":str}}` and **always HTTP 200** ([VERIFIED: be/internal/server/health.go:36-42]) |
| tmux binary resolution | — | `TMUXGUI_TMUX_BIN` passed by the supervisor, resolved Electron-style: explicit env → `%LOCALAPPDATA%\Microsoft\WinGet\Links\tmux.exe` → `"tmux"` ([VERIFIED: desktop/main.js:108-121]); the backend also re-resolves and honors POST-override later ([VERIFIED: be/internal/tmux/binary.go:13-15] "An explicit TMUXGUI_TMUX_BIN value wins; otherwise the platform resolver follows PATH") | keep desktop-side resolve for Electron parity |
| Env override (tests/dev) | — | `SPAWN_OPTS`-style env override (CONTEXT: "env override for tests") | test harness injects `TEST_BACKEND_PATH` (web-term supervisor/tests/integration.rs:6-11) |

**Lifecycle mechanics to port verbatim from web-term `supervisor/lib.rs` (all verified this session):**
- Status plumbing: `tokio::sync::watch` channel + `subscribe()`; initial `BackendStatus::Starting`; `spawn()` is the single async entry that owns all failure transitions.
- Child stdio: `stdin(null)`, `stdout(piped)`, `stderr(piped)`, `kill_on_drop(true)` (lib.rs:120-123); no secret in argv — web-tmux has no secret, keep the argv-clean invariant anyway (spawn args table test).
- Stderr ring buffer: `MAX_STDERR_TAIL_CHARS = 2000` newest-wins (lib.rs:14, 232-248). Keep.
- Handshake: async read-loop with `parse_handshake_line` (lib.rs:147-155); timeout wraps the handshake future (lib.rs:263-289). Failure reason strings: "Backend handshake missing: …" / "Backend handshake timed out after …".
- Spawn failure (bad path): immediate `Failed{reason: "Failed to spawn backend binary '<path>': <io error>", stderr_tail: "Execution error: ..."}` (lib.rs:211-225).
- Readiness probe loop: 250 ms cadence, per-request 1 s timeout, **success if `resp.status().is_success() || status.as_u16() < 500`** (lib.rs:321-340) — keep exactly (`/api/health` is a plain 200; but the `<500` clause costs nothing and survives backend hiccups).
- Early-exit detection inside the probe loop (lib.rs:325-327) — keep.
- `stop(grace)`: unix `SIGTERM` → wait grace → hard kill; Windows falls through to hard kill (lib.rs:394-417). Keep crate-internal.
- Child exit monitor: 100 ms tick (lib.rs:359-383) — **change** the post-ready exit mapping (see delta D2).
- `adopt_or_clear(base_url)` (lib.rs:420-445) re-attaches a previously recorded backend after a crash — keep with the endpoint swapped to `/api/health` and settings field `last_backend_url` [ASSUMED A4 — not an explicit CONTEXT decision; enables the "killed parent, restart, don't spawn a second sidecar" behavior and covers the stale-backend security mitigation from project research].

**Supervisor deltas (complete list, each small):**
- **D1 — env table:** `env_vars()` returns `TMUXGUI_HOST=127.0.0.1`, `TMUXGUI_PORT=0` (string `"0"`), `TMUXGUI_TMUX_BIN=<resolved>`; **do not** pass `--port 0` argv (Go ignores argv; unstaffed env would bind default 4090 — [VERIFIED: config.go:40]). Also strip `TMUX`/`TMUX_PANE`: build the child env via `std::process::Command::env_remove("TMUX").env_remove("TMUX_PANE")` on the resolved command (tokio `Command` supports `env_remove`).
- **D2 — no `Crashed` variant:** exit monitor sends `Failed { reason: "backend exited early (code N)" (or "unknown"), stderr_tail }` per approved UI-SPEC S3 (quote: "BackendStatus for web-tmux is `Starting | Ready | Failed { reason, stderr_tail }` (early-exit folds into `reason, e.g. backend exited early (code 1)` per 01-CONTEXT.md; web-term's `Crashed` variant is not ported."));
- **D3 — probe endpoint:** `/api/settings` → `/api/health` (spawn readiness **and** `adopt_or_clear`);
- **D4 — timeouts:** web-term defaults 15 s/15 s → web-tmux `handshake_timeout = 10s`, `readiness_timeout = 10s` [ASSUMED A1 — CONTEXT/UI-SPEC lock "10s port timeout"; extending 10s to the readiness phase keeps the whole startup inside the FE's proven 10s budget; both remain overridable builder fields as in web-term];
- **D5 — `SpawnOptions::new(backend_path)` single-arg** (drop `db_path`/`encryption_key` args + their error variants); keep `with_allowed_origins`? NO — web-tmux backend has no origin env ([VERIFIED: config.go:56-62] — only Host/Port/TmuxSocket/LogLevel/ScrollbackLines). Drop it.
- **D6 — stdout drain after handshake** (improvement, cheap): web-term drops the stdout reader when the handshake future completes (pipe read-end closes). tmux-gui-server logs only to stderr (`slog` → os.Stderr, [VERIFIED: be/cmd/server/main.go:89]), so dropping it is observable-safe today, but any later stdout write would EPIPE into the child. After the port parse, move the reader into a `tokio::spawn` drain-to-null task (matches the drain note already in web-tmux ARCHITECTURE.md integration gotchas).
- **D7 — binary name:** `backend[.exe]` → `tmux-gui-server[.exe]`.

**Backend-side failure topology the tests must cover** ([VERIFIED: be/cmd/server/main.go:37-40]): with tmux not installed/busy, the backend **exits(1) BEFORE printing `BACKEND_PORT`** (`"Tmux is not installed."` on stderr) — the supervisor's handshake read sees EOF-or-timeout and the Failed page shows that stderr line. This is the natural manual test for the tmux-missing case (STATE-01) — and the reason the Failed page must show a **redacted tail of stderr**, not just the reason line.

### Pattern 2: Settings crate — exact API surface to port

Port web-term `crates/settings` (read this session: lib.rs 234 lines + paths.rs 48 lines). Full structure with deltas:

| web-term item | Port action | Notes |
|---|---|---|
| `SettingsError{Io, Json, InvalidKey}` | keep Io/Json, **drop InvalidKey** | no key handling |
| `Theme{Dark,Light,System}` enum + serde lowercase | keep verbatim | placeholder theme (CONTEXT), defaults Dark |
| `WindowState{x:Option<i32>,y:Option<i32>,width:Option<u32>,height:Option<u32>,maximized:bool}` | **port byte-for-byte** | consumed by `window_state.rs` |
| `SavedSessionTab` + `open_sessions` | **drop** | EXTRA-01 is v2-scope; keep the JSON robust (serde default) |
| `DesktopSettings` fields | keep: `backend_path Option<PathBuf>`, `theme: Theme`, `window_state Option<WindowState>`, `last_backend_url Option<String>`; **drop** `encryption_key`, `theme_preset/theme_mode_filter/font_*/cursor_*/scrollback` (Phase 6 data model per CONTEXT) | `#[serde(default)]` on every field keeps old files forward-compatible |
| `custom_base: Option<PathBuf>` + `#[serde(skip)]` | keep | the load/save injectable-base test seam |
| `load()/load_from(&Path)` | keep, **quote the recovery semantics**: on parse error → `let bak_path = file_path.with_extension("json.bak"); let _ = fs::rename(&file_path, &bak_path);` then regenerate defaults and immediately `save_to` (lib.rs:164-175) | success criteria: corrupt file recovers via .bak |
| `save()/save_to()` | keep (pretty JSON, `[cfg(unix)]` 0600 perms block) | Windows privacy documented as profile-ACL |
| `ensure_encryption_key()` + `is_valid_64_hex` | **drop** | no secret custody in web-tmux |
| `paths.rs` | port with `config_dir_with_base = base.join("tmux-gui-desktop")` [ASSUMED A3]; **drop `db_path*` fns** | distinct from the GTK app's `tmux-gui-gtk` ([VERIFIED: desktop-gtk/internal/backend/manager.go:172-182] `filepath.Join(base, "tmux-gui-gtk")`) and from web-term's `webterm-desktop` |

Verification-vs-plan tests to carry over from `settings/tests/store_test.rs` (verified): roundtrip persist, corrupt-file recovery (asserts `settings.json.bak` exists + defaults returned), first-run defaults. Drop the key-stability test.

### Pattern 3: Window bootstrap (`main.rs` port) + window controls + drag

Bootstrap order is load-bearing (web-term main.rs:15-100, read verbatim this session):

1. `let _tokio_guard = webtmux::app_state::TOKIO_RT.enter();` — **before** anything else (the LazyLock runtime `new_multi_thread().enable_all()` is in app_state.rs:710-715).
2. `DesktopSettings::load().unwrap_or_default()` → resolve frontend path via `resolve_backend_path(&settings)` (bundle.rs port; candidates below) → build `SpawnOptions::new(backend_path)` [adjusted mid-handshake for web-tmux deltas].
3. `window_state::restore(&settings)` → initial `WindowBounds` (defaults fallback: `WindowBounds::Windowed` at GUI default 1200×800 / min 800×500 — web-tmux `desktop/main.js:184-188`, **not** web-term's 1160×660 — UI-SPEC A5).
4. `Application::with_platform(gpui_platform::current_platform(false)).run(move |cx: &mut App| { ... })` — NOT bare `Application::run` (gpui-pre app.rs:178 `pub fn with_platform(platform: Rc<dyn Platform>) -> Self`).
5. Inside run closure, in order: `include_bytes!` JetBrains Mono → `cx.text_system().add_fonts(vec![Cow::Borrowed(bytes)])` (**before any text shaping** — the terminal-measure/`measure_cell` garble failure mode from project PITFALLS); `gpui_component::init(cx)`; theme placeholder apply (`Theme::change(ThemeMode::Dark, None, cx)` — vendored signature `pub fn change(mode: impl Into<ThemeMode>, window: Option<&mut Window>, cx: &mut App)` at gpui-component-0.6.0/src/theme/mod.rs:237).
6. `WindowOptions { window_bounds: Some(initial_bounds), window_min_size: Some(size(px(800), px(500))), titlebar: Some(TitlebarOptions { title: Some("Tmux GUI".into()), appears_transparent: true, ..Default::default() }), ..Default::default() }`.
7. `cx.open_window(...)` callback: DWM dark-frame block (raw-window-handle → `Win32` hwnd → `DwmSetWindowAttribute(hwnd, 20, &dark, 4)` + attr **19** fallback; attrs 20/19 = `DWMWA_USE_IMMERSIVE_DARK_MODE` on Win10-20H1+/legacy — Win32 documentation values, [CITED: Microsoft Learn DWM constants, attrs corroborated by working web-term main.rs:69-90]) → `window_state::observe(window, settings_arc, cx)` → `cx.new(...)` entity → `app_state.update(cx, |this, cx| this.start_supervisor(cx))`.

**Window controls wiring (SHELL-02), exact from web-term tab_strip.rs:285-349:**
- Minimize: `.on_mouse_down(MouseButton::Left, |_, window, _| { window.minimize_window(); })` (window.rs:6212).
- Maximize/Restore: `.on_mouse_down(..., cx.listener(|this, _, window, cx| { let new_max = crate::window_state::toggle_maximize(window); this.is_maximized = new_max; cx.notify(); }))` — icon (`Square`/`Copy` per UI-SPEC) bound to the `is_maximized` field, restored at boot from `settings.window_state.maximized` (app_state.rs:796).
- Close: `.on_mouse_down(MouseButton::Left, |_, window, _| { window.remove_window(); })` (window.rs:2187). The single-window app exits when the window closes; sidecar death is the `kill_on_drop` backstop (see Pitfall P6 for the orphan-on-hard-exit nuance; UI-SPEC also assigns explicit kill-then-quit to the Failed page's Quit button).
- Drag + double-click + Aero snap: **no custom code needed** — paint the bar's background/flex-1 area with `.window_control_area(WindowControlArea::Drag)` (div.rs:731); gpui-pre-windows maps it to `HTCAPTION` when `is_movable` (vendored events.rs:972-980), which yields native drag, double-click-maximize, and snap. This is how web-term gets its "double-click toggle" without a double-click handler (verified: no `on_double`/`double_click` in web-term sources). UI-SPEC's phrase "window.start_window_move()" is superseded by the API the reference actually uses — the planner should spec `WindowControlArea::Drag`.
- **Icon-vs-actual-state gap:** web-term tracks maximized only via the button (`is_maximized`) so OS-initiated maximize (drag-to-top snap, HTCAPTION double-click) leaves the icon stale. UI-SPEC S1 requires the icon set to swap "by actual window state via `IsZoomed`". Cheapest compliant mechanism: at title-bar render time call `window_state::is_window_maximized(window)` (exists and is import-window-safe in web-term window_state.rs:18-32) instead of the stale field — a per-render `IsZoomed` hit; register as a Phase-1 verification item, with the `is_maximized` boot-restore as the initial value. [ASSUMED A1a — per-render Win32 call is cheap but unverified on the paint path; fallback is web-term's field approach + accept stale-on-snap.]

**window_state.rs port (verbatim mechanics, window_state.rs read this session):**
- `restore(settings) -> Option<WindowBounds>`: legacy-size guards (1200×800/940×560 → default), clamps width `1000..1920` / height `560..1200`, degenerate 0×0 rejection, minimized-coordinate rejection (`x/y <= -1000` legacy, `-10000` in extract), `state.maximized → WindowBounds::Maximized(bounds)`. **For web-tmux, re-base these numbers on the pure-function part; keep the guard structure and 0/negative checks, drop web-term's legacy 1200×800/940×560 special-cases only if they reference web-term history** (they are harmless defaults; keeping them byte-for-byte is the lower-risk "verbatim" choice, but the FE defaults differ — decision surface for the planner; recommendation: port structure, adapt clamps to `800..{screen}`-sane values and keep maximized behavior).
- Wait — CONTEX says port `window_state.rs` **verbatim**. Note the tension honestly: web-term's hard-coded 1200×800 "legacy" special-cases and the 1160×660 defaults are web-term history; web-tmux defaults are 1200×800/800×500 (main.js:184-188). The verbatim-plus-rebase approach: port all functions, change only the default constants (`DEFAULT_WIDTH 1200, DEFAULT_HEIGHT 800` and min sizes) and clamp bounds; keep every legacy guard as-is.
- `extract_window_state(window, prev)`: minimized coords guard, maximized → **preserve previous windowed bounds** (the restore-then-compact behavior).
- `is_window_maximized(window)`: `IsZoomed` (Win32) else `window.is_maximized()`.
- `toggle_maximize(window) -> bool`: Windows path = DWM-dark re-assert (attrs 20/19) + `ShowWindowAsync(SW_RESTORE=9 / SW_MAXIMIZE=3)` + `RedrawWindow(RDW_INVALIDATE|RDW_UPDATENOW|RDW_ALLCHILDREN)` + `on_next_frame(refresh)`, returns new state; non-Windows: `window.zoom_window()` + refresh.
- `observe(window, settings_arc, cx)`: web-term's actual implementation persists **on window close** via `window.on_window_should_close(cx, ...)` flush-save (nothing persists on move/resize ticks despite the CONTEXT wording "observe + save on move/resize/maximize" — the reference's real behavior is close-flush only; [VERIFIED window_state.rs:213-236]). Port as-is (verbatim lock wins); note UI-SPEC A/decisions call it "observe + save on move/resize/maximize" — the closest proven behavior is the close-flush; deviation note for the parity audit [ASSUMED A5: keep the reference's close-flush mechanics; saving on every move would re-derive the Electron "persisted position" contract differently and clutter disk I/O].

**Backend path resolution (bundle.rs port with new candidates):** settings override → adjacent-to-current-exe `tmux-gui-server(.exe)` → dev candidates. For web-tmux the repo-root artifact **already exists** (`make be` → `cd be && go build -trimpath -ldflags "-s -w" -o ../tmux-gui-server$(EXE_EXT) ./cmd/server`, Makefile:71-73, [VERIFIED] and `Test-Path` confirmed `tmux-gui-server.exe` at repo root + `desktop/resources/tmux-gui-server.exe` both present). Candidate chain: `tmux-gui-server(.exe)` (cwd = repo root), `../tmux-gui-server(.exe)`, `../../tmux-gui-server(.exe)` (from desktop-gpui/crates/webtmux), `desktop/resources/tmux-gui-server(.exe)`, then `desktop-gpui/test-support/tmux-gui-server(.exe)` (test-support harness parity), fallback first candidate [ASSUMED A6 — exact candidate list is discretion; the kinds of locations are from CONTEXT + Electron's `backendBinary()` (desktop/main.js:99-106) precedence].

### Pattern 4: Status pages + supervisor→UI pump

- `start_supervisor(cx)` wiring to port **1:1** (web-term app_state.rs:1766-1852, read verbatim): missing-spawn-opts → immediate `Failed{reason: "No backend spawn options configured", stderr_tail: "Please build the backend binary: …"}`; reset to `Starting`+`cx.notify()`; `TOKIO_RT.spawn` a task that builds `Supervisor`, subscribes its watch (forwarding `SupervisorEvent::Status`), awaits `supervisor.spawn(opts)` and emits `Ready(BackendInfo)`/`Failed{reason,stderr_tail}` on an unbounded mpsc; a parallel `cx.spawn`/`AsyncApp` loop `rx.recv()`s, then `cx_handle.update(|cx| { weak.upgrade() → entity.update(... set status ...; cx.notify()) })` — **the pump-break contract: failed upgrade = listener dead** (Pitfall 2 prevention, named pattern).
- On `Ready(info)` Phase-1 behavior: store `base_url` (e.g. on `AppState.client: Option<…>` — stub type OK) — Phase 2 hangs REST off it; nothing else (no tree poll yet).
- **Retry = `this.start_supervisor(cx)`** (web-term app_state.rs:3826-3831 verbatim) — the failed attempt's child was already killed inside `Supervisor::spawn`'s failure paths, so re-entry is safe; no leak.
- View routing: `match backend_status { Starting → S2, Ready → S4 shell, Failed → S3 card, }` — web-term's `render_status_page(&status, key_secret, retry_closure, cx)` pattern (aspect keyed to AppState::render `_ =>` branch at app_state.rs:3824-3834). Failed-page visuals per 01-UI-SPEC (locked): card max 700px, heading "Backend Startup Failed" (16px/500 fg), `Reason: {reason}`, ≤10 newest tail lines (empty → block omitted), Quit outline + Retry primary (`#d4d4d4` on `#1e1e1e`, hover `#c5c5c5`).
- Redaction policy (UI-SPEC A4, since web-tmux spawn env carries **no secrets** — defense-in-depth only): keep newest 10 lines; mask any line matching `(?i)(key|secret|token|password)` value patterns with `[REDACTED]` — replaces web-term's key-material-specific redactor (webterm status.rs:19-50 `redact_key_material`).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Dynamic port discovery | Polling fixed ports / port scanning / re-implementing socket probing | stdout `BACKEND_PORT:<n>` handshake (web-term `parse_handshake_line`) | The backend prints the exact port; guessing races, probes pollute; Electron uses the same handshake |
| Readiness detection | Sleep-and-hope / TCP connect tests | reqwest poll of `/api/health` @250ms with early-exit checks | The backend returns a real liveness+tmux-presence JSON; per-request timeout beats stuck sockets |
| Child shutdown | Ad-hoc `taskkill` / batch cleanup | tokio `kill_on_drop(true)` + supervisor `stop(grace)` with unix SIGTERM | Edge-typed cross-platform behavior already solved; kill_on_drop is the leak backstop |
| Window maximize/restore | Naive `window.zoom_window()` calls on Windows | `window_state::toggle_maximize` (`IsZoomed` + `ShowWindowAsync` + DWM re-assert + `RedrawWindow`) | Verified in web-term: GPUI's `zoom()` maximizes but **fails to restore** on Windows; re-entrant WM_SIZE drops paints |
| Title bar drag region | Custom mouse-drag translation logic | `.window_control_area(WindowControlArea::Drag)` | Native HTCAPTION gives drag + double-click-maximize + Aero snap and correct cursor semantics |
| Dark title bar | post-hoc theme painting | `DwmSetWindowAttribute` attrs 20/19 at window open | Win32-level; no GPUI theme path exists |
| Settings persistence | Hand-rolled config files / registry / localStorage analog | JSON + `serde` + `dirs` + `.bak` rename-on-corrupt (proven pattern) | Corrupt-file recovery, defaults-on-first-run and unix perms are already solved; the `.bak` contract is a success criterion |
| Startup-path UI | Re-specing visuals | Approved 01-UI-SPEC tokens + web-term `views/status.rs` structure | Phase 1's visual work is token application, not design |

**Key insight:** Every hard part of this phase (async process supervision, Win32 window restoration, corrupt-settings recovery) already exists in compile-verified form inside the reference; the port is primarily a faithful copy with seven documented supervisor deltas + path/name constants.

## Common Pitfalls

Phase-critical ones first, then the project-research pitfalls that Phase 1 must front-load.

### Pitfall 1 (CRITICAL, corrected in research): argv `--port 0` silently binds port 4090
**What goes wrong:** The locked decision wording says "Spawn with `--port 0`". If the executor passes it as argv, the Go backend ignores unknown argv (no flag parsing exists anywhere in `be/`) and binds `TMUXGUI_PORT` default **4090** — colliding with `make dev-web`/other frontends and defeating the dynamic-port design, with a confusing "port busy, falling back to dynamic port" warning only for explicit ports.
**Why it happens:** Electron's *comment* says "dynamic port 0" but its *code* sets `TMUXGUI_PORT: '0'` in the env.
**How to avoid:** Spawn with empty argv + env `TMUXGUI_PORT=0` (Pattern 1 table). Add a unit test asserting the resolved child command argv is EMPTY and env contains `TMUXGUI_PORT=0` (mirrors web-term's `env_only_secrets` argv assertions).
**Warning signs:** Backend URL shows `:4090`; two instances fight over 4090 in dev.

### Pitfall 2 (project Pitfall 1): a second GPUI flavor in the dependency tree
**What goes wrong:** Adding any crate whose tree contains public `gpui` 0.2.x next to `gpui-pre` 0.3.3 → incompatible `Entity/Window/Context` type worlds and duplicate platform registration panics; a lazy `cargo update` bumps a 0.x minor later and breaks the build with no local change.
**How to avoid (Phase 1 IS the prevention phase):** exact `=` pins verbatim, `Cargo.lock` committed as the *first* artifact, `cargo tree -p <pkg>` gate before adding any future dependency; stub crates get no external deps.
**Warning signs:** two gpui versions in `cargo tree`; "build works only from the old lockfile".

### Pitfall 3 (project Pitfall 2): tokio↔GPUI threading bridge
**What goes wrong:** timers/reqwest/watch from a GPUI callback hang (no runtime context) or a background pump updates entities directly → wrong-thread panic / missed repaints / pump leaks.
**How to avoid:** ONE `LazyLock<Runtime>` `.enter()`ed in `main` before GPUI starts; all supervisor/async work on `TOKIO_RT`; results via `tokio::sync::watch` + mpsc → `cx.spawn`/`AsyncApp` pump with `weak.upgrade()` fail-or-break discipline; `cx.notify()` on every state landing. `start_supervisor` (Pattern 4) IS the canonical example — plans must name it, not re-derive.
**Warning signs:** UI updates only on click; timers never fire on main; updates panic after window close.

### Pitfall 4: fxc missing on Windows release builds
**What goes wrong:** `cargo build --release` on Windows fails in `gpui-pre-windows`' build script — vendored `find_fxc_compiler()` panics with "Failed to find fxc.exe" when neither `GPUI_FXC_PATH` nor a full Windows SDK provides fxc.exe ([VERIFIED vendored build.rs:114-138]; the gate is `#[cfg(all(target_os = "windows", not(debug_assertions)))]` at build.rs:10 — debug builds compile without it).
**How to avoid:** port `tools/fxc` (zero-crate, wraps `d3dcompiler_47.dll`; README verified), compile with `rustc -O main.rs -o fxc.exe`, set `GPUI_FXC_PATH`; a real Windows SDK also satisfies it via the SDK-search fallback ([VERIFIED build.rs:134] `find_latest_windows_sdk_binary("fxc.exe")`). Debug build path must be exercised in CI *as well as* release+fxc.
**Warning signs:** `error: failed to run custom build command for gpui-pre-windows` with fxc panic; builds fine yesterday, release fails today (because someone left GPUI_FXC_PATH unset in a clean shell).

### Pitfall 5: font/UTF-8 + shaping order and DWM timing
- Register the embedded TTF `before` any text render; registering a family name different from what renderer code later requests misaligns metrics; shaping done before `add_fonts` yields tofu/fallback metrics.
- DWM attributes can only fire inside/after `open_window` (the HWND doesn't resolve earlier) — web-term's `#[cfg(target_os = "windows")]` block lives **inside the open_window callback** with `raw_window_handle`.
**Warning signs:** white flashes on maximize; tofu on Linux minimal setups.

### Pitfall 6: orphaned sidecar on hard exit (Windows)
**What goes wrong:** `kill_on_drop` fires when the tokio `Child` is dropped — via static runtime teardown at normal main exit. If the app dies hard (`std::process::exit`, crash, Task-Manager kill), Drop paths may not run and the backend (whose graceful stop needs a signal) may survive as an orphan still holding its tmux control surface; a later launch would then find `last_backend_url` serving (hence `adopt_or_clear`).
**Why it happens:** Windows has no automatic parent-death kill-by-default for detached children (no job object is created by tokio).
**How to avoid (Phase 1 scope):** `kill_on_drop(true)` (already in port) + settings `last_backend_url` + `adopt_or_clear` on a leftover-probe path (Pattern 1); keep the Failed page's Quit contract "kill sidecar if alive, then `cx.quit()`" (UI-SPEC S3 footer) — that removes the common deliberate-exit orphan case. Full job-object hardening is NOT one of Phase 1 needs. [ASSUMED A7 — mechanism nuance, validated by web-term's proven close behavior; period]
**Warning signs:** second launch re-uses a backend serving tmux paths of a dead session; OS process list shows lingering `tmux-gui-server` after crashes.

### Pitfall 7: settings-dir collision with the existing GTK desktop
**What goes wrong:** sharing `dirs::config_dir()` subfolder with `desktop-gtk` (`tmux-gui-gtk`) or with webterm (`webterm-desktop`) corrupts either app's expectations (settings.json venue clash).
**How to avoid:** distinct dir constant: `tmux-gui-desktop` recommended [ASSUMED A3]; verify at UAT that `%APPDATA%\tmux-gui-gtk` is untouched after GPUI write.
**Warning signs:** the GTK app starts with GPUI's theming; settings.json exists in `tmux-gui-gtk/`.

### Pitfall 8: stderr tail truncation semantics
**What goes wrong:** naive `tail -n` of captured stderr pages loses the NEWEST lines (page shows boot spam instead of the fatal error) — main.go fatal lines ("config: …", "Tmux is not installed.", "listen: …") can be at *any* position offset within stderr.
**How to avoid:** web-term's ring char buffer keeps newest-wins 2000 chars (drop head when exceeding vía char-skip), and the view layer keeps newest **10 lines**; iterate on lines-with-stdlib (`chars().count()` for UTF-8 safety — copy the char-count truncation, don't slice bytes).
**Warning signs:** Failed page tail shows startup logs; last fatal line missing.

### Pitfall 9: window-geometry restore flash / bad restore
**What goes wrong:** restoring bounds after `open_window` (flash at default geometry), or a corrupted geometry file launching at 0×0/off-screen.
**How to avoid:** compute `initial_bounds` BEFORE `Application::run`/`open_window` (web-term main.rs order); `restore()` applies its degenerate+out-of-range rejection before returning; on `extract_window_state` reject the minimized -10000 coordinates; clamp sizes away from 0.
**Warning signs:** default-size flash at launch; window restored invisibly off-display.

### Moderate/Minor (from project research, still Phase-1-relevant)
- Keep supervisor/settings **gpui-free** (headless `cargo test -p` in CI), `resolver = "2"`, unified `[workspace.package]`, edition 2021, `lto = "thin"` — verified manifest extras (Minor: workspace topology).
- Never `eprintln!` terminal/WS payloads (log discipline; Phase 1 surfaces only transport reasons/tails).
- `windows_subsystem = "windows"` is web-term's known wart (visible console in release) — web-tmux **may** adopt the attribute in Phase 1 or defer to Phase 7; if adopted, CI must prove the stdout handshake survives when the parent has no console (web-term's own note + STACK.md recommendation both flag the verification). Recommendation: adopt now with the dual-build CI check, or defer explicitly. [ASSUMED A6]

## Code Examples

### 1. Workspace manifest — `desktop-gpui/Cargo.toml` (delta view; full pin block under Standard Stack)
```toml
[workspace]
resolver = "2"
members = [
    "crates/supervisor",
    "crates/settings",
    "crates/backend-client",
    "crates/terminal",
    "crates/webtmux",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
authors = ["web-tmux contributors"]

[workspace.dependencies]
webtmux-supervisor      = { path = "crates/supervisor" }
webtmux-settings        = { path = "crates/settings" }
webtmux-backend-client  = { path = "crates/backend-client" }
webtmux-terminal        = { path = "crates/terminal" }
# … exact pins identical to web-term's (see Standard Stack) …

[profile.release]
lto = "thin"
```
Source: webterm workspace manifest (59 lines, read verbatim); names [ASSUMED A2].

### 2. web-tmux `SpawnOptions` (adapted from web-term lib.rs:49-126 — deltas D1/D3/D4/D5)
```rust
pub struct SpawnOptions {
    pub backend_path: PathBuf,
    /// Max time to wait for the BACKEND_PORT handshake (default 10s — STATE-01).
    pub handshake_timeout: Duration,
    /// Max time for the /api/health probe loop (default 10s — see A1).
    pub readiness_timeout: Duration,
}

impl SpawnOptions {
    pub fn new(backend_path: impl Into<PathBuf>) -> Self { /* … 10s/10s defaults … */ }

    /// Env-only knobs; NO argv (the Go backend reads env exclusively).
    pub fn env_vars(&self) -> Vec<(&'static str, String)> {
        vec![
            ("TMUXGUI_HOST", "127.0.0.1".to_string()),
            ("TMUXGUI_PORT", "0".to_string()),
            ("TMUXGUI_TMUX_BIN", Self::resolve_tmux_binary().into()),
        ]
    }

    pub fn build_command(&self) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(&self.backend_path);
        for (k, v) in self.env_vars() { cmd.env(k, v); }
        // Launched from inside a tmux session would otherwise hijack the
        // backend's tmux calls (Electron desktop/main.js:136-141 parity):
        cmd.env_remove("TMUX").env_remove("TMUX_PANE");
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd.kill_on_drop(true);
        cmd
    }
}
```
`resolve_tmux_binary` order [VERIFIED desktop/main.js:108-121]: `TMUXGUI_TMUX_BIN` env (already set) → `%LOCALAPPDATA%\Microsoft\WinGet\Links\tmux.exe` → `"tmux"`.

### 3. Handshake parsing (reuse verbatim — supervisor/lib.rs:147-155)
```rust
pub fn parse_handshake_line(line: &str) -> Option<u16> {
    if let Some(idx) = line.find("BACKEND_PORT:") {
        let rest = &line[idx + "BACKEND_PORT:".len()..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse::<u16>().ok()
    } else {
        None
    }
}
```

### 4. Readiness probe (endpoint delta D3 only)
```rust
let probe_url = format!("{}/api/health", base_url);         // was /api/settings in web-term
let client = reqwest::Client::builder().timeout(Duration::from_millis(1000)).build().unwrap_or_default();
let readiness_fut = async {
    let start = tokio::time::Instant::now();
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!("backend exited early ({})", status));   // folds into Failed (D2)
        }
        if let Ok(resp) = client.get(&probe_url).send().await {
            if resp.status().is_success() || resp.status().as_u16() < 500 { return Ok(()); }
        }
        if start.elapsed() > opts.readiness_timeout {
            return Err(format!("readiness probe timed out after {:?}", opts.readiness_timeout));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
};
```

### 5. Exit monitor → Failed mapping (delta D2; was `Crashed` in web-term)
```rust
// 100ms tick after Ready — UI-SPEC S3: no Crashed variant.
Ok(Some(exit_status)) => {
    let code = exit_status.code();
    let _ = status_tx.send(BackendStatus::Failed {
        reason: format!("backend exited early (code {})", code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".into())),
        stderr_tail: stderr_tail_snapshot(),
    });
    break;
}
```

### 6. Corrected handshake → background stdout drain (delta D6)
```rust
match tokio::time::timeout(opts.handshake_timeout, handshake_fut).await {
    Ok(Ok(port)) => {
        // keep draining stdout so a late child write can never EPIPE
        // (reader must be moved into the task, not dropped with the future)
        tokio::spawn(drain_to_null(stdout_reader));
        port
    }
    // … unchanged timeout/EOF failure paths (kill child, Failed{reason, stderr_tail}) …
}
```
web-term's version drops the reader (verified); tmux-gui-server logs to stderr only, so either behavior is safe today — the drain is cheap insurance per web-tmux ARCHITECTURE.md integration note.

### 7. `start_supervisor` pump shape (port 1:1 from app_state.rs:1766-1852/3826-3831)
```rust
pub fn start_supervisor(&mut self, cx: &mut Context<Self>) {
    // missing-opts → Failed{reason, stderr_tail: "Please build the backend binary: …"}
    self.backend_status = BackendStatus::Starting;
    cx.notify();
    let view_weak = cx.entity().downgrade();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<SupervisorEvent>();
    TOKIO_RT.spawn(async move { /* Supervisor::new(); watch→Status; spawn(opts)→Ready/Failed on tx */ });
    cx.spawn(move |_view, cx: &mut AsyncApp| {
        let cx_handle = cx.clone();
        async move {
            while let Some(event) = rx.recv().await {
                cx_handle.update(|cx: &mut App| {
                    if let Some(entity) = view_weak.upgrade() {
                        entity.update(cx, |this, cx| { /* map event; cx.notify() */ });
                    }
                });
                // failed upgrade → listener dead; loop breaks on channel close
            }
        }
    }).detach();
}
// Retry button closure (verbatim web-term app_state.rs:3826-3831):
//   |this, _ev, _window, cx| { this.start_supervisor(cx); }
```

### 8. Settings corrupt-file recovery (verbatim port target, web-term lib.rs:159-177)
```rust
match serde_json::from_str::<DesktopSettings>(&content) {
    Ok(mut settings) => { settings.custom_base = Some(base.to_path_buf()); Ok(settings) }
    Err(_) => {
        // Corrupt file recovery: rename to settings.json.bak and regenerate defaults
        let bak_path = file_path.with_extension("json.bak");
        let _ = fs::rename(&file_path, &bak_path);
        let settings = Self { custom_base: Some(base.to_path_buf()), ..Default::default() };
        let _ = settings.save_to(base);
        Ok(settings)
    }
}
```

### 9. main.rs skeleton order (web-term main.rs:15-100 port)
```rust
fn main() {
    let _tokio_guard = webtmux::app_state::TOKIO_RT.enter();               // 1. runtime context
    let mut settings = DesktopSettings::load().unwrap_or_default();       // 2. .bak-recovering load
    let backend_path = resolve_backend_path(&settings);                   // 3. bundle candidates
    let spawn_opts = SpawnOptions::new(backend_path);
    let initial_bounds = window_state::restore(&settings).unwrap_or_else(|| WindowBounds::Windowed(
        Bounds { origin: point(px(0.0), px(0.0)), size: size(px(1200.0), px(800.0)) }));  // 4. restore
    Application::with_platform(gpui_platform::current_platform(false)).run(move |cx: &mut App| { // 5.
        let font = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");           // 6. font first
        let _ = cx.text_system().add_fonts(vec![Cow::Borrowed(font)]);
        gpui_component::init(cx);                                                        // 7. components
        webtmux::theme::apply_placeholder(cx);  // → Theme::change(ThemeMode::Dark, None, cx)
        let window_options = WindowOptions {
            window_bounds: Some(initial_bounds),
            window_min_size: Some(size(px(800.0), px(500.0))),   // web-tmux numbers (main.js:184-188)
            titlebar: Some(TitlebarOptions { title: Some("Tmux GUI".into()), appears_transparent: true, ..Default::default() }),
            ..Default::default()
        };
        let _ = cx.open_window(window_options, move |window, cx| {
            #[cfg(target_os = "windows")]
            { /* DWM 20/19 block — verbatim web-term main.rs:69-90 */ }
            window_state::observe(window, settings_arc, cx);
            let app_state = cx.new(|_cx| AppState::new(settings, Some(spawn_opts)));
            app_state.update(cx, |this, cx| { this.start_supervisor(cx); });
            app_state
        });
    });
}
```

### 10. Windows release fxc bootstrap (from web-term package-windows.ps1:38-51 shape)
```powershell
$FxcTool = "$Root\desktop-gpui\tools\fxc\fxc.exe"
if ([string]::IsNullOrEmpty($env:GPUI_FXC_PATH) -or -not (Test-Path $env:GPUI_FXC_PATH -ErrorAction SilentlyContinue)) {
    # compile-once if missing (README.md: rustc -O main.rs -o fxc.exe)
    rustc -O "$Root\desktop-gpui\tools\fxc\main.rs" -o $FxcTool
    $env:GPUI_FXC_PATH = $FxcTool
}
cargo build --release --manifest-path "$Root\desktop-gpui\Cargo.toml" --package webtmux
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| web-term 4-variant `BackendStatus` (incl. `Crashed{exit_code}`) | web-tmux 3-variant (`Failed` folds early-exit) | 01-UI-SPEC S3 (approved 2026-09-06) | supervisor exit monitor + status.rs match arms port differently |
| web-term probe `/api/settings` (settings-poll workaround) | web-tmux native `GET /api/health` (real liveness + tmux presence) | web-tmux backend (existing) | cleaner readiness semantics than the reference |
| Electron FE infinite "Starting…" wart (`backendTimedOut`, fe App.tsx:59-62 gate) | 10s port timeout landing on the Failed page | STATE-01 (corrected, not ported) | no perpetual spinner |
| Electron shell-only startup | GPUI-native shell (custom controls, DWM, window_state) | this milestone | startup path is native, not DOM |

**Deprecated/outdated in scope:** none — pins ARE the current state of the art per 2026-09-06 ecosystem research (`gpui-pre 0.3.3` and `gpui-component 0.6.0` remain crates.io-latest; STACK.md records deliberate non-adoption of `alacritty_terminal 0.26.0`).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | 10s applies to BOTH handshake and readiness timeouts (STATE-01 labels "10s port timeout"); both stay overridable builder fields | Supervisor deltas D4 | Cosmetic: a slow-health case could show the spinner a bit longer; no functional risk |
| A1a | Per-render `IsZoomed` (via `window_state::is_window_maximized`) is acceptable on the paint path for the max/restore icon; fallback = web-term's `is_maximized` field (boot-restore + button-only updates) | Window controls | LOW: worst case the OS-snap icon goes stale (web-term's shipped behavior) |
| A2 | Crate package names `webtmux-{supervisor,settings,backend-client,terminal}` + app bin `webtmux` | Standard Stack | Naming churn only |
| A3 | Settings/config dir `tmux-gui-desktop` (distinct from `tmux-gui-gtk` and `webterm-desktop`) | Settings crate | LOW: name change later = one `paths.rs` line |
| A4 | Keep `adopt_or_clear` + `last_backend_url` (probe via `/api/health`) though CONTEXT does not name them | Supervisor deltas D | LOW: ~40 lines; wrongfully omitting loses orphan-backend recovery |
| A5 | `window_state.rs` "observe" is ported as the reference's actual close-flush persistence (`on_window_should_close`), not true move/resize tick saving, per verbatim-port lock | window_state port | Parity-audit note only (documented) |
| A5b | Restore() legacy-size/cases: port structure, re-base default constants to web-tmux's 1200×800/800×500, keep legacy guards byte-for-byte | window_state port | LOW |
| A6 | windows_subsystem="windows" adoption is discretionary (web-term ships console); if adopted, CI must build debug+release and prove the stdout handshake with no parent console | Pitfalls 6/minors | Build-config churn only |
| A7 | kill_on_drop-orphan nuance (hard exits) is acceptable for v1 with adopt_or_clear coverage | Pitfalls 6 | MEDIUM: lingering backend after crashes if adopt path isn't included — folded into A4 |
| A8 | The 10s FE budget (fe App.tsx:62) licenses supervisor-side 10s constants independent of Electron (which has no timeout) | Supervisor deltas D4 | none |
| A8 | `prompt_for_paths` (native folder picker verified in STACK.md) is NOT needed in Phase 1 (no Create Session dialog this phase) | scope fence | none |

## Open Questions

1. **Which native surfaces does gpui-pre 0.3.3 handle-maximize on Linux?**
   - What we know: web-term proves the shared path (`is_window_maximized` → `window.is_maximized()`, `toggle_maximize` → `window.zoom_window()`); Linux extra polish is EXTRA-03 (v2).
   - Unclear: whether HTCAPTION-equivalent double-click/snap exists per-WM on Linux (decorations path differences).
   - Recommendation: Phase 1 acceptance runs smoke on Windows + Linux *build*; interactive Linux window-control polish is explicitly out (EXTRA-03). No plan blocker.
2. **`windows_subsystem = "windows"` in Phase 1 or Phase 7?** (see A6) — planner picks; default recommendation: adopt in Phase 1 **with** the dual-config CI assertion, since the release build is already a Phase-1 success criterion.
3. **Default window origin for first run:** Electron (`main.js:184-197`) specifies only width/height (OS-centered default), while web-term centers at a fixed (180,60). UI-SPEC S4: "default window 1200×800 … persisted geometry restored at boot before open". Fixed-origin default is what web-term's `DEFAULT_ORIGIN_X/Y` does; using GPUI's default centered placement is closer to Electron. Decision surface; recommendation: pass `window_bounds: None` when no persisted state → GPUI default centered placement (matches Electron more closely than a fixed offset). [ties to A5b]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| rustc/cargo (MSVC) | workspace build/tests | ✓ | rustc 1.96.0 (2026-05-25), cargo 1.96.0 | — |
| Go toolchain | build `tmux-gui-server` sidecar (dev/test fixture) | ✓ | go 1.23.4 windows/amd64 | use prebuilt `desktop/resources/tmux-gui-server.exe` (committed) |
| make | repo workflow (PKG-03 later; Not Phase 1) | ✓ (GnuWin32) | — | manual go/cargo commands |
| tmux.exe (backend boot check `tmux -V`) | sidecar spawns w/o early exit in tests | ✓ both `C:\msys64\usr\bin\tmux.exe` (PATH) and `%LOCALAPPDATA%\Microsoft\WinGet\Links\tmux.exe` | — | supervisor resolver finds either |
| `d3dcompiler_47.dll` | fxc tool (Windows release build) | ✓ `C:\Windows\System32\d3dcompiler_47.dll` present | ships with Win10+ | full Windows SDK (vendored build.rs SDK fallback) |
| Visual Studio Build Tools (MSVC linker) | cargo link step | ✓ (web-term built on this machine) | — | — |
| Full Windows SDK | raw fxc.exe | NOT required (GPUI_FXC_PATH makes it optional) — [CITED vendored build.rs:114-138] | — | fxc drop-in tool |
| Linux dev packages (fontconfig, xkbcommon, X11+Wayland, Vulkan loader) | gpui-pre-linux build | **no observation** (not probeable from this Windows host) | — | documented package list: `libfontconfig1-dev`, `libwayland-dev`, `libxkbcommon-dev`, `libxkbcommon-x11-dev`, `libx11-dev`, `libxcursor-dev`, `libxrandr-dev`, `libgl1-mesa-dev`, `vulkan-loader` (from web-term research; apt names tagged from ecosystem-level study) |
| tmux on Linux user machine | backend runtime | no observation | — | backend fails visibly at boot ("Tmux is not installed.") — by design STATE-01 |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** full Windows SDK → fxc drop-in tool (verified fallback direction both ways); Linux probes → documentation + first Linux smoke check at execution.

**Runtime State Inventory:** Omitted — greenfield phase (no rename/refactor/migration; no stored data, live-service config, OS registrations, or build artifacts carry a currently-renamed identity). The only pre-existing state: committed dev binaries (`desktop/resources/tmux-gui-server.exe`, root `tmux-gui-server.exe~`) — none require migration.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` / `#[tokio::test]` (tokio 1.53.1 macros feature); `cargo test` |
| Config file | none — tests live in-crate (`#[cfg(test)] mod tests`) + `crates/<crate>/tests/*` (web-term layout, verified) |
| Quick run command | `cargo test -p webtmux-supervisor -p webtmux-settings` |
| Full suite command | `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` |

Fixture seam (port of web-term's `scripts/build-test-backend.sh` → web-tmux): build/copy `tmux-gui-server(.exe)` into `desktop-gpui/test-support/` (or point `TEST_BACKEND_PATH` at repo-root artifact; the repo root binary already exists today). Integration tests skip gracefully when the fixture is absent (pattern verified in web-term supervisor/tests/integration.rs:28-36).

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| STATE-01 | handshake token parses (variants: plain, embedded, non-digit, other log lines) | unit | `cargo test -p webtmux-supervisor parse_handshake_line` | ❌ Wave 0 |
| STATE-01 | argv EMPTY + env has `TMUXGUI_PORT=0`/`TMUXGUI_HOST=127.0.0.1`/resolved `TMUXGUI_TMUX_BIN`; `TMUX`/`TMUX_PANE` removed | unit | `cargo test -p webtmux-supervisor spawn_options` | ❌ Wave 0 |
| STATE-01 | spawn real backend → watch reaches `Ready`, base_url `http://127.0.0.1:<n>` | integration (fixture) | `cargo test -p webtmux-supervisor --test integration reaches_ready` | ❌ Wave 0 |
| STATE-01 | invalid backend path → `Failed` with non-empty reason+tail (no panic) | integration | `cargo test -p webtmux-supervisor invalid_path` | ❌ Wave 0 |
| STATE-01 | handshake timeout (shrunken, e.g. 500ms) → `Failed` with "timed out" reason + child killed | integration | `cargo test -p webtmux-supervisor handshake_timeout` | ❌ Wave 0 |
| STATE-01 | early-exit child (e.g. `cmd /c exit 1` or fixture-less fake) → `Failed{"backend exited early (code 1)"}` (D2 proof) | integration | `cargo test -p webtmux-supervisor early_exit` | ❌ Wave 0 |
| STATE-01 | `adopt_or_clear` returns info on live `/api/health`, None after `stop` | integration | `cargo test -p webtmux-supervisor adopt_or_clear` | ❌ Wave 0 |
| SET-05 | settings roundtrip (fields → save → load equality) | unit | `cargo test -p webtmux-settings roundtrip` | ❌ Wave 0 |
| SET-05 | corrupt settings.json → defaults returned, `settings.json.bak` created (success criterion) | unit | `cargo test -p webtmux-settings corrupt_recovery` | ❌ Wave 0 |
| SET-05 | first run (no file) → defaults + `custom_base` set; re-save round-trips | unit | `cargo test -p webtmux-settings first_run` | ❌ Wave 0 |
| SHELL-02 (logic parts) | `window_state::restore/extract` guards: clamps, 0-size reject, minimized coords, maximized preserves previous bounds | unit | `cargo test -p webtmux window_state` | ❌ Wave 0 |
| SHELL-02 (interactive) | min/max/restore buttons, drag region, double-click, geometry flash-free restart, theme-placeholder persistence | manual (UI) | manual UAT script in plan (launch dev exe; exercise controls; restart app; corrupt settings file by hand) | — manual-only: GPUI window behavior is not meaningfully automatable headless; logic fully covered by the unit layer above |
| PKG-01 (Windows debug) | `cargo build` succeeds **without** fxc | build | `cargo build --manifest-path desktop-gpui/Cargo.toml --package webtmux` (must run BEFORE GPUI_FXC_PATH is set in the check) | ❌ Wave 0 (first workspace task) |
| PKG-01 (Windows release) | release builds with fxc tool compiled + `GPUI_FXC_PATH` set | build (smoke) | `rustc -O tools/fxc/main.rs -o tools/fxc/fxc.exe; $env:GPUI_FXC_PATH=…; cargo build --release …` | ❌ Wave 0/late-phase task |
| PKG-01 (Linux) | build works after documented package install | documented/manual (no Linux runner in this env) | doc file `desktop-gpui/docs/BUILDING.md` (or README §Build) lists packages + commands; verified by review, UAT-gated | ❌ Wave 0 (doc) |

### Sampling Rate
- **Per task commit:** `cargo test -p <touched crate>` + `cargo build` (quick compile gate)
- **Per wave merge:** `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace`
- **Phase gate:** full workspace suite green + Windows debug build green + release-with-fxc smoke + UAT (startup path, window controls, persistence, corrupt-file recovery) before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `desktop-gpui/Cargo.toml` workspace + 5 crate skeletons (pkg names per A2), `resolver=2`, exact pins, **commit Cargo.lock first artifact**
- [ ] `crates/supervisor/src/{lib,handshake?,exit?}.rs` (+ `tests/integration.rs` real-backend suite above)
- [ ] `crates/settings/src/{lib,paths}.rs` (+ `tests/store_test.rs` roundtrip/corrupt/first-run)
- [ ] `crates/backend-client/src/lib.rs`, `crates/terminal/src/lib.rs` — compile-only stubs (no external deps)
- [ ] `crates/webtmux/src/{main,app_state,state modules,theme placeholder,bundle,window_state,icons}.rs` + `views/{mod,status,tab_strip minimal}.rs` (+ `tests/window_state.rs` + bundle-resolution test)
- [ ] `desktop-gpui/assets/fonts/JetBrainsMono-Regular.ttf` (copy or reference the exact FE node_modules JetBrains Mono file for byte-parity)
- [ ] `desktop-gpui/tools/fxc/{main.rs,README.md}` (rustc compile step in build docs/package script)
- [ ] `scripts/build-test-backend.{sh,cmd}`+ test-support harness (web-term parity; adapted to repo-root binary output)

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | n/a (local desktop, loopback sidecar) |
| V3 Session Management | no | n/a (no WS/user sessions yet) |
| V4 Access Control | no | n/a |
| V5 Input Validation | yes (minimal) | serde typed deserialization with `#[serde(default)]`; corrupt file → .bak + defaults (never crash loop); settings `backend_path` override validated by existence at spawn (Failed with reason otherwise) |
| V6 Cryptography | no | no crypto surfaces (encryption key custody dropped from the port) |
| V14 Config | yes | secrets-in-env (nothing to hold, invariant asserted), loopback pinning `TMUXGUI_HOST=127.0.0.1`, child-env hygiene (`TMUX`/`TMUX_PANE` removal), Windows profile-dir ACL privacy rationale |

### Known Threat Patterns for (Rust GPUI app spawning a Go sidecar)
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Secret material in argv visible in process listing | Information Disclosure | no secrets exist in web-tmux spawn env; argv stays EMPTY (tested invariant) |
| Stale/orphan backend adopted blindly | Spoofing/Tampering (wrong app talks to leftover server) | `adopt_or_clear` probe + handshake freshness; orphan-risk mitigations (Pitfall 6) |
| Backend bound to 0.0.0.0 / LAN exposure | Elevation via network | env pins `TMUXGUI_HOST=127.0.0.1` (config default is 127.0.0.1, [VERIFIED config.go:57]) |
| Unvalidated backend_path override → arbitrary process spawn | Tampering/Elevation | validate existence before spawn; failure surfaces Failed{reason}, never shell-outs; treat override as dev feature (matching web-term) |
| Terminal/WS payload logging (future phases) | Information Disclosure | log-transport-state-only discipline starts now (supervisor logs metadata only) |
| Corrupt/poisoned settings file | DoS at launch | `.bak` rename + defaults; load is `unwrap_or_default`-safe |

## Sources

### Primary (HIGH confidence — all read in full this session)
- `E:\Coding Stuff\web-term\desktop-gpui\Cargo.toml` — pinned manifest (verbatim quotes used)
- `E:\Coding Stuff\web-term\desktop-gpui\crates\supervisor\src\lib.rs` — BackendStatus, handshake, probe loop, stop, adopt_or_clear (491 lines read)
- `E:\Coding Stuff\web-term\desktop-gpui\crates\settings\src\{lib.rs,paths.rs}` (+ `tests/store_test.rs`) — settings API + `.bak` recovery
- `E:\Coding Stuff\web-term\desktop-gpui\crates\webterm\src\{main.rs,window_state.rs,bundle.rs,app_state.rs(lines 700-860, 1766-1880, 3824-3837),views\status.rs,views\tab_strip.rs(lines 265-349),theme.rs(lines 640-824),Cargo.toml}`
- `E:\Coding Stuff\web-term\desktop-gpui\crates\supervisor\tests\{integration.rs,loopback_test.rs}` — test-fixture patterns
- `E:\Coding Stuff\web-term\desktop-gpui\tools\fxc\{main.rs(80 lines read),README.md}` + `scripts/package-windows.ps1(fxc lines)` + `scripts/build-test-backend.sh`
- `E:\Coding Stuff\web-tmux\be\cmd\server\main.go` — `BACKEND_PORT:%d\n` stdout print (:61), tmux-missing early exit (:37-40), slog→stderr (:89)
- `E:\Coding Stuff\web-tmux\be\internal\config\config.go` — env-only config (TMUXGUI_* names, defaults)
- `E:\Coding Stuff\web-tmux\be\internal\tmux\binary.go` — TMUXGUI_TMUX_BIN wins / SetBinary validation
- `E:\Coding Stuff\web-tmux\be\internal\server\{router.go:27,health.go}` — `/api/health` route + always-200 shape
- `E:\Coding Stuff\web-tmux\desktop\main.js` — Electron spawn/env/strip/handshake/exit contract (283 lines read)
- `E:\Coding Stuff\web-tmux\desktop-gtk\internal\backend\manager.go:172-182` — `tmux-gui-gtk` config dir (collision check)
- `E:\Coding Stuff\web-tmux\Makefile:71-73` — `make be` → repo-root `tmux-gui-server[.exe]`
- `E:\Coding Stuff\web-tmux\fe\src\App.tsx:59-62,190,213` — 10s timeout anchor + copy
- Vendored crate sources on this machine: `gpui-pre-0.3.3/src/{app.rs:178,elements\div.rs:731,elements\animation.rs:506,window.rs:{741,2187,2684,6212,1464-1473}}`, `gpui-pre-windows-0.3.3/src/events.rs:970-980` (HTCAPTION mapping), `gpui-pre-windows-0.3.3/build.rs:10,114-138` (fxc gate), `gpui-component-0.6.0/src/{spinner.rs (read in full),icon.rs:85-96,theme\mod.rs:237}`, `gpui-pre-platform-0.3.3/src/gpui_platform.rs:57`

### Secondary (MEDIUM confidence)
- Context7 `/websites/rs_gpui` — WindowOptions/TitlebarOptions/WindowBounds docs (digest cached, key `e2b405e3…`)
- Context7 `/longbridge/gpui-component` — Spinner API docs (key `8f9a28b4…`) — corroborated by vendored spinner.rs
- crates.io live API via `gsd_run query package-legitimacy check` (2026-09-06) — existence/age/downloads/repo per package
- Microsoft Win32 documentation for `DWMWA_USE_IMMERSIVE_DARK_MODE` (attrs 20/19), `ShowWindowAsync`, `IsZoomed`, `RedrawWindow` semantics — recited from web-term working code; not re-fetched this session [CITED: Microsoft Learn Desktop Technologies / DWM constants]

### Tertiary (LOW confidence / needs validation)
- None material. Residual uncertainties are enumerated in Assumptions Log (A1–A8) and Open Questions.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — pins read from the reference manifest; registry re-check same-day; vendored sources present
- Architecture (spawn/handshake/settings/window mechanics): HIGH — every mechanic read line-by-line in both codebases + vendored crate sources
- Spawn-contract correction: HIGH — three independent local sources agree (config.go env-only, main.js env-based spawn, main.go looks only at cfg)
- Pitfalls: HIGH for local-code ones; MEDIUM for cross-cutting threading behaviors confirmed only through the working reference
- Validation architecture: HIGH (framework + fixture harness proven in web-term; maps to this repo's existing sidecar artifact)

**Research date:** 2026-09-06
**Valid until:** ~2026-10-06 (30 days; pins and sidecar contract are stable/locked; re-check only `gpui-pre` newest-versions freshness before re-pin far in the future)

**Note for the planner:** 01-UI-SPEC.md (approved) is the visual source of truth for ALL status-page/title-bar visuals — this file intentionally does NOT re-specify visuals; it supplies the *mechanics* (state machine, APIs, data flow, wiring) beneath them. One wording correction over CONTEXT.md (argv vs env port binding) is documented in Pattern 1 and MUST propagate to the plan's spawn task.
