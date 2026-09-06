# Project Research Summary

**Project:** web-tmux (Tmux GUI) — milestone v1.0 "Desktop GPUI"
**Domain:** Brownfield native desktop frontend port (Rust/GPUI) for an existing Go + tmux backend
**Researched:** 2026-09-06
**Confidence:** HIGH — every load-bearing claim is grounded in two local codebases read line-by-line (the proven reference `E:\Coding Stuff\web-term\desktop-gpui` and the port target `E:\Coding Stuff\web-tmux`); external API claims (gpui-component docs, crates.io, tmux wiki) are MEDIUM and validated against the working reference where possible.

## Executive Summary

This milestone is a **frontend-only port, not a greenfield build**. The goal is a native GPUI (Rust) desktop app that replicates web-tmux's Electron UI exactly (1:1) on Windows + Linux, spawning the existing Go backend (`tmux-gui-server`) as a sidecar and consuming the unchanged REST/WS protocol. The decisive research finding: `E:\Coding Stuff\web-term\desktop-gpui` already proved this exact pattern end-to-end on this exact stack — a 5-crate workspace (supervisor + settings + backend-client + terminal + app), sidecar backend spawn on a dynamic port with a stdout handshake, alacritty_terminal rendering inside GPUI, and a 1:1 GPUI re-implementation of a web UI. The correct strategy is therefore **faithful copying**: fork the web-term workspace manifest verbatim (exact `=` pins, committed `Cargo.lock`), reuse/port the crates, and concentrate novel engineering on exactly two places: **(a) the pane grid** (tmux cell geometry → absolute rects, zoom, divider drag → `paneResize`, overlap-based dividers — web-term has one terminal per tab and no pane concept) and **(b) real text input** (web-term ships zero text-entry code; web-tmux needs Create Session, rename dialogs, command palette, and tmux binary path — provided by the already-pinned `gpui-component` 0.6 `Input`/`Dialog`/`Command` components). The other big architectural adaptation is the **model mapping**: web-term is one WS = one terminal = one tab; web-tmux is one WS = one tmux session = N windows → each visible pane is its own alacritty terminal instance, driven by full-replace `state.snapshot`/`state.delta`, with capture-replay ingestion (history-delta counter + `screenRows` split) ported from the Electron `terminalRegistry`.

The recommended stack is **exactly web-term's pinned set, zero new crates**: `gpui-pre =0.3.3` (NOT the upstream `gpui 0.2.2` — gpui-component 0.6.0 binds to the fork channel and mixing them is a hard build break), `gpui-pre-platform =0.3.3`, `gpui-component =0.6.0`, `alacritty_terminal =0.25.1` (deliberately not 0.26.0), tokio 1.53.1 / reqwest 0.12.28 (rustls — no OpenSSL on Linux CI) / tokio-tungstenite 0.26.2 / flume 0.12.0 / serde / parking_lot / dirs. Everything web-tmux needs is already inside the pinned set: lucide icons render as inline `&[u8]` SVG constants (`svg().data()`), the native folder picker ships in gpui-pre (`App::prompt_for_paths` — do NOT add `rfd`), and the only binary asset is the JetBrains Mono TTF via `include_bytes!`.

The dominant risk profile is unusual: architecture risk is LOW (proven reference), **pattern-fidelity risk is HIGH**. The five failure modes research verified are (1) dependency skew — silently pulling a second GPUI flavor or letting a `0.x` minor bump break the build weeks later → exact `=` pins + committed lockfile + `cargo tree` as a named check; (2) the tokio↔GPUI threading bridge → ONE `LazyLock<Runtime>` entered before `Application::run`, channel-back pumps via `cx.spawn`/`AsyncApp`, weak-entity upgrade as the pump-kill signal — established in Phase 1 as a named pattern every later phase copies; (3) terminal-capture semantics — a capture is a **replace**, not an append (history-delta + `screenRows` split), the difference between doubled scrollback and clean resync; (4) WS lifecycle — generation counter + wrong-session snapshot rejection (skipping it produces "UI blinks between sessions" for days); (5) resize storms — resize checks keyed on actual cols/rows change inside the render path, with the FE's debounce+invalidate dance (≈100 ms resize, 150 ms/325 ms layout resync). All defenses are "copy the reference" or "contract-test against the unchanged backend."

