---
phase: "04-terminal-engine-live-rendering"
verified: 2026-09-06T22:30:00Z
status: human_needed
score: 12/12 must-haves verified
behavior_unverified: 0
overrides_applied: 0
must_haves_total: 12
methodology: goal-backward (code inspection + commands actually executed by the verifier)
evidence_commands:
  - "cargo test --manifest-path desktop-gpui/Cargo.toml --workspace -> PASS (25/25 suites ok, 82 tests, 0 failed, exit 0)"
  - "file existence, grep wiring/prohibition/stub scans, git commit + diff scope check (no be/ or fe/ changes)"
human_verification_items: 5
gaps: 0
requirement_ids: [TERM-01, TERM-02, TERM-03, TERM-04, TERM-05, TERM-06, TERM-07]
gaps_applied: []
---

# Phase 4: Terminal Engine & Live Pane Rendering — Verification Report

**Phase Goal:** Every pane renders a live alacritty_terminal-backed terminal through the full capture-replay → live-output pipeline, with byte-safe input, scrollback, selection, and storm-free resize
**Verified:** 2026-09-06T22:30:00Z
**Status:** human_needed
**Re-verification:** No — initial verification (no 04-VERIFICATION.md existed)

**Verifier:** gsd-verifier (goal-backward; evidence below produced by commands I actually ran on 2026-09-06, Windows / PowerShell, pwsh 7).
**Scope discipline:** changes limited to this VERIFICATION.md file only; no code re-implemented. Project-wide checks were NOT run beyond the desktop-gpui workspace per instruction.

---

## 1. Observable Truths (must-haves from 04-01-PLAN + 04-02-PLAN + roadmap success criteria)

Classification rule applied throughout (same as 02-VERIFICATION §1): straightforward wired call-chains are VERIFIED by code + wiring + component tests (GPUI is not headless-renderable — see 04-VALIDATION.md manual-only rows). Only **concurrency/cancellation/cleanup/ordering invariants** with no exercising test would park as ⚠️ PRESENT_BEHAVIOR_UNVERIFIED, and only plan `verification: backstop` truths abstain on principle. None of the 12 truths needed parking: the two backstop truths assert **deterministic pure-function invariants** (debounce collapse/clamp, burst clamp) that the passing unit tests drive directly through the real production helpers — unlike 02's backstops, which asserted timing/race behavior through proxy counters. Wall-clock timing (100/150/325ms fires) and pixel rendering remain manual-UAT class per the plans themselves and ride as HV advisory items, exactly like 02's VERIFIED-with-pixel-HV truths (T8/U2).

