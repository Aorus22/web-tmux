# Stack Research

**Domain:** Native GPUI (Rust) desktop frontend for a tmux GUI app (brownfield milestone — frontend ships alongside existing Go backend + React/Electron/GTK frontends)
**Researched:** 2026-09-06
**Confidence:** HIGH (every version verified against the reference repo's committed `Cargo.lock` AND live crates.io API on research date; every API claim verified against the actual vendored crate source or reference implementation source)

## Recommended Stack

**Strategy in one sentence:** copy the web-term `desktop-gpui` workspace manifest verbatim with exact pins and a committed `Cargo.lock`, then add **zero** new runtime crates — everything web-tmux needs (icons, folder picker, fonts) is already inside the pinned `gpui-pre`/`gpui-component` set.

### Core Technologies

All versions verified in `E:\Coding Stuff\web-term\desktop-gpui\Cargo.toml` + `Cargo.lock` (resolved), then cross-checked against crates.io latest (2026-09-06).

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `gpui` (package: `gpui-pre`) | `=0.3.3` | GPU-accelerated UI framework (window, views, text, svg, input) | CRITICAL: this is **not** the official `gpui` crate. `gpui-pre` is a snapshot fork of Zed gpui ("zed@5b055fa"), and it is currently the crates.io **latest** (max stable = 0.3.3, verified). `gpui-component 0.6.0` is wired against `gpui-pre 0.3.3` — mixing in the upstream `gpui 0.2.x` crate breaks the build. The reference is proven 1:1 on Windows + Linux with this exact version; pre-1.0 breaking-change churn is real (the fork renamed platform crates), so exact-pin + commit the lock. |
| `gpui-platform` (package: `gpui-pre-platform`) | `=0.3.3` | Platform implementations gluing gpui-pre to each OS | Easily missed: the app crate needs it (web-term adds it in `crates/webterm`, entry via `Application::with_platform(gpui_platform::current_platform(false))`). Same pin as gpui-pre. |
| `gpui-component` | `=0.6.0` | Component library: theme registry, TitleBar, inputs, dialogs, keybindings init | Latest stable on crates.io = 0.6.0 — web-term's pin is still current. Provides the widget primitives needed for UI parity (custom title bar, themed inputs in Create Session/Settings dialogs). `gpui_component::init(cx)` required at startup. Verified in lock with `gpui-component-macros` + `gpui-kit-assets` all at 0.6.0. |
| `alacritty_terminal` | `=0.25.1` | Terminal emulation engine (VT parse, scrollback grid, `Term` state machine) | **User-decided** and matching reference. Note: crates.io latest is 0.26.0 (verified today) — deliberately **NOT** adopted: the web-term terminal crate's rendering/event pump code is written against 0.25.1 APIs, an upgrade is a code port with zero benefit (tmux feeds standard VT streams), and parity-with-proven beats newest-here. Log as future upgrade, not milestone work. |
| `tokio` | `=1.53.1` | Async runtime for supervisor/backend-client | Pins with features `["rt-multi-thread","process","macros","sync","time","net"]` only (copy the workspace dep *with* features). `tokio::process::Command` with `kill_on_drop` is the supervisor's backend lifecycle backstop. A shared runtime entered via `Runtime::enter()` on the GPUI main thread is the ported pattern (`TOKIO_RT.enter()` in web-term `main.rs`). |
| `reqwest` | `=0.12.28` | HTTP client for REST (`/api/health`, `/api/tmux/*`, `/api/sessions`) | Pinned with `default-features = false`, `features = ["json","multipart","rustls-tls"]` — rustls avoids the OpenSSL system dependency on Linux CI/build boxes. (gpui-pre drags its own `gpui-pre-reqwest 0.12.15` fork into the tree for its http-client — that is normal; your code emits plain `reqwest 0.12.28` calls.) |
| `tokio-tungstenite` | `=0.26.2` | Per-session WS client (`terminal.input/resize`, `state.snapshot/delta`, `terminal.snapshot/output` …) | Pinned with `default-features = false, features = ["connect","handshake"]` (loopback `ws://127.0.0.1:<port>/ws/...` — TLS unused). Funnel mpsc into `flume` so GPUI `Context::BackgroundExecutor` and tokio can share channels. |
| `flume` | `=0.12.0` | Sync/async channel bridging backend threads ↔ GPUI render loop | Terminal event pump + WS→GPU thread handoff in the reference; both-side capable (`send` from tokio, `recv` in GPUI). |
| `serde` / `serde_json` | `=1.0.229` / `=1.0.151` | DTO mirroring of the Go REST/WS protocol (`hello`, `state.snapshot`, `terminal.snapshot`, …) | The unique JSON payload names/shapes must mirror `be/`'s Go structs (and the Eigenstat docs in PROJECT.md) exactly; serde is the whole DTO layer. |
| `futures-util` | `=0.3.32` | `StreamExt`/`SinkExt` for the tokio-tungstenite WS split | WS duplex loop plumbing only. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `parking_lot` | `=0.12.5` | `Mutex`/`OnceLock` shared state across GPUI + tokio | Settings handle, AppState fields, supervisor state advertised into views (reference pattern throughout). |
| `anyhow` | `=1.0.104` | Ad-hoc errors | Terminal crate + misc glue; pairs with `thiserror` for typed crates. |
| `thiserror` | `=2.0.20` | Typed error enums | `backend-client`, `supervisor`, `settings` crates (reference pattern). |
| `dirs` | `=6.0.0` | Settings location: `dirs::config_dir()` on Windows + Linux | Settings store dir (`<config>/<app-name>/settings.json`). |
| `rand` | `=0.10.2` | Supervisor token generation / IDs | Supervisor handshake + settings; keep even if web-tmux needs fewer secrets. |
| `tempfile` | `=3.27.0` | Dev/test deps | `--dev`/tests: ephemeral settings + fake backend integration tests. |
| `libc` | `=0.2.189` | Unix-only supervisor tests | `[target.'cfg(unix)'.dev-dependencies]` style — only in dev-deps (reference placement). |
| `raw-window-handle` | `=0.6.2` | HWND access for DWM dark title bar on Windows | App crate only — port web-term's `DwmSetWindowAttribute(hwnd, 19/20, …)` block verbatim; this is how the transparent dark titlebar is drawn on Windows. |

**	for icons/image assets/fonts — see "Additional Crates Needed" below: NONE.** Lucide icons + folder picker + font are all handled without any new crate.

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `rustc` / `cargo` (1.8x-1.9x, MSVC toolchain on Windows) | Build + test | Reference has **no** `rust-toolchain.toml`, no edition-bump — `edition = "2021"`, `resolver = "2"` at workspace root, `[profile.release] lto = "thin"`. Match all three. |
| `tools/fxc` drop-in shader compiler (copy the ~100-line rustc-compiled tool from web-term) | **Windows release builds of `gpui-pre-windows` require `fxc.exe`** to compile HLSL shaders (`shaders.hlsl`, `color_text_raster.hlsl`) into byte arrays (`#[cfg(not(debug_assertions))]`). The real fxc only ships in the multi-GB Windows SDK; the tool wraps `d3dcompiler_47.dll` (already in `C:\Windows\System32`) instead. Compiled with `rustc -O main.rs -o fxc.exe` (zero crates) and exported via env `GPUI_FXC_PATH`. **Debug builds do not need it.** Absolutely port this — release packaging breaks without it. |
| `scripts/package-windows.ps1` / `package-windows.cmd` | Assemble Windows dist bundle | Pattern: build Go backend (`go build -ldflags "-s -w" -o dist\<app>-windows-x64\<backend>.exe`) → set `GPUI_FXC_PATH` → `cargo build --release --manifest-path desktop-gpui\Cargo.toml --package <app>` → copy exe + assets into one folder (side-by-side backend). Adapt the copy to web-tmux's `be/` + Makefile. |
| `scripts/package-linux.sh` | Assemble Linux dist bundle | `cargo build --release --manifest-path ... --package <app>` + copy backend + exe. No fxc step needed on Linux. |
| `scripts/build-test-backend.sh` / `.cmd` | Build real Go backend into `test-support/backend` for dev loop | Critical dev-loop tool: supervisor + backend-client integration tests spawn a REAL backend binary (start it, hit `/api/health`... works headless in CI). Port with web-tmux's server binary name (`tmux-gui-server`). |
| Linux system packages (build-time only) | `gpui-pre-linux` needs fontconfig, xkbcommon, X11 + Wayland dev libs, and a Vulkan-capable driver (`gpui-pre-wgpu` → wgpu 29.x: Vulkan, with GL fallback) | apt-equivalent: `libfontconfig1-dev libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-dev libxcursor-dev libxrandr-dev libgl1-mesa-dev vulkan-loader`. No OpenSSL needed (rustls). |
| Windows toolchain | `x86_64-pc-windows-msvc` target + Visual Studio Build Tools (C++ linker) | No Windows SDK needed thanks to the fxc tool. `d3dcompiler_47.dll` ships with Windows 10+. |

## Installation

```toml
# desktop-gpui/Cargo.toml — copy web-term's manifest verbatim, renaming crates:

[workspace]
resolver = "2"
members = [
    "crates/supervisor",
    "crates/settings",
    "crates/backend-client",
    "crates/terminal",
    "crates/app",   # web-term calls it "webterm"; rename freely, keep 5-crate shape
]

[workspace.dependencies]
# Internal
app-supervisor      = { path = "crates/supervisor" }
app-settings        = { path = "crates/settings" }
app-backend-client  = { path = "crates/backend-client" }
app-terminal        = { path = "crates/terminal" }

alacritty_terminal = "=0.25.1"
flume = "=0.12.0"
anyhow = "=1.0.104"
tokio-tungstenite = { version = "=0.26.2", default-features = false, features = ["connect", "handshake"] }
futures-util = "=0.3.32"

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

[profile.release]
lto = "thin"
```

```toml
# crates/app/Cargo.toml (delta only):
[dependencies]
gpui = { workspace = true }
gpui-platform = { package = "gpui-pre-platform", version = "=0.3.3" }   # REQUIRED for Application::with_platform
gpui-component = { workspace = true }
raw-window-handle = "=0.6.2"                                            # Windows DWM dark titlebar
# + internal crates, tokio, serde, serde_json, parking_lot

[dev-dependencies]
# tokio-tungstenite + futures-util + tempfile for live-backend integration tests
```

```bash
# One-time per machine (Windows dev):
cargo build --manifest-path desktop-gpui/Cargo.toml   # debug build needs NO fxc
# From a NEW machine's release path, before first release build:
rustc -O tools/fxc/main.rs -o tools/fxc/fxc.exe
$env:GPUI_FXC_PATH = "<abs path>\tools\fxc\fxc.exe"
```

## Additional Crates Needed (deltas web-tmux must understand)

**None. New capabilities map onto existing pinned crates:**

1. **SVG / lucide icon embedding → NO SVG crate, NO AssetSource, NO rust-embed, NO asset build step.** Verified against reference `crates/webterm/src/icons.rs`: icons are inline Rust byte-string constants (`pub const SERVER_SVG: &[u8] = br#"<svg …/>"#"#) rendered with `gpui::svg().path(sprite_name).style(...)` — gpui parses + rasterizes SVG internally (usvg is already in the dependency tree). The font is the **only** binary asset: `include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf")` + `cx.text_system().add_fonts(vec![Cow::Borrowed(bytes)])` at startup. For web-tmux: copy the pattern, and **generate your `icons.rs` icon list by pulling the exact `lucide-react` component path data from `fe/`'s node_modules** (same icons the Electron UI uses) — that is what guarantees pixel parity with the existing UI.
2. **Native directory picker for Create Session dialog cwd field → NO extra crate.** `gpui-pre 0.3.3` ships it (verified in vendored source, `src/app.rs:1582`): `App::prompt_for_paths(PathPromptOptions { files, directories, multiple, prompt }) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>>`, plus `prompt_for_new_path`. Windows backend implements it natively (IFileOpenDialog with folder flag — verified in `gpui-pre-windows-0.3.3/src/platform.rs`); Linux backend implements it too. Set `directories: true, files: false, multiple: false`. Caught pitfall: this is impure-async (non-`Send` task) — the receiver should be polled/forwarded into flume/tokio, exactly the web-term modal pattern; do not `.await` it directly on the GPUI thread without spawn handling.
3. **Image/photo assets (app icon in taskbar/title bar)** → web-term ships the default Rust icon (no `.ico` embedding, no build.rs). If web-tmux wants later, add `winres` in a `build.rs` + `.ico` — **optional, defer**, parity with WebTerm (and 1:1 with the reference) does not require it.

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `gpui-pre =0.3.3` (exact pin) | upstream `gpui =0.2.2` crate | Never for this milestone — `gpui-component 0.6.0` targets gpui-pre; using upstream `gpui` 0.2.x breaks the component library. Only reconsider when deliberately re-baselining both together as a future milestone. |
| `alacritty_terminal =0.25.1` | `alacritty_terminal =0.26.0` (crates.io latest) | Only if a fresh milestone re-baselines the terminal crate too (needs code port in `crates/terminal` — event enum + grid API changes). Not during this one. |
| Inline `&[u8]` SVG consts + `include_bytes!` font | `rust-embed` / gpui `AssetSource` registry | Not needed for this feature set. Use only if you later ship user-selectable binary assets at runtime (images, fonts loaded from disk). |
| `gpui::prompt_for_paths` (built-in) | `rfd` crate for native dialogs | Avoid — `rfd` re-implements what gpui already provides and risks conflicting COM/msg-loop assumptions on the GPUI window thread. Only if you need open-save dialogs beyond what gpui exposes. |
| `reqwest` (rustls) | `ureq`, `hyper` direct | Only if blocking-IO preference surfaces; web-term chose reqwest+rustls so secrets stay in one code path; keep it 1:1 with reference. |
| `flume` | `tokio::sync::mpsc` alone | Only if you drop the ferro-pattern entry direction (we don't — GPUI render loop uses sync receiving). |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `portable-pty` (or any local PTY stack) | PROJECT.md: **tmux is the single source of truth**; the GPUI frontend consumes tmux through the Go backend's REST/WS, exactly like the Electron FE does. A local PTY would duplicate state and bypass the validated backend. | `backend-client` WS → `alacritty_terminal` state machine → gpui render (the reference terminal crate does this) |
| Any backend protocol change | Project constraint: frontend-only milestone; REST/WS surface already serves 3 frontends with stable ids (`@N`/`%N`), `send-keys -H` batching, pipe-pane streaming | Log gaps as backend issues, don't fork behavior in the GPUI crate |
| Upgrading pins mid-milestone (`tokio`, `gpui-pre`, block churn) | gpui-pre is pre-1.0; web-term chose exact pins + committed Cargo.lock after churn pain; upgrading re-opens verification on a codebase health-checked as "proven 1:1" | Adopt web-term's exact manifest (it is crates.io-latest for gpui-pre 0.3.3 / gpui-component 0.6.0 anyway), only ever upgrade across milestone boundaries |
| `AssetSource`/`rust-embed` machinery, icon-asset pipelines | Over-engineering for a desktop app with one TTF + inline SVG consts; web-term (the parity reference) does not have one | `include_bytes!` + inline `br#"…"#` constants |
| Electron/Tauri/wry, GTK, Qt | Contradicts milestone: this is the native-GPUI frontend milestone | GPUI only (wgpu via gpui-pre handles all rendering) |
| `directories` crate (instead of `dirs =6.0.0`) | Reference uses `dirs`; keep the exact set to avoid cross-locating settings paths inconsistently across crates | `dirs::config_dir()` |

## Stack Patterns by Variant

**If building on Windows (dev):**
- Debug build (`cargo build`): MSVC target, no fxc needed, console window stays visible (fine — it's where dev logs / backend stdout visibility comes from).
- Release build: `rustc -O tools/fxc/main.rs -o fxc.exe` first, set `GPUI_FXC_PATH`, then `cargo build --release --package <app>`. Missing `GPUI_FXC_PATH` = release build failure with an fxc-not-found error (this is the #1 Windows build landmine).
- Consider `#[cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]` on `main.rs` so shipped exe shows no console (web-term ships the console — a wart; do better if trivial, match web-term otherwise).

**If building on Linux (dev/CI):**
- Install system dev packages once (fontconfig + xkbcommon + X11/Wayland + Vulkan loader); no fxc equivalent needed.
- Release lto="thin" applies identically; no other platform flags required.

**Bundle layout (both platforms):** dist/<app>-<os>-x64/ = backend binary + app exe side-by-side + assets/ (font). App resolves backend at runtime: settings override → adjacent-to-current-exe (`tmux-gui-server(.exe)`) → dev fallback paths in workspace (port web-term's `bundle.rs::resolve_backend_path` candidates to web-tmux's `be/` output; keep screens with a settings field for a user override).

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `gpui-pre 0.3.3` | `gpui-component 0.6.0`, `gpui-pre-platform 0.3.3` | Live-verified (2026-09-06): both are crates.io-current latest. gpui-component carries `gpui-kit-assets 0.6.0`; gpui-pre decomposes into the `-apple/-linux/-windows/-wgpu/-platform` subcrates all at 0.3.3 — resolved together by the lock. |
| `tokio 1.53.1` | `tokio-tungstenite 0.26.2`, `reqwest 0.12.28`, `flume 0.12.0` | Locks cleanly in web-term's Lockfile (same resolution verified there). |
| `gpui-pre 0.3.3` internal reqwest | `gpui-pre-reqwest 0.12.15` (separately-locked fork) | Coexists with your `reqwest 0.12.28`; both end up in lock (they use a rename) — normal, do not try to deduplicate. |
| `alacritty_terminal 0.25.1` | `gpui-pre 0.3.3` | Compatible — that's exactly the reference pairing; renders through flume → gpui glyph pipeline. |
| wgpu chain | `gpui-pre-wgpu 0.3.3` → `wgpu 29.0.4` | Resolved automatically from the gpui-pre pin; Windows uses DirectX (via wgpu/DX12), Linux uses Vulkan/GL. Wildcard versions anywhere upstream = do not touch (gpui-pre's fork ecosystem is version-sensitive; the lockfile is the contract). |

## Integration Points with the Existing Go Backend (for REQUIREMENTS.md/plan-phase)

- **Spawn contract:** `tmux-gui-server --port 0` (NOT web-term's `WEBTERM_PORT` env). The Electron shell already established the handshake: backend prints `BACKEND_PORT:<port>\n` on **stdout** after binding (verified in web-tmux `desktop/main.js:147` — same token regex `BACKEND_PORT:(\d+)` as web-term). Port the supervisor crate 1:1: spawn with `--port 0`, parse that stdout line, don't poll-guess the port.
- **Readiness:** poll `GET /api/health` (web-tmux has a real health endpoint — better than web-term's settings-poll workaround); then `GET /api/tmux/info` as the first real call. Errors → backend-crashed state in UI.
- **Consumed (no changes):** session tree REST (`GET /api/sessions`, `POST /api/sessions` create w/ cwd + shell for the dialog) and one WS per open session (`hello`, `terminal.input/resize/capture`, `pane.*`, `window.*`, `session.*`, events `state.snapshot/delta`, `terminal.snapshot/output`, `connection.ready`, `tmux.disconnected/reconnecting`, `server.error`). DTOs mirror these exactly; stable ids (`@N`/`%N`).
- **Settings store (desktop-side):** UI prefs only (theme, window state, backend path override, tmux binary location override) — `dirs::config_dir()/<app>/settings.json` via `serde_json`, graceful corrupt-file fallback (web-term pattern: rename to `.bak`, regenerate defaults). **No DB, no tmux state duplication** (PRD principle).

## Sources

- `E:\Coding Stuff\web-term\desktop-gpui\Cargo.toml` — authoritative final pin set (HIGH; supersedes the older 20-RESEARCH §1 table which was written before the `gpui-pre` pivot)
- `E:\Coding Stuff\web-term\desktop-gpui\Cargo.lock` — resolved versions of the full gpui-pre family, wgpu 29.0.4, fontconfig/x11/wayland stack on Linux (HIGH)
- `crates.io API` for `gpui-pre` (=0.3.3 latest), `gpui-component` (=0.6.0 latest), `alacritty_terminal` (=0.26.0 exists; 0.25.1 pinned deliberately) — fetched 2026-09-06 (HIGH)
- Vendored crate source inspection (`~\.cargo\registry\src\…gpui-pre-0.3.3`, `gpui-pre-windows-0.3.3`, `gpui-pre-linux-0.3.3`) — `App::prompt_for_paths`, `PathPromptOptions { files, directories, multiple, prompt }`, Windows IFileOpenDialog folder flag (HIGH)
- Reference app code: `crates/webterm/src/{main.rs,icons.rs,bundle.rs}`, `crates/{terminal,backend-client,supervisor,settings}/Cargo.toml` — API usage patterns, font registration, inline-SVG rendering, backend path resolution, DWM titlebar (HIGH)
- `E:\Coding Stuff\web-term\.planning\phases\20-desktop-foundation-backend-integration\20-RESEARCH.md` — supervisor design + hand-shake contract rationale, original pin rationale, Terraform-style risk list (HIGH for repo facts; superseded §1 version table)
- `E:\Coding Stuff\web-tmux\desktop\main.js` — the actual web-tmux Electron handshake (BACKEND_PORT stdout regex; `--port 0` argv shape) (HIGH)
- `scripts/{package-windows.ps1,package-linux.sh,build-test-backend.sh}`, `tools/fxc/README.md` — Windows fxc landmine + bundle layout + test-backend workflow (HIGH)

---
*Stack research for: web-tmux desktop-gpui frontend (v1.0 Desktop GPUI milestone)*
*Researched: 2026-09-06*