## Key Findings

### Recommended Stack

Strategy: **copy web-term's `desktop-gpui/Cargo.toml` verbatim with exact pins and a committed `Cargo.lock`, then add zero new runtime crates.** Full detail in [STACK.md](STACK.md).

**Core technologies (all verified same-day against reference Cargo.lock + crates.io):**
- `gpui-pre =0.3.3` (aliased as `gpui`) — the Zed gpui snapshot fork `gpui-component 0.6.0` binds to; upstream `gpui 0.2.x` in the tree = fatal conflict. Exact pin is crates.io-latest as of research date.
- `gpui-pre-platform =0.3.3` — required for `Application::with_platform(...)`; easily missed.
- `gpui-component =0.6.0` — theme registry, TitleBar, `Input`/`Dialog`/`ContextMenu`/`Tooltip`/`Command`; `gpui_component::init(cx)` at startup. This covers the one substrate web-term lacks (text entry).
- `alacritty_terminal =0.25.1` — terminal emulation engine; the terminal crate's rendering/event pump is written against these APIs. 0.26.0 exists but is a code port with zero benefit → future milestone.
- `tokio =1.53.1` + `reqwest =0.12.28` (rustls) + `tokio-tungstenite =0.26.2` (connect/handshake only) + `flume =0.12.0` — async runtime, REST, per-session WS, sync/async channel bridge into GPUI.
- `serde/serde_json` — DTO mirroring of the Go protocol; `parking_lot`, `dirs`, `thiserror`/`anyhow` — reference glue; `raw-window-handle 0.6.2` — Windows DWM dark titlebar.

