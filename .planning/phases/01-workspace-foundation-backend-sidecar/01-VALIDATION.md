---
phase: "1"
slug: "workspace-foundation-backend-sidecar"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-06"
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded from 01-RESEARCH.md `## Validation Architecture` (reader-verified reference mechanics).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` / `#[tokio::test]` (tokio 1.53.1 macros feature) — `cargo test` |
| **Config file** | none — tests live in-crate (`#[cfg(test)] mod tests`) + `crates/<crate>/tests/*` (web-term layout) |
| **Quick run command** | `cargo test -p webtmux-supervisor -p webtmux-settings` |
| **Full suite command** | `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` |
| **Estimated runtime** | ~120 s (compile-heavy workspace; incremental runs faster) |

Fixture seam: build/copy `tmux-gui-server(.exe)` into `desktop-gpui/test-support/` (or point `TEST_BACKEND_PATH` at the repo-root artifact that already exists from `make be`). Integration tests skip gracefully when the fixture is absent (web-term `supervisor/tests/integration.rs:28-36` pattern). Companion script: `scripts/build-test-backend.{sh,cmd}`.

---

## Sampling Rate

- **After every task commit:** `cargo test -p <touched crate>` + `cargo build` quick compile gate
- **After every plan wave:** `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace`
- **Before `/gsd-verify-work`:** Full suite green + Windows debug build green + release-with-fxc smoke
- **Max feedback latency:** ~120 seconds

---

## Per-Task Verification Map

Rows seeded from the research test map; Task IDs are bound to plan tasks when the planner lands (executor maps them 1:1 into the plan's verify blocks).

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| parse_handshake_line | TBD | W0 | STATE-01 | — | handshake token parses (plain, embedded, non-digit, other log lines) | unit | `cargo test -p webtmux-supervisor parse_handshake_line` | ❌ W0 | ⬜ pending |
| spawn_options | TBD | W0 | STATE-01 | — | argv EMPTY + env `TMUXGUI_PORT=0`/`TMUXGUI_HOST=127.0.0.1`/resolved `TMUXGUI_TMUX_BIN`; `TMUX`/`TMUX_PANE` stripped | unit | `cargo test -p webtmux-supervisor spawn_options` | ❌ W0 | ⬜ pending |
| reaches_ready | TBD | W0 | STATE-01 | — | real backend reaches `Ready`, base_url `http://127.0.0.1:<n>` | integration | `cargo test -p webtmux-supervisor --test integration reaches_ready` | ❌ W0 | ⬜ pending |
| invalid_path | TBD | W0 | STATE-01 | — | invalid backend path → `Failed` reason+tail, no panic | integration | `cargo test -p webtmux-supervisor invalid_path` | ❌ W0 | ⬜ pending |
| handshake_timeout | TBD | W0 | STATE-01 | — | shrunken timeout (500ms) → `Failed` "timed out" + child killed | integration | `cargo test -p webtmux-supervisor handshake_timeout` | ❌ W0 | ⬜ pending |
| early_exit | TBD | W0 | STATE-01 | — | early-exit child → `Failed{"backend exited early (code 1)"}` | integration | `cargo test -p webtmux-supervisor early_exit` | ❌ W0 | ⬜ pending |
| adopt_or_clear | TBD | W0 | STATE-01 | — | returns info on live `/api/health`, None after `stop` | integration | `cargo test -p webtmux-supervisor adopt_or_clear` | ❌ W0 | ⬜ pending |
| settings_roundtrip | TBD | W0 | SET-05 | — | fields → save → load equality | unit | `cargo test -p webtmux-settings roundtrip` | ❌ W0 | ⬜ pending |
| corrupt_recovery | TBD | W0 | SET-05 | — | corrupt settings.json → defaults + `settings.json.bak` created | unit | `cargo test -p webtmux-settings corrupt_recovery` | ❌ W0 | ⬜ pending |
| first_run | TBD | W0 | SET-05 | — | no file → defaults + `custom_base`; re-save round-trips | unit | `cargo test -p webtmux-settings first_run` | ❌ W0 | ⬜ pending |
| window_state_guards | TBD | W0 | SHELL-02 | — | restore/extract clamps, 0-size reject, minimized coords, maximized preserves bounds | unit | `cargo test -p webtmux window_state` | ❌ W0 | ⬜ pending |
| build_debug_win | TBD | W0 | PKG-01 | — | debug build WITHOUT fxc (run before GPUI_FXC_PATH ever set) | build | `cargo build --manifest-path desktop-gpui/Cargo.toml --package webtmux` | ❌ W0 | ⬜ pending |
| build_release_fxc | TBD | W-late | PKG-01 | — | release build with tools/fxc compiled + `GPUI_FXC_PATH` set | build smoke | `rustc -O tools/fxc/main.rs -o tools/fxc/fxc.exe` then `$env:GPUI_FXC_PATH=…; cargo build --release` | ❌ W0/late | ⬜ pending |
| linux_doc | TBD | W0 | PKG-01 | — | BUILDING.md lists Linux packages + commands (review-verified; no Linux runner) | doc | `desktop-gpui/docs/BUILDING.md` review | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `desktop-gpui/Cargo.toml` workspace + 5 crate skeletons (resolver=2, exact pins, **commit Cargo.lock as first artifact**)
- [ ] `crates/supervisor/src/{lib}.rs` + `tests/integration.rs` (reaches_ready / invalid_path / handshake_timeout / early_exit / adopt_or_clear)
- [ ] `crates/settings/src/{lib,paths}.rs` + `tests/store_test.rs` (roundtrip / corrupt_recovery / first_run)
- [ ] `crates/backend-client/src/lib.rs`, `crates/terminal/src/lib.rs` — compile-only stubs (no external deps)
- [ ] `crates/webtmux/src/{main,app_state,bundle,window_state,icons}.rs` + `views/{mod,status,tab_strip minimal}.rs` (+ `tests/window_state.rs`, bundle-resolution test)
- [ ] `desktop-gpui/assets/fonts/JetBrainsMono-Regular.ttf` (copy the exact FE node_modules JetBrains Mono file for byte-parity)
- [ ] `desktop-gpui/tools/fxc/{main.rs,README.md}` (rustc compile step in build docs / package script)
- [ ] `scripts/build-test-backend.{sh,cmd}` + test-support harness
- [ ] `desktop-gpui/docs/BUILDING.md` (Linux dev packages + commands — PKG-01 documented path)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Startup path renders Starting → Ready | STATE-01 | GPUI window visuals not headless-automatable | Launch dev exe; observe "Starting backend…" page; observe cleared body on ready |
| Failed page with Retry/Quit | STATE-01 | Visual + interaction | Point app at invalid backend path (env override); verify reason + redacted tail; Retry transitions back to Starting; Quit exits |
| Window controls + drag | SHELL-02 | Interactive window chrome | Minimize/maximize/restore/close each control; drag region; double-click toggles maximize; Aero-snap |
| Geometry + theme persistence | SHELL-04/SET-05 | Restart-observable | Move/resize window, restart app → geometry restored; corrupt settings.json by hand → app launches with defaults + .bak created |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
