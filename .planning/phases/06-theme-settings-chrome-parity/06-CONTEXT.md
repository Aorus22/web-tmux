# Phase 6: Theme System, Settings & Chrome Parity - Context

**Gathered:** 2026-09-07
**Status:** Ready for planning
**Mode:** Auto-accepted (autonomous planning lane, user sleeping; D1–D10 ratified de-facto by 06-RESEARCH.md, flagged for post-hoc audit per the Phase-4/5 precedent)

<domain>
## Phase Boundary

Appearance and preferences reach Electron parity inside the Phase-5 workspace: a checked-in
`themes_generated.rs` (102 UI + 78 terminal presets via `scripts/generate-gpui-themes.mjs`),
`AppState::set_theme_preset` live-apply (preset → Dark/Light → `Theme::change` → palette push
to every terminal → persist), a full Settings page over the existing `showing_settings` route
(3-column card grid + dark/light filter, terminal prefs, tmux-binary validation, three
kill-confirm switches), one new REST method (`POST /api/tmux/binary`), the command palette on
Ctrl+Shift+P (vendored `command` module, 7 actions + Open Session group), three new lucide
icons, and live font/scrollback pref apply through the existing `set_palette`/`set_font` seams.
Delivers SHELL-04, SHELL-05, SET-01, SET-02, SET-03, SET-04, THEME-01, THEME-02, THEME-03,
DLG-01, DLG-03. Out: backend changes (validation probe already exists in `be/`), palette
actions beyond the FE 7 + Open Session (v2), multi-font stacks (single embedded family),
screen-pixel proof (Phase 7 parity audit).

</domain>

<decisions>
## Implementation Decisions

### Generated theme tables (D1, THEME-01)
- New `scripts/generate-gpui-themes.mjs` (Node, FE toolchain already requires it) regex-parses
  `fe/src/features/settings/data/ui-themes.ts` (102 `name:` entries) and `terminal-themes.ts`
  (78 entries) and emits `desktop-gpui/crates/webtmux/src/themes_generated.rs`:
  `UI_THEMES: &[UiThemePreset]` (name/label/`terminalTheme` link + 15 `u32` colors),
  `TERMINAL_THEMES: &[TerminalThemePreset]` (name/label + 19 `u32` colors).
- Output is **checked in** — `cargo build` never runs the generator. Headless
  `tests/theme_test.rs` asserts 102/78 lengths, every UI `terminalTheme` link resolves, and
  spot-check hex (`default-dark.background == 0x1e1e1e`). Regen is manual on FE theme changes.
- Rejected: hand-transcription (180 presets = guaranteed typos), build-script generation
  (build-time Node dep + nondeterministic diffs).

### Settings model extends, never renames (D2, SHELL-04/SET-02)
- Add to `DesktopSettings` with serde defaults (legacy files keep working, mirroring the
  `confirm_kill_*` `default_true` precedent): `tmux_binary: String` (`""`),
  `font_family: String` (`"JetBrains Mono"`), `font_size: f32` (`14.0`),
  `line_height: f32` (`1.35`), `scrollback_lines: usize` (`2000`),
  `tui_scroll_default: bool` (`true`), `theme_mode_filter: String` (`"all"`).
- Keep `theme: Theme` + `theme_preset` (`"default-dark"`). FE `settingsStore.ts:33-45`
  defaults are the verbatim value source. `tui_scroll()` falls back to `tui_scroll_default`
  instead of hardcoded `true`.

### Live-apply shape (D3, THEME-02/THEME-03)
- `AppState::set_theme_preset(id, cx)`: look up generated UI table (fallback `[0]`, FE
  `getUiTheme` parity), write `settings.theme_preset`, derive `settings.theme = Dark/Light`
  from `is_dark` (luminance rule, FE `isLightUiTheme` parity), call existing
  `theme::apply_theme` (`Theme::change`), push `ColorPalette::from_rgb_u32(...)` of the
  **linked** terminal preset to every live `terminal_views` entry via existing `set_palette`,
  store as default for views created later, `cx.notify()`, `settings.save()`.
