---
phase: "6"
slug: "theme-settings-chrome-parity"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-07"
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded from 06-RESEARCH.md `## Validation Architecture`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (built-in) |
| **Config file** | `desktop-gpui/Cargo.toml` |
| **Quick run command** | `cargo test -p webtmux -p webtmux-settings -p webtmux-backend-client` |
| **Full suite command** | `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` |
| **Estimated runtime** | ~15–90 s incremental per target (cold GPUI builds dominate; reuse `CARGO_TARGET_DIR=C:\cargo-target\web-tmux`, `CARGO_BUILD_JOBS=2..4`, one TESTNAME filter per invocation) |

Fixture: in-process mock HTTP server via `tokio::net::TcpListener` (no external mock crates — pin-safe), serving `{"path"}` POST shapes and `TmuxInfo` / `{"error"}` replies for the binary contract.

---

## Sampling Rate

- **After every task commit:** `cargo test -p webtmux -p webtmux-settings -p webtmux-backend-client`
- **After every plan wave:** `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace`
- **Before `/gsd-verify-work`:** full workspace suite green
- **Max feedback latency:** ~90 s

---

## Per-Task Verification Map

Rows seeded from the research test map; Task IDs bind to plan tasks on landing.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| test_theme_table_lengths | 06-01 | W1 | THEME-01 | T-06-01 | 102/78 drift alarm + spot hex | unit | `cargo test -p webtmux theme_tables` | ❌ W1 | ⬜ pending |
| test_terminal_link_resolution | 06-01 | W1 | THEME-01/02 | T-06-01 | every link resolves | unit | `cargo test -p webtmux terminal_link` | ❌ W1 | ⬜ pending |
| test_get_ui_theme_fallback | 06-01 | W1 | THEME-02 | — | unknown id → UI_THEMES[0] | unit | `cargo test -p webtmux ui_fallback` | ❌ W1 | ⬜ pending |
| test_luminance_buckets | 06-01 | W1 | THEME-03 | — | FE luminance parity | unit | `cargo test -p webtmux luminance` | ❌ W1 | ⬜ pending |
| test_theme_filter_counts | 06-01 | W1 | SET-01 | — | all/dark/light sum to 102 | unit | `cargo test -p webtmux filter_counts` | ❌ W1 | ⬜ pending |
| test_new_pref_defaults | 06-01 | W1 | SET-02 | T-06-02 | seven new defaults | unit | `cargo test -p webtmux-settings pref_defaults` | ❌ W1 | ⬜ pending |
| test_legacy_settings_back_compat | 06-01 | W1 | SHELL-04 | T-06-02 | legacy JSON loads | unit | `cargo test -p webtmux-settings back_compat` | ❌ W1 | ⬜ pending |
| test_terminal_pref_clamps | 06-01 | W1 | SET-02 | T-06-03 | FE clamp parity | unit | `cargo test -p webtmux pref_clamps` | ❌ W1 | ⬜ pending |
| test_set_tmux_binary_shape | 06-02 | W2 | SET-03 | T-06-04 | `{path}` POST + ok/error mapping | unit | `cargo test -p webtmux-backend-client binary_shape` | ❌ W2 | ⬜ pending |
| test_binary_status_copy | 06-02 | W2 | SET-03 | — | Using/error line copy | unit | `cargo test -p webtmux binary_status` | ❌ W2 | ⬜ pending |
| test_palette_groups | 06-02 | W2 | DLG-01 | T-06-05 | 7 actions + sessions | unit | `cargo test -p webtmux palette_groups` | ❌ W2 | ⬜ pending |
| test_palette_filter | 06-02 | W2 | DLG-01 | — | label+keyword filter | unit | `cargo test -p webtmux palette_filter` | ❌ W2 | ⬜ pending |
| test_kill_gates (exists) | 06-02 | W2 | DLG-03 | T-06-06 | per-flag gates | unit | `cargo test -p webtmux kill_confirm` | ✅ exists | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `scripts/generate-gpui-themes.mjs` (D1 generator)
- [ ] `desktop-gpui/crates/webtmux/src/themes_generated.rs` (102 + 78 tables + lookups)
- [ ] `desktop-gpui/crates/webtmux/tests/theme_test.rs` (lengths/links/fallback/luminance/filter)
- [ ] `desktop-gpui/crates/settings/tests/store_test.rs` — extend (defaults + legacy back-compat)
- [ ] `desktop-gpui/crates/backend-client/tests/binary_test.rs` (`set_tmux_binary` shape)
- [ ] `desktop-gpui/crates/webtmux/src/actions.rs` (keybinding compile + keystroke-string check)
- [ ] Read vendored `command/command.rs` + `command/state.rs` render API before the palette view task (A3)
- [ ] `desktop-gpui/crates/webtmux/tests/palette_test.rs` (groups/filter, only if model factored pure)

---

## Manual-Only Verifications (→ Phase 7 parity audit as HV items)

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Card-grid pixels + live re-theme flash | THEME-02/SET-01 | GPUI not headless-renderable | Open Settings → Appearance; click a light card; all chrome + terminals restyle with no flash of mixed presets |
| Restart restores preset + geometry | SHELL-04 | Process lifecycle | Pick a theme, resize window, restart; preset + size/position restored |
| Palette open/filter/select from terminal focus | DLG-01 | Interactive keys | Focus a pane; Ctrl+Shift+P opens palette; type "split"; Enter dispatches |
| Binary live validation | SET-03 | Live backend probe | Enter a bad path → backend error shows; enter real tmux → `Using … (x.y)` |
| Kill-toggle flows | SET-04/DLG-03 | Interactive dialogs | Toggle each switch off/on; kill pane/window/session honors it with FE copy |
| Font render + size/line-height apply | SET-02/SHELL-05 | Pixel proof | Bump font size; cols/rows resync without wrap drift; glyphs render in JetBrains Mono |
| Light theme + dark OS frame | THEME-03/Pitfall 6 | OS chrome | Note frame mismatch as accepted v1.0 deviation for the audit |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Full workspace suite green before `/gsd-verify-work`
- [ ] HV items enumerated in plan SUMMARYs for the Phase-7 audit
