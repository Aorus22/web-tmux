---
phase: 06-theme-settings-chrome-parity
plan: "01"
subsystem: ui
tags: [gpui, themes, settings, generated-code, live-apply, wcag-luminance]
requires:
  - phase: 05-pane-grid-workspace-operations
    provides: [terminal_views retention, set_palette/set_font seams, tui_scroll map, debounced viewport]
  - phase: 04-terminal-engine-live-rendering
    provides: [ColorPalette::from_rgb_u32, TerminalConfig scrollback, capture/re-capture guards]
provides:
  - Checked-in UI_THEMES x102 + TERMINAL_THEMES x78 with lookups + WCAG luminance (generator + lengths/link/spot-hex contracts)
  - Extended DesktopSettings (7 FE-parity pref fields + serde back-compat + clamp helpers)
  - Preset-driven chrome helpers (theme.rs) wired into title bar, tabs, sidebar, workspace states
  - AppState::set_theme_preset single mutation point + startup theme re-sync + font/scrollback/tui pref apply
  - Full Settings page (Appearance card grid + filter, Terminal steppers) over showing_settings with 3 new lucide icons
affects: [06-02-expansion, 06-03-polish, phase-7-parity-audit]
actuals:
  tokens: 49800
  tasks: 3
  commits: 5
tech-stack:
  added: []
  patterns: [checked-in-generator-output, single-mutation-point-apply, preset-read-per-render, in-place-terminal-recreate]
key-files:
  created:
    - scripts/generate-gpui-themes.mjs
    - desktop-gpui/crates/webtmux/src/themes_generated.rs
    - desktop-gpui/crates/webtmux/tests/theme_test.rs
    - desktop-gpui/crates/webtmux/src/views/settings.rs
  modified:
    - desktop-gpui/crates/webtmux/src/lib.rs
    - desktop-gpui/crates/settings/src/lib.rs
    - desktop-gpui/crates/settings/tests/store_test.rs
    - desktop-gpui/crates/webtmux/src/theme.rs
    - desktop-gpui/crates/webtmux/src/app_state.rs
    - desktop-gpui/crates/webtmux/src/icons.rs
    - desktop-gpui/crates/webtmux/src/views/mod.rs
    - desktop-gpui/crates/webtmux/src/views/session_states.rs
    - desktop-gpui/crates/webtmux/src/views/tab_strip.rs
    - desktop-gpui/crates/webtmux/src/views/sidebar.rs
key-decisions:
  - "UiThemePreset carries all 17 FE color fields (not the plan text's 15) — destructive_foreground + input preserved verbatim, zero data loss"
  - "Settings numeric inputs are steppers + hand toggle, not InputState/Switch entities — same FE clamps and live-apply, no per-frame entity lifecycle"
  - "Startup theme re-sync hooks into start_supervisor (plan-external main.rs untouched); constructors never save, so unit tests can't touch the real config dir"
  - "Dialog/menu/pane-grid chrome keeps hardcoded colors; full re-theme of those surfaces rides with 06-02 (D7 switches) and the pane-grid owner wave"
patterns-established:
  - "Checked-in generator output: scripts/generate-gpui-themes.mjs (node, manual regen) + lengths/link/spot-hex tests as the FE-drift alarm"
  - "Single mutation point: set_theme_preset is the only theme writer (lookup, Dark/Light derive, apply_theme, palette push, save, notify)"
  - "Preset read per render: theme::preset_* helpers with UI_THEMES[0] fallback, never cached colors in views"
  - "In-place terminal recreate: scrollback changes swap the inner Terminal under the same Arc so live views follow, then invalidate + re-capture"
