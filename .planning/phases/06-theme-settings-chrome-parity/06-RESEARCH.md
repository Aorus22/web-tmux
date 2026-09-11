# Phase 6: Theme System, Settings & Chrome Parity - Research

**Researched:** 2026-09-07
**Domain:** GPUI theme tables (102 UI + 78 terminal presets), Settings page, command palette, terminal prefs, tmux-binary validation, kill-confirm wiring, embedded font/icons
**Confidence:** HIGH — every load-bearing claim was Read-verified this session across `fe/`, `be/`, `desktop-gpui/`, the web-term reference, and the vendored `gpui-component 0.6.0` sources
**Schema:** 02

## Summary

Phase 6 is a **data-port + chrome-wiring phase, not an engine phase**. No new crates, no new packages, no backend changes, no protocol fork. The Electron ground truth is fully mapped: `fe/src/features/settings/data/ui-themes.ts` holds **102 UI presets** and `terminal-themes.ts` holds **78 terminal presets** (counts verified by `grep -c` this session), the FE applies themes by writing 15 design-token CSS variables plus sidebar vars and `colorScheme` (`fe/src/App.tsx:71-111`), terminals **always follow the UI theme** through `resolvedTerminalTheme` (the persisted `terminalTheme` override is legacy and intentionally ignored — `fe/src/stores/settingsStore.ts:13-15`, `ui-themes.ts:2507-2515`), and the command palette is a 7-action + Open-Session grouped dialog on Ctrl+Shift+P.

The GPUI side is unusually well-prepared: `TerminalView::set_palette/set_font/set_font_size` already exist (`terminal_view.rs:168-193`), `ColorPalette::from_rgb_u32` is the exact construction seam for generated presets (`colors.rs:186-208`), all three kill-confirm gates plus their dialogs exist (`app_state.rs:1607-1615`, three `Kill*Form` views), `JetBrains Mono` is already embedded (`main.rs:42-44`), and — the biggest find — **vendored `gpui-component 0.6.0` ships a purpose-built `command` palette module** (`Command`/`CommandState`/`CommandEntry`/`CommandGroup`/`CommandItem`, verified in the registry source this session), so the palette is an assembly job, not a custom widget. The web-term reference contributes the proven `set_theme_preset` live-apply shape (sync preset → derive Dark/Light → `Theme::change` → push palette to every terminal → persist) and the 3-column card-grid settings pattern (`views/settings.rs:296-404`), but has **no palette** (zero grep hits) — the palette follows the FE `CommandPalette.tsx` spec instead.

**Primary recommendation:** check in a Node generator (`scripts/generate-gpui-themes.mjs`) that parses the two TS data files and emits one `themes_generated.rs` (102 UI rows + 78 terminal rows + `terminalTheme` link names), guard it with a lengths test (102/78) so FE drift is caught, extend `DesktopSettings` with the six missing FE fields, port the reference card-grid settings page over the existing `showing_settings` route, build the palette on the vendored `command` module with a `ctrl-shift-p` action, add the one missing REST method (`POST /api/tmux/binary`), and push prefs live through the existing `set_palette`/`set_font` seams with re-capture on scrollback change.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Theme preset tables (102 UI + 78 terminal) | GPUI client (`webtmux` crate, generated file) | Generator script (build-time only, output checked in) | Static data; build must never depend on the generator — lengths test catches drift |
| Chrome re-theming (title bar, sidebar, headers, dialogs, settings) | GPUI client (`AppState` preset helpers) | — | All current chrome hardcodes `rgb(0x…)`; helpers replace them with preset reads |
| Terminal palette + font application | GPUI client (`AppState` push over `terminal_views` + store defaults for new views) | `webtmux-terminal` (owns `ColorPalette`/`TerminalRenderer`) | Views are dumb clones of store terminals; palette/font must reach both live views and future views |
| Settings persistence | Settings crate (JSON via `dirs`, `.bak` recovery) | App crate (save on change) | Proven Phase-1 pattern; six new fields with serde defaults for back-compat |
| tmux binary validation | Go backend (`POST /api/tmux/binary`, `tmux -V` probe) | GPUI client (thin POST + status line) | Backend owns validation; client only surfaces `{binary, version, ok}` / error string |
| Kill-confirm behavior | GPUI client (existing gates + dialogs) | — | All three gates/dialogs already exist; Phase 6 only wires the three switches |
| Command palette | GPUI client (vendored `command` module + `actions.rs`) | Existing correlated sends | Palette is chrome over existing mutations; Open Session group reads the polled tree |

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SHELL-04 | Window geometry + theme preference persist across restarts | Geometry persists via `window_state::observe` already [VERIFIED: `crates/webtmux/src/window_state.rs:176-187`]; theme persists via `theme_preset` + new fields in settings crate (D2); load-time Dark/Light re-sync (Pitfall 2) |
| SHELL-05 | Embedded JetBrains Mono + embedded lucide SVGs, no system deps | Font embedded [VERIFIED: `crates/webtmux/src/main.rs:42-44`]; 24 icons present, 3 to add (D8); single-family note for `fontFamily` (Pitfall 7) |
| SET-01 | UI theme picker: 3-column card grid + dark/light filter | FE `UiThemeSettings.tsx` + reference `settings.rs:296-404` card pattern (D4) |
| SET-02 | Terminal prefs: font size, line height, scrollback, per-pane TUI-scroll default | FE defaults [VERIFIED: `fe/src/stores/settingsStore.ts:33-45`]; live-apply seams exist (D9) |
| SET-03 | tmux binary path validated against `/api/tmux/binary` with status/error in UI | `HandleBinary` + `SetBinary` contracts [VERIFIED: `be/internal/server/health.go:64-83`, `be/internal/tmux/binary.go:19-43`]; missing REST method is the only gap (D6) |
| SET-04 | Three kill-confirmation switches | Gates + dialogs exist; switches are new settings-page rows (D7) |
| THEME-01 | All 102 UI + 78 terminal presets as generated files | Counts verified (102/78); generator + lengths test (D1) |
| THEME-02 | Live apply across chrome; terminal follows matched theme unless overridden | `resolvedTerminalTheme` semantics (override ignored) + `set_theme_preset` push shape (D3) |
| THEME-03 | Dark/light counterparts behave with Electron dark semantics (`colorScheme`) | `isLightUiTheme` luminance rule + `ThemeMode` sync (D3, Pitfall 2) |
| DLG-01 | Command palette on Ctrl+Shift+P with filter + Open Session group | Vendored `command` module + `actions.rs` pattern + FE item list (D5) |
| DLG-03 | Kill confirmations honor per-surface toggles exactly like Electron helpers | `shouldConfirm` parity with existing gates (D7) |