### 1a. Plan 04-01 truths (tracer: engine, store, live view)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| T1 | A pane's live incremental output renders in the terminal grid through process_bytes (scripted vim/htop byte stream yields identical grid text vs the recorded fixture) | ✓ VERIFIED | `test_live_output_parity` green (capture_test suite 5/5); `Terminal::process_bytes` → `Processor::advance` (terminal.rs:101-104) + `grid_text` headless dump (:212-228). On-screen pixels → HV-1 |
| T2 | Opening a pane with history replays captured scrollback exactly once then continues live: first terminal.snapshot replaces, the second identical snapshot drops, post-invalidate snapshot applies, ingestedHistory accumulates across frames | ✓ VERIFIED | `test_snapshot_exactly_once` + `test_stale_pane_retirement` green (terminal_test 10/10); `apply_capture` split/delta/shrank/legacy branches (capture.rs:28-69) + `snapshotWritten` gate in commit arms (app_state.rs:80, :1219-1240). History-doubling regression → HV-5 watch item |
| T3 | Typed input including special keys reaches the owning session socket byte-safe as terminal.input{paneId, data} via String::from_utf8 over the keystroke_to_bytes table (round-trip locked for the full table plus CJK/emoji paste) | ✓ VERIFIED | `test_keystroke_table` + `test_input_byte_roundtrip` (input suite 8/8) + `test_terminal_input_ownership` green; `build_terminal_input` pure constructor + owning-socket `send_command` (app_state.rs:1339-1369); zero `from_utf8_lossy` hits repo-wide (grep). Interactive echo → HV-2 |
| T4 | Terminal events for a non-active (hidden) session commit to the store, and the triple guard layers still drop stale-generation, envelope-mismatched, and retired-pane terminal frames | ✓ VERIFIED | `test_hidden_session_ingest` green; commit arms never consult `active_session` (app_state.rs:744); guard-first ordering per Pattern 2. Reshow catch-up → HV-5 |
| T5 | Stale pane entries retire when absent from the latest committed snapshot, and reconnect re-arms plus re-captures every registered pane of the session | ✓ VERIFIED | `test_stale_pane_retirement` + `test_hidden_session_recapture` (recapture scoped to session only) green; `recapture_session` (app_state.rs:1399) wired to reconnect path |
| T6 | TerminalView renders one live terminal per pane of the active window and Ctrl+C with empty selection passes through as interrupt while Ctrl+Shift+C with a selection copies | ✓ VERIFIED | session_states.rs:407-450 renders one `TerminalView` per `active_window_pane_ids()` with initial `request_pane_capture` on creation; key routing via `decide_key_route` + `test_copy_paste_keys` green; copy→`write_to_clipboard`, paste→one `terminal.input` (terminal_view.rs:218-240). Visual/interactive sign-off → HV-1/HV-2/HV-4 |

### 1b. Plan 04-02 truths (expansion: wheel, clipboard, resize, interop)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| E1 | Wheel over scrollback scrolls the terminal viewport; with TUI-scroll on (default) the wheel sends PageUp/PageDown repeats to TUI panes with 100px/notch accumulation and burst clamp 3; mouse-reporting apps get SGR 64/65 | ✓ VERIFIED | `test_wheel_policy` green; pure `decide_wheel_action` (app_state.rs:177) + `apply_wheel` emit (:878) + per-pane accumulator; SGR branch kept, ALT_SCREEN arrow fallback provably uncalled (`scroll_to_arrow_keys` has `#[allow(dead_code)]`, zero callers repo-wide). Feel/default-on-fresh-install → HV-3 |
| E2 | User selects terminal text with the mouse and copies it via Ctrl+Shift+C (and Cmd+C) to the system clipboard, and pastes clipboard content via Ctrl+Shift+V (and Cmd+V) into the terminal as terminal.input | ✓ VERIFIED | `test_selection_text` + `test_copy_paste_keys` green; `pane_selection_text` headless lock (app_state.rs:866) + `KeyRoute` helper mirroring view checks (:239); drag geometry via ported mouse.rs. Real-OS clipboard round-trip → HV-4 |
| E3 | Window resizes send exactly one debounced terminal.resize{cols>=2, rows>=1} per settle (~100ms) with no per-paint sends; layout-key change schedules the 150ms resize plus 325ms invalidate-and-recapture pair, skipped on first mount | ✓ VERIFIED | `test_resize_dance` + `test_layout_key_resync` green; arm-only paint callback (terminal_view.rs:540) + `arm_viewport_for_pane`/`take_debounced_resize`/`build_terminal_resize` (app_state.rs:919-1017) with `max(2)/max(1)` clamp + zero-size-never-arms. Live-drag storm behavior → HV-5 |
| E4 | A mock WS server sending terminal.snapshot{replace, screenRows} plus terminal.output frames drives a real client from pane-register through capture commit to live grid content, and asserts the debounced terminal.resize shape | ✓ VERIFIED | `test_terminal_frames` green (ws_test suite); scripted capture reply + `replace:false` chunks + one `replace:true` frame from `terminal_ws_frames.json`, capture-present + resize-shape + hello-absent assertions (ws_test.rs:444-600) |
| E5 | {backstop} Rapid window drags never emit more than one resize per debounce window and never shrink the shared tmux viewport to zero/1x1 dimensions | ✓ VERIFIED | Deterministic invariant, not a race: supersede via dedicated `resize_seq` + fire-time recompute tested by `test_resize_dance` (collapse-to-one); clamp `max(2)/max(1)` + zero-size-never-arms tested in the same test; `build_terminal_resize` carries "no hello" in its contract. Wall-clock drag storm → HV-5 advisory |
| E6 | {backstop} Wheel bursts (e.g. high-resolution trackpads) never emit more than 3 page repeats per notch batch | ✓ VERIFIED | Deterministic invariant: burst clamp 3 in the pure decision function, tested directly by `test_wheel_policy` (incl. 110px-batch remainder semantics). Trackpad feel → HV-3 advisory |

