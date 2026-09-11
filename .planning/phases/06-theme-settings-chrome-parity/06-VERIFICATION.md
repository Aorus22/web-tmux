---
phase: "06-theme-settings-chrome-parity"
verified: 2026-09-07T13:00:00Z
status: human_needed
score: 13/15 must-haves verified
behavior_unverified: 2
overrides_applied: 0
must_haves_total: 15
methodology: goal-backward (code inspection + commands actually executed by the verifier)
evidence_commands:
  - "cargo test --manifest-path desktop-gpui/Cargo.toml --workspace -> PASS (116 passed / 0 failed, summed over 32 test-result lines)"
  - "cargo test -p webtmux --test theme_test -> PASS (5/5)"
  - "cargo test -p webtmux --test palette_test -> PASS (5/5)"
  - "cargo test -p webtmux-backend-client --test binary_test -> PASS (2/2)"
  - "cargo test -p webtmux-settings --test store_test -> PASS (10/10)"
  - "cargo test -p webtmux --test kill_confirm_test -> PASS (5/5, pre-existing gates still green)"
  - "file existence, grep wiring/anti-pattern scans, FE source cross-check (CommandPalette.tsx, TerminalSettings.tsx, SessionContextMenu.tsx, commands.ts, WindowTabs.tsx, PaneContextMenu.tsx)"
human_verification_items: 7
gaps: 0
requirement_ids: [THEME-01, THEME-02, THEME-03, SET-01, SET-02, SET-03, SET-04, SHELL-04, SHELL-05, DLG-01, DLG-03]
gaps_applied: []
behavior_unverified_items:
  - truth: "Rapid theme swaps mid-session never leave chrome and terminals on different presets"
    test: "Pick themes rapidly in succession (including light<->dark flips) while panes stream output; watch chrome and every terminal"
    expected: "Chrome and all terminals always converge on the same preset; no mixed-preset flash persists"
    why_human: "Plan truth carries verification: backstop. set_theme_preset is synchronous single-mutation (lookup, derive, apply, palette push, save, notify) which converges by construction, but no test drives two swaps, so the interleaving invariant is unexercised. Presence+wiring never qualifies for a backstop truth."
  - truth: "Palette actions fired with no applicable pane/session degrade to the same guards as their button/menu equivalents, never a raw backend error"
    test: "Open the palette with no session (and with a session but no active pane); confirm each listed action; observe the outcome"
    expected: "Inapplicable actions are absent from the list; confirming any visible action dispatches the normal flow or a guarded no-op — never a raw backend error string"
    why_human: "Plan truth carries verification: backstop. Guard-filtering at group-build time plus miss-safe submits (confirm_palette_selection) are present and wired, and the pure guard fn is unit-proven (test_palette_guards), but no test drives the confirm path with an empty tree, so the degrade branch is unexercised."
deferred:
  - truth: "Pane grid/headers, context menus, and kill/rename dialogs re-theme with the picked preset (full all-chrome live re-theme)"
    addressed_in: "Phase 7"
    evidence: "Phase 7 success criterion 4: side-by-side parity audit against Electron over the daily-driver checklist (incl. theme swap mid-session) shows 1:1 behavior"
---

# Phase 6: Theme System, Settings & Chrome Parity — Verification Report

**Phase Goal:** Appearance and preferences reach Electron parity — all generated themes live-applied, a full Settings page, the command palette, and embedded font/icons
**Verified:** 2026-09-07T13:00:00Z
**Status:** human_needed
**Re-verification:** No — initial verification (no 06-VERIFICATION.md existed)

**Verifier:** gsd-verifier (goal-backward; evidence below produced by commands I actually ran on 2026-09-07, Windows / PowerShell, pwsh 7).
**Scope discipline:** changes limited to this VERIFICATION.md file only; no code re-implemented. Project-wide checks were NOT run beyond the desktop-gpui workspace per instruction.

---

## 1. Observable Truths (must-haves from 06-01-PLAN + 06-02-PLAN + roadmap success criteria)