(No CONTEXT.md exists for Phase 6 — no discuss-phase constraints to honor. D1–D10 below are auto-accepted autonomous decisions, flagged for post-hoc audit per the Phase-4/5 precedent.)

## Locked Decisions (auto-accepted, autonomous mode)

- **D1 — Generated theme tables with a checked-in output + lengths test.** New `scripts/generate-gpui-themes.mjs` (Node — FE toolchain already requires it) regex-parses `fe/src/features/settings/data/ui-themes.ts` (102 `name:` entries [VERIFIED by count]) and `terminal-themes.ts` (78 entries [VERIFIED by count]) and emits `crates/webtmux/src/themes_generated.rs`: `UI_THEMES: &[UiThemePreset]` (name/label/`terminalTheme` link + 15 `u32` colors), `TERMINAL_THEMES: &[TerminalThemePreset]` (name/label + 19 `u32` colors). Output is **checked in** — `cargo build` never runs the generator. A headless test asserts `UI_THEMES.len() == 102`, `TERMINAL_THEMES.len() == 78`, every UI `terminalTheme` link resolves, and spot-check hex values (`default-dark.background == 0x1e1e1e`). Regen is manual on FE theme changes; the lengths test is the drift alarm. Rejected: hand-transcription (180 presets × colors = guaranteed typos) and build-script generation (build-time Node dependency + nondeterministic diffs).
- **D2 — Settings model extends, never renames.** Add to `DesktopSettings` with serde defaults (legacy files keep working, mirroring the `confirm_kill_*` `default_true` precedent [VERIFIED: `crates/settings/src/lib.rs:62-71`]): `tmux_binary: String` (`""`), `font_family: String` (`"JetBrains Mono"`), `font_size: f32` (`14.0`), `line_height: f32` (`1.35`), `scrollback_lines: usize` (`2000`), `tui_scroll_default: bool` (`true`), `theme_mode_filter: String` (`"all"`, reference parity [VERIFIED: web-term `crates/settings/src/lib.rs:57-59`]). Keep `theme: Theme` + `theme_preset` (`"default-dark"` [VERIFIED: `crates/settings/src/lib.rs:40-42`]). FE `settingsStore.ts` defaults quoted verbatim as the value source: `tmuxBinary: ''`, `uiTheme: 'default-dark'`, `fontFamily: 'JetBrains Mono, Menlo, Consolas, monospace'`, `fontSize: 14`, `lineHeight: 1.35`, `scrollbackLines: 2000`, `tuiScrollPanes: {}`, confirms `true` [VERIFIED: `fe/src/stores/settingsStore.ts:33-45`]. `tui_scroll()` falls back to `tui_scroll_default` instead of hardcoded `true` (success criterion "per-pane TUI-scroll default").
- **D3 — Live-apply follows the reference `set_theme_preset` shape exactly.** `AppState::set_theme_preset(id, cx)`: look up generated UI table (fallback `[0]`, FE `getUiTheme` parity [VERIFIED: `ui-themes.ts:2491-2493`]), write `settings.theme_preset`, derive `settings.theme = Dark/Light` from `is_dark` (luminance rule, FE `isLightUiTheme` parity [VERIFIED: `ui-themes.ts:2495-2497`]), call existing `theme::apply_theme` (`Theme::change`, [VERIFIED: `crates/webtmux/src/theme.rs:8-15`]), push `ColorPalette::from_rgb_u32(...)` of the **linked terminal preset** (`terminalTheme` name → terminal table, FE `resolvedTerminalTheme` parity [VERIFIED: `ui-themes.ts:2507-2515`]) to every live `terminal_views` entry via existing `set_palette`, store as default for views created later, `cx.notify()`, `settings.save()`. Chrome helpers on `AppState` (`bg/fg/card/border/muted/primary/…`, reference `app_state.rs:889-902` pattern) replace the hardcoded `rgb(0x1e1e1e)` etc. in `tab_strip.rs`, `sidebar.rs`, `session_states.rs`, pane headers — one preset read per render, no caching (presets are `&'static`).
- **D4 — Settings page ports the reference card grid over the existing route.** Replace `render_settings_placeholder` behind the existing `showing_settings` route [VERIFIED: `crates/webtmux/src/views/session_states.rs:37-47`] with `views/settings.rs` ported from the reference: max-w-672 centered column, Appearance section (filter row + `"{n} themes available"` + flex-wrap 3-column `w(px(204))` cards with 56px swatch, 3 dots, 2 bars, check overlay, click-to-apply [VERIFIED reference pattern: web-term `views/settings.rs:296-404`]), Terminal section (tmux-binary input + status line, font-size/line-height/scrollback numeric inputs via gpui-component `InputState`, three kill-confirm `Switch` rows — `Switch::new(id)` [VERIFIED: vendored `switch.rs:31`]). Filter dropdown hand-rolls the reference `show_theme_mode_picker` toggle pattern (no `select` widget port — REQUIREMENTS out-of-scope bans shadcn 1:1 widget ports). FE copy preserved: `"Filter by dark or light appearance"`, `"Choose the same tmux installation…"`, `"The selected theme is applied to every pane and app surface."`
- **D5 — Palette builds on the vendored `command` module, spec'd by FE `CommandPalette.tsx`.** New `actions.rs` with `TogglePalette` (+ per-action structs as needed) bound via `cx.bind_keys([KeyBinding::new("ctrl-shift-p", TogglePalette, None)])` (reference `actions.rs:5-41` pattern). Palette view uses `Command`/`CommandState`/`CommandEntry`/`CommandGroup`/`CommandItem` [VERIFIED: vendored `command/mod.rs` — *"a search field over a filtered list of commands, with groups, Action keybinding hints and keyboard navigation"*; `CommandItem::label/keywords/action/checked` builders VERIFIED in `item.rs`]. Groups mirror FE exactly: Actions = New Session / New Window / Split Right / Split Down / Zoom Pane / Next Layout / Kill Pane [VERIFIED: `fe/src/features/palette/CommandPalette.tsx:48-97`], Open Session = tree session names [VERIFIED: `:98-115`]. Actions reuse existing correlated sends (fire-and-forget like chip clicks); Open Session calls `open_session`. `TerminalView::on_key_down` gets an explicit early-return for ctrl+shift+P so the keystroke bubbles to the action instead of entering the pty (Pitfall 5).
- **D6 — One new REST method, FE-shaped status UX.** Add `RestClient::set_tmux_binary(path) -> Result<TmuxInfo, RestError>` posting `{path}` to `/api/tmux/binary` (existing `extract_error_message` surfaces the backend string; `TmuxInfo{version, ok, binary}` model already exists [VERIFIED: `crates/backend-client/src/models.rs:29-36`]). Backend contract: empty clears override (`Unsetenv`), bare names resolve via `LookPath`, validation runs `resolved -V` under 3s, failure is `400 {"error": "tmux binary %q is not usable: …"}` [VERIFIED: `be/internal/tmux/binary.go:19-43`, `be/internal/server/health.go:64-83`]. Settings page validates on explicit Check/Enter (not on every keystroke — FE `onBlur`/Enter parity [VERIFIED: `TerminalSettings.tsx:16-24,35-38`]); status line shows `Using {binary} ({version})` or the error. Apply-once-on-ready after backend connects (FE `App.tsx:120-127` parity).
- **D7 — Kill-confirm needs switches only, plus a copy audit.** All three gates (`kill_requires_confirm[_pane|_window]`) and all three `Kill*Form` dialogs exist [VERIFIED: `app_state.rs:1607-1615`, grep over `views/`]. Phase 6 adds the three `Switch` rows (FE labels verbatim: `"Confirm before killing pane/window/session"` [VERIFIED: `TerminalSettings.tsx:91-108`]) persisting through existing `save()`. Audit dialog copy against FE (`Kill session "{name}"?` / `"This terminates the tmux session and all processes inside it."` [VERIFIED: `SessionContextMenu.tsx:136-155`]) and the `shouldConfirm(kind)` dispatch shape [VERIFIED: `fe/src/lib/commands.ts:40-50`]. No dialog-behavior changes expected.
- **D8 — Three icons appended, font stays single-family.** Append `CHECK_SVG`, `SEARCH_SVG`, `PAINTBRUSH_SVG` (lucide, `stroke-width="1.5"` to match the 24 existing consts [VERIFIED: `icons.rs:1-53`, bash count 24]) for the card check, palette input, and Appearance header. FE `fontFamily` default is a CSS stack (`'JetBrains Mono, Menlo, Consolas, monospace'`) but GPUI takes one family — settings default is `"JetBrains Mono"` (the embedded face); free-text families render via system fallback with no new embedding (documented, not blocked).
- **D9 — Pref apply rules: font live, scrollback recreates.** Font size/family/line-height apply live via existing `set_font`/`set_font_size` [VERIFIED: `terminal_view.rs:179-193`] on all live views + renderer defaults for new views, then re-arm the debounced viewport (`arm_viewport_for_pane` + `schedule_debounced_resize`) so tmux learns the new cols/rows (FE rebuilds terminals on `[fontFamily, fontSize, lineHeight, …]` change [VERIFIED: `useTerminal.ts:213`]). Scrollback (`scrolling_history` is construction-time [VERIFIED: `terminal/src/terminal.rs:83-86`]) recreates each store `Terminal` with the new limit, then `invalidate_pane_snapshot` + `request_pane_capture` per pane — the `ingestedHistory` exactly-once guard dedupes the re-capture (same mechanism as reconnect). FE clamps preserved: font 8–32, line-height 1–2 step 0.05, scrollback 100–50000 step 100 [VERIFIED: `TerminalSettings.tsx:54-90`].
- **D10 — Headless tests for data + decisions; pixels to Phase 7.** New/extend `crates/webtmux/tests/`: theme-table lengths (102/78), link resolution, `getUiTheme` fallback, luminance buckets for known presets, filter counts, settings serde back-compat (legacy JSON without new keys → defaults; mirrors existing `store_test.rs` legacy test), `set_tmux_binary` request shape against a mock HTTP stub, palette filter/group model if factored pure. Screen-level checks (card pixels, live re-theme flash, palette open/filter/select, font render) join the Phase 7 parity audit as HV items per the Phase-2/4/5 precedent.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `gpui` (`gpui-pre`) | `=0.3.3` [VERIFIED: `desktop-gpui/Cargo.toml:33`] | Settings page views, palette view, keybindings, svg | Milestone-pinned; pre-1.0 churn is why the pin + committed lockfile exist |
| `gpui-component` | `=0.6.0` [VERIFIED: `desktop-gpui/Cargo.toml:34`] | `command` palette module, `Switch`, `InputState`, `Dialog`, `Theme::change` | Vendored palette primitive removes the only custom-widget risk; Switch/Input/Dialog already proven by Phases 3–5 |
| `webtmux-settings` | workspace path | Six new pref fields + existing atomic save/`.bak` | Phase-1 pattern; additive serde defaults only |
| `webtmux-backend-client` | workspace path | New `set_tmux_binary` POST + existing `TmuxInfo` model | One method; error extraction already handles `{error}` bodies |
| `webtmux-terminal` | workspace path | `ColorPalette::from_rgb_u32`, `TerminalRenderer` fields | Construction seam exists; no engine changes |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| node (dev-only, regen) | [ASSUMED] present (FE Vite toolchain) | Runs `scripts/generate-gpui-themes.mjs` | Only when FE theme data changes; build never needs it (output checked in) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Checked-in generated `.rs` | `build.rs` regeneration every build | Rejected: build-time Node dep + noisy diffs; lengths test already guards drift |
| Vendored `command` module | Hand-rolled palette (divs + InputState + list) | Rejected: filter/focus/scroll/keyboard nav already implemented + tested upstream in the pinned crate |
| Linked terminal themes (78 real presets) | Reference-style derived ANSI (fg/bg/primary + 9 hardcoded sets) | Rejected: FE ships exact per-theme ANSI tables; linking is a table lookup, derivation is guesswork |
| Per-keystroke binary validation | Explicit Check/Enter (chosen, FE parity) | Keystroke validation spams `tmux -V` spawns and flickers status; FE validates on blur/Enter |

