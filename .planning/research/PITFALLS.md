# Pitfalls Research

**Domain:** Native GPUI (Rust) desktop frontend added to an existing tmux GUI (web-tmux: Go backend + React FE + Electron + GTK4)
**Researched:** 2026-09-06
**Confidence:** HIGH for all findings tied to the reference implementation (`web-term/desktop-gpui`) and the web-tmux protocol — verified by reading source; MEDIUM for external metadata (crates.io API, tmux Control-Mode wiki)

**Reading guide for the REQUIREMENTS author / roadmapper:** this milestone is unusually low-risk on architecture (`web-term/desktop-gpui` already proved the 5-crate workspace, supervisor, and terminal engine on the exact same stack) and unusually high-risk on *faithful copying* — the two failure modes are (a) accidentally deviating from the proven pins/patterns, and (b) web tranglating the React/tmux-specific protocol semantics. Every pitfall below carries "copy the reference" or "contract-test against the backend" as its primary defense.

---

## Critical Pitfalls

### Pitfall 1: The gpui version trap — public `gpui`, `gpui-pre`, and `gpui-component` version skew

**What goes wrong:**
- Adding an external crate that depends on `gpui` (the crates.io name) while the workspace pins `gpui-pre` (the pre-release fork channel `gpui-component` 0.6.0 requires) compiles **two incompatible GPUIs into one binary**: `Entity<T>`, `Window`, `App`, `Context` types cannot mix, and duplicate global platform registration panics at startup. Verified in web-term's Phase 21 spike: `gpui-terminal 0.1.0` (pins `gpui = "0.2.2"`) is a **fatal conflict** with `gpui-pre =0.3.3` — NO-GO.
- `cargo update` or a newly added dep silently bumps a `0.x` minor across a breaking release; the build breaks weeks later with no local change. All three core crates churn hard: public `gpui` is at 0.2.2 on crates.io with a history of yanked versions, while `gpui-component` has already published 25 versions (crates.io API, 2026-09-06 — MEDIUM confidence).
- Pointing web-tmux at the *public* `gpui = 0.2.2` instead of web-term's proven `gpui-pre =0.3.3` "because it's the official name" breaks `gpui-component =0.6.0` compatibility — the entire reason the reference exists in its pinned shape.

**Why it happens:**
All core deps are pre-1.0, so `minor = breaking`; Rust's default flow (loose semver + lockfile) hides the breakage until the first `cargo update`. Devs also default to the crate name they know (`gpui`) instead of the fork channel (`gpui-pre`) that gpui-component actually binds to.