**Score: 12/12 truths verified, 0 present-but-behavior-unverified, 0 gaps.**

### 1c. Roadmap success criteria → truth mapping

| SC | Criterion | Status | Covering truths |
|----|-----------|--------|-----------------|
| SC1 | Live output renders in the grid; vim/htop identical to other frontends | ✓ VERIFIED | T1 (+T6 render path; pixel side-by-side → HV-1) |
| SC2 | History replays exactly once, continues live seamlessly incl. reconnect; never doubled/blank | ✓ VERIFIED | T2, T5, E4 (interop replace:true frame) |
| SC3 | Typed input byte-safe via terminal.input; wheel scrolls scrollback, TUI-on wheel sends PageUp/PageDown | ✓ VERIFIED | T3, E1 (+E6 burst backstop; interactive feel → HV-2/HV-3) |
| SC4 | Select/copy to clipboard; paste clipboard into terminal | ✓ VERIFIED | T6 (key routing), E2 (OS clipboard round-trip → HV-4) |
| SC5 | Debounced cols/rows (no storms); hidden tabs ingest and catch up on reshow | ✓ VERIFIED | E3, E4 (resize shape + no hello), T4 (+E5 drag backstop; live timing → HV-5) |

---

## 2. Required Artifacts (exists → substantive → wired)

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `terminal/src/terminal.rs` | Term + Processor engine, D1 scrollback 2000 | ✓ VERIFIED | 238 lines; `scrollback_limit: 2000` default (:52-61), `process_bytes`/`resize`/`scroll_display`/selection API/`with_term*`/`grid_text`/`drain_events`; wired by store + view |
| `terminal/src/capture.rs` | Byte-oriented apply_capture port | ✓ VERIFIED | 69 lines; CRLF-normalize-first, char-boundary split, explicit per-row origin, delta join `\r\n`, legacy + shrank branches; called from snapshot/output commit arms |
| `terminal/src/input.rs` | Verbatim keystroke_to_bytes table + ported tests | ✓ VERIFIED | 8/8 input tests incl. full-table + round-trip; consumed by view key path (terminal_view.rs:298) |
| `terminal/src/mouse.rs` | Geometry/SGR helpers, D5 arrow fallback uncalled | ✓ VERIFIED | 199 lines; D5 comment at :98-103; `scroll_to_arrow_keys` dead-ported with `#[allow(dead_code)]`, zero callers repo-wide (grep) |
| `terminal/src/render.rs` + `colors.rs` | Measure/layout/paint, D2 dark default | ✓ VERIFIED | render 19 unit tests green (incl. merge_backgrounds, renderer creation); D2 font tokens asserted (render.rs:572-578) |
| `terminal/src/event.rs` | GpuiEventProxy + TerminalEvent | ✓ VERIFIED | Re-exported via lib.rs; `drain_events` consumed for D8 title commits |
| `terminal/src/lib.rs` | Re-exports + alacritty type facade | ✓ VERIFIED | 32 lines; all 7 modules + `apply_capture`, `keystroke_to_bytes`, geometry/mode types (no direct alacritty dep needed downstream) |
| Fixtures (vim/unicode/output-frames) | Real-shaped capture blobs + scripted stream | ✓ VERIFIED | All 3 on disk; drive 5/5 capture tests incl. CJK/emoji CRLF safety + live parity |
| `webtmux/src/app_state.rs` | TerminalStore, guarded arms, sends, wheel/resize/resync | ✓ VERIFIED | ~85KB; all 15 probed symbols present (`decide_wheel_action`, `decide_key_route`, `build_terminal_resize`, `tui_scroll`, `pane_selection_text`, `apply_wheel`, `arm_viewport_for_pane`, `take_debounced_resize`, `decide_layout_resync`, `layout_resync_panes`, `send_terminal_input`, `request_pane_capture`, `recapture_session`); 64 total keyword hits |
| `webtmux/src/views/terminal_view.rs` | Canvas + focus/key/mouse handlers, arm-only resize | ✓ VERIFIED | ~21KB; `keystroke_to_bytes` key path, clipboard copy/paste, `on_scroll`→`apply_wheel`, zero-size early return, arm-only `resize_cb`, layout-key observer hook; D2 tokens (14px/1.35/JetBrains Mono :62-67) |
| `webtmux/src/views/session_states.rs` | Per-pane live rendering, placeholder replaced | ✓ VERIFIED | :407-450: one `TerminalView` per `active_window_pane_ids()`, store-owned handles, initial `request_pane_capture` per pane on creation, connecting fallback pre-snapshot |
| `webtmux/src/views/mod.rs` | Module registration | ✓ VERIFIED | `pub mod terminal_view` (:10) |
| Tests: `webtmux/tests/terminal_test.rs` | Exactly-once/hidden/retirement/ownership/wheel/selection/resize/recapture | ✓ VERIFIED | 10/10 green (all plan-named tests present) |
| Tests: `backend-client/tests/ws_test.rs` + `terminal_ws_frames.json` | Socket interop capture→live→resize | ✓ VERIFIED | `test_terminal_frames` green; fixture on disk; asserts capture present, resize shape, grid substrings, hello absent |

