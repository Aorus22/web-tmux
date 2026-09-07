---
phase: "7"
slug: "resilience-packaging-parity-audit"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-07"
---

# Phase 7 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded from 07-RESEARCH.md `## Validation Architecture`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (built-in) |
| **Config file** | `desktop-gpui/Cargo.toml` |
| **Quick run command** | `cargo test -p webtmux --test reconnect_test` |
| **Full suite command** | `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` |
| **Estimated runtime** | ~15 s quick (headless models); full suite cache-hot in seconds |
| **Build env** | Repo root, `CARGO_TARGET_DIR=C:\cargo-target\web-tmux`, `CARGO_BUILD_JOBS=2` (06-VERIFICATION §11) |

No new harness needed (RESEARCH Wave 0: test files only, no config changes).

---

## Sampling Rate

- **After every task commit:** single-filter quick command for the touched test file
  (`--test reconnect_test` / `--test toast_test` / `--test bundle_test` / `--test parity_audit_test`)
  + `git diff --stat` over `Cargo.toml Cargo.lock desktop-gpui/Cargo.toml desktop-gpui/Cargo.lock` empty
- **After every plan wave:** `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace`
- **Before `/gsd-verify-work`:** full workspace suite green (116 baseline + new, 0 failed)
- **Max feedback latency:** ~15 s quick / ~60 s full

---

## Per-Task Verification Map

Rows seeded from the research test map; Task IDs bind to plan tasks when the planner lands.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| test_transport_lost_distinct_from_tmux_disconnected | 07-01 | W1 | STATE-03 | T-07-01 | close-synthesis ≠ server constant; distinct state+retry arm | unit | `cargo test -p webtmux --test reconnect_test` | ❌ W0 | ⬜ pending |
| test_tmux_reconnecting_maps_transient | 07-01 | W1 | STATE-03 | — | no client retry on server tmux events | unit | `cargo test -p webtmux --test reconnect_test` | ❌ W0 | ⬜ pending |
| test_backoff_ladder_verbatim | 07-01 | W1 | STATE-03 | T-07-03 | BACKOFF == FE ladder | unit | `cargo test -p webtmux --test reconnect_test` | ❌ W0 | ⬜ pending |
| test_retry_timer_bound_to_generation | 07-01 | W1 | STATE-03 | T-07-03 | stale timers die on rename/close | unit | `cargo test -p webtmux --test reconnect_test` | ❌ W0 | ⬜ pending |
| test_toast_queue_cap_and_dedupe | 07-01 | W1 | STATE-03 | T-07-02 | flap-safe overlay | unit | `cargo test -p webtmux --test toast_test` | ❌ W0 | ⬜ pending |
| test_toast_copy_keys | 07-01 | W1 | STATE-03 | T-07-01 | taxonomy copy keys | unit | `cargo test -p webtmux --test toast_test` | ❌ W0 | ⬜ pending |
| test_command_error_surfaces_toast | 07-01 | W1 | STATE-03 | T-07-01 | backend-verbatim error toast | unit | `cargo test -p webtmux --test toast_test` | ❌ W0 | ⬜ pending |
| test_adjacent_backend_resolution | 07-02 | W1 | PKG-02 | T-07-04 | dist layout matches resolver | unit | `cargo test -p webtmux --test bundle_test` | ❌ W0 | ⬜ pending |
| test_settings_override_wins | 07-02 | W1 | PKG-02 | T-07-04 | absolute pin beats adjacency | unit | `cargo test -p webtmux --test bundle_test` | ❌ W0 | ⬜ pending |
| test_flagged_chrome_reads_preset_tokens | 07-03 | W2 | STATE-03* | T-07-07 | D7-gated threading | unit | `cargo test -p webtmux --test parity_audit_test` | ❌ W0 | ⬜ pending |
| test_theme_swap_convergence_model | 07-03 | W2 | STATE-03* | T-07-07 | no stale cached colors | unit | `cargo test -p webtmux --test parity_audit_test` | ❌ W0 | ⬜ pending |
| test_generation_guard_regression | 07-03 | W2 | STATE-03* | — | no stale-session render (Ph3 HV-4) | unit | `cargo test -p webtmux --test parity_audit_test` | ❌ W0 | ⬜ pending |
| test_rename_migration_model | 07-03 | W2 | STATE-03* | — | no ghost windows (Ph3 HV-3) | unit | `cargo test -p webtmux --test parity_audit_test` | ❌ W0 | ⬜ pending |
| test_kill_gate_model | 07-03 | W2 | STATE-03* | T-07-06 | verbatim kill gates (Ph5 HV-5) | unit | `cargo test -p webtmux --test parity_audit_test` | ❌ W0 | ⬜ pending |