**How to avoid:**
- Copy web-term's workspace pins verbatim — they are proven as a set, not individually: `gpui = { package = "gpui-pre", version = "=0.3.3" }`, `gpui-pre-platform = "=0.3.3"`, `gpui-component = "=0.6.0"`, `alacritty_terminal = "=0.25.1"`, `tokio = "=1.53.1"`, `tokio-tungstenite = "=0.26.2"` (default-features off, `connect`+`handshake`), `reqwest = "=0.12.28"` (rustls-tls — avoids the OpenSSL system dep on Linux), `flume = "=0.12.0"`, `parking_lot = "=0.12.5"`.
- Exact `=` pins for every core dep + **commit `Cargo.lock` as the first artifact** (web-term commits it; verified in git).
- Never add an external crate whose dependency tree contains any GPUI flavor not already in the tree — check with `cargo tree -p <name>` before adding. When a starter crate helps (e.g. an alacritty+GPUI terminal), **vendor its architecture** into the workspace `terminal` crate instead (web-term's proven move).
- Upgrade only in deliberate, isolated commits: bump → `cargo build --workspace` + smoke test → verify gpui-component ↔ gpui-pre alignment.

**Warning signs:**
Two gpui versions (or `gpui` AND `gpui-pre`) in `cargo tree`; "build works on the lockfile from last month"; a dep added for one feature pulls its own gpui.

**Phase to address:**
Phase 1 (workspace foundation) — pins + committed lockfile + build CI is the first task, before any feature work. Also gov: first terminal-crate task repeats Phase-21-style check for any tempting external gpui-terminal crate.

---

### Pitfall 2: Threading model — async WS/REST tasks vs the GPUI main thread

**What goes wrong:**
Tokio primitives (timers, channels, reqwest, tungstenite) used from a GPUI callback hang or panic because GPUI's main thread is not on a Tokio runtime context; or, in the opposite direction, WS pump tasks try to update GPUI entities directly from a background thread — invalid `Context` access, missed repaints, or UB-style diagnostics. Also classic: every inbound WS chunk calls `entity.update()` directly from the pump and never checks weak-entity upgrade, so pumps keep running forever after a tab closes.

**Why it happens:**
GPUI entities are main-thread-only by design; async reactivity (`cx.spawn` + `AsyncApp`) feels like React's useEffect but requires explicit bridging: weak-entity upgrade, `cx.update()`, and `cx.notify()` every time off-main-thread state lands in UI state.

**How to avoid (all verified in web-term source — HIGH):**
- Create ONE `LazyLock<tokio::runtime::Runtime>` (multi-thread, `enable_all`) and **`.enter()` it on the main thread at the top of `main()`**, before GPUI starts. This single guard is what makes timers/channels/reqwest work across the main GPUI thread and foreground async tasks.
- All REST/WS/pump work: `TOKIO_RT.spawn(...)` → results returned via an mpsc/flume channel → consumer is a GPUI `cx.spawn`/`AsyncApp` task that does `weak.upgrade()` → `entity.update(cx, |this, cx| { ...; cx.notify() })`.
- Every async loop must treat `cx_handle.update(...) == false` (or `None` upgrade) as a **break signal** — the latch that makes pumps die with their view. Both the terminal event pump (`view.rs`) and the WS output pump (`session.rs attach_handle`) do exactly this.
- Keep the supervisor crate Tokio-only (no GPUI dep) so spawn/handshake/readiness logic is testable headless — web-term's explicit validation architecture.

**Warning signs:**
UI updates only when the user clicks; tokio timers mysteriously never fire on the main thread; `entity.update` panics about wrong thread after a tab closes; open sessions keep painting after their tab was closed.

**Phase to address:**
Phase 1 (foundation) — establish the tokio↔GPUI bridge pattern (runtime guard, channel-back pump, weak-entity upgrade) in the first vertical slice; every later phase then copies it. Require it in plans as a named pattern, not ad-hoc wiring.

---

### Pitfall 3: Resize storms — canvas-driven grid resize vs tmux pane geometry

**What goes wrong:**
The GPUI terminal pane measures its cell grid during the paint pass; every layout change (window resize, sidebar toggle, pane-split drag, zoom) recomputes cols/rows and fires `term.resize` + a `terminal.resize`/`pane.resize` message to the backend. Without throttling/dedup, a single drag flop sends dozens of resizes; tmux then reflows the pane repeatedly and the backend's geometry commands (and control-mode stream) stutter — visible as flicker and, on slow tmux servers, as temporary desync between GPUI grid size and actual tmux pane size.

**Why it happens:**
In web-term the delta-check is built in: the render closure computes cols/rows and calls `term.resize()` **only when dimensions actually changed**, then fires the resize callback (view.rs). Porters copying "resize = recompute + send" from the React side (ResizeObserver debounced at 80ms in `useTerminal.ts`) lose that debouncing the moment they drive resize directly from layout/paint — GPUI has no ResizeObserver, and paints fire far more often.

**How to avoid:**
- Exactly the web-term pattern: keep the **resize check inside the render path, keyed on actual (cols, rows) change** (`if cols != term.cols() || rows != term.rows()`), never unconditional.
- Reconcile GPUI-side and tmux-side: keyboard/pane-grid resizes go through the existing `pane.resize` GUI action (direction/amount), while PTY-geometry resizes flow as `terminal.resize` messages from the measured local grid. Don't conflate them.
- Keep a debounce floor (~web's 80ms equivalent or a "max one send per frame-paint that changed dims") so rubber-band window resizing doesn't flood the WS.
- On reconnect or snapshot application, re-send the current grid dims — tmux needs them and a stale size after re-attach produces clipped output.

**Warning signs:**
Backend logs full of `terminal.resize`; pane flickers during a sidebar toggle; after zooming a pane and unzooming, the text wraps differently than the grid shows.

**Phase to address:**
Phase for pane-grid/terminal embedding (the main pane-grid phase). Verify with a drag-resize test that produces one resize message per settle.

---

### Pitfall 4: Snapshot-on-open semantics — capture ≠ stream buffer

**What goes wrong:**
Each tmux pane's terminal is hydrated by a full capture (`terminal.capture` → `terminal.snapshot` with `screenRows`), then continued by live `terminal.output` frames. The classic error is treating the snapshot as *more bytes to append*: scrollback fills with duplicated rows, or the screen is drawn from the wrong line boundary (the capture's leading rows are tmux **scrollback above a `screenRows`-tall visible window** — protocol.ts documents `screenRows`, and sockets.ts routes snapshots with `writeSnapshot`/`replaceScreen` **never append**). In GPUI there is also no xterm-style "clear + write" helper: the alacritty grid must be reset and the capture replayed explicitly.

**Why it happens:**
The web frontend's terminalRegistry cleanly separates `write` (append) vs `writeSnapshot`/`replaceScreen` (replace). A GPUI port has to implement both code paths for `Term<...>` by hand, and the split of a capture blob into (optional history rows | screen rows) is easy to get wrong when one sets of `screenRows` semantics is shared across `terminal.snapshot`, `terminal.output {replace:true}` (integrity frame), and live output.

**How to avoid:**
- Three explicit grid insertion modes mirroring `terminalRegistry` exactly: `write(ahead)`, `write_snapshot(data, screenRows)` (wipe grid → scrollback rows → screen rows), and `output(replace=true, screenRows)` (full-screen integrity replacement — count leading rows above `screenRows` as scrollback).
- Introduce a **grid-wipe + replay primitive** on the terminal engine (alacritty `Grid` reset via fresh `Term` or explicit clear + explicit history fill) rather than hoping `process_bytes` replay approximates it.
- Wire the backend's own ordering to your advantage: the pipe stream arms with a snapshot barrier (buffered | overflow → `replace` integrity frame on `streamBarrierCap` overflow), so if the client honors `replace` you can never double-count.
- Per pane: the initial capture is requested when the pane workspace mounts (web: `tmuxSocket.terminalCapture(paneId)` right after registering the terminal). In GPUI, request it immediately after the WS session for the tab's session connects, and after `pane.select` when opening an inactive pane of an already-open window.
- Respect the backend's `screenRows` everywhere scrollback is reconstructed; never assume a capture is exactly viewport-sized.

**Warning signs:**
Doubled prompt lines on tab open; scrollback rows duplicated after reconnect; output missing entirely on a slow pane until the next keystroke.

**Phase to address:**
The per-pane terminal embedding phase (same phase as Pitfall 3, likely). Contract-test: open → capture → live stream must render exactly once through the test harness.

---

### Pitfall 5: WS-per-tab lifecycle mistakes — serials, stale sessions, and wrong banners

**What goes wrong:**
- One WS **per tmux session tab** (`/api/ws?session=<name>`). Porters coming from web-term (one WS per SSH/PTY tab) or naive singletons from web punch adulthood mistakes: closing a tab leaves the WS open (backend keeps running its monitor), or opening a second session's tab tears down the first's WS.
- **The generation-counter race**: a socket whose reconnect loop resurfaces after the user switched sessions can deliver *the wrong session's snapshots*, making the UI flip between two sessions' windows. The web FE explicitly bumps `generation` on every `open()` and rejects superseded sockets' `onclose` (no reconnect) and stale-messages; and dispatch double-checks `msg.session !== this.session` before applying a snapshot (verbatim from `websocket.ts` — verified). Skip it and you'll debug "UI blinks between sessions" for days.
- Treating app restart, WS drop, and tmux-level `tmux.disconnected/reconnecting` as the same state. They aren't: WS-drop = transport banner + autos backoff reconnect (250ms → 10s); `tmux.reconnecting`/`tmux.disconnected` = the tmux server/control connection changed state and the app must banner accordingly and then re-request full state (`state.resync`) once reconnected.
- Reconnect-resume without a fresh `state.resync` after (re)connect — the web explicitly requests live state after (re)connect because a reconnected monitor can produce a new full snapshot, and pane windows may have changed while away.

**Why it happens:**
The multi-session store (per-session `sockets` Map keyed by name + facade proxy to the active session) is easy to re-implement subset: `getOrCreateSocket` also exists so messages queued *before* connect get flushed on `onopen` — easy to skip, leading to "first keystroke after tab open silently dropped".

**How to avoid:**
- Mirror the web FE structure 1:1: `ensureSocket(session)` on tab open, `closeSocket(session)` on tab close, generation counter in the connection object, queued-send before connect, backoff 250/500/1000/2000/5000/10000ms.
- On `connection.ready` or new-generation connect: flush queue → send `hello(cols, rows)` → request `state.resync`.
- Banner states in the desktop must map: `connecting` (spinner in tab), `reconnecting` (amber banner with attempt counter), `disconnected` (red banner with reason + manual reconnect/close) — web-term's `reconnect_banner.rs` is the proven GPUI component; restyle it for web-tmux colors.
- Track pane-id scoping: pane IDs are unique per tmux server, so output routing by paneId across sessions is unambiguous — the web FE relies on this; keep it (don't add session-prefixed pane keys, they'd desync from protocol messages).

**Warning signs:**
The store alternates snapshots between sessions after rapid tab switches; a closed tab's backend monitor keeps firing captures inside backend logs; after a backend reconnect, the UI keeps stale window lists until a manual interaction.

**Phase to address:**
Session/tab lifecycle phase (and the supervisor/backend-client phase sets the transport). The generation counter + wrong-session-drop guard should be a named contract test (`tests/…, chaos_test` analog: kill the WS mid-session, kill the backend process mid-session).

---

### Pitfall 6: Mouse reporting vs selection — two worlds, both must hurt correctly

**What goes wrong:**
tmux panes commonly enable mouse mode (mouse-mode on): when the alacritty `TermMode` reports `MOUSE_REPORT_CLICK/MOTION/DRAG`, raw clicks must be encoded as SGR 1006 sequences and forwarded — not consumed for text selection. Conversely, when reporting is off, drag must produce a *selection* and wheel must scroll the GPUI-side scrollback (the grid), *not* send anything. Common porting failures:
- Sending wheel input as mouse sequences when tmux mouse mode is on but the pane's terminal view has its own wheel-to-`PageUp/PageDown` ("TUI scroll") policy — web-tmux's FE *intercepts the wheel in capture phase* before the terminal gets it and translates notches → burst-clamped `\x1b[5~/\x1b[6~` repeats (with accumulation for trackpad pixel deltas). A GPUI port that just wires the reference's `scroll_report` misses this tmux-specific UX entirely.
- Pixel→cell mapping wrong at boundaries: ignoring canvas padding, not clamping to `cols-1`, not flooring negative positions (reference `mouse.rs` clamps both dims and uses the captured `last_bounds` + padding origin).
- Selection state left "active" when focus is lost, or `update_selection` called against a `Term` that has since been resized (point out-of-bounds panic).
- Modifiers not flattened into SGR button bits (shift=4, alt=8, ctrl=16) — selection-style bugs in TUI apps like midnight commander.

**Why it happens:**
The reference mouse path (SGR reporting, wheel-64/65, alt-screen scrolling, click-count selection types) was written for PTY-style shells; web-tmux adds a per-pane "wheel = page keys for TUI panes" toggle that must survive the port.

**How to avoid:**
- Vendor the reference `mouse.rs` + `input.rs` translation tables verbatim (they're unit-tested: SGR `\x1b[<btn;col;rowM/m`, wheel 64/65, alt-screen arrow keys with `APP_CURSOR` handling).
- Add web-tmux's wheel policy on top: a per-pane toggle in pane settings maps wheel notches → PageUp/PageDown repeats (respecting `WHEEL_NOTCH_PX` accumulation and `WHEEL_MAX_BURST=3` clamp) when "TUI scroll" is enabled; reference behavior otherwise.
- Clamp+ranges all pixel→cell conversions; compute point *after* checking bounds exist; `drop(term)` before writing bytes to avoid holding the term lock during send.
- Shift-drag must still start a selection even under mouse reporting (adb: escape sequences in reporting mode report the modifier so tmux sees it).

**Warning signs:**
Selecting text in a vim-powered pane opens link/helper UIs; copy after select doesn't work in some TUIs; wheel scrolls *contents* of a tmux-captured pane by terminating history oddly; selection ghost rectangles across pane splits.

**Phase to address:**
Terminal rendering/embedding phase. Verify by porting the wheel-policy toggle + selection copy tests straight from the FE contract (`useTerminal.ts`).

---

## Moderate Pitfalls

### Pitfall 7: Theme parity drift — CSS variables and generated presets vs GPUI palettes

**What goes wrong:**
- xterm happily consumes *raw CSS strings* like `var(--term-bg)`; GPUI needs concrete RGBA/HSLA — so the "fallback to CSS variables" path in `useTerminal.ts` cannot port. Porters hardcode the *current* panel instead and lose the default-theme terminal colors entirely once the preset list evolves.
- GPUI's `Hsla` conversion is not render-identical to DOM: go through `Rgba { r,g,b,a }` → `.into()` to avoid naive HSL round-trips washing out saturated ANSI colors (the FE's ANSI 16-entries + `terminal-themes.ts` palette). Also selection must keep alpha (`0x3b82f666`-style, alpha 0x66/0x88 in the reference) — porters commonly flatten alpha to 1.0 and lose the blue/glass overlay effect.
- Electron's dark/light is driven by DOM classes + CSS variables; GPUI themes must be a typed static table (web-term's `theme.rs :: ThemePreset { id, label, is_dark, …14 named u32 values }`) ported per-preset from `fe/src/features/settings/data/ui-themes.ts` — no dynamic CSS variable resolution exists.