Supporting deltas verified present: pending_viewport/resize_seq/wheel_accum/last_layout_key fields; `send_terminal_resize` clamped window-level sender; 100ms + 150/325ms timer schedulers; pane-session attribution map with reset-on-reaffiliation.

---

## 3. Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| terminal_view.rs | terminal crate | View borrows store `Terminal`, routes keys through `keystroke_to_bytes` | ✓ WIRED | Import (:22) + call site (:298) + `term.mode()`; SGR passthrough under mouse bits |
| app_state.rs | backend-client/ws.rs | Store commits feed Processor; sends ride `send_command` fire-and-forget | ✓ WIRED | `send_terminal_input`/`request_pane_capture`/`send_terminal_resize` → owning-session `send_command`; receiver dropped (uncorrelated) |
| app_state.rs | terminal/capture.rs | Snapshot/output arms run `apply_capture` with per-pane `ingested_history` | ✓ WIRED | Import (:20) + both commit sites (:1235, :1255); no `pendingScreen` queue (only the deliberate doc comment) |
| terminal_view.rs | app_state.rs | Wheel reads `tui_scroll` + TermMode bits; resize arms `pending_viewport` | ✓ WIRED | `apply_wheel` (:453), `arm_viewport_for_pane` (:540), `observe_layout_key_and_schedule` (:473) |
| backend-client ws_test.rs | app_state.rs (contract) | Interop asserts capture request, snapshot commit, resize envelope end to end | ✓ WIRED | `terminal.capture` wait + snapshot reply + resize-shape assert + `saw_hello == false` |
| session_states.rs | terminal_view.rs | Active-window per-pane creation + initial capture | ✓ WIRED | Import (:10), `TerminalView::new` per pane (:446), `request_pane_capture` on creation |