- Chrome helpers on `AppState` (`bg/fg/card/border/muted/primary/…`) replace hardcoded
  `rgb(0x…)` in `tab_strip.rs`, `sidebar.rs`, `session_states.rs`, pane headers — one preset
  read per render, no caching (presets are `&'static`). Terminals **always follow the UI
  theme** through the link; the legacy explicit terminal override stays ignored
  (`resolvedTerminalTheme` parity).
- Load-time re-sync: re-derive `settings.theme` from preset `is_dark` on startup (Pitfall 2).

### Settings page (D4, SET-01/SET-02/SET-04)
- Replace `render_settings_placeholder` behind the existing `showing_settings` route with
  `views/settings.rs` ported from the web-term reference: max-w-672 centered column,
  Appearance section (filter row + `"{n} themes available"` + flex-wrap 3-column `w(px(204))`
  cards with 56px swatch, 3 dots, 2 bars, check overlay, click-to-apply), Terminal section
  (tmux-binary input + status line, font-size/line-height/scrollback numeric inputs via
  gpui-component `InputState`, three kill-confirm `Switch` rows via `Switch::new(id)`).
- Filter dropdown hand-rolls the reference toggle pattern (no `select` widget port).
- FE copy preserved: `"Filter by dark or light appearance"`,
  `"The selected theme is applied to every pane and app surface."`

### Command palette (D5, DLG-01)
- New `actions.rs` with `TogglePalette` bound via
  `cx.bind_keys([KeyBinding::new("ctrl-shift-p", TogglePalette, None)])` (reference pattern).
- Palette view on the vendored `command` module
  (`Command`/`CommandState`/`CommandEntry`/`CommandGroup`/`CommandItem`): Actions group =
  New Session / New Window / Split Right / Split Down / Zoom Pane / Next Layout / Kill Pane
  (FE `CommandPalette.tsx:48-97`), Open Session group = tree session names (`:98-115`).
- Actions reuse existing correlated sends (fire-and-forget like chip clicks); Open Session
  calls `open_session`. `TerminalView::on_key_down` gets an explicit early-return for
  ctrl+shift+P so the keystroke bubbles to the action (Pitfall 1). Locked to the FE list —
  extras are v2.

### tmux binary validation (D6, SET-03)
- Add `RestClient::set_tmux_binary(path) -> Result<TmuxInfo, RestError>` posting `{path}` to
  `/api/tmux/binary` (existing `extract_error_message`; `TmuxInfo` model already exists).
- Backend contract (already shipped): empty clears override, bare names resolve via
  `LookPath`, `resolved -V` under 3s, failure is `400 {"error": …}`.
- Settings page validates on explicit Check/Enter only (FE `onBlur`/Enter parity — never
  per-keystroke). Status line shows `Using {binary} ({version})` or the error.
  Apply-once-on-ready after backend connects (FE `App.tsx:120-127` parity).

### Kill-confirm wiring (D7, SET-04/DLG-03)
- All three gates (`kill_requires_confirm[_pane|_window]`) and all three `Kill*Form` dialogs
  already exist — Phase 6 adds only the three `Switch` rows (FE labels verbatim:
  `"Confirm before killing pane/window/session"`) persisting through existing `save()`.
- Audit dialog copy against FE (`Kill session "{name}"?` /
  `"This terminates the tmux session and all processes inside it."`) and the
  `shouldConfirm(kind)` dispatch shape. No dialog-behavior changes expected.

### Icons + font (D8, SHELL-05)
- Append `CHECK_SVG`, `SEARCH_SVG`, `PAINTBRUSH_SVG` (lucide, `stroke-width="1.5"` to match
  the 24 existing consts) for card check, palette input, Appearance header.
- Settings default is `"JetBrains Mono"` (the embedded face); free-text families render via
  system fallback with no new embedding (documented, not blocked).

### Pref apply rules (D9, SET-02)
- Font size/family/line-height apply live via existing `set_font`/`set_font_size` on all
  live views + renderer defaults for new views, then re-arm the debounced viewport
  (`arm_viewport_for_pane` + `schedule_debounced_resize`) so tmux learns the new cols/rows.