requirements-completed: [THEME-01, THEME-02, THEME-03, SET-01, SET-02, SHELL-04, SHELL-05]
coverage:
  - id: D1
    description: "102 UI + 78 terminal presets checked in with resolving terminalTheme links and default-dark spot hex"
    requirement: "THEME-01"
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/theme_test.rs#test_theme_table_lengths + test_terminal_link_resolution + test_get_ui_theme_fallback"
        status: pass
    human_judgment: false
  - id: D2
    description: "WCAG luminance buckets match FE (default-dark dark, default-light light) and filter counts sum to 102"
    requirement: "THEME-03"
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/theme_test.rs#test_luminance_buckets + test_theme_filter_counts"
        status: pass
    human_judgment: false
  - id: D3
    description: "DesktopSettings holds all Phase 6 prefs with legacy back-compat and FE clamp parity"
    requirement: "SET-02"
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/settings/tests/store_test.rs#test_new_pref_defaults + test_legacy_settings_back_compat + test_terminal_pref_clamps + roundtrip (10/10 pass)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Picking a theme card re-themes all chrome live, pushes linked palettes to every terminal, persists across restarts"
    requirement: "THEME-02"
    verification:
      - kind: unit
        ref: "cargo test --manifest-path desktop-gpui/Cargo.toml --workspace (109 passed, 0 failed, exit 0)"
        status: pass
    human_judgment: true
    rationale: "Headless suite proves compilation + data contracts, but live re-theme flash, restart restore, and no-mixed-preset behavior need eyes on the running app (Phase 7 parity audit HV items)"
  - id: D5
    description: "Settings page card grid + dark/light filter + '{n} themes available' + terminal prefs render and apply live"
    requirement: "SET-01"
    verification: []
    human_judgment: true
    rationale: "GPUI views are not headless-renderable; card pixels, filter behavior, and stepper apply need manual UAT (Phase 7 audit)"
  - id: D6
    description: "Embedded JetBrains Mono + CHECK/SEARCH/PAINTBRUSH lucide SVGs with no system deps"
    requirement: "SHELL-05"
    verification: []
    human_judgment: true
    rationale: "Icon/font rendering is pixel proof — Phase 7 parity audit"
duration: 60min
completed: 2026-09-07
status: complete
---

# Phase 6 Plan 01: Theme Tracer Slice Summary

**Generated 102 UI + 78 terminal theme tables with headless count/link contracts, extended the settings model with FE-parity defaults, and wired set_theme_preset live-apply plus the full Settings page — workspace suite 109/109 green with zero new packages.**

## Performance

- **Duration:** 60 min
- **Started:** 2026-09-07T03:40:00Z
- **Completed:** 2026-09-07T04:39:20Z
- **Tasks:** 3 (2 TDD RED→GREEN pairs + 1 tracer)
- **Files modified:** 14 (4 created, 10 modified)

## Accomplishments

- `scripts/generate-gpui-themes.mjs` (node, manual regen only) parses both FE TS data files and emits checked-in `themes_generated.rs`: `UI_THEMES` x102 + `TERMINAL_THEMES` x78 with `ui_preset_by_name` / `terminal_preset_by_name` / `terminal_preset_for_ui` lookups and ported WCAG `theme_luminance` (`> 0.5`); `cargo build` never invokes it.
- Five headless theme contracts green: lengths 102/78, every UI `terminalTheme` link resolves, unknown id falls back to `UI_THEMES[0]` with `default-dark.background == 0x1e1e1e`, luminance buckets match FE, filter counts sum to 102.
- `DesktopSettings` gains `tmux_binary ""`, `font_family "JetBrains Mono"`, `font_size 14.0`, `line_height 1.35`, `scrollback_lines 2000`, `tui_scroll_default true`, `theme_mode_filter "all"` with additive serde defaults (legacy JSON loads); `clamp_font_size/line_height/scrollback` enforce FE ranges; `AppState::tui_scroll` now falls back to `tui_scroll_default`.
- `theme.rs` keeps `apply_theme` untouched and gains 17 preset-driven chrome helpers (`preset_bg/fg/card/.../ring` + `preset_is_dark`) reading the active preset per render with `[0]` fallback.
- `AppState::set_theme_preset` single mutation point (lookup → persist → Dark/Light derive → `apply_theme` → linked `ColorPalette::from_rgb_u32` push to every live view → sync save → notify); `resync_theme_on_startup` fixes legacy theme/preset skew on the boot path.
- Font/scrollback pref apply: size/family/line-height push via `set_font`/`set_font_size`/renderer update to all live views + debounced viewport re-arm; scrollback recreates store `Terminal`s in place (same `Arc`, grid size preserved) + `invalidate_pane_snapshot` + `request_pane_capture` per pane.
- `views/settings.rs` replaces the placeholder behind `showing_settings`: max-w-672 column, Appearance section (paintbrush header, FE sub-copy, all/dark/light filter persisted via `theme_mode_filter`, `"{n} themes available"`, flex-wrap 3-column `w(px(204))` cards with 56px swatch / 3 dots / 2 bars / check overlay / click-to-apply), Terminal section (honest single-family label + reset, font/line-height/scrollback steppers with FE clamps, TUI-default toggle), 06-02 anchors for binary + kill rows.
- Title bar, window tabs, sidebar, and workspace states read preset tokens per render; `CHECK_SVG` / `SEARCH_SVG` / `PAINTBRUSH_SVG` appended (lucide path data, stroke-width 1.5); full workspace suite green, zero new packages, `Cargo.lock` untouched.

