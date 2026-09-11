---
phase: "4"
slug: "terminal-engine-live-rendering"
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-09-06"
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded from 04-RESEARCH.md `## Validation Architecture`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (built-in) |
| **Config file** | `desktop-gpui/Cargo.toml` |
| **Quick run command** | `cargo test -p webtmux-terminal` |
| **Full suite command** | `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` |
| **Estimated runtime** | ~15 s (fixtures are in-process; no live backend needed) |

Fixture: in-process mock WS server via `tokio::net::TcpListener` + `accept_async`
(Phase-3 shape, no new crates), scripting `connection.ready` + `state.snapshot`,
then `terminal.snapshot{replace:true, screenRows}` replies, `terminal.output`
chunks, and one `replace:true` frame. Grid fixtures: `terminal_capture_vim.json`
(history + alt-screen app bytes), `terminal_capture_unicode.json` (CJK/emoji,
CRLF mix), `terminal_output_frames.json`, `terminal_ws_frames.json`.

> Cargo 1.96 accepts a SINGLE test-name filter per invocation (03-01/03-02
> deviation) — run one filter per command, full package/suite as the gate.

---

## Sampling Rate

- **After every task commit:** `cargo test -p webtmux-terminal` (and `-p webtmux` when touching `app_state.rs`)
- **After every plan wave:** `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace`
- **Before `/gsd-verify-work`:** full workspace suite green
- **Max feedback latency:** ~15 s

---

## Per-Task Verification Map