- Scrollback (`scrolling_history` is construction-time) recreates each store `Terminal` with
  the new limit, then `invalidate_pane_snapshot` + `request_pane_capture` per pane — the
  `ingestedHistory` exactly-once guard dedupes the re-capture.
- FE clamps preserved: font 8–32, line-height 1–2 step 0.05, scrollback 100–50000 step 100.

### Headless tests; pixels to Phase 7 (D10)
- New/extend `crates/webtmux/tests/`: theme-table lengths (102/78), link resolution,
  `getUiTheme` fallback, luminance buckets, filter counts, settings serde back-compat,
  `set_tmux_binary` shape (mock HTTP stub), palette filter/group model if factored pure.
- Screen-level checks (card pixels, live re-theme flash, palette open/filter/select, font
  render) join the Phase 7 parity audit as HV items per the Phase-2/4/5 precedent.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets (from Phases 1–5)
- `TerminalView::set_palette/set_font/set_font_size` live-apply seams (`views/terminal_view.rs:168-193`)
- `ColorPalette::from_rgb_u32` construction seam (`terminal/src/colors.rs:186-208`)
- `theme::apply_theme` (`Theme::change`) bridge (`theme.rs:8-15`); hardcoded `rgb(0x…)` helpers (`theme.rs:18-23`) to be preset-driven
- All three kill gates + `Kill*Form` dialogs (`app_state.rs:1607-1615`, `views/`)
- `showing_settings` route + `render_settings_placeholder` (`views/session_states.rs:37-47`)
- `JetBrains Mono` embedded (`main.rs:42-44`); 24 lucide consts (`icons.rs`)
- `TmuxInfo{version, ok, binary}` model (`backend-client/src/models.rs:29-36`); `extract_error_message` pattern (`rest.rs`)
- `terminal_views: HashMap<String, Entity<TerminalView>>` store (`app_state.rs:474`)
- Vendored `gpui-component 0.6.0` `command` module + `Switch::new(id)` + `InputState`

### Established Patterns
- Exact-pinned deps + committed Cargo.lock; zero new packages this phase
- Atomic commits per task; TDD RED→GREEN for pure modules
- Per-target cargo invocations with `CARGO_BUILD_JOBS=2..4` under `CARGO_TARGET_DIR=C:\cargo-target\web-tmux` (E: drive full, Phase-4/5 precedent)
- Single-filter verify rule: one TESTNAME per `cargo test` invocation
- Correlated sends for mutations, fire-and-forget for selects; poll re-render-safe stable element ids

### Integration Points
- `fe/src/features/settings/data/ui-themes.ts` + `terminal-themes.ts` are the data ground truth (generator input)
- `fe/src/features/palette/CommandPalette.tsx:48-115` is the palette item spec
- `fe/src/stores/settingsStore.ts:33-45` is the defaults source; `fe/src/lib/api.ts:56-57` is the binary-client shape
- `be/internal/server/health.go:64-83` + `be/internal/tmux/binary.go:19-43` is the binary-validation contract (no backend work)
- Web-term reference `views/settings.rs:296-404` + `app_state.rs:866-881` (`set_theme_preset`) is the port shape
</code>

<specifics>
## Specific Ideas

- "DATA-PORT, NOT ENGINE" — every hard problem is already solved somewhere (FE data, vendored
  palette widget, `TerminalView` seams, reference settings shape); the work is porting + wiring
  guarded by counts tests
- Lengths test is the drift alarm: FE adds a theme → test fails loudly → one `node` regen
- One theme for the whole app: terminal follows the linked preset, override ignored
- User is away: decisions auto-accepted; screen-level proof deferred to Phase 7 HV items
</specifics>

<deferred>
## Specific Ideas / Deferred Ideas

- Palette actions beyond the FE 7 + Open Session (Settings/rename shortcuts) — v2
- Multi-font embedding / CSS font-stack parsing — documented fallback only
- DWM dark-frame toggle on theme change (reference does the same; Phase 7 audit records it)
- Per-keystroke binary validation — rejected (FE parity is Check/Enter)
- `build.rs` theme regeneration — rejected (build-time Node dep)
</deferred>