Prohibitions (04-01 ×7 + 04-02 ×6): **P1** no pendingScreen queue — VERIFIED (absent; only the rationale comment). **P2** no `from_utf8_lossy` — VERIFIED (zero hits repo-wide; `from_utf8` expect-ok + contract test). **P3** no active-proxy sends for non-active panes — VERIFIED (owning-socket resolve in all three send helpers + `test_terminal_input_ownership`). **P4** no Terminals in views — VERIFIED (store owns `Arc<Mutex<Terminal>>`; views hold clones; `pane_entry` ensures store-first). **P5** no byte-slicing of blobs — VERIFIED (`replace`/`split` char APIs only + unicode fixture test). **P6** no hello — VERIFIED (only doc comments + test `saw_hello` assertions; interop asserts absence on the wire). **P7** no backend protocol changes — VERIFIED (`git diff` over the 10 task commits touches zero `be/` or `fe/` files). **P8** no arrow-key TUI fallback — VERIFIED (D5 dead-port + zero callers). **P9** no sync resize from paint — VERIFIED (arm-only callback + test). **P10** no hello-with-zero/1x1 — VERIFIED (zero-size-never-arms + clamp). **P11** no TUI-default-OFF — VERIFIED (`unwrap_or(true)` at app_state.rs:859). **P12** no client chunking of pastes — VERIFIED (one-message paste path + comment).

---

## 4. Data-Flow Trace (Level 4)

| Artifact | Data variable | Source | Produces real data | Status |
|----------|---------------|--------|--------------------|--------|
| Snapshot commit | capture blob → `apply_capture` feed → `Processor` grid | Real `terminal.snapshot` frames (interop-proven over a real socket) | YES | ✓ FLOWING |
| Live output | `terminal.output` bytes → Processor (raw or capture path by `replace`) | Real output chunks + `replace:true` frames (interop + parity fixtures) | YES | ✓ FLOWING |
| Grid render | `grid_text` / Terminal handles → per-pane `TerminalView` | AppState store entries (created on first frame, retired per D7) | YES (headless grid proven; pixels → HV-1) | ✓ FLOWING |
| Input send | keystroke/clipboard bytes → `terminal.input` on owning socket | Real key table + clipboard (byte path unit-proven; OS clipboard → HV-4) | YES | ✓ FLOWING |
| Resize send | settled sizes → debounced `terminal.resize` | Paint measures (arm-only) → fire-time recompute + clamp | YES (shape proven on wire; storm-freedom under drag → HV-5) | ✓ FLOWING |
| Hidden ingest | non-active session frames → store commits | Forward pump for all sessions (TERM-07 by construction, test-proven) | YES | ✓ FLOWING |

No hollow props, no static fallbacks, no mocks in the production path (mocks exist only inside test files; fixtures are recorded shapes served by in-process test servers).

---

## 5. Behavioral Spot-Checks (commands run by me)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full workspace suite | `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` (CARGO_TARGET_DIR=C:\cargo-target\web-tmux, CARGO_BUILD_JOBS=4) | **exit 0; 25/25 suites ok, 82 tests, 0 failed** (log: C:\cargo-target\web-tmux-verify-phase4.log) | ✓ PASS |
| Terminal engine suites | Same run (cached re-run for full listing) | render 19/19, capture 5/5, input 8/8 | ✓ PASS |
| webtmux terminal tests | Same run | terminal_test 10/10 (exactly-once, hidden ingest, retirement, ownership, wheel, selection, copy/paste keys, resize dance, layout resync, recapture) | ✓ PASS |
| Socket interop | Same run | `test_terminal_frames` ok (capture→live→resize, hello absent) | ✓ PASS |
| Named Phase-4 tests | Enumerated via suite output (no extra full runs) | All 18 plan-named tests ok (5 capture + 8 input + 10 webtmux listed, incl. keystroke_table, roundtrip, parity; ws interop) | ✓ PASS |