*STATE-03* = audit-row filed under the SC4 parity plan (07-03 carries all three requirement IDs).*

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `desktop-gpui/crates/webtmux/tests/reconnect_test.rs` (transport mapping + BACKOFF + generation-guard)
- [ ] `desktop-gpui/crates/webtmux/tests/toast_test.rs` (cap/dedupe/copy-key model)
- [ ] `desktop-gpui/crates/webtmux/tests/bundle_test.rs` (adjacency + override with tempdir fake exe)
- [ ] `desktop-gpui/crates/webtmux/tests/parity_audit_test.rs` (D7 + HV model guards)
- [ ] `desktop-gpui/scripts/package-gpui-windows.ps1` + `package-gpui-linux.sh` (Task 0 ports)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Banner/toast pixels, kill-tmux vs kill-backend drill, flap behavior | STATE-03 | GPUI not headless-renderable | Kill tmux server → amber `Reconnecting to tmux…`; kill backend → `Connection lost — retrying… (attempt N)` + auto-retry + manual Reconnect; recovery → banner clears + `Reconnected` toast |
| Windows bundle launches to live session | PKG-02 | Packaged-exe smoke | Run `dist/tmux-gui-windows-x64/webtmux.exe` with MSYS2 tmux live; reach a session |
| Linux build + smoke (human_needed) | PKG-02 | No Linux host on this box | On Linux: `./desktop-gpui/scripts/package-gpui-linux.sh`, launch `dist/tmux-gui-linux-x64/webtmux` on a clean Ubuntu/VM; confirm ldd closure note |
| `make` targets per-OS (Linux half human_needed) | PKG-03 | Host-OS execution | Windows: `make desktop-gpui/dev-gpui/package-gpui-windows`; Linux human: `make package-gpui-linux` |
| A1–A5 side-by-side pixels + folded-HV eyes-on rows | SC4 | Human eyes | RESEARCH § Parity Audit Checklist rows; screenshot refs recorded in 07-03-SUMMARY |

---

## Parity Checklist Tracker (D6 — executor ticks, 07-03 owns)

| # | Row | Verdict | Screenshot | Notes |
|---|-----|---------|------------|-------|
| A1 | vim/htop visual parity | ⬜ | | incl. Ph4 HV-1, Ph5 HV-1 |
| A2 | Rapid switching under churn | ⬜ | | incl. Ph3 HV-4, Ph2 HV-5, Ph6 HV-8 |
| A3 | Resize flood | ⬜ | | incl. Ph4 HV-5, Ph5 HV-2 |
| A4 | Rename mid-session | ⬜ | | incl. Ph3 HV-3, Ph5 HV-4 |
| A5 | Theme swap mid-session (3 themes) | ⬜ | | incl. Ph6 HV-1/2/3; drives D7 |
| B1 | Sidebar/tree residuals | ⬜ | | Ph2 HV-1/5, Ph3 HV-2 |
| B2 | Dialog residuals | ⬜ | | Ph2 HV-2/3, Ph5 HV-4 |
| B3 | State pages + startup | ⬜ | | Ph2 HV-4, Ph1 HV-1 |
| B4 | Terminal I/O residuals | ⬜ | | Ph4 HV-2/3/4 |
| B5 | Workspace ops residuals | ⬜ | | Ph5 HV-3/5, Ph6 HV-7 |
| B6 | Settings/palette/font residuals | ⬜ | | Ph6 HV-3..8, Ph1 HV-3/4 |
| B7 | Failed-backend loop | ⬜ | | Ph1 HV-2 (exercises 07-01 path) |
| R1 | Resilience drill (kill-tmux vs kill-backend) | ⬜ | | 07-01 backstop |
| R2 | Windows bundle smoke | ⬜ | | 07-02 SC2 |
| R3 | Linux bundle smoke (human_needed) | ⬜ | | 07-02 SC2, no Linux host |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