**Installation:**
```bash
# No new packages. Pins unchanged; lockfile committed.
cargo build -p webtmux
```

**Version verification:** No new dependencies (all imports resolve to `gpui-pre =0.3.3`, `gpui-component =0.6.0`, workspace crates in `desktop-gpui/Cargo.toml:24-55` [VERIFIED]). Nothing to `cargo search`. The `gsd_run query package-legitimacy` seam is not on PATH in this environment, but the gate is vacuous: zero new package names appear anywhere in this research (same standing as Phase 5).

## Package Legitimacy Audit

> No external packages are installed by this phase.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| *(none — no new packages)* | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
FE theme data (ui-themes.ts ×102, terminal-themes.ts ×78)
        │  scripts/generate-gpui-themes.mjs (manual regen only)
        ▼
themes_generated.rs ──► AppState::set_theme_preset(id)
   UI_THEMES ─┐              │ ① settings.theme_preset + theme Dark/Light (luminance)
   TERMINAL_  │ link         │ ② Theme::change(mode) → gpui-component widgets
   THEMES ────┘ name         │ ③ from_rgb_u32(linked) → set_palette(all live views + default)
                             │ ④ cx.notify + settings.save()
                             ▼
              ┌── chrome renders via preset helpers (title/sidebar/headers/dialogs/settings)
              └── palette (Ctrl+Shift+P) + settings page read the same AppState