SUMMARY claims re-verified independently — 04-01's "75/75" is consistent with the current 82 (75 + 7 new 04-02 tests: wheel, selection, copy/paste keys, resize dance, layout resync, recapture, terminal_frames). (A second cached workspace run was used only to capture the per-suite listing to a log file; tests spawn in-process mock listeners and mutate no lasting state.)

### Probe Execution

No probes declared: neither PLAN mentions `probe-*.sh`, and no `scripts/*/tests/probe-*.sh` covers this phase. SKIPPED (no applicable probes).

---

## 6. Requirements Coverage

| Requirement | Source plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| TERM-01 | 04-01 | alacritty-backed grid per pane, vim/htop parity | SATISFIED | T1 + parity test (grid-text identical; pixel side-by-side → HV-1) |
| TERM-02 | 04-01 | History replay exactly once + live continue incl. reconnect | SATISFIED | T2, T5, E4 + 5 capture tests + recapture/interop tests |
| TERM-03 | 04-01 | Byte-safe keyboard input via terminal.input | SATISFIED | T3 + full table + round-trip + ownership tests (interactive echo → HV-2) |
| TERM-04 | 04-02 | Wheel scrollback; TUI-on wheel Pages TUI panes | SATISFIED | E1 + E6 + wheel policy test (feel → HV-3) |
| TERM-05 | 04-02 | Selection copy/paste via clipboard | SATISFIED | E2 + T6 routing + selection/copy tests (OS round-trip → HV-4) |
| TERM-06 | 04-02 | Debounced resize dance + layout resync, no storms | SATISFIED | E3 + E5 + dance/resync/interop tests (drag storms → HV-5) |
| TERM-07 | 04-01 | Hidden sessions ingest while not rendered | SATISFIED | T4 + hidden-ingest test (reshow catch-up → HV-5) |

No orphaned Phase-4 requirements: REQUIREMENTS.md maps exactly {TERM-01..TERM-07} → Phase 4, each claimed by ≥1 plan (04-01: TERM-01/02/03/07; 04-02: TERM-04/05/06).

---

## 7. Anti-Patterns Found

`grep` for `TODO|FIXME|XXX|TBD|HACK|unimplemented|todo!|placeholder|coming soon|not yet implemented|return null|return []|return {}` across `terminal/src`, `webtmux/src/views/terminal_view.rs`, `webtmux/src/app_state.rs` → **one benign hit**: `app_state.rs:387` "Settings placeholder page flag (SHELL-01, Phase-6-owned page)" — the intentional Phase-6 seam (Settings gear page), not Phase-4 stub-rot. No debt markers, no stub returns, no `from_utf8_lossy`, no `hello` sends, no `pendingScreen` queue. Both SUMMARYs' "no new crates / no protocol changes" claims **confirmed** (zero `be/`/`fe/` files in the 10 task commits; all 10 commits present in git log: 643eff1, e3826f6, 933bcc0, 3fd6b23, b66d3cd, 0a5e279, 2ad07c9, f6294cf, bae1ab7, 25b81ce).

No blockers, no warnings — ℹ️ info only: tracer layout stacks panes full-width (Phase 5 owns cell geometry; declared in both SUMMARYs and session_states.rs:409).

---

## 8. Human Verification Required

Residuals that code inspection + headless tests cannot close (GPUI needs a display; 04-VALIDATION.md lists the same six manual-only rows). Advisory — automated checks all pass; these close the pixels/timing/clipboard loop in the Phase 7 parity audit if not sooner.

### HV-1: Live vim/htop renders identically to Electron (pixels)
**Test:** Open the same workload (vim with syntax file, htop) in the GPUI app and Electron side by side; compare grid content, colors, cursor, alt-screen enter/exit.
**Expected:** Indistinguishable grids; no doubled history lines after tab switch/zoom/layout change; no blank flash on pane open.
**Why human:** GPUI pixel rendering, palette raster, and cursor paint are not headless-assertable (covers T1/T6 pixels, SC1, VALIDATION.md manual row 1).

