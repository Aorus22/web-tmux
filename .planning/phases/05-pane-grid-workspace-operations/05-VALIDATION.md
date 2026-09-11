---
phase: "5"
slug: "pane-grid-workspace-operations"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-07"
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded from 05-RESEARCH.md `## Validation Architecture`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (built-in) |
| **Config file** | `desktop-gpui/Cargo.toml` |
| **Quick run command** | `cargo test -p webtmux pane_geometry` |
| **Full suite command** | `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` |
| **Estimated runtime** | ~15 s quick (headless geometry/gate tests); full battery needs a 600 s timeout on cold builds (04-02 precedent) |

Build notes (04-01/04-02 precedent): reuse `CARGO_TARGET_DIR=C:\cargo-target\web-tmux` with `CARGO_BUILD_JOBS=4` (E: drive is full); cargo 1.96 accepts a SINGLE test-name filter per invocation — one filter per command. Fixture: in-process mock WS server via `tokio::net::TcpListener` (no new crates), serving scripted pane/window command frames.

---

## Sampling Rate

- **After every task commit:** `cargo test -p webtmux pane_geometry && cargo test -p webtmux-backend-client`
- **After every plan wave:** `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace`
- **Before `/gsd-verify-work`:** full workspace suite green
- **Max feedback latency:** ~15 s (quick), ~600 s (cold full battery)

---

## Per-Task Verification Map

Rows seeded from the research test map; Task IDs bind to plan tasks when the planner lands.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| test_pixel_rect_scale | 05-01 | W1 | PANE-01 | T-05-02 | scale formula + zero-dim guard | unit | `cargo test -p webtmux pane_geometry` | ❌ W0 | ⬜ pending |
| test_close_pane_gaps | 05-01 | W1 | PANE-01 | T-05-02 | 1-cell absorption, order-independence | unit | `cargo test -p webtmux pane_geometry` | ❌ W0 | ⬜ pending |
| test_divider_adjacency | 05-01 | W1 | PANE-01 | — | T-layout overlap, non-adjacent exclusion | unit | `cargo test -p webtmux pane_geometry` | ❌ W0 | ⬜ pending |
| test_drag_step_incremental | 05-01 | W1 | PANE-04 | T-05-05 | incrementality + FLIP map | unit | `cargo test -p webtmux pane_geometry` | ❌ W0 | ⬜ pending |
| test_window_state_clamps | 05-01 | W1 | STATE gap | — | 800..3840 / 500..2160 / sentinel | unit | `cargo test -p webtmux window_state` | ❌ W0 | ⬜ pending |
| test_ws_pane_window_envelopes | 05-01 | W1 | PANE-02/03/05/06/08 | — | 15 command shapes, no pane.join | unit | `cargo test -p webtmux-backend-client ws_pane_window` | ❌ W0 | ⬜ pending |
| test_pane_kill_gate | 05-01 | W1 | PANE-05 | — | pane-flag branching | unit | `cargo test -p webtmux kill_confirm` | ✅ extend | ⬜ pending |
| test_window_kill_gate | 05-01 | W1 | PANE-08 | — | window-flag branching | unit | `cargo test -p webtmux kill_confirm` | ✅ extend | ⬜ pending |
| test_rename_prefill | 05-01 | W1 | DLG-02 | T-05-01 | prefill sources + non-empty gate | unit | `cargo test -p webtmux rename_prefill` | ❌ W0 | ⬜ pending |
| test_split_direction_map | 05-01 | W1 | PANE-02 | — | right→horizontal, down→vertical | unit | `cargo test -p webtmux pane_ops` | ❌ W0 | ⬜ pending |
| test_drag_throttle | 05-02 | W2 | PANE-04 | T-05-05 | 40ms window, last_cells accounting | unit | `cargo test -p webtmux pane_ops` | ❌ W0 | ⬜ pending |
| test_drag_flip | 05-02 | W2 | PANE-04 | — | negative→flip+abs, step==0 drop | unit | `cargo test -p webtmux pane_ops` | ❌ W0 | ⬜ pending |
| test_drag_no_viewport_send | 05-02 | W2 | PANE-04 | — | no terminal.resize/hello from drag path | unit | `cargo test -p webtmux pane_ops` | ❌ W0 | ⬜ pending |
| test_swap_picker_scope | 05-02 | W2 | PANE-05 | T-05-06 | same-window filter, empty-disabled | unit | `cargo test -p webtmux pane_ops` | ❌ W0 | ⬜ pending |
| test_layout_strings | 05-02 | W2 | PANE-06 | — | six preset strings verbatim | unit | `cargo test -p webtmux pane_ops` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `desktop-gpui/crates/webtmux/src/pane_geometry.rs` + `tests/pane_geometry_test.rs` (PANE-01/PANE-04 math)
- [ ] `desktop-gpui/crates/webtmux/tests/window_state_test.rs` (STATE.md clamp-guard gap)
- [ ] `desktop-gpui/crates/backend-client/tests/ws_pane_window_test.rs` (15 pane/window envelopes)
- [ ] Extend `desktop-gpui/crates/webtmux/tests/kill_confirm_test.rs` (pane/window gates)
- [ ] Rename prefill/gate helpers factored pure for headless test (else manual-only with justification)
- [ ] `desktop-gpui/crates/webtmux/tests/pane_ops_test.rs` (drag throttle/flip, swap scope, layout strings)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Pixel parity vs Electron (incl. T-layouts, zoom fill) | PANE-01/03 | GPUI not headless-renderable (Ph2/Ph4 precedent) | Side-by-side same layout; panes align within 1px; zoom fills workspace, next action restores |
| Dialog typing/prefill/focus (pane + window renames) | DLG-02 | Interactive UI | Open each rename; prefill correct; type; Enter submits; empty Rename disabled |
| Drag feel incl. fast-drag past handle | PANE-04 | Pointer-capture question (A2) | Drag slowly + fling past the 4px handle; moves keep streaming; no storms in tmux logs |
| Menus survive poll re-render | PANE-05/08 | Interactive GPUI behavior | Open each menu across a 1.5s poll tick; stable ids keep it open; every row acts |
| Layout presets settle once | PANE-06 | Visual judgment | Apply each preset; panes settle without rewrap flicker; Next cycles |
| Header/toolbar tooltips + TUI switch | PANE-07 | Visual judgment | Hover each action; TUI switch toggles per-pane wheel behavior immediately |

Deferred to the Phase-7 parity audit per the Ph2/Ph4 precedent.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