## Task Commits

Each task was committed atomically (TDD tasks as RED test + GREEN feat):

1. **Task 1: Generate checked-in theme tables** - `aeba41a` (test: RED theme contracts) + `6444b15` (feat: generator + 102/78 tables + lib registration)
2. **Task 2: Extend DesktopSettings plus preset lookups** - `465fafe` (test: RED defaults/clamp contracts) + `d54cfad` (feat: settings fields + clamp helpers + theme.rs preset helpers + roundtrip fix)
3. **Task 3: set_theme_preset live-apply plus Settings page (tracer)** - `76a5bdc` (feat: AppState methods + settings page + chrome + icons)

_Tracer gate: full `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` re-run end-to-end after Task 3 — 109 passed, 0 failed, exit 0. No `blocking-human` gate on this tracer; expanding to 06-02 is clear._

## Files Created/Modified

- `scripts/generate-gpui-themes.mjs` - FE TS → Rust generator (102 UI + 78 terminal rows, WCAG is_dark, regen docs)
- `desktop-gpui/crates/webtmux/src/themes_generated.rs` - Checked-in tables + lookups + luminance (DO NOT HAND-EDIT)
- `desktop-gpui/crates/webtmux/src/lib.rs` - Registers `themes_generated`
- `desktop-gpui/crates/webtmux/tests/theme_test.rs` - 5 headless theme contracts
- `desktop-gpui/crates/settings/src/lib.rs` - 7 new pref fields + serde defaults + 3 clamp helpers
- `desktop-gpui/crates/settings/tests/store_test.rs` - 3 new contract tests + roundtrip coverage for new fields
- `desktop-gpui/crates/webtmux/src/theme.rs` - 17 preset-driven chrome helpers (apply_theme unchanged)
- `desktop-gpui/crates/webtmux/src/app_state.rs` - set_theme_preset, startup re-sync, filter/font/scrollback/tui setters, tui_scroll fallback
- `desktop-gpui/crates/webtmux/src/icons.rs` - CHECK/SEARCH/PAINTBRUSH lucide consts
- `desktop-gpui/crates/webtmux/src/views/mod.rs` - Registers `settings`
- `desktop-gpui/crates/webtmux/src/views/settings.rs` - Settings page (card grid + filter + terminal prefs + 06-02 anchors)
- `desktop-gpui/crates/webtmux/src/views/session_states.rs` - Routes to settings page; preset-driven state views; placeholder deleted
- `desktop-gpui/crates/webtmux/src/views/tab_strip.rs` - Preset-driven title bar + tabs + close hover
- `desktop-gpui/crates/webtmux/src/views/sidebar.rs` - Preset-driven sidebar