Classification rule applied throughout: straightforward wired call-chains are VERIFIED by code + wiring + component tests (GPUI is not headless-renderable — see 06-VALIDATION.md manual-only rows and the Phase 2/4/5 precedent). Only **concurrency/cancellation/cleanup/ordering invariants** with no exercising test are parked as ⚠️ PRESENT_BEHAVIOR_UNVERIFIED, and only plan `verification: backstop` truths abstain on principle.

### 1a. Plan 06-01 truths (tracer: generated tables, settings model, live-apply, Settings page)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| T1 | All 102 UI + 78 terminal presets exist as checked-in generated rows; lengths test pins the counts so FE drift fails loudly | ✓ VERIFIED | `themes_generated.rs` checked in (121KB); struct+entry+fn-signature grep arithmetic resolves to exactly 102 UI / 78 terminal rows; `test_theme_table_lengths` green; generator `scripts/generate-gpui-themes.mjs` header documents manual regen; `cargo build` never invokes it (no build.rs) |
| T2 | Every UI preset's terminalTheme link resolves to a real terminal preset; default-dark.background == 0x1e1e1e spot-check holds | ✓ VERIFIED | `test_terminal_link_resolution` (all 102 links + default-dark→"default" canonical link) and `test_get_ui_theme_fallback` (unknown id → UI_THEMES[0] + spot hex) green; `ui_preset_by_name` / `terminal_preset_by_name` / `terminal_preset_for_ui` lookups present with [0]-fallbacks |
| T3 | Picking a theme restyles all chrome live and terminals follow the linked preset; selection plus window geometry persist across restarts | ✓ VERIFIED | `set_theme_preset` (app_state.rs:1780) is the single mutation point: table lookup → persist preset → Dark/Light derive → `apply_theme` → `ColorPalette::from_rgb_u32(linked)` push to every live `terminal_views` entry → sync `save()` → `notify()`; 55 preset-helper reads across tab_strip/sidebar/session_states/settings/palette; geometry persists via untouched Phase-1 `window_state` (window_state_test green in workspace run). Live flash + restart-restore pixels → HV-1/HV-2. Residual: pane grid/headers, menus, dialogs still hardcoded — see §10 deferred, not a gap |
| T4 | Dark/light counterparts derive Dark/Light from the ported luminance rule with Electron colorScheme semantics; load-time re-sync fixes legacy theme/theme_preset skew | ✓ VERIFIED | `theme_luminance` (WCAG weights, `> 0.5`) + precomputed `is_dark` in generator; `test_luminance_buckets` green (default-dark dark, default-light light, black/white extremes); `resync_theme_from_preset` + `resync_theme_on_startup` present and hooked into `start_supervisor` (:670-673); `apply_theme` signature unchanged |
| T5 | Settings page shows the 3-column card grid with dark/light filter and '{n} themes available' | ✓ VERIFIED | settings.rs: filter row persisted via `theme_mode_filter` (`set_theme_mode_filter`, all/dark/light guarded), `"{n} themes available"` count (:161), flex-wrap 3-column `w(px(204))` cards with swatch/dots/bars/check overlay, click → `set_theme_preset` (:262); `test_theme_filter_counts` (dark+light=102) green. Card pixels + filter behavior → HV-3 |
| T6 | Terminal prefs (font size, line height, scrollback, per-pane TUI-scroll default) take effect immediately with FE clamps and persist | ✓ VERIFIED | `clamp_font_size` (8–32) / `clamp_line_height` (1–2) / `clamp_scrollback` (100–50000) pure helpers + `test_terminal_pref_clamps` green; `test_new_pref_defaults` + `test_legacy_settings_back_compat` green (7 FE-parity defaults incl. `tui_scroll_default true`); font apply pushes via set_font/size + `arm_viewport_for_pane` + `schedule_debounced_resize`; scrollback recreates in place + `invalidate_pane_snapshot` + `request_pane_capture`; TUI default flips live via `set_tui_scroll_default`. Numeric-apply pixels → HV-4 |
| T7 | All text renders from the embedded JetBrains Mono face and new chrome uses embedded lucide SVGs with no system deps | ✓ VERIFIED | `main.rs:43-44` embeds `JetBrainsMono-Regular.ttf` via `include_bytes!` + `add_fonts`; `CHECK_SVG` / `SEARCH_SVG` / `PAINTBRUSH_SVG` appended in icons.rs (lucide, stroke-width 1.5); settings label honest (`"Font family (embedded: JetBrains Mono)"` + Reset button, no stack parsing); zero new packages (`Cargo.toml`/`Cargo.lock` untouched since Phase 3). Glyph/icon pixels → HV-5 |
| T8 | {backstop} Rapid theme swaps mid-session never leave chrome and terminals on different presets | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | Single synchronous mutation point converges by construction, but no test drives two swaps — backstop truths abstain on presence. → behavior_unverified_items[0], HV-1 |