### HV-2: Typing + special keys echo correctly (interrupt passthrough)
**Test:** Type text, arrows, F-keys, Ctrl+C (empty selection → interrupt; check shell interrupts), Ctrl+Shift+C with and without selection, in shell and vim.
**Expected:** Correct echo everywhere; Ctrl+C with selection copies instead of interrupting; without selection the pane receives `\x03`.
**Why human:** Interactive input against live tmux (covers T3/T6 key paths, SC3, VALIDATION.md manual row 2).

### HV-3: Wheel pages vim / scrolls shell scrollback (TUI default)
**Test:** Wheel over vim (pages) and over shell scrollback (scrolls history) on a fresh install (no TUI pref ever set); try a fast trackpad flick burst.
**Expected:** Vim pages (never cursor-moves — the D5 arrow prohibition made visible); shell scrolls; default is TUI-ON; bursts never jump more than ~3 pages per notch batch.
**Why human:** Scroll feel + fresh-install default + burst behavior under a real device (covers E1/E6, SC3, VALIDATION.md manual row 3).

### HV-4: Select-copy-paste round-trip (OS clipboard)
**Test:** Drag-select terminal text, Ctrl+Shift+C, paste into another app; copy in another app, Ctrl+Shift+V into the terminal (include CJK/emoji).
**Expected:** Exact text both directions; paste lands in the pane as input; no mojibake.
**Why human:** System clipboard integration is unavailable headless (covers E2, SC4, VALIDATION.md manual row 4).

### HV-5: Window drag resizes without storms; hidden tab catches up
**Test:** Drag window edges continuously while watching Electron panes on the same session (they must not rewrap mid-drag; final size settles once ~100ms after release); change layout and confirm quick resize + scrollback resync; run output in hidden tab B, switch to A, wait, switch back.
**Expected:** One resize per settle, clamped dims, no 1x1 collapse; B shows all missed output with no blank flash.
**Why human:** Wall-clock timing + cross-frontend effect + multi-tab interaction (covers E3/E5/T4/T2 reconnect clause, SC2/SC5, VALIDATION.md manual rows 5-6). Timer bodies are manual-UAT class per the 04-02 plan itself.

---

## 9. Gaps Summary

**No gaps.** All 12 must-have truths verified with test + wiring evidence, all 7 TERM requirements satisfied, all 13 prohibitions hold, the full workspace battery is green (25/25 suites, 82/82 tests), and the 10 task commits plus both plan files are present with zero out-of-scope (`be/`/`fe/`) changes. The phase re-scores to full marks on automation; per the Step 9 decision tree the status is `human_needed` (not `passed`) solely because HV-1..HV-5 need a human at a screen — same routing as 02-VERIFICATION's HV items.

## 10. Deferred Items

None — no failed truth exists to defer, and nothing in later milestone phases (5–7) repairs Phase-4 scope; Phase 5 consumes the store/viewport machinery unchanged (Step 9b checked against ROADMAP phases 5–7).

## 11. Environment Caveats That Shaped the Verdict

1. Tests were run from the repo root with `CARGO_TARGET_DIR=C:\cargo-target\web-tmux` + `CARGO_BUILD_JOBS=4` per the task instruction (E: drive is full per the 04-01 SUMMARY deviation); no project-wide checks (lint/clippy/fmt/release build) were executed.
2. The workspace run re-executed Phases 1–3 suites as a regression sweep — all still green, so no regressions from Phase-4 edits.
3. The verifier's second workspace invocation was cache-warm (~seconds) and used only to capture the per-suite listing to `C:\cargo-target\web-tmux-verify-phase4.log` (outside the repo; no tree pollution).
4. `git status`/diff inspection confirms the verifier modified nothing outside this VERIFICATION.md; all phase source files were already committed.

---

_Verified: 2026-09-06T22:30:00Z_
_Verifier: gsd-verifier agent_