```

Settings-page data flow: view edits → `AppState` field → immediate apply (D3/D9) → `settings.save()` (atomic, Phase-1 pattern). tmux-binary flow: input → `RestClient::set_tmux_binary` → `POST /api/tmux/binary` → status line (`Using v (x.y)` / error). Palette flow: `ctrl-shift-p` action → `CommandState` dialog → item action → existing correlated send / `open_session`.

### Recommended Project Structure
```
desktop-gpui/crates/webtmux/src/
├── themes_generated.rs   # NEW (D1): UI_THEMES ×102 + TERMINAL_THEMES ×78 + lookups
├── views/settings.rs     # NEW (D4): settings page (replaces placeholder route)
├── views/palette.rs      # NEW (D5): command palette view + CommandState
├── actions.rs            # NEW (D5): TogglePalette (+ item actions), bind_keys
├── app_state.rs          # EXTEND: preset helpers, set_theme_preset, palette state
├── theme.rs              # EXTEND: generated-preset lookups (keep apply_theme)
├── icons.rs              # EXTEND: CHECK + SEARCH + PAINTBRUSH (D8)
desktop-gpui/crates/settings/src/lib.rs  # EXTEND: 7 new fields + defaults (D2)
desktop-gpui/crates/backend-client/src/rest.rs  # EXTEND: set_tmux_binary (D6)
scripts/generate-gpui-themes.mjs  # NEW (D1): TS → Rust generator
```

### Pattern 1: Linked theme resolution (FE `resolvedTerminalTheme` parity)
**What:** UI preset owns a `terminalTheme` link name; terminal palette resolves through it. The legacy explicit terminal-override is ignored — one theme for the whole app.
**When to use:** Every terminal palette derivation (live apply + new-view defaults).
**Example:**
```rust
// Source: fe/src/features/settings/data/ui-themes.ts:2507-2515 (resolvedTerminalTheme)
pub fn terminal_preset_for_ui(ui_name: &str) -> &'static TerminalThemePreset {
    let ui = ui_preset_by_name(ui_name); // fallback UI_THEMES[0], getUiTheme parity
    terminal_preset_by_name(ui.terminal_theme) // getMappedTerminalTheme parity
}
```

### Pattern 2: Reference `set_theme_preset` live-apply (D3)
**What:** Single mutation point syncs persisted preset, Dark/Light mode, widget theme, all terminal palettes, and disk.
**When to use:** Card click in settings; load-time re-sync.
```rust
// Source: E:\Coding Stuff\web-term\desktop-gpui\crates\webterm\src\app_state.rs:866-881
pub fn set_theme_preset(&mut self, preset_id: &str, cx: &mut Context<Self>) {
    self.settings.theme_preset = preset_id.to_string();
    let preset = ui_preset_by_name(preset_id);
    self.theme = if preset.is_dark { Dark } else { Light };
    self.settings.theme = self.theme;
    crate::theme::apply_theme(self.theme, cx);
    let palette = palette_for_ui_preset(preset);
    for view in self.terminal_views.values() { view.update(cx, |v, cx| v.set_palette(palette.clone(), cx)); }
    let _ = self.settings.save();
    cx.notify();
}
```

### Pattern 3: Palette items as data over existing sends (D5)
**What:** `CommandItem::label(...).keywords([...]).action(boxed Action)` entries in two `CommandGroup`s; actions dispatch through `.on_action` handlers that call the same correlated sends as buttons/menus.
**When to use:** All 7 FE actions + one entry per tree session in Open Session.
```rust
// Source: vendored gpui-component-0.6.0/src/command/item.rs (label/keywords/action/checked builders)
// + fe/src/features/palette/CommandPalette.tsx:48-115 (groups + items)
CommandItem::new().label("Split Right").keywords(["split", "horizontal", "pane"])
```

### Anti-Patterns to Avoid
- **Hand-rolling a palette list widget:** the vendored `command` module already owns filter/focus/scroll/keyboard — custom code reintroduces all four bug classes.
- **Caching resolved colors in views:** presets are `&'static`; read helpers per render so a theme pick repaints everything on the next `cx.notify()` with no invalidation bookkeeping.
- **Optimistic theme writes:** apply-then-save is local-only (no server round-trip), but still write `settings.save()` synchronously in the same handler — a crash between paint and save resurrects the old theme on reboot.
- **Per-keystroke `tmux -V` validation:** spawns a process per keystroke; validate on explicit Check/Enter (FE parity).
- **Deriving ANSI tables from chrome colors:** the 78 real terminal tables exist — use them; derivation mismatches vim/htop colors vs Electron.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Command-palette filter/focus/keyboard | Custom searchable list | `gpui-component::command::{Command, CommandState, CommandEntry, CommandGroup, CommandItem}` | Vendored + pinned; groups, keybinding hints, keyboard nav included |
| Toggle switches | Custom bool rows | `gpui-component::Switch::new(id)` | Stock behavior + theming; proven import path |
| Text/number inputs | Custom editors | gpui-component `InputState` (rename-dialog pattern, Phase 3) | Prefill/autofocus/submit guards already solved |
| Terminal ANSI tables | Hand-tuned palettes | Generated 78-preset table + `ColorPalette::from_rgb_u32` | 19 exact colors per theme; 256-table derives automatically via `generate_256_table` [VERIFIED: `colors.rs:113-139`] |
| Dark/light bucketing | Eyeballing backgrounds | Ported `themeLuminance` (WCAG weights, `> 0.5`) | [VERIFIED: `terminal-themes.ts:2004-2015`]; decides filter + `colorScheme` + `ThemeMode` consistently |
| tmux binary validation | Client-side PATH probing | `POST /api/tmux/binary` | Backend runs the real `tmux -V` probe with timeout + error strings |

