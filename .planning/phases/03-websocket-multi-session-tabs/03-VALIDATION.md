---
phase: "3"
slug: "websocket-multi-session-tabs"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-06"
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded from 03-RESEARCH.md `## Validation Architecture`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (built-in) |
| **Config file** | `desktop-gpui/Cargo.toml` |
| **Quick run command** | `cargo test -p webtmux-backend-client` |
| **Full suite command** | `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` |
| **Estimated runtime** | ~20 s (fixtures + in-process mock WS server; no live backend needed) |

Fixture: in-process mock WS server via `tokio::net::TcpListener` + `tokio_tungstenite::accept_async` (both already in the dependency closure — no new crates), serving recorded payloads: `ws_snapshot_full.json`, `ws_snapshot_null_slices.json`, `ws_delta.json`.

---

## Sampling Rate

- **After every task commit:** `cargo test -p webtmux-backend-client`
- **After every plan wave:** `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace`
- **Before `/gsd-verify-work`:** full workspace suite green
- **Max feedback latency:** ~20 s

---

## Per-Task Verification Map

Rows seeded from the research test map; Task IDs bind to plan tasks when the planner lands.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| test_ws_envelope_roundtrip | 03-01 | W1 | STATE-04 | T-03-01 | envelope DTOs round-trip requestId/newName/session tags | unit | `cargo test -p webtmux-backend-client test_ws_envelope_roundtrip` | ❌ W0 | ⬜ pending |
| test_snapshot_null_normalization | 03-01 | W1 | STATE-04 | T-03-01 | null windows/panes → empty; replace/seq defaults | unit | `cargo test -p webtmux-backend-client test_snapshot_null_normalization` | ❌ W0 | ⬜ pending |
| test_kill_by_name_serialization | 03-01 | W1 | SESS-05 | — | kill carries explicit session field | unit | `cargo test -p webtmux-backend-client test_kill_by_name_serialization` | ❌ W0 | ⬜ pending |
| test_normalize_ws_url | 03-01 | W1 | SESS-02 | T-03-04 | scheme swap + query-encoding of names | unit | `cargo test -p webtmux-backend-client test_normalize_ws_url` | ❌ W0 | ⬜ pending |
| test_ws_bootstrap_flow | 03-01 | W1 | SESS-02 | — | unsolicited ready/snapshot, resync answered, no hello | integration | `cargo test -p webtmux-backend-client test_ws_bootstrap_flow` | ❌ W0 | ⬜ pending |
| test_ws_correlated_command | 03-01 | W1 | STATE-04 | T-03-03 | requestId correlation + forget-timeout | integration | `cargo test -p webtmux-backend-client test_ws_correlated_command` | ❌ W0 | ⬜ pending |
| test_ws_generation_guard | 03-01 | W1 | STATE-04 | T-03-02 | stale dropped, current applied, unknown/mismatch dropped | unit | `cargo test -p webtmux test_ws_generation_guard` | ❌ W0 | ⬜ pending |
| test_tab_lifecycle | 03-01 | W1 | SESS-02 | — | open/switch-no-touch/close-neighbor semantics | unit | `cargo test -p webtmux test_tab_lifecycle` | ❌ W0 | ⬜ pending |
| test_rename_reresolution | 03-01 | W1 | SESS-04 | T-03-06 | old→new migration + generation bump + stale drop | unit | `cargo test -p webtmux test_rename_reresolution` | ❌ W0 | ⬜ pending |
| test_kill_confirm_defaults | 03-02 | W2 | SESS-05 | — | flags default true fresh + legacy JSON | unit | `cargo test -p webtmux-settings test_kill_confirm_defaults` | ❌ W0 | ⬜ pending |
| test_kill_confirm_legacy_json | 03-02 | W2 | SESS-05 | — | legacy file keeps confirming; explicit false survives | unit | `cargo test -p webtmux-settings test_kill_confirm_legacy_json` | ❌ W0 | ⬜ pending |
| test_kill_flow_gate | 03-02 | W2 | SESS-05 | T-03-07 | confirm-required vs direct-kill branching | unit | `cargo test -p webtmux test_kill_flow_gate` | ❌ W0 | ⬜ pending |
| test_window_tabs_model | 03-01 | W1 | SHELL-01 | — | active window derived; empty snapshot → empty, no panic | unit | `cargo test -p webtmux test_window_tabs_model` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `desktop-gpui/crates/backend-client/tests/ws_test.rs` (envelope, null-normalization, url, bootstrap interop, correlation, malformed-frame)
- [ ] `desktop-gpui/crates/backend-client/tests/fixtures/ws_snapshot_full.json`, `ws_snapshot_null_slices.json`, `ws_delta.json`
- [ ] `desktop-gpui/crates/webtmux/tests/tabs_test.rs` (lifecycle, rename re-resolution, window-tabs model)
- [ ] `desktop-gpui/crates/webtmux/tests/ws_guard_test.rs` (generation-guard apply function)
- [ ] `desktop-gpui/crates/webtmux/tests/kill_confirm_test.rs` (kill flow gate)
- [ ] `desktop-gpui/crates/settings/tests/store_test.rs` (extend — kill-confirm defaults + legacy migration)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Multi-tab open/switch feels instant, no re-handshake flicker | SESS-02 | Visual liveness | Open 2+ sessions; switch rapidly; window lists stay correct, no blank flashes |
| Title-bar window tabs + gear match Electron pixels | SHELL-01 | GPUI launch not headless-assertable | Side-by-side with Electron: chips, gear, drag region, divider geometry |
| Rename dialog typing/autofocus/Enter + propagation incl. mid-session | SESS-04 | Interactive UI | Right-click row → Rename; type; Enter; sidebar + tabs update; repeat while session churns |
| Kill confirm appears (or not) per setting; tab closes cleanly | SESS-05 | Interactive UI | Right-click row → Kill; confirm or direct per flag; neighbor tab activates |
| Rapid switching while churning never shows wrong-session windows | STATE-04 | Race timing | Stress: churn tmux windows while fast-switching tabs; assert no cross-session render |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