### 1b. Plan 06-02 truths (expansion: binary validation, palette, kill switches)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| U1 | The tmux binary path setting validates against POST /api/tmux/binary with Using {binary} ({version}) / error status surfaced in the Settings UI | ✓ VERIFIED | `RestClient::set_tmux_binary` posts exactly `{path}` to `{base}/api/tmux/binary` (mock-captured request line + body asserted); 200 `{binary,version,ok}` → `TmuxInfo`, 400 `{"error"}` → verbatim `RestError::Api` (never a generic code); `binary_status_copy` renders FE-verbatim `Using {binary} ({version})` / raw error; settings binary block (InputState draft + Check/Enter only, never per-keystroke) + apply-once-on-ready after supervisor Ready + `tmux_binary_status` surviving poll re-renders all present. Live-backend interop → HV-6 |
| U2 | Kill-confirmation dialogs honor the three per-surface settings toggles exactly like Electron's shouldConfirm dispatch | ✓ VERIFIED | Three `Switch` rows with FE-verbatim labels (`"Confirm before killing pane/window/session"`, settings.rs:690-706); `KillConfirmKind::{Pane,Window,Session}` + `set_confirm_kill` mirrors FE `shouldConfirm` switch shape (commands.ts:40-50, byte-compared this session); apply-then-save + notify in the same handler (no restart); pre-existing gate tests 5/5 green in this run. Toggle-to-dialog flows → HV-7 |
| U3 | Dialog copy matches Electron verbatim and the three Switch rows persist round-trip through save/load | ✓ VERIFIED | Copy spot-checked against FE this session: `Kill session "{name}"?` + `This terminates the tmux session and all processes inside it.` (SessionContextMenu.tsx:139-141) match; `Close window?` (WindowTabs.tsx:223) and `Kill pane {id}?` (PaneContextMenu.tsx:211 / PaneHeader.tsx:172) match after the two one-line fixes in f7e4a30; `set_confirm_kill` persists via `save()` (round-trip covered by store_test 10/10 incl. kill-legacy JSON). Dialog pixels → HV-7 |
| U4 | Ctrl+Shift+P opens a filterable command palette with exactly the 7 FE actions plus the Open Session group; Open Session opens the picked session | ✓ VERIFIED | `TogglePalette` + `bind_palette_keys("ctrl-shift-p")` (actions.rs, reference pattern); `palette_action_items` yields exactly the 7 FE labels in FE order (test locks extras out); `open_session_items` one-per-tree-session in tree order, group hidden when empty; filter is label+keyword substring case-insensitive (incl. keyword-only "horizontal" hit); `on_confirm` dispatches existing correlated sends / `open_session`; `on_cancel` hides; `CommandState` drops on hide (fresh query per open); placeholder/empty copy byte-match FE (`Type a command or search.` / `No results found.`). Open/filter/select pixels → HV-8 |
| U5 | Palette never loses to the focused terminal: ctrl+shift+P bubbles to the action instead of entering the pty | ✓ VERIFIED | `TerminalView::on_key_down` early-returns via `is_palette_keystroke` before any pty conversion (terminal_view.rs:260-263); ctrl-only binding matches exactly what is registered (no swallowed keystroke without dispatch); `test_palette_keystroke_route` (P/p positive, shift-only/ctrl-only/bare/c/v negative) + `test_palette_keybinding_parses` (KeyBinding::new panics on bad grammar, so construction locks A2) green. Live key from terminal focus → HV-8 |
| U6 | Palette + binary status + switch rows render from the embedded font/icons with no system deps | ✓ VERIFIED | Palette overlay reads `preset_card`/`preset_border` tokens (no hardcoded chrome); binary/switch rows live inside the preset-driven settings page; no new crates or packages (Cargo diff clean). Pixels → HV-5 |
| U7 | {backstop} Palette actions fired with no applicable pane/session degrade to the same guards as their button/menu equivalents, never a raw backend error | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | Guard-filtering at group-build time + `confirm_palette_selection` miss-safe submits (T-06-05) present and wired, and the pure guard fn is unit-proven (`test_palette_guards`), but no test drives the confirm path against an empty tree — backstop truths abstain on presence. → behavior_unverified_items[1], HV-8 |