**Key insight:** Phase 6 looks large (180 presets, a page, a palette) but every hard problem is already solved somewhere: the data exists in FE, the palette widget exists in the pinned crate, the apply seams exist in `TerminalView`, and the reference proves the settings-page shape. The work is porting + wiring, guarded by counts tests.

## Common Pitfalls

### Pitfall 1: Ctrl+Shift+P eaten by the focused terminal
**What goes wrong:** Palette never opens when a pane is focused because `TerminalView::on_key_down` converts the keystroke to pty bytes first.
**Why it happens:** GPUI dispatches keystrokes to the focused view before window/app-level bindings; the terminal handler has no allowlist.
**How to avoid:** D5's explicit early-return for ctrl+shift(+platform)+`p` in `on_key_down` + App-level `bind_keys`; headless-test the guard via `decide_key_route`-style pure fn if factored.
**Warning signs:** Palette opens from sidebar focus but not from terminal focus.

### Pitfall 2: Stale `Theme` enum vs picked preset after restart
**What goes wrong:** `theme_preset = "ocean-light"` but `theme = Dark` (legacy file) → gpui-component widgets render dark under a light chrome.
**Why it happens:** Two sources of truth (`theme` enum + `theme_preset` name) can disagree; `apply_theme` reads only the enum.
**How to avoid:** On load AND on pick, re-derive `settings.theme` from preset `is_dark` (D3); save immediately. FE never has this bug (single `uiTheme` string + derived `colorScheme` [VERIFIED: `App.tsx:107-110`]).
**Warning signs:** Restart flips widget styling while chrome stays correct.