**Build/tooling landmines (from STACK.md, must appear in plans):**
- **Windows release builds need the `fxc` drop-in shader tool** (copy the ~100-line zero-crate tool from web-term; wraps `d3dcompiler_47.dll`; exported via `GPUI_FXC_PATH`). Debug builds don't need it. Missing it = the **#1 Windows build failure**.
- Linux build-time packages: fontconfig, xkbcommon, X11+Wayland dev libs, Vulkan loader.
- Bundle layout: backend binary + app exe side-by-side + assets/font; backend path resolution = settings override → adjacent-to-exe (`tmux-gui-server(.exe)`) → dev candidates.
- Spawn contract: `TMUXGUI_HOST=127.0.0.1`, `TMUXGUI_PORT=0`, parse `BACKEND_PORT:<n>` stdout line (Electron's `desktop/main.js` already establishes the same regex), probe `GET /api/health`, then `GET /api/tmux/info`.

**Decisions already resolved by research (don't re-litigate):**
- Use `gpui-pre`, never upstream `gpui`; no `cargo update` mid-milestone.
- Deliberately stay on `alacritty_terminal 0.25.1`.
- Do NOT add `rfd` for the folder picker — gpui-pre 0.3.3 ships `App::prompt_for_paths` with native Windows (`IFileOpenDialog` folder flag) + Linux backends (verified in vendored source). FEATURES.md's earlier "e.g. rfd" assumption is superseded.
- No `AssetSource`/`rust-embed`; icons are inline `br#"<svg.../>"#` constants matching the exact lucide component paths from `fe/`'s node_modules (guarantees pixel parity).
- Never add `portable-pty` or any local PTY stack — tmux via the Go backend is the single source of truth.
- The separately-locked `gpui-pre-reqwest 0.12.15` coexisting with `reqwest 0.12.28` in the lockfile is normal — do not deduplicate.

### Expected Features

Full Electron→GPUI mapping study in [FEATURES.md](FEATURES.md). Every UI surface has a verified implementation path from the reference or the FE source.

**Must have (table stakes — all P1; the milestone's "minimum" is "complete UI surface"):**
- Frameless title bar with sidebar toggle, tmux-window tabs of the active session, drag region, custom min/max/close + DWM dark fix (port `tab_strip.rs`/`window_state.rs` nearly line-for-line)
- Multi-session tabs that all stay connected (per-session `Entity` state; only active session rendered — the GPUI-native equivalent of React's keep-mounted trick)
- Collapsible session→window→pane sidebar tree with 1.5 s REST polling and context menus
- Pane grid from real tmux geometry: `pixelRect`, `closePaneGaps`, zoom fill, 4px divider drag → `paneResize`, layout preset toolbar (**the most complex novel piece, HIGH**)
- Terminal panes: capture/history-delta replay + live `terminal.output`, input via `keystroke_to_bytes`, wheel scroll, selection, clipboard (**HIGH** — capture-replay layer is novel), pane headers
- Create Session dialog (name/cwd/initial command + directory browse) — the **proving ground for gpui-component `Input`**
- Rename/kill dialogs with per-surface confirm toggles; command palette (Ctrl+Shift+P) via gpui-component `Command`
- Settings page: 102 UI theme presets + 78 terminal presets (script-generated from `fe/`'s TS files), font/cursor/scrollback, tmux binary path with validation, 3 kill-confirm toggles
- Empty/error/select-session states, reconnect/status banners, backend start/failed pages with stderr tail

**Should have (differentiators — free or cheap given the stack):**
- Native rendering performance (~10× lower idle memory vs Electron)
- Tooltips with auto keybinding hints (`tooltip_with_action`) and palette rows showing bound keys
- Per-pane TUI-scroll toggle (wheel → PageUp/PageDown bursts for TUI panes — web-tmux-specific policy that must survive the port)
- Optional desktop-native extra: restore open session tabs after restart (Electron restores nothing; tmux sessions survive — decide explicitly in planning)

**Defer (v2+):**
- Sidebar width animation (no CSS transitions in GPUI; binary snap accepted for v1)
- Session keyboard shortcuts beyond the palette (v1.x polish)
- Anything touching the backend protocol; app icon `.ico` embedding via winres

**Anti-features (explicitly avoid):**
- Porting shadcn components one-for-one into_gpui widgets; adopting stock gpui-component widget styling where it diverges from Electron's visual rhythm
- Frame-by-frame animations; CSS-invisibility keep-mounted tricks (use entity persistence instead)

### Architecture Approach

Architecture detail in [ARCHITECTURE.md](ARCHITECTURE.md): keep web-term's proven 5-crate workspace, swap the SSH-PTY WS client for web-tmux's per-session JSON WS, and adapt the app-state model from "one terminal per tab" to "one session tab = snapshot + pane map." tmux owns all state; the app renders `state.snapshot` and never stores derived layout.

**Major components (desktop-gpui/ Rust workspace, pinned):**
1. **`supervisor`** — spawns `tmux-gui-server` (env-only knobs, `--port 0`, stdout `BACKEND_PORT:` handshake, `/api/health` probe, stderr tail, exit watch; **strip `TMUX`/`TMUX_PANE` from the child env** — critical when launched from inside a tmux session). GPUI-free, headless-testable.
2. **`settings`** — JSON at `dirs::config_dir()/<app>/settings.json`; corrupt `.bak` recovery; schema: ui theme/terminal override, font/cursor/scrollback, tmuxBinary, confirm-kill toggles, window state, open-sessions.
3. **`backend-client`** — REST (`health`, `tmux/info`, `tmux/binary`, `sessions` tree/create, snapshot) + **NEW `tmux_ws.rs`** (JSON per-session WS: generation guard, backoff 250 ms→10 s, send queue, requestId correlation, `hello(cols,rows)` → `state.resync`); the only full rewrite among ported crates.
4. **`terminal`** — **port web-term's crate verbatim** (7 modules: terminal/view/render/input/mouse/colors/event); alacritty grid rendering, SGR mouse reporting, batched text runs; add settings-driven scrollback and the web-tmux wheel→PageUp policy.
5. **app crate (`webtmux`)** — single root `Entity<AppState>` **split into modules** (`sessions.rs`, `ws_pump.rs`, `tree_poll.rs`) since web-term's flat 3,837-line app_state was painful; views = free-function render fns; theme.rs generated from `fe/ui-themes.ts` + `terminal-themes.ts` (NOT web-term's palette values); `icons.rs` lucide consts; `window_state.rs`/`bundle.rs` verbatim ports.

**Named patterns every phase must honor:**
- tokio↔GPUI bridge: `TOKIO_RT.enter()` guard → `TOKIO_RT.spawn` → flume/mpsc → `cx.spawn` + weak-entity upgrade + `cx.notify()`; pump break on failed entity upgrade.
- Command correlation: every mutation carries `requestId`; resolve on `command.success`/`command.error`; 10 s forget timeout; create-session is **REST, not WS** (zero sessions case).
- Viewport discipline: report real scaled viewport debounced ~100 ms; `layoutKey` change → resize then forced captures (150 ms/325 ms); capture ingestion preserves the `ingestedHistory` delta guard and `screenRows` split.
- Anti-patterns to reject: one-terminal-per-tab shape, global xterm-style registry, app-derived layouts, binary WS reuse, tmux state duplication (all detailed in ARCHITECTURE.md §Anti-Patterns).

### Critical Pitfalls

Top five from [PITFALLS.md](PITFALLS.md) (each has a prevention phase in the mapping table below):

1. **The gpui version trap** — a second GPUI flavor (e.g. crates.io `gpui` or an external `gpui-terminal` crate) in the tree = incompatible type worlds + duplicate platform panics. Avoid: exact `=` pins verbatim from web-term, commit `Cargo.lock` first, `cargo tree` check before adding any dependency, vendor architectures instead of adding gpui-dependent crates. Prevent: Phase 1.
2. **Threading model** — tokio on the GPUI main thread without a runtime context hangs; background updates via unguarded handles leak pumps. Avoid: single `LazyLock<Runtime>` + `.enter()` before GPUI start (named pattern in the first vertical slice); every pump breaks on weak-entity upgrade failure. Prevent: Phase 1.
3. **Resize storms** — paint-driven resize checks flood tmux with `terminal.resize`s without change-keying + debounce. Port web-term's "resize only when cols/rows actually changed, inside the render path". Prevent: terminal/phom-grid phase; verify with a drag-settle test.
4. **Snapshot-on-open semantics** — capture ≠ append; three explicit grid modes (`write`, `write_snapshot(data, screenRows)`, `output(replace)`), grid-wipe primitive, `screenRows` honored everywhere. Prevent: terminal embedding phase; contract-test capture→live roundtrip.
5. **WS-per-tab lifecycle** — generation counter + wrong-session snapshot drop (`msg.session !== name` → drop), one WS per tab closed with the tab, send queue before connect, `hello(cols,rows)` on open, `state.resync` after (re)connect, and a strict banner taxonomy (WS drop ≠ tmux `disconnected/reconnecting`). Prevent: WS/SessionTab phase; chaos-test killing the WS/backend mid-session.

Also load-bearing (moderate): theme parity drift (machine-port palettes from the TS sources, RGBA conversion path, full ANSI-256 table, alpha kept in selection color, live 3-way theme sync); Windows DWM attr 19/20 + font registration before shaping; Linux `Application::with_platform(...)` pin, rustls (no OpenSSL), font fallback, distinct config dir from the GTK4 app; SVG icon subset (stroke paths only, `text_color` on every render site).

## Implications for Roadmap

Based on combined research, a **dependency-driven 7-phase structure** mirroring the reference build order (ARCHITECTURE.md §Suggested Build Order), with the terminal spine deliberately de-risked before grid math lands. Phase numbers below are suggestions for /gsd-roadmapper.

### Phase 1: Workspace Skeleton + Supervisor + Settings (the proven foundation)
**Rationale:** Nothing renders without the sidecar; the pins/lockfile and the threading pattern are the foundation all later phases copy. Highest-confidence phase — nearly a verbatim port.
**Delivers:** `desktop-gpui/` workspace committed with exact pins + `Cargo.lock`; supervisor crate (web-tmux env/probe deltas, tmux binary resolution, TMUX env hygiene); settings crate; minimal GPUI window with status page (Starting → Ready/Failed{stderr_tail}), DWM dark titlebar, bundled JetBrains Mono registered before shaping, `fxc` tool ported, backend status → UI.
**Addresses:** Stack foundation; startup/status surfaces; window chrome bootstrap.
**Avoids:** Pitfall 1 (version trap), Pitfall 2 (threading bridge — establish the named pump pattern here), Pitfall 8 (Windows specifics), workspace-topology minors (gpui-free headless crates, `resolver = "2"`).
**Research flag:** Standard patterns (web-term proven) — skip research-phase; include the `cargo tree` / lockfile CI checks as first tasks.

### Phase 2: Backend Client REST + App Scaffolding
**Rationale:** The sidebar tree, status bar, tmux-binary apply, and state replacement pages all hang off REST; the Create Session dialog (via REST POST, works with zero sessions) is the proving ground for gpui-component `Input`.
**Delivers:** `types.rs` + `rest.rs` (endpoint set + tmux types mirroring `tmux-types.ts`); 1.5 s tree poll task; status bar (tmux version); Create Session dialog w/ name/cwd/initial command + native directory picker (`prompt_for_paths` — see pitfall-resolved note); empty/error/select-session states.
**Uses:** supervisor Ready → BackendClient; first real use of gpui-component `Input` + `Dialog`.
**Avoids:** "Fire-and-forget mutations" and free-form reqwest-in-views debt patterns (contract tests start here; REST error-body `{error}` → toast).
**Research flag:** Light — gpui-component Input/Dialog APIs are MEDIUM-confidence (Context7); recommend a small research-phase check or an in-phase spike validating focus/IME/Enter handling with real dialogs.

### Phase 3: WS Client (`tmux_ws.rs`) + SessionTab Lifecycle
**Rationale:** Proves the multi-session socket discipline before any terminal complexity — generation guard, backoff, queues, correlation as contract-tested behavior.
**Delivers:** Per-session JSON WS client ported 1:1 from `fe/src/lib/websocket.ts`; `SessionManager`/`SessionTab` snapshot model; open/close/switch tab lifecycle; `connection.ready` → flush queue → `hello(cols,rows)` → `state.resync`; window tab strip + toolbar with placeholder pane rects; optional open-tab restore after restart (decision point).
**Avoids:** Pitfall 5 (lifecycle races — name the generation counter + wrong-session drop guard as contract tests, including rename-mid-session socket re-resolution), requestId correlation (10 s forget).
**Research flag:** Moderate — verify the exact keystroke byte↔string mapping against the Go WS handler (`terminal.input` payload shape) during planning; the rest is a FE-source port.

### Phase 4: Terminal Engine Port + Single-Pane Rendering
**Rationale:** The long pole. One visible pane rendering live through the full capture→output pipeline de-risks the highest-novelty chunk (capture/replay ingestion) before grid math lands.
**Delivers:** Verbatim port of web-term's `terminal` crate (7 modules); WS output pump → `Terminal.process_bytes` by paneId; `applyCapture` port (history-delta + `screenRows`-split + idempotent first-write); keystrokes → `terminal.input`; selection/clipboard; scrollback from settings; wheel→PageUp/PageDown TUI-scroll per-pane policy; SGR mouse reporting.
**Avoids:** Pitfall 3 (resize storms — change-keyed resize in render path, debounce floor), Pitfall 4 (snapshot semantics — three grid modes + grid-wipe primitive), Pitfall 6 (mouse/selection + web-tmux wheel policy).
**Research flag:** Needed at plan time (alacritty embedding nuances, capture replay contract tests) — the engine is ported verbatim but the capture layer is the novel merge point.

### Phase 5: Pane Grid (the hardest new code)
**Rationale:** Depends on snapshots (P3) and rendered terminals (P4); tmux owns layout — the grid only displays reported geometry. Second-largest novelty concentration after the port itself.
**Delivers:** `geometry.ts` port (`pixelRect`, `closePaneGaps`, `pxToColsRows`, bit-identical to the FE); absolute-positioned `TerminalView`s; zoom fill; T-split dividers with drag → incremental `paneResize` steps (incl. the `lastCells` + direction-flip semantics); layout preset toolbar (even-h/v, main-h/v, tiled, `next`); pane headers w/ TUI-scroll switch and tooltips; pane/window context menus (split/zoom/swap/break/join/kill/rename/move); viewport + `layoutKey` resync discipline (100/150/325 ms debounce dance).
**Avoids:** Pitfall 3 (during drags), stale-grid after layout change (invalidate-before-write forced captures), anti-pattern 3 (never derive layout locally).
**Research flag:** Needed — this is the genuinely novel GPUI code (drag math, child positioning, viewport scaling).

### Phase 6: Theme System + Shell Parity (settings, palette, icons)
**Rationale:** Cross-cutting polish isolated from protocol wiring; theme tables are generated artifacts, and the same generation script feeds the parity audit.
**Delivers:** `theme.rs` generated from `fe/ui-themes.ts` + `terminal-themes.ts` (17 UI colors incl. input/ring/destructiveForeground; full ANSI-256; linked terminal theme per UI preset; live set_palette across hidden tabs); gpui-component theme-token bridge so stock modals/menus match; Settings page (theme grid, terminal themes, font/cursor/scrollback, tmux binary path w/ validation against `/api/tmux/binary`, confirm toggles); command palette via gpui-component `Command` in `open_dialog`; `icons.rs` lucide set pulled from `fe/`'s node_modules paths; titlebar/window-state polish.
**Avoids:** Pitfall 7 (theme drift — machine-port, table-diff test), Pitfall 10 (`text_color` on every icon site), minor pitfalls (window restore before open).
**Research flag:** Mostly standard (web-term scaffolding + generation script); no dedicated research-phase expected.

### Phase 7: Hardening + Packaging + Parity Audit
**Rationale:** Cross-platform acceptance and the "looks done but isn't" checklist close the milestone; packaging is non-trivial on Windows (fxc).
**Delivers:** `tmux.disconnected/reconnecting` + reconnect banners (web-term `reconnect_banner.rs` restyled); ErrorState retry; backend kill/WS-drop chaos tests; `scripts/package-windows.ps1` (+fxc env step) and `package-linux.sh` adapted to web-tmux's Makefile/backend; `build-test-backend` integration harness; the full "Looks Done But Isn't" checklist (vim/htop daily-driver test, rapid session switching, resize flood check, rename mid-session, mid-session theme swap); GTK4/GPUI config-path non-collision check.
**Avoids:** Pitfalls 8/9 verification (Windows CI build incl. `windows_subsystem` release build; Linux X11/Wayland smoke).
**Research flag:** Standard patterns; packaging scripts exist as reference copies.

### Phase Ordering Rationale

- **Hard dependency chain:** supervisor → REST → WS → terminal engine → pane grid → shell → packaging. Nothing renders without the spawn + snapshot client; the terminal engine is the long pole and must render one pane before grid math is layered; theme/palette are isolated polish feeding the audit.
- **Groupings match risk:** Phases 1–3 are high-confidence ports of proven code (wrong moves there are rare and cheap); Phases 4–5 carry the real engineering risk (alacritty embedding, capture semantics, drag/resize math) and get the named contract tests; Phase 6 is mechanical (generated data) and Phase 7 is acceptance.
- **Pitfall landmines are front-loaded:** every Critical pitfall except theme/icons is prevented by behavior established in Phases 1–5; the pitfall-to-phase mapping in PITFALLS.md should be mirrored in each phase's plan.

### Research Flags

Phases likely needing `/gsd-plan-phase --research-phase <N>`:
- **Phase 3:** verify `terminal.input` byte↔string mapping against the Go WS handler before building the pump (flagged by ARCHITECTURE.md as a must-verify protocol seam).
- **Phase 4:** alacritty + capture-replay ingestion nuances; the only hand-written merge into the verbatim engine.
- **Phase 5:** novel GPUI geometry/drag/viewport code; FE contract (`PaneWorkspace` debounce dance) is the spec — worth its own research pass.
- **Phase 2 (light):** gpui-component Input/Dialog API claims are MEDIUM confidence (Context7) and untested in this repo's reference; a small verification task in the phase plan is enough.

Phases with standard/proven patterns (skip research-phase):
- **Phase 1** (web-term skeleton port verbatim), **Phase 6** (generated tables + web-term settings/palette scaffolding), **Phase 7** (packaging scripts exist; acceptance checklist defined).

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Every pin verified against reference `Cargo.lock` and live crates.io same-day; API claims verified in vendored crate source. gpui-component doc claims (MEDIUM per Context7) corroborated by working code elsewhere. |
| Features | HIGH | Both codebases read line-by-line; operation-level Electron→GPUI mapping tables. gpui-component `Input`/`Dialog`/`ContextMenu`/`Command` behaviors carry MEDIUM confidence from docs. |
| Architecture | HIGH | Derived from first-hand reads of both repos; pattern-level GPUI claims (async app pump, prompt_for_paths, DWM) verified in source/vendor. tmux control-mode external docs MEDIUM (backend WS contract is the actual consumer). |
| Pitfalls | HIGH | Every reference-side pitfall verified by reading source (web-term phases 20/21/27 planning docs included); external metadata (crates.io churn, tmux wiki) MEDIUM. |

**Overall confidence:** HIGH — unusually so for greenfield-scale work, because the architecture is a proven port; residual risk concentrates in pattern fidelity and the two novel code areas, which are exactly where phases 4–5 research is flagged.

### Gaps to Address

- **`terminal.input` payload shape:** keystroke bytes are sent as a JSON `data` string; the exact byte↔string mapping (JS keeps bytes in a plain UTF-16-backed string) must be verified against the Go WS handler decode in Phase 3 before trusting `keystroke_to_bytes` output routing.
- **gpui-component Input/Dialog/Command runtime behavior:** Context7-docs MEDIUM; validate focus/IME/Enter/validation in a Phase-2 spike when building the Create Session dialog (fallback is hand-rolling per webterm's overlay pattern, but expect Input to work).
- **Open-tab persistence after restart:** Electron restores nothing; web-term desktop does; tmux sessions survive re-attach. Both are defensible — decide explicitly in requirements (recommended: adopt persistence as a desktop-native extra in Phase 3).
- **Sidebar collapse animation:** accepted v1 deviation (binary snap, no CSS transition); not a gap, just record it for the parity audit so it isn't re-argued.
- **Folder picker resolution:** FEATURES.md tentatively suggested `rfd`; STACK.md verified gpui-pre's built-in `App::prompt_for_paths` (native folder dialog both OSes) and warns against `rfd` on the GPUI window thread. **Resolved: use the built-in picker; do not add `rfd`.** Note: it's impure-async — poll/forward the receiver via channels, don't `.await` directly on the GPUI thread.
- **Windows console window in release:** web-term ships a visible console (wart). If `windows_subsystem = "windows"` is adopted, verify the backend's stdout handshake still works with no parent console — CI must build both configurations (PITFALLS.md minor + STACK.md variant note).
- **Backend gap discovered mid-build:** if any protocol gap appears (e.g., missing metadata the Electron UI shows), log it as a backend issue — do not fork behavior in the GPUI crate (PROJECT.md constraint).

## Sources

### Primary (HIGH confidence)
- `E:\Coding Stuff\web-term\desktop-gpui\` — the proven reference: `Cargo.toml`/`Cargo.lock` (pin set), `crates/terminal` (all 7 modules), `crates/{supervisor,settings,backend-client}`, `crates/webterm/src/{main,app_state,theme,icons,bundle,window_state}.rs`, `views/{tab_strip,nav,settings,status,reconnect_banner}.rs`; Phase 20/21/27 research docs (pins, NO-GO on external gpui-terminal, parity matrix)
- `E:\Coding Stuff\web-tmux\fe\src\` — port target ground truth: `lib/{protocol,tmux-types,websocket,sockets,commands,api,geometry}.ts`, `stores/{tmuxStore,appStore,settingsStore}.ts`, `features/{panes,terminal,settings,palette}`, `App.tsx`
- `E:\Coding Stuff\web-tmux\be\` + `desktop\main.js` — integration ground truth: BACKEND_PORT handshake, env config, per-session WS handler, hello→snapshot ordering, pipe-pane streaming, Windows monitor path
- Vendored crate source (`~\.cargo\registry\src\…gpui-pre-0.3.3`, `gpui-pre-windows-0.3.3`) — `App::prompt_for_paths`, `PathPromptOptions`, Windows `IFileOpenDialog` folder flag

### Secondary (MEDIUM confidence)
- crates.io API (2026-09-06): `gpui-pre =0.3.3` latest, `gpui-component =0.6.0` latest / 25 versions churn, `alacritty_terminal 0.26.0` deliberate non-adoption
- Context7 gpui-component 0.6 docs (`/longbridge/gpui-component`) + gpui docs (`/websites/rs_gpui`) — ContextMenu/Dialog/Tooltip/Input/Command APIs
- tmux Control-Mode wiki — control-mode protocol constraints (backend consumes; desktop sees only the WS contract)
- `scripts/{package-windows.ps1,package-linux.sh,build-test-backend.sh}`, `tools/fxc/README.md` (web-term) — packaging + fxc landmine

### Tertiary (LOW confidence / needs validation)
- None material. Residual uncertainties are tracked in Gaps to Address (byte↔string WS mapping; gpui-component widget behaviors), both flagged for phase-level verification against the working backend/crates rather than external sources.

---
*Research completed: 2026-09-06*
*Ready for roadmap: yes*