**Score: 13/15 truths verified, 2 present-but-behavior-unverified, 0 failed.**

### 1c. Roadmap success criteria → truth mapping

| SC | Criterion | Status | Covering truths |
|----|-----------|--------|-----------------|
| SC1 | Settings 3-column card grid + dark/light filter; pick re-themes chrome live, terminals follow, selection + geometry persist | ✓ VERIFIED | T1, T2, T3, T5 (+T8 backstop residual → HV-1; pane-grid/menu/dialog threading deferred → §10) |
| SC2 | Terminal prefs take effect immediately and persist | ✓ VERIFIED | T6 (+T4 luminance/theme-mode half; pixels → HV-4) |
| SC3 | Binary path validates via /api/tmux/binary with status/error in UI | ✓ VERIFIED | U1 (live interop → HV-6) |
| SC4 | Kill dialogs honor three toggles like Electron; dark/light counterparts use Electron dark semantics | ✓ VERIFIED | U2, U3, T4 (flows + pixels → HV-7) |
| SC5 | 102+78 generated presets; Ctrl+Shift+P palette incl. Open Session; embedded font/icons, no system deps | ✓ VERIFIED | T1, T7, U4, U5, U6 (+U7 backstop residual → HV-8; pixels → HV-5) |

---

## 2. Required Artifacts (exists → substantive → wired)

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `scripts/generate-gpui-themes.mjs` | TS → Rust generator (102 UI + 78 terminal rows + lookups + is_dark) | ✓ VERIFIED | 10KB; quote-aware name/label/link parsing (Synthwave '84 fix in HISTORY); WCAG luminance port; manual-regen docs; local-only supply chain |
| `desktop-gpui/crates/webtmux/src/themes_generated.rs` | Checked-in UI_THEMES x102 + TERMINAL_THEMES x78 + lookup helpers | ✓ VERIFIED | 121KB checked in; row arithmetic confirms 102/78; `ui_preset_by_name` / `terminal_preset_by_name` / `terminal_preset_for_ui` / `theme_luminance` all present with fallbacks; registered in lib.rs |
| `desktop-gpui/crates/webtmux/src/theme.rs` | Preset-driven chrome helpers, apply_theme intact | ✓ VERIFIED | 17 helpers (`preset_bg/fg/card/…/ring` + `preset_is_dark`) reading active preset per render with UI_THEMES[0] fallback; `apply_theme` signature unchanged |
| `desktop-gpui/crates/webtmux/src/views/settings.rs` | Settings page (Appearance card grid + filter, Terminal prefs, binary block, Safety switches) | ✓ VERIFIED | 29KB; all four sections present; 06-01 anchors fully replaced by 06-02 binary + Safety rows; 22 preset-helper reads |
| `desktop-gpui/crates/webtmux/tests/theme_test.rs` | Headless lengths/link/fallback/luminance/filter contracts | ✓ VERIFIED | 5/5 green in this run |
| `desktop-gpui/crates/backend-client/src/rest.rs` | `set_tmux_binary` POST + existing TmuxInfo reuse | ✓ VERIFIED | `set_tmux_binary` mirrors `create_session` POST/error shape; `binary_status_copy` helper; no new error type or crate |
| `desktop-gpui/crates/backend-client/tests/binary_test.rs` | Binary shape/status-copy contracts | ✓ VERIFIED | 2/2 green in this run (capturing-mock POST proof + verbatim-error proof + copy proof) |
| `desktop-gpui/crates/webtmux/src/actions.rs` | TogglePalette action + keybinding registration | ✓ VERIFIED | `actions!(webtmux_palette, [TogglePalette])` + `bind_palette_keys("ctrl-shift-p")`; keybinding-parse test green |
| `desktop-gpui/crates/webtmux/src/views/palette.rs` | Command-palette dialog (7 actions + Open Session) over existing sends | ✓ VERIFIED | Pure model (items/guards/filter/key-route) + vendored `Command` overlay with on_confirm/on_cancel; 2 preset-token reads; registered in views/mod.rs; palette state + TogglePalette handler + overlay in app_state.rs |
| `desktop-gpui/crates/webtmux/tests/palette_test.rs` | Palette group/filter/guard model contracts | ✓ VERIFIED | 5/5 green in this run |
| `desktop-gpui/crates/settings/src/lib.rs` | 7 FE-parity pref fields + serde back-compat + clamp helpers | ✓ VERIFIED | Additive-only fields with serde defaults; `tui_scroll` falls back to `tui_scroll_default`; store_test 10/10 green |
| `desktop-gpui/crates/webtmux/src/icons.rs` | CHECK/SEARCH/PAINTBRUSH lucide consts | ✓ VERIFIED | All three present, stroke-width 1.5, existing 24 consts untouched |

Supporting edits verified present: `terminal_view.rs` palette early-return; `tab_strip.rs` + `pane_context_menu.rs` verbatim title fixes; `backend-client/src/lib.rs` re-export; `views/mod.rs` + `lib.rs` module registrations.

---

## 3. Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| settings.rs card click | app_state.rs `set_theme_preset` | click handler calls setter that apply-then-saves synchronously | ✓ WIRED | settings.rs:262 → app_state.rs:1780 (lookup, derive, apply_theme, palette push, save, notify) |
| theme.rs helpers | themes_generated.rs tables | per-render preset reads with [0] fallback, no caching | ✓ WIRED | 55 reads across 5 chrome files; `ui_preset_by_name` pattern confirmed |
| app_state.rs startup | theme re-sync | `start_supervisor` → `resync_theme_on_startup` → save + `apply_theme` | ✓ WIRED | app_state.rs:670-673 → :1821 |
| settings.rs binary block | backend-client `set_tmux_binary` | Check/Enter invokes POST; status line renders version or backend error | ✓ WIRED | `set_tmux_binary_path` / `check_tmux_binary` (:1972+) + `tmux_binary_status: Option<Result<TmuxInfo, String>>` surviving re-renders; apply-once-on-ready after Ready (:2002-2013) |
| palette.rs groups | app_state.rs sends | `on_confirm` → `confirm_palette_selection` → existing correlated sends / `open_session` | ✓ WIRED | palette.rs:202 + app_state confirm fn with guard-filtered indices |
| terminal_view.rs | actions.rs TogglePalette | `on_key_down` early-return for ctrl+shift+P before pty conversion | ✓ WIRED | terminal_view.rs:260-263 via `is_palette_keystroke`; sidebar/other focus paths keep working (root-view `.on_action` handler at app_state.rs:3343) |
| settings.rs switches | app_state.rs `set_confirm_kill` | Switch on_click → kind writer → save + notify | ✓ WIRED | apply-then-save in same handler; gates read settings live (no restart) |

Prohibitions: D1 no hand-transcribed colors — VERIFIED (generator output only; rows not hand-edited). No build-time Node dep — VERIFIED (no build.rs, checked-in output). No cached colors — VERIFIED (one preset read per render). No terminal-override path — VERIFIED (`terminal_preset_for_ui` link-only; legacy override ignored). No font-stack parsing — VERIFIED (single embedded family + honest label). D6 no per-keystroke validation — VERIFIED (Check/Enter only). No hand-rolled palette widget — VERIFIED (vendored `command` module). No palette extras — VERIFIED (test fails on extras). No client-side tmux probing — VERIFIED (opaque string; backend probes). No dialog-behavior changes — VERIFIED (switches + 2 copy one-liners only).

---

## 4. Data-Flow Trace (Level 4)

| Artifact | Data variable | Source | Produces real data | Status |
|----------|---------------|--------|--------------------|--------|
| theme tables | `UiThemePreset` / `TerminalThemePreset` rows | FE `ui-themes.ts` / `terminal-themes.ts` via checked-in generator output | YES (102/78 rows, link + spot-hex proven) | ✓ FLOWING |
| chrome render | preset colors per render | Active `theme_preset` → `ui_preset_by_name` per render | YES (title/tabs/sidebar/states/settings/palette) | ✓ FLOWING |
| terminal palette | `ColorPalette` per theme pick | Linked terminal preset via `terminal_preset_for_ui` → `from_rgb_u32` → `set_palette` on all live views | YES (chain wired; pixel proof → human) | ✓ FLOWING |
| binary status | `tmux_binary_status` | Real POST /api/tmux/binary ok/error (mock-server proven shape) | YES | ✓ FLOWING |
| palette rows | action + session items | Pure FE-order model + live tree session names | YES | ✓ FLOWING |
| kill gates | `confirm_kill_*` booleans | Persisted settings → live gate reads | YES | ✓ FLOWING |
| prefs | font/scrollback/tui values | Settings fields → live engine apply + save | YES | ✓ FLOWING |

No hollow props, no static fallbacks, no mocks in the production path (mocks exist only inside test files). Pane grid/headers, menus, and dialogs still read hardcoded `rgb(0x…)` literals (e.g. pane_grid.rs:72, pane_context_menu.rs:238) — real data but stale source; deferred to Phase 7 audit (§10).

---

## 5. Behavioral Spot-Checks (commands run by me)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full workspace suite | `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` (CARGO_TARGET_DIR=C:\cargo-target\web-tmux, JOBS=2) | 116 passed / 0 failed across 32 test binaries (summed `test result:` lines; includes 2 pre-existing supervisor integration + live Go sidecar) | ✓ PASS |
| Theme table contracts | `cargo test -p webtmux --test theme_test` | 5 passed / 0 failed (lengths, link resolution, fallback+spot-hex, luminance, filter counts) | ✓ PASS |
| Palette model contracts | `cargo test -p webtmux --test palette_test` | 5 passed / 0 failed (groups, filter, guards, keystroke route, keybinding parse) | ✓ PASS |
| Binary contracts | `cargo test -p webtmux-backend-client --test binary_test` | 2 passed / 0 failed (POST shape + ok/error mapping + status copy) | ✓ PASS |
| Settings store contracts | `cargo test -p webtmux-settings --test store_test` | 10 passed / 0 failed (defaults, legacy back-compat, clamps, kill defaults/legacy, roundtrip, atomic save, recovery) | ✓ PASS |
| Kill gates (regression) | `cargo test -p webtmux --test kill_confirm_test` | 5 passed / 0 failed | ✓ PASS |

SUMMARY claims "116/116 green" and "109/109 green" **confirmed independently** — not trusted, re-run (116 = 109 prior + 7 new: 2 binary + 5 palette; per-target counts above cross-check).

### Probe Execution

No probes declared: neither PLAN mentions `probe-*.sh`, and no `scripts/*/tests/probe-*.sh` relates to this phase. SKIPPED (no applicable probes).

---

## 6. Requirements Coverage

| Requirement | Source plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| THEME-01 | 06-01 | 102 UI + 78 terminal presets as generated files | SATISFIED | T1/T2 + theme_test 5/5 |
| THEME-02 | 06-01 | Live apply across chrome; terminal follows matched theme | SATISFIED | T3 + single-mutation-point chain (pixels → HV-1) |
| THEME-03 | 06-01 | Dark/light counterparts with Electron dark semantics | SATISFIED | T4 + luminance test + startup re-sync |
| SET-01 | 06-01 | Theme picker card grid + dark/light filter | SATISFIED | T5 + filter-counts test (pixels → HV-3) |
| SET-02 | 06-01 | Terminal prefs: size/height/scrollback/TUI default | SATISFIED | T6 + store_test clamps/defaults (pixels → HV-4) |
| SET-03 | 06-02 | Binary path validated via /api/tmux/binary + status/error | SATISFIED | U1 + binary_test 2/2 (live interop → HV-6) |
| SET-04 | 06-02 | Three kill-confirmation switches | SATISFIED | U2/U3 + kill_confirm 5/5 + store roundtrip (flows → HV-7) |
| SHELL-04 | 06-01 | Geometry + theme preference persist across restarts | SATISFIED | T3/T4 + legacy back-compat test + window_state regression green (restart proof → HV-2) |
| SHELL-05 | 06-01/06-02 | Embedded JetBrains Mono + embedded lucide SVGs | SATISFIED | T7/U6, zero new packages (pixels → HV-5) |
| DLG-01 | 06-02 | Palette on Ctrl+Shift+P with filter + Open Session | SATISFIED | U4/U5 + palette_test 5/5 (pixels → HV-8) |
| DLG-03 | 06-02 | Kill confirmations honor per-surface toggles like Electron | SATISFIED | U2/U3 + verbatim copy audit (flows → HV-7) |

No orphaned Phase-6 requirements: ROADMAP maps exactly {SHELL-04, SHELL-05, SET-01..04, THEME-01..03, DLG-01, DLG-03} → Phase 6, each claimed by ≥1 plan (06-01: THEME-01/02/03, SET-01/02, SHELL-04/05; 06-02: SET-03/04, DLG-01/03, SHELL-05).

---

## 7. Anti-Patterns Found

`Select-String "TODO|FIXME|XXX|TBD|HACK|unimplemented!|todo!|placeholder|coming soon|not yet implemented|not available"` across `views/settings.rs`, `views/palette.rs`, `actions.rs`, `themes_generated.rs`, `rest.rs`, `theme.rs`, `app_state.rs` → **2 matches, both benign**: palette.rs:8 (doc comment naming the vendored `.placeholder(..)` builder API) and palette.rs:189 (`Command::placeholder("Type a command or search.")` — FE-verbatim UI copy, also byte-matched against CommandPalette.tsx). No debt markers, no stub returns, no hardcoded empty data flowing to render. Both SUMMARYs' "Known Stubs: None" claims **confirmed**. No blockers, no warnings.

---

## 8. Human Verification Required

Residuals that code inspection + headless tests cannot close (GPUI needs a display; 06-VALIDATION.md lists the same manual-only rows; both SUMMARYs enumerate the same Phase-7 handoff):

### HV-1: Theme-swap flash (no mixed presets)
**Test:** Open a multi-pane workspace streaming output; in Settings → Appearance click several cards in succession, including light↔dark flips.
**Expected:** All chrome + every terminal converge on each picked preset; no persistent mixed-preset state (covers T3, T8 backstop).
**Why human:** GPUI pixel rendering + swap timing are not headless-assertable.

### HV-2: Restart restores preset + geometry
**Test:** Pick a non-default theme, resize/move the window, restart the app.
**Expected:** Theme preset + window size/position restored; no legacy theme/preset skew (covers T3/SHELL-04, T4 re-sync).
**Why human:** Process lifecycle cannot run headless.

### HV-3: Card-grid pixels + filter behavior
**Test:** Open Settings → Appearance; toggle all/dark/light; observe the grid and count line.
**Expected:** 3-column cards (swatch + dots + bars + check on active), `"{n} themes available"` updates per filter, click applies live (covers T5).
**Why human:** Visual parity judgment by definition.

### HV-4: Font render + pref apply without wrap drift
**Test:** Bump font size / line height / scrollback; toggle TUI-scroll default; observe panes.
**Expected:** Glyphs render in JetBrains Mono; cols/rows resync without wrap drift; scrollback depth honors the new limit (covers T6/T7).
**Why human:** Pixel proof + tmux geometry resync feel.

### HV-5: Font/icon pixel proof
**Test:** Inspect settings, palette, and card check overlays at both dark and light themes.
**Expected:** All text in the embedded face; check/search/paintbrush icons crisp; no system-fallback glyphs (covers T7/U6).
**Why human:** Pixel proof by definition. Note accepted v1.0 deviation: light chrome under the force-dark DWM frame (Pitfall 6) — record, do not re-argue.

### HV-6: Binary live interop
**Test:** In Settings → Terminal, enter a bad tmux path → Check; then a real tmux → Check; restart with a stored path and watch after backend connect.
**Expected:** Bad path shows the backend error verbatim; good path shows `Using {binary} ({version})`; stored path applies once on ready with a `Checking…` transient (covers U1).
**Why human:** Needs the live backend `tmux -V` probe.

### HV-7: Kill-toggle flows + dialog copy
**Test:** Toggle each Safety switch off → kill that surface; toggle on → kill again; read every Kill dialog.
**Expected:** Off goes direct, on shows the dialog with FE-verbatim title/description/buttons; no restart needed (covers U2/U3/DLG-03).
**Why human:** Interactive dialog flows + visual copy proof.

### HV-8: Palette open/filter/select from terminal focus
**Test:** Focus a pane; press Ctrl+Shift+P; type "split"; Enter; try "zzz-no-match"; Esc with non-empty query then again; test with no session open.
**Expected:** Palette opens from terminal focus; filter narrows both groups; Enter dispatches (split/zoom/kill act on the focused pane); nonsense shows `No results found.`; first Esc clears query, second closes; empty tree hides Open Session; never a raw backend error (covers U4/U5/U7 backstop).
**Why human:** Interactive keys + focus behavior; macOS Cmd+Shift+P is an explicit non-goal this phase (ctrl-only by design).

---

## 9. Gaps Summary

**No gaps.** All 13 non-backstop must-haves are substantively implemented, wired, and covered by green headless contracts; the 2 backstop truths are present + wired with their behavior-unverified residuals routed to HV-1/HV-8 above. The one partial (pane-grid/menu/dialog chrome still hardcoded) is an explicitly scoped, honestly documented deferral owned by the Phase 7 parity audit — see §10 — not a hidden stub.

---

## 10. Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | Pane grid/headers, context menus, and kill/rename dialogs read preset tokens per render (completing all-chrome live re-theme) | Phase 7 | Phase 7 success criterion 4: side-by-side parity audit against Electron over the daily-driver checklist (incl. theme swap mid-session) shows 1:1 behavior |

---

## 11. Environment Caveats That Shaped the Verdict

1. Tests were run from the repo root with `--manifest-path desktop-gpui/Cargo.toml` per the task instruction (`CARGO_TARGET_DIR=C:\cargo-target\web-tmux`, `CARGO_BUILD_JOBS=2`); no project-wide checks (lint/clippy/fmt/release build) were executed.
2. The workspace run re-executed Phases 1–5 suites as a regression sweep — all still green, so no regressions from Phase-6 edits.
3. All Phase-6 test invocations after the first workspace run were cache-hot (seconds each); no server was started and no state was mutated by the verifier.
4. `git status` shows only planning-doc drift (pre-existing untracked VERIFICATION/plan files from prior phases, `M .planning/config.json`) — no source modifications by the verifier; the tree was clean for all phase files.
5. Row-count grep arithmetic (104/81 raw `Preset {` hits) was reconciled against struct + fn-signature lines — the authoritative count is the green `test_theme_table_lengths` (102/78), not my grep.

---

_Verified: 2026-09-07T13:00:00Z_
_Verifier: gsd-verifier agent_