**Why it happens:**
The web data files are generated TS objects; a hand-copy invites typos/missing keys, and URL round-trip differences (e.g. `rgb(...)` vs hex, colorspace gamma handling in Rust vs CSS) silently diverge.

**How to avoid:**
- Port *machinely*: script or careful table-copy of `ui-themes.ts` + `terminal-themes.ts` into typed Rust tables (never freehand-tune while porting); then diff hex values by CI or grep (there is also explicit risk of the generated file being regenerated later — re-run the port script on parity audit).
- Synchronize: one app-level theme switch must update (a) gpui-component theme, (b) each `TerminalView`'s palette (`set_palette` propagating live), and (c) the terminal's grid colors — reference does all three in `set_theme_preset` without app restart.
- Terminal palette completeness: build the full ANSI-256 lookup (16 base + 6×6×6 cube steps `[0x00,0x5f,0x87,0xaf,0xd7,0xff]` + 24 grays `8+i*10`) not just 16 — full-screen TUI apps land heavily in the 16..231 range.
- Mid-session theme toggle test: switch Dark→Light in Settings and assert sidebar, tabs, terminal grid, and prompts all re-render with no restart (web-term has `settings_theme_test.rs` + `palette_update_test.rs` for this).

**Warning signs:**
"Tint slightly off from the Electron app", scrolling through a `ls` colorized output renders some cells near-black, theme change needs restart, or changing `--term-color-N` VS doesn't change terminal.