### Pitfall 3: Scrollback change without re-capture blanks history
**What goes wrong:** Recreating store `Terminal`s with a new `scrollback_limit` drops buffer; panes show live output only.
**Why it happens:** `scrolling_history` is construction-time; new instance starts empty.
**How to avoid:** D9 — recreate, then `invalidate_pane_snapshot` + `request_pane_capture` per pane; the `ingestedHistory` guard prevents doubling.
**Warning signs:** Scrollback setting change → `Shift+PgUp` shows nothing until reconnect.

### Pitfall 4: Font change without viewport re-arm desyncs tmux geometry
**What goes wrong:** New cell size changes cols/rows but tmux still formats for the old viewport → wrapped/truncated lines.
**Why it happens:** `set_font` updates renderer estimates; tmux learns size only via `terminal.resize`.
**How to avoid:** D9 — re-measure + `arm_viewport_for_pane` + `schedule_debounced_resize` after every font apply (same dance as window resize, TERM-06).
**Warning signs:** Font bump → pane content wraps wrong until window nudge.

### Pitfall 5: Generator drift when FE adds a theme
**What goes wrong:** FE ships 103 UI themes; GPUI still shows 102 with no error.
**Why it happens:** Checked-in output is a snapshot.
**How to avoid:** D1 lengths test fails loudly; regen is one `node` command. Document the regen step in the script header.
**Warning signs:** Settings shows "102 themes available" while FE shows 103.

### Pitfall 6: Light theme with dark OS frame
**What goes wrong:** Light chrome under a dark Windows title-bar/frame looks broken.
**Why it happens:** `main.rs` forces DWM dark frame unconditionally (`dark: 1`, attrs 19+20 [VERIFIED: `main.rs:63-84`]).
**How to avoid:** Accept as v1.0 deviation (reference does the same — ported verbatim Phase 1); record in the Phase 7 parity audit so it is not re-argued. Optional: set attr from `is_dark` at startup only (no live toggle — DWM attrs apply at window creation).
**Warning signs:** UAT flags "title bar doesn't match theme".

### Pitfall 7: `fontFamily` free text implying multi-font support
**What goes wrong:** User types `"Fira Code, monospace"`; GPUI takes it as one family name and falls back unpredictably.
**Why it happens:** FE accepts CSS stacks; GPUI `Font.family` is a single name and only JetBrains Mono is embedded.
**How to avoid:** D8 — label the field honestly (`"Font family (embedded: JetBrains Mono)"`), default `"JetBrains Mono"`, no stack parsing.
**Warning signs:** Font field change renders tofu/fallback.

## Code Examples

### UI preset row (generator output shape)
```rust
// Source: fe/src/features/settings/data/ui-themes.ts:12-36 (interface) + :38-62 (default-dark values)
pub struct UiThemePreset {
    pub name: &'static str,        // stable key, persisted (e.g. "default-dark")
    pub label: &'static str,       // display name (e.g. "Default")
    pub terminal_theme: &'static str, // linked terminal preset (e.g. "default")
    pub is_dark: bool,             // from themeLuminance(background) > 0.5 at generate time
    pub background: u32, pub foreground: u32, pub card: u32, pub card_foreground: u32,
    pub primary: u32, pub primary_foreground: u32, pub secondary: u32,
    pub secondary_foreground: u32, pub muted: u32, pub muted_foreground: u32,
    pub accent: u32, pub accent_foreground: u32, pub destructive: u32,
    pub border: u32, pub input: u32, pub ring: u32,
}
```

### Terminal preset → palette (existing seam)
```rust
// Source: desktop-gpui/crates/terminal/src/colors.rs:186-208 (from_rgb_u32)
pub fn palette_for_terminal_preset(p: &TerminalThemePreset) -> ColorPalette {
    ColorPalette::from_rgb_u32(
        hex(p.foreground), hex(p.background), hex(p.cursor),
        hex(p.foreground), // selection rides fg w/ alpha (from_rgb_u32 applies 0x66)
        [hex(p.black), hex(p.red), hex(p.green), hex(p.yellow),
         hex(p.blue), hex(p.magenta), hex(p.cyan), hex(p.white),
         hex(p.bright_black), hex(p.bright_red), hex(p.bright_green),
         hex(p.bright_yellow), hex(p.bright_blue), hex(p.bright_magenta),
         hex(p.bright_cyan), hex(p.bright_white)],
    )
}
```

### tmux-binary REST method (only missing client piece)
```rust
// Source: fe/src/lib/api.ts:56-57 (setTmuxBinary) + crates/backend-client/src/rest.rs (create_session pattern :117-143)
pub async fn set_tmux_binary(&self, path: &str) -> Result<TmuxInfo, RestError> {
    let url = format!("{}/api/tmux/binary", self.base_url);
    let resp = self.http.post(&url).json(&serde_json::json!({ "path": path }))
        .send().await.map_err(|e| RestError::Request { url: url.clone(), source: e })?;
    if !resp.status().is_success() { /* extract_error_message, mirror create_session */ }
    resp.json::<TmuxInfo>().await.map_err(|e| RestError::Decode { url, source: e })
}
```

