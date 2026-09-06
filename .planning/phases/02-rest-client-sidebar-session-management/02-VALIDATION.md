---
phase: "2"
slug: "rest-client-sidebar-session-management"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-06"
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded from 02-RESEARCH.md `## Validation Architecture`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (built-in) |
| **Config file** | `desktop-gpui/Cargo.toml` |
| **Quick run command** | `cargo test -p webtmux-backend-client` |
| **Full suite command** | `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` |
| **Estimated runtime** | ~15 s (fixtures are in-process; no live backend needed) |

Fixture: in-process mock HTTP server via `tokio::net::TcpListener` (no external mock crates — pin-safe), serving recorded payloads from `be/` handlers: `health.json`, `tree_full.json`, `tree_empty.json`, `tree_null_slices.json`, `create_error_duplicate.json`.

---

## Sampling Rate

- **After every task commit:** `cargo test -p webtmux-backend-client`
- **After every plan wave:** `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace`
- **Before `/gsd-verify-work`:** full workspace suite green
- **Max feedback latency:** ~15 s

---

## Per-Task Verification Map

Rows seeded from the research test map; Task IDs bind to plan tasks when the planner lands.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| test_parse_sessions_tree | TBD | W0 | SESS-01 | — | typed TmuxTree parse + null normalization | unit | `cargo test -p webtmux-backend-client test_parse_sessions_tree` | ❌ W0 | ⬜ pending |
| test_polling_generation_guard | TBD | W0 | SESS-01 | — | 1.5s interval updates state; generation guard discards stale ticks | unit | `cargo test -p webtmux test_polling_generation_guard` | ❌ W0 | ⬜ pending |
| test_create_session_request | TBD | W0 | SESS-03 | — | POST body {name,cwd,initialCommand}; 201/400/409 mapping | unit | `cargo test -p webtmux-backend-client test_create_session_request` | ❌ W0 | ⬜ pending |
| test_session_name_validation | TBD | W0 | SESS-03 | — | client validator: empty, colon, dot, leading `$` rejected | unit | `cargo test -p webtmux-backend-client test_session_name_validation` | ❌ W0 | ⬜ pending |
| test_cli_session_polling_integration | TBD | W0 | SESS-06 | — | CLI-created session appears after poll tick | integration | `cargo test -p webtmux test_cli_session_polling_integration` | ❌ W0 | ⬜ pending |
| test_sidebar_toggle_snap | TBD | W0 | SHELL-03 | — | width state snaps 240px ↔ 0px | unit | `cargo test -p webtmux test_sidebar_toggle_snap` | ❌ W0 | ⬜ pending |
| test_workspace_state_routing | TBD | W0 | STATE-02 | — | correct page per AppState (Empty/Error/SelectSession) | unit | `cargo test -p webtmux test_workspace_state_routing` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `desktop-gpui/crates/backend-client/tests/rest_test.rs` (SESS-01/03 REST contracts + DTO parsing)
- [ ] `desktop-gpui/crates/backend-client/tests/validation_test.rs` (SESS-03 name validation)
- [ ] `desktop-gpui/crates/webtmux/tests/polling_test.rs` (SESS-01/06 generation-guarded polling)
- [ ] `desktop-gpui/crates/webtmux/tests/sidebar_test.rs` (SHELL-03 toggle + expansion state)
- [ ] `desktop-gpui/crates/webtmux/tests/state_view_test.rs` (STATE-02 routing)
- [ ] Fixtures: `health.json`, `tree_full.json`, `tree_empty.json`, `tree_null_slices.json`, `create_error_duplicate.json`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Sidebar renders + toggles visually | SHELL-03 | GPUI launch not headless-assertable | Launch debug webtmux.exe; sidebar visible 240px; toggle snaps; tree expands/collapses |
| Create Session dialog typing + native dir picker | SESS-03 | Interactive UI | Open dialog; type name; Browse opens the native picker; submit creates a real tmux session visible in the tree |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
