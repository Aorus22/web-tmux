# Feature Research

**Domain:** Native GPUI (Rust) desktop frontend for a tmux GUI — 1:1 port of the existing Electron UI
**Researched:** 2026-09-06
**Confidence:** HIGH (primary sources are two local codebases studied line-by-line: the proven reference `E:\Coding Stuff\web-term\desktop-gpui\crates\webterm\src\` and the port target `E:\Coding Stuff\web-tmux\fe\src\`; gpui-component 0.6 docs cross-checked via Context7, MEDIUM confidence)

## Feature Landscape

This section doubles as the **Electron → GPUI mapping study**. Every feature row states *how the Electron UI does it*, *how the reference GPUI app (web-term) implements the equivalent pattern*, and what complexity that implies for the web-tmux port.

### Core mapping rules (from the reference implementation)

| Electron pattern (fe/src) | web-term GPUI equivalent (crates/webterm/src/) | Notes |
|---------------------------|------------------------------------------------|-------|
| `WindowTabs` in `AppTitleBar` (flex-1 middle) | `views/tab_strip.rs` — hand-rolled `div()` row inside a 40px transparent title bar | Direct copy possible |
| Frameless drag (`data-drag-region` + `WebkitAppRegion`) | `.window_control_area(WindowControlArea::Drag)` on a spacer div | Direct |
| Custom min/max/close buttons | `window.minimize_window()`, `window_state::toggle_maximize()`, `window.remove_window()`; Windows dark frame via `DwmSetWindowAttribute` in `main.rs` | Port `window_state.rs` as-is |
| Custom ladders of shadcn components (button, input, dialog, …) | **Raw `div()` composition everywhere** — web-term imports only `gpui_component::{init, Theme, ThemeMode}`; every "Button" is a div with `.hover()` + `on_mouse_down` | The reference proved you can hit 1:1 without widgets; use widgets only where input/list is needed |
| lucide-react icons | `icons.rs` — 40+ lucide SVGs as `pub const *_SVG: &[u8]` rendered with `svg().data(CONST).size(px(n)).text_color(tint)` | Copy icons verbatim from lucide (same stroke attrs); tint via text_color (icons use `stroke="currentColor"`) |
| Theme CSS variables on `<html>` (`ui-themes.ts`, 102 presets) | `theme.rs` `THEME_PRESETS` const array + `AppState.bg_color()/card_bg()/…` helper methods read at render time; `gpui_component Theme::change(ThemeMode, cx)` only sets the dark/light base | Port via a generation script (fe's ui-themes.ts is itself script-generated — keep files generator-compatible) |
| Terminal theme (`terminal-themes.ts`, 78 presets) | `terminal_palette_for_preset()` → `webterm_terminal::ColorPalette::from_rgb_u32(fg, bg, cursor, selection, ansi[16])` | Mechanical conversion, best done by the same generation script |
| Zustand stores + TanStack Query | Single monolithic `AppState` entity (`app_state.rs`) + `cx.notify()`; free-function views `(app, cx) -> AnyElement` | Mirror appStore/tmuxStore/settingsStore as AppState fields; TanStack polling becomes a tokio interval refetching the tree |
| Popovers/dropdowns (shadcn Popover/Select) | `show_x_picker: bool` flags + overlay backdrop + `.absolute().top().left()` card rendered at the view root (`nav.rs` layering) | Works; ordering of `.children()` = z-order |
| Toasts (sonner) | `notification: Option<String>` + top-right absolute div with auto-dismiss | Covers all `toast.success/error` call sites in the Electron FE |

### Table Stakes (Users Expect These — required for the "sama persis" 1:1 UI)

| Feature | Why Expected | Complexity | Notes (Electron → GPUI implementation) |
|---------|--------------|------------|-------|
| Frameless title bar with sidebar toggle, app identity, window tabs, drag region, custom window controls | Every open Electron screen shows this; first thing users see | LOW | Port `tab_strip.rs` pattern nearly line-for-line; drag area = `window_control_area(Drag)`; controls via GPUI window API; ENV-SPECIFIC: web-tmux's title bar shows **tmux-window tabs of the active session** (`WindowTabs.tsx`, "index: name") plus a Settings gear — not session tabs like web-term |
| Multi-session tabs (all stay alive) | Core of the web-term pattern; switching tabs never tears down a connection | MEDIUM | In Electron, session "tabs" are stacked absolutely-positioned workspaces kept mounted, inactive ones `invisible pointer-events-none` (App.tsx) — no visible session tab row; switching happens via the sidebar tree. GPUI equivalent: keep one `Entity` of per-session state (WS + snapshot + TerminalViews) alive per open session and render only the active session's workspace into the element tree; alacritty grid state keeps accumulating off-render |
| Collapsible sidebar: session→window→pane tree | PRD §33; Electron AppSidebar with 1.5s TanStack polling, refresh/new-session buttons, expanding window/pane rows | MEDIUM | Same hand-rolled nav-shell/sidebar divs as webterm `nav.rs`, but tree is 3 levels, with `expanded: HashSet<String>` state instead of 4 static items |
| Window tabs inside title bar with context menu (rename/move left/right/break/kill + confirm dialogs) | PRD §34/§39 | MEDIUM | webterm composes raw div tab buttons; context menus = `ContextMenuExt` from gpui-component (`.context_menu(...)` on the tab div) or the webterm hand-rolled overlay pattern |
| Pane grid with real tmux geometry | PRD §13/§19 — panes positioned from `pane_left/top/width/height`, zoom fills workspace, 4px drag dividers, T-split dividers where edges overlap | HIGH | This is the single most complex piece. web-term's pane-grid doesn't exist here (one terminal per tab). Port `geometry.ts` (pixelRect, closePaneGaps, pxToCells/resizeDragStep) → Rust; render as `div().absolute().top().left().w(px).h(px)` in a relative flex container; dividers = 4px absolute divs with `on_mouse_down`/`on_mouse_up` + `window.mouse_position()` for pointermove throttled ~40ms; background = terminal theme bg like Electron |
| Pane headers (path, `pane.id`, TUI-scroll switch, split/zoom/kill with tooltips, double-click zoom, kill confirm) | Always-visible quick actions per pane | MEDIUM | Copy header div pattern from PaneHeader.tsx; use gpui-component `Switch` + `.tooltip()`; double-click zoom via on_mouse_down click_count check (uses `ev.click_count >= 2` in webterm SFTP) |
| Multi-terminal rendering: scrollback, capture-pane replay, selection, clipboard | PRD §21–24 — xterm.js per pane; `terminalRegistry` writes WS output directly; capture history delta logic must keep cursor anchor | HIGH | web-term terminal crate (alacritty_terminal 0.25.1) gives grid render, selection, wheel scroll, clipboard via `copy_selection`/`paste_clipboard`, title/bell callbacks — but web-term's backend never sends capture commands, so **the capture-replay layer is novel**: port `terminalRegistry.applyCapture` (history lines → scrollback delta, then clear+home+positioned screen rebuild) and the per-pane `ingestedHistory` counter into the webtmux session/pane state layer feeding `TerminalView.process_output` |
| Layout presets toolbar (even-h/v, main-h/v, tiled, next) | PRD §16, user-visible single icon row | LOW | Icon-button div row; layout ids are strings forwarded to `windowLayout` WS call (logic identical to WindowToolbar.tsx / LayoutSelector) |
| Create Session dialog (name/cwd/initial command + directory browse) | PRD §14 gateway for first session; EmptyState opens it | MEDIUM | Single real gap vs the reference: needs **real text entry** — use gpui-component `Input` (InputState: focus, IME, masking, validation) which web-term lacks but is already pinned (gpui-component = 0.6.0, `init()` already called). The Browse button (Electron `desktop.pickDirectory()` IPC) needs a native folder picker in the Rust shell (e.g. `rfd`) — new small integration, no backend change |
| Context menus (session/window/pane) incl. Swap list, Break pane, warnings | PRD §39 — every list item and pane has one | MEDIUM | gpui-component `PopupMenu/ContextMenuExt` handles right-click natively at trigger position (Electron parity, and unlike webterm SFTP it avoids the hardcoded 1200×720 flip heuristic) — fallback = webterm positioned-overlay pattern if exact pixel-match needed |
| Rename dialogs (session/window/pane) + kill confirmations (per-surface safety toggles) | Every rename flow in Electron opens a Dialog with Input | MEDIUM | Use `window.open_dialog` (gpui-component) + Input; per-dialog state as webterm-style `Option<…FormState>` fields on AppState; `shouldConfirm(...)` reads the settings crate's three kill-confirm toggles exactly like the Electron helpers |
| Command palette Ctrl+Shift+P | PRD §38 — shadcn CommandDialog with filter-as-type, actions + "Open Session" group | MEDIUM | gpui-component 0.6 has a dedicated `Command` component (CommandState/Command/CommandGroup/CommandItem, placeholder, empty state, virtualized list, auto keybinding hints) designed to be hosted via `window.open_dialog` — maps 1:1 to Electron's CommandDialog incl. the "Type a command or search…" placeholder and "No results found." empty view |
| Settings page (UI theme grid 3-col, theme mode filter dropdown, terminal prefs incl. tmux binary path, numeric font/lineheight/scrollback, 3 kill-confirm toggles) | PRD §41–42 users configure the app here | MEDIUM | Copy webterm `settings.rs` structure (1027 lines proves the layout) — theme cards + hand-rolled dropdown pickers; tmux binary path needs a real Input + validate-on-blur/Enter against the existing `/api/tmux/binary` call, surfacing the status line ("Using … (version)" / error) |
| Theme system with dark+light counterparts, 102 UI + 78 terminal presets | User picks a card in the grid; terminal auto-follows | MEDIUM | Port ui-themes.ts (scripted) into `theme.rs` consts; `terminal-themes` → `ColorPalette` per preset + per-terminal-theme; live `set_palette` across open terminal views like webterm `set_theme_preset` |
| Empty/Error/SelectSession replacement states, connect/startup states | Missing any = broken-feeling app | LOW | Direct div ports of EmptyState/ErrorState/SelectSessionView; startup ordering maps to webterm `status.rs` — `BackendStatus::Starting` ("Starting backend…") page while the sidecar warms up, `Failed { reason, stderr_tail }` page with redacted stderr + Retry/Quit, plus the Electron 10s backend-port timeout fallback |
| Windows-style window drag/resize + zoom; DWM visual fixes | On Windows, backdrop/dark title text is off-by-one without DWM attrs | LOW | Port webterm `window_state.rs` (restore/observe, 211 lines) + `main.rs` DWM block verbatim, unchanged across EITHER OS |

### Differentiators (Competitive Advantage)

| Feature | Value Prop | Complexity | Notes |
|---------|-------------------|------------|-------|
| True native rendering (no Chromium) | Same UI, ~10× lower idle memory vs Electron; instant window open | FREE by stack choice | Comes from the architecture already pinned (supervisor + gpui-pre stack) |
| Tooltips with keybinding hints | Electron shows tooltips on every toolbar button (§16/§35) | LOW | gpui-component `.tooltip_with_action(text, action, ctx)` renders the bound keys automatically once tab/grid keybindings are registered |
| Frameless window drag on all surfaces | GPUI drag region is click-efficient; keep active tab clickable regions via `window_control_area` exclusions | LOW | Same pattern as webterm tab_strip |
| Per-pane TUI-scroll switch (PageUp/PageDown to TUIs) | Electron shows it in every pane header (settingsStore.tuiScrollPanes + Switch) | LOW | Hand-rolled header row + gpui-component `Switch`; extending wheel behavior in the terminal input shader is front-end only, per-pane override map like the Electron store |
| Command palette keybinding hints | Let users see shortcuts in the palette rows (Electron's CommandDialog doesn't, but GPUI affordance is free) | LOW | gpui-component `CommandItem.action(...)` renders bound keys automatically once actions are registered in `cx.bind_keys` |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Porting React component libraries (shadcn) one-for-one into GPUI widgets | "Reuse the pages with the same components" | Adds a heavy abstraction mismatch; gpui-component widget default styling diverges from Tailwind/shadcn visual rhythm, dragging the 1:1 goal away | Keep the web-term pattern — hand-rolled styled `div()`s wherever shadcn provides only presentation; lean on gpui-component only for behaviors GPUI lacks (Input, Dialog, ContextMenu, Tooltip, Command) |
| Re-skinning to trendy GPUI built-in widgets (Icon/Tab/Calendar/Notification), HUD styling | gpui-component ships richer widgets with built-in animated menus | Breaks pixel-perfect UI against Electron; widgets carry their own spacing/radius/typography | Style the same visual values manually (`.text_xs()`, `px()` swaps instead of `className`s) so every card/panel matches |
| Adding frame-by-frame animations everywhere (hover transitions, sliding panels) | Electron CSS transitions feel fluid | GPUI has no CSS-transition equivalent; every animation means manual per-frame state that recomputes layout | Static hover/snap state changes exactly like webterm used across the same widget surface (0 flicker issues) — restrict animation to the wheel-scroll and drag interactions the terminal genuinely needs |
| Terminal cursor-space selection redraw | Prettier mouse-follow UI | Requires re-render loop of grid each frame | Do exactly what webterm does — immediate `cx.notify` on write + cell-aligned selection redraw in the canvas renderer |
| Keeping every session workspace "mounted" via CSS invisibility | React needs it (`invisible pointer-events-none` keeps xterm alive); tempting to mimic with hide-flags | In GPUI, off-tree elements simply don't render — there is no CSS visibility to fake it, and hacking `opacity` still pays the render cost only when rendered | The GPUI-native equivalent is already the right one: buffer state lives in the alive `Entity` (terminal grid, WS pump), so unrendered sessions keep accumulating output; only the active session is in the element tree (same as web-term's inactive `View::Hosts` terminal handling) |

## Feature Dependencies

```
[Go backend sidecar spawn (supervisor + port-0 handshake + IPC to learn the port)]
    └──requires──> [Backend client: REST + WS protocol layer]
                          └──requires──> [Per-session WS pump: events → app state]
                                                └──requires──> [Terminal pipeline: WS terminal.output → alacritty grid renderer]

[Theme system (ui-themes.rs consts + terminal palettes)]
    └──wired-into──> every single UI surface

[Base window chrome: tab strip + title bar + window controls (window_state.rs, DWM)]
    └──requires──> [Theme system] + [actions.rs/keybinding substrate]

[Multi-session tab manager (all open tabs stay connected)]
    └──requires──> [Backend client] + [tab strip] + [appStore-equivalent state]

[Session→window→pane sidebar tree]
    └──requires──> [REST tree polling 1.5s] + [session manager]

[Text input substrate (gpui-component Input)] ──used-by──> [Create Session dialog]
                                                     ──used-by──> [Rename dialogs (session/window/pane)]
                                                     ──used-by──> [Command palette] + [tmux binary path in Settings]

[Command palette (Ctrl+Shift+P)]
    └──requires──> [Command component in open_dialog] + [tree data] + [active snapshot]

[Create Session dialog]
    └──requires──> [REST create-session call] + [Text input substrate] + [native dir picker (rfd)]

[Pane grid (absolute rects from tmux cells + zoom + closePaneGaps)]
    └──requires──> [geometry.rs port] + [per-session snapshot state]

[Pane resize drag]
    └──requires──> [Pane grid] + [incremental pane.resize step semantics]

[Layout presets toolbar + next-layout]
    └──requires──> [Active window id from snapshot] + [windowLayout WS command]

[Settings page (theme grid + terminal prefs + kill-confirm toggles)]
    └──requires──> [Theme system] + [settings persistence (webterm settings crate pattern)]
```

### Dependency Notes

- **Terminal path is the spine:** supervisor spawn → backend port → REST/WS client → per-session event pump → `TerminalView`; the pane grid only displays what this pipeline is already streaming, so the supervisor/backend-client/terminal phases come before any pane-grid work
- **Theme system precedes all visual work:** web-term solved this by injecting `theme.rs` helper methods (`app.bg_color()` etc.) called in every free-function view; land it first so every ported view can read colors immediately
- **Text entry is the one substrate web-term lacks:** the reference ships zero text-input code (all "inputs" in its modals are static display divs — verified: even the SFTP modal input and quick-connect search bar never receive typed chars). gpui-component `Input` is already pinned and covers focus/IME/masking/validation; build the Create Session dialog first as its proving ground, then reuse the same pattern for the three rename dialogs and the tmux-binary path field
- **Keep the pane grid and terminal fit properly coupled:** the Electron FE needed three coordinated fixes to avoid resize races — measure the container on the next animation frame, debounce the `terminal.resize` report (~100ms) and the layout-change capture resync (150ms/325ms), and invalidate each pane's snapshot before force-capturing after a layout-key change. The Rust port must reproduce this debounce+invalidate dance, not just the rect math
- **Pane resize drag sends incremental cell steps** (never cumulative — the Electron code learned this via `resizeDragStep()`); the GPUI port must reproduce the `lastCells` accumulation plus negative-direction flip (`dir = step < 0 ? FLIP[direction] : direction`) or drags break after tmux clamps a resize

## MVP Definition

### Launch With (v1 — the milestone's table stakes)

The milestone goal is UI parity with Electron — "minimum" = "complete UI surface, protocol unchanged, Windows + Linux":

- [ ] Windows shell skeleton: supervisor crate spawns the Go backend on a dynamic port and publishes it to the UI; settings persistence (webterm `settings` crate pattern) restores window geometry/theme; title bar with window controls, tab strip, and icon set ported
- [ ] Theme system: 102 UI presets + 78 terminal presets ported (script-generated like fe) and applied live (dark/light counterparts)
- [ ] Multi-session stacked workspaces (all stay connected), session tree sidebar with 1.5s polling, per-session WS lifecycle
- [ ] Pane grid from tmux geometry (rects + closePaneGaps + zoom + resize dividers + layout preset bar)
- [ ] Terminal panes: consume `terminal.output`/`terminal.capture` over the existing WS protocol (history-delta + positioned screen replay), input, wheel scroll, selection, clipboard, font-fit reporting
- [ ] Commands & dialogs: create/rename/kill session/window/pane, split/zoom/swap/break/join, layout presets, with kill-confirm toggles
- [ ] Create Session dialog, rename dialogs, command palette (Ctrl+Shift+P), Settings (theme cards, terminal prefs, tmux binary path), empty/error/select states, reconnect/status banners

The web-term app already proved every generalized app-level pattern. The only genuinely novel engineering vs web-term: **pane grid geometry + drag** (absent in web-term) and **text input** (absent in web-term, provided by pinned gpui-component `Input`).

### Add After Validation (v1.x)

- [ ] Session-level keyboard shortcuts (webterm `actions.rs` precedent: ctrl-t/ctrl-w ctrl-tab alt-1..9 → adapt to session switching) — non-critical now that the palette covers mouse-free use
- [ ] Window-control parity polish on Linux (drag/maximize behaviors differ from Windows DWM path) — land after the Windows build proves the chrome

### Future Consideration (v2+)

- [ ] Sidebar width animation (CSS-like slide) — pure polish, GPUI has no native animation
- [ ] Anything touching the backend protocol (session persistence, pane metadata enrichment) — explicitly out of scope this milestone

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Window chrome + tab strip + theme applicator | HIGH | LOW | P1 |
| Multi-session tabs + sidebar tree + polling | HIGH | MEDIUM | P1 |
| Terminal pipeline: WS → alacritty grid → pane rects + header/terminal interactions | HIGH | HIGH | P1 |
| Pane grid, zoom, resize, layout presets | HIGH | HIGH | P1 |
| All commands/dialogs (create/rename/kill/split/swap/break/join) + palette | HIGH | MEDIUM | P1 |
| Settings page + per-pane TUI-scroll setting | MEDIUM | LOW | P1 |
| Empty/error/select states + reconnect banner + status toasts | MEDIUM | LOW | P1 |
| Icon set, monospace font embedding, window state persistence | MEDIUM | LOW | P1 |

**Priority key:**
- P1: Must have for launch
- P2: Should have, add when possible
- P3: Nice to have, future consideration

## Competitor Feature Analysis

("Competitor" here = the two implementations being bridged.)

| Feature | Electron (port target) | GPUI Reference (web-term) | Our Approach |
|---------|------------------------|---------------------------|--------------|
| Title bar | `AppTitleBar.tsx` h-11 + minimized controls region | `tab_strip.rs` (h-40px) hand-rolled divs + window_control_area | 1:1 via web-term pattern with Electron tab rows |
| Window controls | click → `desktop.window.*` IPC | direct `window.minimize_window()` etc. + DWM dark fix | Same flow: interact with the window API directly; max/restore state tracked via `window_state.rs` |
| Popover | shadcn `Popover` | `.absolute()` overlay + bool flag | web-term pattern |
| Inputs | shadcn `Input` | **none** (all modal inputs are inert display divs) | gpui-component `Input`/`InputState` (first-class focus/IME/masking) |
| Context menu | shadcn `ContextMenu` | hand-rolled overlay with absolute coords `+ viewport-edge flip (hardcoded 1200×720 rect)` | gpui-component `ContextMenuExt` where possible; hand-rolled overlay as fallback |
| Dialog | shadcn `Dialog`/`AlertDialog` | hand-rolled full-screen overlay divs | Same overlay pattern for pixel-match; `open_dialog` when 100% parity insufficient |
| Command palette | shadcn `CommandDialog` | none in the reference | gpui-component `Command` component hosted in `open_dialog` (placeholder, groups, empty state, keyboard nav, auto keybinding hints) |
| Toasts | sonner | `notification` div | web-term pattern |
| Terminal | xterm.js (`@xterm/xterm`, `FitAddon`) | the `terminal` crate (alacritty_terminal 0.25.1 + custom `TerminalRenderer` canvas) | reuse web-term `terminal` crate as-is |
| Theme application | CSS variables on `<html>` | `THEME_PRESETS` consts + `AppState.bg_color()` helpers | generate from fe's ui-themes.ts + terminal-themes.ts scripts |
| Sidebar tree | React SessionContextMenu + expand | static 4-item nav | extend hand-rolled sidebar for 3-level nesting (expanded HashSet + child rows), no widget needed |
| Pane grid | absolute rect grid via `pixelRect` from tmux cells | (none — the single-terminal web-term has no pane concept) | new code, math ported from `fe/src/lib/geometry.ts` |
| Multi-pane command orchestration | `tmuxSocket.*` + requestId correlation + `runCommand` timeout | n/a (web-term's backend-client has simpler request/response) | reuse the same backend-client crate; re-implement the requestId/`pending` correlation + timeout in the webtmux app-state layer |
| 1.5s tree polling | TanStack `useQuery(refetchInterval: 1500)` | n/a (refetch on demand) | tokio interval in the webtmux app state refreshing `BackendClient` tree; manual refresh button triggers the same path |

## No-Direct-GPU-Equivalent / Custom Work Inventory

| Piece | GPUI situation | Mitigation (already proven or planned) |
|-------|----------------|---------------------------------------|
| Freeform text entry | web-term dodged it; gpui has no stdlib input widget | gpui-component 0.6 `Input`/`InputState` (MEDIUM confidence docs) — already in the pinned stack |
| 1:1 capture-diff replay per pane | xterm-level code shipped in `terminalRegistry.ts` only | port `applyCapture` + `ingestedHistory` counter into the webtmux session/pane state layer (tokio side channel into `TerminalView.process_output`) |
| Drag-resize with pointer capture during a live drag | GPUI has `window_control_area` + raw mouse events but no React pointer-capture | mouse-down starts drag state on `div` (or element with `.overflow_hidden` + `.absolute()`), then continues on `window`/MouseMoves (`window.mouse_position()`); throttled in ~40ms timer logic |
| Scrollable containers (`overflow-x-auto` tab strip; `ScrollArea` sidebar) | GPUI scroll needs `.id(...)` on the scrollable element (proven in webterm settings.rs + sftp list) | Use `.id("...").overflow_y_scroll()` (and horizontal variants for the window-tab strip); the sidebar tree and tab strip scroll through the same mechanism |
| Sidebar collapse animation | Electron slides w-60 ↔ w-0 with `transition-[width] duration-200`; GPUI has no width interpolation | Binary `sidebarOpen: bool` like webterm `sidebar_open`; flex layout frees/reclaims the 240px column (visual snap instead of a 200ms slide — accepted v1 deviation) |
| Divider-edge pixel rounding | `pixelRect` rounds to integers; floating drift could misalign T-junctions | Port `Math.round` semantics identically (Rust `f32::round`) and lean on closePaneGaps' integer cell math, which keeps every shared edge exactly aligned |

## Sources

- **Port target (studied line-by-line):** `E:\Coding Stuff\web-tmux\fe\src\` — App.tsx, AppTitleBar/AppSidebar, features/sessions|panes|windows|palette|settings|terminal, stores/, lib/geometry.ts, all components/ui/*
- **Reference GPUI app (studied line-by-line):** `E:\Coding Stuff\web-term\desktop-gpui\crates\webterm\src\` — views/{tab_strip,nav,new_tab,modals,sftp,settings,status,reconnect_banner}.rs, theme.rs, icons.rs, app_state.rs, session.rs, actions.rs, window_state.rs; `crates/terminal/src/{view,terminal,render,mouse,input,event,colors}.rs`; workspace Cargo.toml (pinned versions: gpui-pre =0.3.3, gpui-component =0.6.0, alacritty_terminal =0.25.1)
- **gpui-component 0.6 API (Context7, MEDIUM confidence):** ContextMenu / PopupMenu extension (`.context_menu()` on divs, anchor + mouse_button config), Dialog/AlertDialog (`window.open_dialog`, DialogFooter/DialogClose/DialogAction, on_close), Tooltip (`.tooltip(text)`, `.tooltip_with_action`), Input/InputState (focus, IME, masking, validation, events)
- **Project context:** `E:\Coding Stuff\web-tmux\.planning\PROJECT.md` (milestone v1.0 Desktop GPUI, in-scope/out-of-scope, stack pins)

---
*Feature research for: web-tmux Desktop GPUI milestone (Electron UI → GPUI 1:1 port)*
*Researched: 2026-09-06*

<!-- gsd-note: confidence = HIGH for code-sourced mappings (both repos read directly); MEDIUM for gpui-component API claims (Context7 docs, not cross-tested in this repo). -->