**Phase to address:**
Settings/theme porting phase; verified in the late parity-audit phase.

---

### Pitfall 8: Windows specifics — DWM, fonts, paths, and process spawn

**What goes wrong:** (all items verified in web-term source — HIGH confidence):
- **DWM title bar stays white in dark theme** unless `DwmSetWindowAttribute(hwnd, 20, dark, 4)` (and attribute 19 for fallback) is called with the Win32 hwnd — obtained via `raw-window-handle 0.6.x` inside `open_window`'s callback, not earlier (the handle may not resolve pre-window). Requires manual `extern "system"` decl (web-term pattern) or windows-sys.
- **Fonts**: GPUI does NOT reliably inherit system defaults; the app must `include_bytes!` the mono font (`JetBrainsMono-Regular.ttf`) and register it via `cx.text_system().add_fonts(...)` BEFORE any text shaping — the terminal renderer's `measure_cell('M')` collapses to garbage if resolution happens before successful font load. Registering late or with a different family name string than the renderer uses yields UI metrics ≠ terminal metrics.
- **Paths**: settings directory via `dirs::config_dir()` (i.e. `%APPDATA%`), DB path passed as an absolute absolute (env `WEBTERM_DB_PATH`) so backend cwd is irrelevant. Backend binary resolution: settings override → adjacent-to-exe bundle layout (`.exe` suffix) → dev candidates list.
- **Spawn UX**: `WEBTERM_PORT=:0` + stdout `BACKEND_PORT:` handshake (never poll-guess ports); env-only secret key (never argv); `kill_on_drop(true)` plus explicit graceful stop; if a release build uses `#![windows_subsystem = "windows"]` keep the **child's stdout** alive for the handshake while the parent has no console (dev builds keep the console for logs).
- Console access for `.exe` resolution: current_exe paths, MSYS2/winget tmux discovery (the backend handles; assert in the desktop's Settings page which binary the backend claims).

**Phase to address:**
Foundation phase (DWM/font bootstrap in the GPUI window skeleton) and supervisor-in-foundation phase for spawn details. Windows CI build as acceptance.

### Pitfall 9: Linux specifics (X11/Wayland, fonts, dynamic-linking)

**What goes wrong:**
- Pin the platform explicitly (`Application::with_platform(gpui_platform::current_platform(false))` as web-term does); GPUI builds target X11 on Linux — on Wayland sessions, implicit platform auto-detection can surprise-map Wayland → X11 (or fail), so assert both desktops in the hardening phase.
- Cross-platform pitfalls copied from web-term's research: with `default-features = false` on reqwest + `rustls-tls`, OpenSSL disappears from the dependency path — critical for Linux CI builds (web-term's 20-RESEARCH rationale, verbatim).
- Font fallback: on minimal Linux setups (containers, Arch/i3 minimal desktops), *unregistered* font families silently degrade to tofu — bundle the mono font and set WM shell **font fallbacks only after registration**.
- GTK4 desktop (existing) + GPUI desktop (new) run side-by-side: ensure no config/data path collision between the two — use distinct `dirs::config_dir()` folder names and distinct window titles.
- Window icon/decoration: reference relies on `appears_transparent` titlebar + custom TitleBar; on some WMs custom frameless titlebar handles may misbehave without explicit decoration setting (`titlebar` + `appears_transparent` in `WindowOptions`).