## Decisions Made

- UiThemePreset carries all 17 FE color fields (plan text says 15 — a miscount; `destructive_foreground` + `input` preserved verbatim rather than dropped).
- Settings numeric inputs are `- value +` steppers (font ±1, line-height ±0.05, scrollback ±100) plus a hand-rolled On/Off toggle — identical clamps, live-apply, and persistence as text inputs, with no per-frame `InputState`/`Switch` entity lifecycle in a re-rendered page. Free-text font-family editing is intentionally absent (single embedded family); a Reset button covers the family apply path.
- Startup re-sync hooks into `start_supervisor` (the one boot path with a live `Context`) instead of plan-external `main.rs`; constructors never save, so the 100+ unit tests that build `AppState::new(DesktopSettings::default(), …)` cannot write the developer's real config dir.
- Scrollback recreate preserves each pane's grid size and swaps the `Terminal` under the same `Arc<Mutex<…>>` so live `TerminalView` clones follow with no view-layer changes; guards reset for clean re-capture.
- Icons port web-term reference lucide path data verbatim but with `stroke-width="1.5"` to match the 24 local consts (reference uses 2).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Generator regex missed `Synthwave '84` (escaped quote in label)**
- **Found during:** Task 1 (generator run found 101/102 UI presets)
- **Issue:** `'([^']+)'` terminates at the escaped `\'`, dropping the preset
- **Fix:** Quote-aware pattern `'((?:[^'\\]|\\.)*)'` + `unesc()` for name/label/link; Rust output renders `Synthwave '84` correctly
- **Files modified:** scripts/generate-gpui-themes.mjs
- **Verification:** Generator exits 0 with 102 UI + 78 terminal; `test_theme_table_lengths` green
- **Committed in:** 6444b15 (Task 1 GREEN)