### Keybinding registration (reference pattern)
```rust
// Source: E:\Coding Stuff\web-term\desktop-gpui\crates\webterm\src\actions.rs:5-41
actions!(webtmux_palette, [TogglePalette]);
pub fn bind_palette_keys(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-shift-p", TogglePalette, None)]);
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Reference derives terminal ANSI from chrome colors (+ 9 hardcoded sets) | Link to 78 exact terminal tables | Phase 6 (this phase) | vim/htop colors match Electron exactly instead of approximately |
| FE `terminalTheme` explicit override | Override ignored; terminal follows UI theme | Pre-GSD (FE `resolvedTerminalTheme`) | One theme for the whole app; nothing to port for the override |
| Settings placeholder page | Full settings page | Phase 6 (this phase) | `showing_settings` route already exists; swap the view |
| No palette in reference/client | Vendored `command` module | gpui-component 0.6.0 (pinned) | Palette is assembly, not invention |

**Deprecated/outdated:**
- `TerminalView` fixed `dark_default()` palette + 14px/1.35 constants (`terminal_view.rs:64-69`, Phase-4 D2): replaced by preset-driven palette + settings-driven font in this phase
- `TerminalConfig` hardcoded 2000 (`terminal.rs:52-61`, Phase-4 D1): fed from `scrollback_lines` in this phase
- `tui_scroll` hardcoded `unwrap_or(true)` (`app_state.rs:915-917`, Phase-4 D3): falls back to `tui_scroll_default` in this phase

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Node is available in the dev environment for running the one-shot generator | Standard Stack / D1 | LOW — generator runs once by the executor; if absent, hand-port + lengths test still works, or run on any machine with node and copy the file |
| A2 | `KeyBinding::new("ctrl-shift-p", …)` string is accepted by gpui-pre 0.3.3 keystroke parsing | D5 | MEDIUM — reference binds `"ctrl-shift-tab"` [VERIFIED in reference actions.rs], so multi-modifier + letter follows the same grammar; a compile-time/runtime check in Wave 0 confirms |
| A3 | Vendored `Command`/`CommandState` render API fits a modal dialog over the workspace without extra deps | D5 | MEDIUM — module doc + item builders verified, `Command` struct render API not yet read; Wave 0 must read `command.rs`/`state.rs` signatures before planning the view task |
| A4 | `Switch`/`InputState` theming follows `Theme::change` Light/Dark automatically | D3/D4 | LOW — reference settings page uses the same subsystem under both modes; visual check lands in Phase 7 audit |
| A5 | FE theme-file shape stays regex-parseable (flat `name:`/`label:`/hex colors) | D1 | LOW — files are machine-generated with a stable shape ("Generated by scripts — do not hand-edit"); generator test fails loudly on shape change |

## Open Questions

1. **Should the palette also expose Settings/rename actions beyond the FE 7 + Open Session?**
   - What we know: FE lists exactly 7 actions + Open Session group; reference has no palette at all.
   - What's unclear: whether executor will gold-plate; scope says Electron parity.
   - Recommendation: lock to the FE list (DLG-01 names "incl. Open Session group" only); extras are v2.

2. **Does `window.create` from the palette need an open session context like FE's `activeWindow` guard?**
   - What we know: FE `New Window` sends on the active session socket with no guard; pane actions guard on `activePane`.
   - What's unclear: nothing material — mirror the guards (window actions need active session; pane actions need active pane).
   - Recommendation: mirror FE guards verbatim; disabled/hidden when inapplicable.

## Environment Availability

> Phase 6 is code/config-only with no new packages; the generator runs once on the executor machine and its output is checked in, so no build-time external dependency exists.

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo + rustc | build + tests | ✓ (prior phases green: 101/101 Phase 5) | 1.96.0 (per 01-RESEARCH env audit) | — |
| Go backend (`tmux-gui-server`) | `set_tmux_binary` interop test | ✓ (repo-root binary, prior phases) | — | Mock HTTP stub for the REST shape test |
| node | one-shot theme generator | [ASSUMED] (FE Vite toolchain) | — | Hand-port + lengths test; or run generator elsewhere and copy output |

**Missing dependencies with no fallback:** none
**Missing dependencies with fallback:** node (fallback above; build never needs it)

## Validation Architecture

> `.planning/config.json` sets no `workflow.nyquist_validation` key → treated as enabled; section included.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust built-in, no config file) |
| Config file | none — workspace `Cargo.toml` + existing `crates/webtmux/tests/` |
| Quick run command | `cargo test -p webtmux -p webtmux-settings` |
| Full suite command | `cargo test --workspace` (from `desktop-gpui/`) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| THEME-01 | 102 UI + 78 terminal rows; every link resolves; spot hex | unit | `cargo test -p webtmux theme_tables` | ❌ Wave 0 (`tests/theme_test.rs` + generator) |
| THEME-02 | preset pick → mode sync + palette push + save (pure parts) | unit | `cargo test -p webtmux theme_apply` | ❌ Wave 0 |
| THEME-03 | luminance buckets match FE for all 102 backgrounds | unit | `cargo test -p webtmux theme_luminance` | ❌ Wave 0 |
| SET-01 | filter counts (all/dark/light sum correctly) | unit | `cargo test -p webtmux settings_filter` | ❌ Wave 0 |
| SET-02 | clamps (8–32 / 1–2 / 100–50000); default fallbacks | unit | `cargo test -p webtmux terminal_prefs` | ❌ Wave 0 |
| SET-03 | POST shape `{path}` → `TmuxInfo` / `{error}` surfaced | unit (mock HTTP) | `cargo test -p webtmux-backend-client set_tmux_binary` | ❌ Wave 0 |
| SET-04 | three switches persist round-trip | unit | `cargo test -p webtmux-settings kill_switches` | ❌ Wave 0 (extend `store_test.rs`) |
| SHELL-04 | legacy settings JSON (no new keys) loads with defaults | unit | `cargo test -p webtmux-settings back_compat` | ❌ Wave 0 (mirror existing legacy test) |
| DLG-01 | palette groups (7 actions + sessions) + filter matching | unit | `cargo test -p webtmux palette_model` | ❌ Wave 0 (only if model factored pure; else manual-UAT) |
| DLG-03 | gates already covered (`kill_confirm_test.rs` exists) | unit | `cargo test -p webtmux kill_confirm` | ✅ exists |
| SHELL-05 | font embedded + icons render | manual-UAT → Phase 7 | — | n/a (pixels not headless-assertable) |

### Sampling Rate
- **Per task commit:** `cargo test -p webtmux -p webtmux-settings -p webtmux-backend-client`
- **Per wave merge:** `cargo test --workspace` (from `desktop-gpui/`)
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `crates/webtmux/src/themes_generated.rs` — D1 output (102 + 78 tables + lookups)
- [ ] `crates/webtmux/tests/theme_test.rs` — lengths/links/luminance/filter
- [ ] `crates/settings/tests/store_test.rs` — extend: new-field defaults + legacy back-compat
- [ ] `crates/backend-client` binary test — `set_tmux_binary` shape (mock stub or interop)
- [ ] `scripts/generate-gpui-themes.mjs` — D1 generator
- [ ] `crates/webtmux/src/actions.rs` — D5 keybinding (compile + keystroke-string check)
- [ ] Read vendored `command/command.rs` + `command/state.rs` render API before planning the palette view task (A3)

*(Existing `kill_confirm_test.rs`, `window_state_test.rs`, terminal tests keep covering DLG-03/SHELL-04-geometry.)*

## Security Domain

> `security_enforcement` is not disabled in config → section included. Scope: settings inputs + a path-valued setting.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | localhost-only backend, no auth surface touched |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes | Numeric clamps (FE values, D9); `tmux_binary` is an opaque string to the client — validation happens server-side via `tmux -V` probe, client never executes it |
| V6 Cryptography | no | No secrets in this phase (settings file already 0600 on unix, Phase 1) |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious `tmux_binary` path → command execution | Tampering / Elevation | Backend allowlist-by-probe: only `tmux -V`-passing executables are stored; backend is localhost-only [VERIFIED: `health.go:62-63` comment]; client sends the string, never spawns it |
| Corrupt settings file → launch break / pref injection | Tampering | Existing `.bak` recovery + serde defaults (Phase 1, extended in D2) |
| Generator script supply chain | Tampering | Script is local-only, reads two in-repo files, writes one in-repo file; output is reviewed diff, lengths test pins counts |

## Sources

### Primary (HIGH confidence)
- `fe/src/features/settings/data/ui-themes.ts` (102 presets, interface, `getUiTheme`/`isLightUiTheme`/`resolvedTerminalTheme`) + `terminal-themes.ts` (78 presets, `getTerminalTheme`/`themeLuminance`) — Read + counted this session
- `fe/src/stores/settingsStore.ts:33-45` (defaults), `fe/src/features/settings/{SettingsPage,UiThemeSettings,TerminalSettings,ThemeCard}.tsx`, `fe/src/features/palette/CommandPalette.tsx`, `fe/src/lib/commands.ts:40-50`, `fe/src/lib/api.ts:56-57`, `fe/src/App.tsx:71-127,176-178`, `fe/src/features/terminal/useTerminal.ts:84-105,213`, `fe/src/hooks/useKeyboardShortcut.ts:13-25` — Read this session
- `be/internal/server/health.go:64-83`, `be/internal/tmux/binary.go:19-43` — Read this session
- `desktop-gpui/crates/{settings/src/lib.rs,webtmux/src/{theme,icons,window_state,views/session_states,views/terminal_view,app_state},terminal/src/{colors,terminal,render,lib},backend-client/src/{rest,models}}.rs` — Read this session
- `E:\Coding Stuff\web-term\desktop-gpui\crates\webterm\src\{theme.rs,actions.rs,views/settings.rs,app_state.rs}` + `crates/settings/src/lib.rs` — Read this session
- Vendored `gpui-component-0.6.0/src/{command/mod.rs,command/item.rs,switch.rs,lib.rs}` — Read this session
- Prior phases: `01-RESEARCH.md` (pins, settings pattern, DWM), `04-RESEARCH.md` (D1/D2/D3 deferrals to Phase 6, FE defaults), `04-CONTEXT.md`, `05-RESEARCH.md` (D7 kill flags, DLG1 pattern), `05-02-SUMMARY.md` context

### Secondary (MEDIUM confidence)
- None — no web lookups were needed; all ground truth is in-repo.

### Tertiary (LOW confidence)
- A1–A5 in Assumptions Log (keybinding string grammar, `Command` render API fit, node presence) — flagged for Wave 0 verification, never stated as fact above.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — zero new packages; all imports resolve to pinned workspace deps verified in `Cargo.toml`
- Architecture: HIGH — every seam (generator I/O, `set_theme_preset` shape, `set_palette`/`set_font`, `set_tmux_binary`, `command` module, kill gates) Read-verified on both FE and GPUI sides
- Pitfalls: HIGH for port mechanics (re-capture, re-arm, dual-source theme truth all have in-repo precedent); MEDIUM for keybinding dispatch ordering (A2) and `Command` render-API fit (A3) — both have explicit Wave 0 reads

**Research date:** 2026-09-07
**Valid until:** 30 days (stable domain: pinned crates + in-repo FE data; only FE theme additions age D1's counts, guarded by test)