Rows seeded from the research test map; Task IDs bind to plan tasks when the planner lands.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| test_keystroke_table | 04-01 | W1 | TERM-03 | — | verbatim table incl. app-cursor/modifier/F-key branches | unit | `cargo test -p webtmux-terminal test_keystroke_table` | ❌ W0 | ⬜ pending |
| test_input_byte_roundtrip | 04-01 | W1 | TERM-03 | T-04-03 | from_utf8 exact round-trip, full table + CJK/emoji paste (lossy banned) | unit | `cargo test -p webtmux-terminal test_input_byte_roundtrip` | ❌ W0 | ⬜ pending |
| test_apply_capture_split | 04-01 | W1 | TERM-02 | T-04-01 | history-delta + positioned screen, ingestedHistory advances | unit | `cargo test -p webtmux-terminal test_apply_capture_split` | ❌ W0 | ⬜ pending |
| test_apply_capture_legacy | 04-01 | W1 | TERM-02 | T-04-01 | screenRows<=0 repaints all lines positioned | unit | `cargo test -p webtmux-terminal test_apply_capture_legacy` | ❌ W0 | ⬜ pending |
| test_apply_capture_shrank_history | 04-01 | W1 | TERM-02 | — | counter reset + precise screen paint | unit | `cargo test -p webtmux-terminal test_apply_capture_shrank_history` | ❌ W0 | ⬜ pending |
| test_apply_capture_unicode_crlf | 04-01 | W1 | TERM-02 | T-04-04 | CJK/emoji CRLF blob, no panic, runes preserved | unit | `cargo test -p webtmux-terminal test_apply_capture_unicode_crlf` | ❌ W0 | ⬜ pending |
| test_live_output_parity | 04-01 | W1 | TERM-01 | — | scripted vim/htop stream → identical grid text | unit | `cargo test -p webtmux-terminal test_live_output_parity` | ❌ W0 | ⬜ pending |
| test_snapshot_exactly_once | 04-01 | W1 | TERM-02 | T-04-02 | repeat snapshot drops; invalidate re-arms; counter accumulates | unit | `cargo test -p webtmux test_snapshot_exactly_once` | ❌ W0 | ⬜ pending |
| test_hidden_session_ingest | 04-01 | W1 | TERM-07 | — | non-active commits; guard layers drop stale/mismatched frames | unit | `cargo test -p webtmux test_hidden_session_ingest` | ❌ W0 | ⬜ pending |
| test_stale_pane_retirement | 04-01 | W1 | TERM-02 | — | absent-from-snapshot entries retire with counters | unit | `cargo test -p webtmux test_stale_pane_retirement` | ❌ W0 | ⬜ pending |
| test_terminal_input_ownership | 04-01 | W1 | TERM-03 | — | owning-socket resolve; unknown pane = typed miss, no send | unit | `cargo test -p webtmux test_terminal_input_ownership` | ❌ W0 | ⬜ pending |
| test_wheel_policy | 04-02 | W2 | TERM-04 | T-04-06 | SGR vs PageUp/Down vs scrollback; default-on; burst clamp 3 | unit | `cargo test -p webtmux test_wheel_policy` | ❌ W0 | ⬜ pending |
| test_selection_text | 04-02 | W2 | TERM-05 | — | programmatic selection → expected non-empty text | unit | `cargo test -p webtmux test_selection_text` | ❌ W0 | ⬜ pending |
| test_copy_paste_keys | 04-02 | W2 | TERM-05 | T-04-07 | copy-vs-interrupt routing; paste → terminal.input | unit | `cargo test -p webtmux test_copy_paste_keys` | ❌ W0 | ⬜ pending |
| test_resize_dance | 04-02 | W2 | TERM-06 | T-04-05 | arms collapse to one clamped send; zero-size never arms | unit | `cargo test -p webtmux test_resize_dance` | ❌ W0 | ⬜ pending |
| test_layout_key_resync | 04-02 | W2 | TERM-06 | T-04-05 | key change → 150/325 pair; same key → nothing; first mount skips | unit | `cargo test -p webtmux test_layout_key_resync` | ❌ W0 | ⬜ pending |
| test_terminal_frames | 04-02 | W2 | TERM-02/06 | T-04-08 | mock-server capture→live→debounced-resize, no hello on wire | integration | `cargo test -p webtmux-backend-client test_terminal_frames` | ❌ W0 | ⬜ pending |
| test_hidden_session_recapture | 04-02 | W2 | TERM-02/07 | — | reconnect re-arms + captures panes of that session only | unit | `cargo test -p webtmux test_hidden_session_recapture` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `desktop-gpui/crates/terminal/tests/capture_test.rs` (TERM-01/02 splits, legacy, shrank, CRLF/CJK, byte round-trip, parity fixtures)
- [ ] `desktop-gpui/crates/terminal/tests/input_test.rs` (TERM-03 ported keystroke table; file preferred for reference-layout parity)
- [ ] `desktop-gpui/crates/webtmux/tests/terminal_test.rs` (TERM-02/03/04/05/06/07: exactly-once, hidden ingest, retirement, ownership, wheel, selection, resize, recapture)
- [ ] `desktop-gpui/crates/backend-client/tests/ws_test.rs` (extend — TERM-02/06 terminal frame interop over the mock server)
- [ ] Fixtures: `terminal_capture_vim.json`, `terminal_capture_unicode.json`, `terminal_output_frames.json`, `terminal_ws_frames.json`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Live vim/htop renders identically to Electron | TERM-01 | GPUI pixels not headless-assertable | Open same workload in both frontends side by side; compare grid content, colors, cursor |
| Typing + special keys echo correctly | TERM-03 | Interactive input | Type text, arrows, F-keys, Ctrl+C interrupt in shell and vim; verify echo |
| Wheel pages vim / scrolls shell scrollback | TERM-04 | Interactive scroll | Wheel over vim pages; wheel over shell scrolls history; toggle default verified by fresh-install behavior |
| Select-copy-paste round-trip | TERM-05 | Clipboard integration | Drag-select text, Ctrl+Shift+C, paste into another app and back with Ctrl+Shift+V |
| Window drag resizes without storms | TERM-06 | Timing + cross-frontend effect | Drag window edges; Electron panes must not rewrap mid-drag; final size settles once |
| Hidden tab catches up on reshow | TERM-07 | Multi-tab interaction | Run output in tab B, switch to A, wait, switch back — B shows all missed output, no blank flash |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