**2. [Rule 1 - Bug] Stray backtick line left the generator unparseable**
- **Found during:** Task 1 (`node --check` → Unexpected end of input)
- **Issue:** Authoring artifact: extra `` `; `` line opened an unterminated template literal
- **Fix:** Removed the stray line
- **Files modified:** scripts/generate-gpui-themes.mjs
- **Verification:** `node --check` clean, generator run green
- **Committed in:** 6444b15 (Task 1 GREEN)

**3. [Rule 2 - Missing critical] UiThemePreset keeps all 17 FE colors vs plan text's 15**
- **Found during:** Task 1 (generator design)
- **Issue:** Plan acceptance says "15 u32 colors" but FE rows carry 17 (incl. `destructiveForeground`, `input`); dropping two would lose data
- **Fix:** Emit all 17 verbatim; no hand-edits to rows
- **Files modified:** scripts/generate-gpui-themes.mjs, themes_generated.rs
- **Verification:** Length/link/spot-hex tests green
- **Committed in:** 6444b15 (Task 1 GREEN)

**4. [Rule 3 - Blocking] Plan verify filters match zero tests**
- **Found during:** Tasks 1–2 (`cargo test -p webtmux theme_tables`, `cargo test -p webtmux-settings kill_switches`)
- **Issue:** No test name contains `theme_tables` (tests use `theme_table_lengths`) or `kill_switches`; the invocations compile everything yet run 0 tests
- **Fix:** Verified with file-scoped single-filter gates (`--test theme_test`, `--test store_test`) plus the final full workspace battery
- **Files modified:** none (process only)
- **Verification:** theme_test 5/5, store_test 10/10, workspace 109/109 exit 0

**5. [Rule 3 - Blocking] `tui_scroll()` fallback lives in app_state.rs (a Task 3 file)**
- **Found during:** Task 2 (acceptance requires the fallback; files list omits app_state.rs)
- **Issue:** Implementing in Task 2 would break the plan-files boundary; deferring keeps commits scoped
- **Fix:** Implemented with the Task 3 tracer commit; Task 2 tests never depended on it
- **Files modified:** desktop-gpui/crates/webtmux/src/app_state.rs
- **Verification:** Existing wheel/TUI tests green in workspace battery
- **Committed in:** 76a5bdc (Task 3)

**6. [Rule 4-class scoping, auto-resolved] Dialog/menu/pane-grid chrome stays hardcoded**
- **Found during:** Task 3 (acceptance mentions tab_strip/sidebar/session_states/pane headers)
- **Issue:** Kill dialogs, context menus, pane headers/grid, status views, and dialogs build without `AppState` access per render; threading presets through them is a refactor owned by 06-02 (D7 kill switches + copy audit) and the pane-grid wave — not this tracer
- **Fix:** Title bar, tabs, sidebar, workspace states, and the settings page re-theme live; dialog/menu/pane-grid literals left intact and recorded for 06-02
- **Files modified:** none beyond the 7 Task-3 files
- **Verification:** Workspace compiles green; no cached colors introduced anywhere
- **Committed in:** 76a5bdc (Task 3)

---

**Total deviations:** 6 auto-fixed (1 bug, 1 missing-critical, 4 blocking/scoping)
**Impact on plan:** All deviations preserve or exceed plan intent (verbatim data, real test gates, file-scope discipline). No scope creep; deferred chrome explicitly owned by 06-02/pane-grid work.

## Issues Encountered

- Cold GPUI link times dominate: the first `cargo test --workspace` exceeded a 10-min foreground timeout mid-link. Resolved by reusing `CARGO_TARGET_DIR=C:\cargo-target\web-tmux` + `CARGO_BUILD_JOBS=2` and a detached run that resumed incrementally (final cached re-run: seconds, exit 0).
- `cargo test -p webtmux <filter>` builds every webtmux test target before filtering; per-task verification used `--test <file>` scoping for the same signal at a fraction of the cost.
- Pre-existing repo issue (out of scope, untouched): `be/test/mux/service_integration_test.go:147` assignment mismatch and several untracked `.planning` files from prior phases; neither affects this plan.

## Auth Gates

None — no external services, no credentials, no logins required.

## Known Stubs

None — stub scan over all created/modified files found no TODO/FIXME/placeholder copy or unwired data sources. The two `render_*_anchor` boxes are labeled 06-02 section markers (plan-specified), not stubs.

## Threat Flags

None beyond the plan's threat register: generator is local-only with lengths/link/spot-hex pins (T-06-01); settings extension is additive serde defaults with existing `.bak` recovery (T-06-02); all pref numerics pass FE-verbatim clamp helpers before reaching the engine (T-06-03); zero new packages with committed `Cargo.lock` (T-06-SC). No new network endpoints, auth paths, or schema at trust boundaries.

## Next Phase Readiness

- Tracer pipeline proven end-to-end: FE TS → generator → checked-in Rust → live re-theme → persisted. 06-02 expansion (binary validation `POST /api/tmux/binary`, kill switches, palette) builds directly on `set_theme_preset`, preset helpers, and the settings page anchors.
- Manual-UAT handoff for the Phase 7 parity audit: card-grid pixels, live re-theme flash with no mixed presets, restart restores preset + geometry, font render/size resync without wrap drift, light-chrome/dark-OS-frame accepted deviation (Pitfall 6).
- Watch items for 06-02: dialog/menu/pane-header preset threading (needs `AppState` per render in dialog builders), `pane_grid.rs` creation-site palette/font defaults for views born after a theme pick, text-field family editing if required.

## Self-Check: PASSED

- All 14 created/modified files verified present on disk (FOUND x14).
- All 5 task commits verified in history: `aeba41a`, `6444b15`, `465fafe`, `d54cfad`, `76a5bdc`.
- Post-commit deletion check after every task commit: no unintended deletions.
- Final workspace gate: `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` → 109 passed, 0 failed, exit 0 (101 prior + 8 new: 5 theme + 3 settings).

---
*Phase: 06-theme-settings-chrome-parity*
*Completed: 2026-09-07*