**Phase to address:**
Shell/UI phases; cross-platform acceptance in the final hardening phase (web-term's Phase 27 pattern).

---

### Pitfall 10: SVG icon embedding pitfalls

**What goes wrong:**
- GPUI's SVG renderer supports a subset of lucide; icons relying on masks/defs/gradients render blank. Keep to the stroke-path subset (web-term embeds lucide SVGs verbatim as `pub const *_SVG: &[u8]` `br#"<svg .../>"` constants in `icons.rs`).
- `currentColor` semantics: GPUI `svg().text_color(...)` is what recolors; a colorless icon renders black/first-path-color — must call `.text_color` on **every** site or theme changes leave black icons on dark themes (built-in parity risk).
- Size mismatch: web uses 16/20/24px lucide sizes; hardcoding one size loses pixel-perfect parity with Electron — store each icon usage's size explicitly alongside the SVG constant.
- Do NOT embed icons via file paths at runtime (`AssetSource` indir); inline `&[u8]` constants (web-term pattern) have zero runtime IO and are byte-compared to the FE's icon set at parity-audit time.

**Warning signs:**
An icon renders in one theme only; a missing icon shows as an empty rect; icons look 1:1 on Windows but slightly bolder on Linux (font fallback affecting glyph backgrounds) — usually a font metrics change, not icons.

**Phase to address:**
First UI-shell phase that renders the sidebar/title bar (icons.rs is the pattern), then the parity audit.

---

## Minor Pitfalls

- **Workspace layout mistakes**: (a) putting `gpui` as a dependency of the supervisor/settings/backend-client crates couples the headless-test crates to the GUI stack — web-term keeps them gpui-free so `cargo test -p webterm-supervisor` runs headless in CI (20-RESEARCH "Validation Architecture"); (b) forgetting `[workspace.package]` to unify version/edition across 5 crates; (c) forgetting `resolver = "2"`; (d) mixing unsafe `openssl-sys` in the Linux build tree by leaving reqwest default features on.
- **App-state decomposition**: web-term keeps one root `AppState` entity with all views (simpler); do not over-decompose into dozens of entity types before the first vertical slice proves the app-shell wiring.
- **Window-state persistence race**: restore `WindowBounds` before `open_window` (web-term's `window_state::restore` pattern) — a port that restores size after opening flashes at default geometry every launch.
- **Log noise**: WS terminal bytes are shell traffic — never `eprintln!` them (secret leakage / log explosion); log transport state only (web-term logs taps metadata, not payloads, in supervisor and backend-client).
- **Dev/release subsystem split**: dev builds keep console for logs, release uses `windows_subsystem="windows"`; CI must build both configurations once so the handshake-by-stdout path is exercised in both.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Adding the crates.io `gpui`/`gpui-terminal` crate instead of vendoring the terminal engine | One less crate | Version-skew landmines, duplicate runtime panics, unmaintained dep | Never |
| Free-form reqwest calls in views | Faster early screens | Untestable UI, protocol drift with the Go backend | Never — keep the `backend-client` crate boundary (web-term pattern) |
| Hand-rolled DTO structs without contract tests | Quicker client work | Silent breakage on backend changes three frontends depend on | Prototype only; add contract tests by the first pane session phase |
| Single-crate GPUI app | Simpler start | gpui churn infects app+test code; supervisor can't be tested headless | Never for this milestone's risk profile |
| Copying React layout ad-hoc rather than reading the Electron app's actual component tree | Divergence from "UI identical to Electron" requirement on day one | Every screen re-argued at parity audit | Never (parity is an explicit constraint) |
| Sending tmux commands via pooled shell strings instead of typed WS messages | Feels expedient | Bypasses requestId correlation and the "client never sends raw tmux" backend invariant | Never |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Go backend spawn (supervisor) | Polling for a fixed port / assuming port 8080 is free | `WEBTERM_PORT=:0` + stdout `BACKEND_PORT:` handshake + ≤15s timeout + readiness probe on `/api/health` + stderr tail surfaced to UI on failure |
| Per-session WS | Opening one WS for the whole app | One WS **per open session tab**, opened on tab open, closed on tab close (backend caps monitor per session; closing frees server resources while tmux session persists) |
| tmux stable IDs | Using window/pane index instead of `@N`/`%N` | Always send GUI actions by stable ID; `index` is display-only (tmux-types.ts: "Stable IDs: session = name, window = @N, pane = %N") |
| `session.rename` | Renaming then continuing to use the old-name-keyed socket | After rename, re-resolve the socket by new name (and re-open if the tab's identity keyed by name) — verify against the Go backend behavior with a contract test; do not silently cache the old WS URL |
| `hello(cols,rows)` | Skipping initial resize | Present viewport dims via `hello` right after WS open so the backend sizes the tmux window before the first snapshot (handler.go: hello with cols/rows → ResizeTerminal) |
| requestId correlation | Fire-and-forget mutations | Every mutating GUI action carries `requestId`; only resolve UI state on matching `command.success`/`command.error` (web FEs abort on `command.error`) |
| Two frontends, one tmux | Assuming desktop "owns" tmux | tmux stays source of truth; client-side alacritty grid is a *renderer*, not an authority — `state.resync` and `tmux.disconnected/reconnecting` events must refresh it |
| OS-specific tmux | Assuming `tmux -CC` works on Windows | The Go backend uses control mode only on POSIX; Windows monitor polls captures + pipe-pane streams (`control_windows.go`). The desktop frontend must not assume `-CC` semantics — rely on the WS contract |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Blocks on the main thread (REST, blocking WS reads) | Frozen UI during disconnect / slow backend WS | All backend I/O on `TOKIO_RT`; results via channel→`cx.spawn` (web-term pattern) | First slow/lossy network op |
| Per-cell per-byte text shaping | Fan spins during `yes`-style output; slow vim | Batched text runs by identical style + merged background quads (reference `render.rs`); `cx.notify()` is coalesced by GPUI, but **do** keep per-chunk `process_bytes` on the main pump minimal | Full-screen redraw under heavy output |
| Unbounded WS→grid queue | Memory climbs during build-log floods | Bounded channels and coalescing (web-term binds flume channels and lets the barrier drop buffers to an integrity frame rather than growing forever) | Log storms over slow network |
| Re-shaping the 'M' measurement every frame at full grid | Frame hitches while resizing sidebars | Keep `measure_cell` per-paint (reference does this; it is cheap) but never shape per-cell: reuse `BatchedTextRun` shapes | Very high grid sizes (>200 cols) |
| Painting the whole window on every terminal chunk | Blurry redraw UX, CPU spike during compile dumps | Only `cx.notify()` from the owning terminal's context, not the app root entity | Multiple panes streaming simultaneously |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Passing tmux/backend paths or secrets via argv | Visible in Task Manager/process listing | Env vars only (supervisor `SpawnOptions` asserts "no secret material in argv" via test) |
| Logging terminal bytes or WS payloads (debug `eprintln!`) | Shell secrets exposed in plaintext logs | Log transport state only; strip payload logging in `backend-client` |
| Backend bound to 0.0.0.0 while the desktop assumes loopback | LAN-exposed tmux proxy on user machines | Keep the backend's loopback-only env (`127.0.0.1`) in the spawn config |
| Reusing a stale backend from a previous crashed run (adopted port) | Talking to an orphan serving the wrong app state | `adopt_or_clear(base_url)` probe from the supervisor, plus the handshake; never assume recorded port is ours |
| Unvalidated `Settings::backend_path` override | `spawn` of arbitrary binaries | Validate + surface errors (web-term treats it as a dev override; treat unknown paths as failures with stderr tail surfaced to UI) |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Terminal "works in dev, never exercised under vim/htop" | Doesn't pass daily-driver test | Explicit verification checklist: vim edit, `htop`, copy from mid-screen output, paste a multi-line snippet with Ctrl+Shift+V |
| Connecting commands without user feedback until tmux re-flow arrives | UI feels laggy/uncertain | Show spinner in pane header; resolve on `command.success`/`error`; the web FEs set this pattern |
| Missing empty/error/select states | Greys areas appear broken | Port Electron's Create Session dialog, empty, error, session-select states verbatim (they exist in the web FE for a reason) |
| No backend start progress feedback | App feels hung on cold start (sidecar boot 200–800ms+) | Show backend startup state + stderr tail diagnostics when readiness fails |
| Forgetting tmux's dual control | Users confused why "mouse wheel" toggled between page and scrollback | Keep the per-pane "TUI scroll panes" toggle (wheel → PageUp/PageDown) from the web FE survived in the GPUI context menu |

## "Looks Done But Isn't" Checklist

- [ ] **Terminal:** verify vim edit, `htop`, colored `ls`, scrollback continuity after kill+reconnect, `clear`-then-snapshot parity
- [ ] **Multi-session tabs:** open 2+ sessions rapidly, verify per-session socket, resizing pane in one doesn't reflow other session, closing tab frees backend monitor (check server logs)
- [ ] **Resize:** drag window edge/pane divider and confirm *no* repeated resize floods + grid==tmux dims after settle
- [ ] **Reconnect banner:** WS drop (backend kill) shows reconnect with backoff countdown; tmux session death shows `disconnected` banner with a different copy/action set
- [ ] **Rename session mid-session:** WS keeps working; state snapshot reflects new name; no blinking between sessions
- [ ] **Themes:** change theme mid-session; assert sidebar, terminal grid, prompt colors and fonts all swap without restart (web-term regression checklist carries over here)
- [ ] **Icons:** hover/toggled/disabled states of every toolbar item match Electron 1:1 (colors too, via `text_color`)
- [ ] **Settings:** tmux binary path override, scrollback limit, font family/size — all take effect without re-spawn (or only the documented ones)

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Dep skew discovered late | LOW-MEDIUM | Roll lockfile back to web-term's known-good pin set; re-run CI |
| Wrong-thread panic discovered | MEDIUM | Convert each spawn point to the channel→`cx.spawn` pattern; add async-pump contract tests |
| Resize storm traced to tmux | LOW | Wrap the change-detection around resize callback; re-verify with a drag log |
| Scrollback garbage / duplicate rows from capture-on-open | LOW-MEDIUM | Add the grid-wipe + replay primitive; ensure `screenRows` is honored in `replaceScreen` path |
| Session blur/flip between two sessions | LOW | Add the generation counter and wrong-session guard, rerun rapid-switch test |
| Theme drift | LOW | Re-run the palette port script, re-audit with `settings_theme_test`-style diff between Rust tables and the TS source |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| gpui/gpui-component/alacritty pinning, no external gpui-terminal | Phase 1 (workspace foundation) | Lockfile committed; build+smoke CI; `cargo tree` shows single gpui-pre |
| Threading bridge (tokio ↔ GPUI) | Phase 1 (foundation vertical slice) | Background-fetch test proves `cx.notify` still repaints after WS disconnect |
| Resize throttling + hello | Phase for terminal embedding | Drag-resize test → exactly one resize msg; hello carries cols/rows |
| Snapshot/integrity-frame semantics | Terminal embedding phase | Capture→output roundtrip test asserting `replaceScreen` semantics and no duplicate scrollback |
| WS lifecycle & generation counter | Session/tab lifecycle phase | Rapid open/close/switch test; backend monitor frees on tab close |
| Mouse wheel policy & selection | Terminal embedding phase | Copy from a tmux pane with mouse mode on; TUI toggle forwards PageDown bursts |
| Theme parity | Settings/theme phase | Table diff test vs TS presets; mid-session toggle test |
| SVG icons | First UI-shell phase | Icon list diff vs FE; check `text_color` coverage |
| DWM/fonts/paths (Windows) | Phase 1 (window skeleton) | Dark title bar + no tofu glyphs on Windows CI |
| Linux build (X11/Wayland) | Cross-platform hardening phase | Linux CI build + smoke run + GTK4/GPUI.setState clashes |
| Workspace topology | Phase 1 | `cargo test -p <supervisor-like>` runs headless |
| Stable IDs, rename handling, InputBatcher trust | Protocol-integration phase | Contract tests asserting `@N`/`%N` inputs and requestId correlation |

## Sources

**Verified by reading reference/application source (HIGH confidence)**

- `E:\Coding Stuff\web-term\desktop-gpui\`:
  - `crates\webterm\src\main.rs` — Tokio runtime guard, DWM attrs 19/20, bundled JetBrains Mono, `Application::with_platform`, `WindowOptions.titlebar`/`window_min_size`.
  - `crates\webterm\src\app_state.rs` — `TOKIO_RT` LazyLock, REST fetch → mpsc → `cx.spawn/AsyncApp` pattern, off-thread spawn flow.
  - `crates\session.rs`, `crates\webterm\src\views\reconnect_banner.rs` — per-tab WS handle, connect/disconnect lifecycles, reconnection banner design.
  - `crates\terminal\src\*` — alacritty crate wiring, combined mouse/keyboard/streaming, batched rendering, SGR mouse reporting, selection types.
  - `crates\webterm\src\icons.rs`, `theme.rs`, `bundle.rs`, window_state.rs, settings/paths.rs.
  - `Cargo.toml` (workspace-deps, exact pins, committed `Cargo.lock`, rustls-tls).
  - Phase 20/21/27 planning docs (`20-RESEARCH.md` §1 pins, §2 supervisor contract; `21-RESEARCH.md` + `SPIKE-gpui-terminal.md` NO-GO external gpui-terminal + alacritty 0.25.1 vendoring; `27/.../PARITY-MATRIX.md` 100% web-feature parity list).
- `E:\Coding Stuff\web-tmux\be\internal\*` (monitor.go, stream.go pipe-pane -IO combined, input_batcher.go, realtime/handler.go hello→snapshot ordering, OriginPatterns)
- `E:\Coding Stuff\web-tmux\fe\src\lib\*` (websocket.ts generation counter/backoff, sockets.ts per-session lifecycle, useTerminal.ts wheel/capture/registry flow, tmux-types.ts stable IDs, protocol.ts envelope semantics)

**External metadata (MEDIUM confidence — crates.io API + tmux wiki, 2026-09-06)**

- crates.io API: `gpui` max stable = 0.2.2, 7 versions (yank history present); `gpui-component` max = 0.6.0, 25 versions — confirms fast churn in the component library and pin-copy as correct strategy.
- tmux Control-Mode wiki (github.com/tmux/tmux wiki): control-mode protocol constraints (`%begin`/`%end` guard lines, `%output %<paneID>` notifications with octal-escaped bytes, control clients draw no terminal output themselves — apps must use capture-pane, `refresh-client -C` to declare a size, flow-control `%pause`/`%extended-output`). Relevant to the backend's Unix control path; the desktop consumes the WS contract, not `-CC` directly.

---
*Pitfalls research for: web-tmux desktop-gpui (GPUI frontend for a tmux GUI)*
*Researched: 2026-09-06*
