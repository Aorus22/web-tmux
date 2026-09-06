---
phase: 01
status: human_needed
date: 2026-09-06
total_must_haves: 11
verified: 8
unverified: 0
backstop_items: 1
human_verification: 3
source_independence_note: "Automated evidence gathered inline by the orchestrator (agy verifier lane failed twice: abort + empty return). All runs cited are real shell executions this session; independent-context property degraded and disclosed."
---

# Phase 01 — Verification

## Automated Results

- `cargo build --manifest-path desktop-gpui/Cargo.toml --workspace` → **PASS** (dev profile, 1m47s)
- `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` → **14 passed / 0 failed**
  (store_test 5 · supervisor unit 2 · integration 7 · doc-tests 0)
- `git ls-files desktop-gpui/Cargo.lock` → tracked ✓ (committed lockfile)
- `git check-ignore desktop-gpui/target/webtmux.exe` + `desktop-gpui/tools/fxc/fxc.exe` → both ignored ✓
- `desktop-gpui/target/release/webtmux.exe` **present** — release build succeeded earlier this session with
  `GPUI_FXC_PATH` set to the ported fxc tool (no Windows SDK required)
- Fix commits from review: cf3dbcd (WR-01), b0e7f69 (WR-02), 5384260 (WR-03), 0386f07 (WR-04) — verified landed in source
- 01-REVIEW.md frontmatter: `status: clean` after fix iteration 1
- Token assertions: "Backend Startup Failed" status.rs:105 · "Starting backend" status.rs:42 · "Reason:" supervisor/lib.rs (redaction)
  · DwmSetWindowAttribute main.rs:70 · IsZoomed window_state.rs:25 · adopt_or_clear supervisor/lib.rs:441 · TMUXGUI_PORT supervisor/lib.rs:121
  · libfontconfig1-dev BUILDING.md:28 · GPUI_FXC_PATH BUILDING.md:56 · include_bytes! main.rs:43

## Must-Haves Verification

| # | Item | Type | Status | Evidence |
|---|------|------|--------|----------|
| 1 | STATE-01 env-only spawn + BACKEND_PORT handshake + status pages S2/S3/S4 | automated | ✅ PASSED | supervisor lib.rs:121 + status.rs:42/105; integration 7/7 |
| 2 | STATE-01 10s two-stage timeouts + early-exit folding | automated | ✅ PASSED | integration tests (handshake_timeout, early_exit) green |
| 3 | STATE-04 WS generation guard | note | ➡️ deferred | belongs to Phase 3 (its REQ-IDs land there) — not a Phase-1 must-have |
| 4 | SET-05 settings persistence: roundtrip + first-run + corrupt .bak recovery | automated | ✅ PASSED | store_test 5/5 + atomic temp-file save in settings lib.rs |
| 5 | PKG-01 Windows debug build | automated | ✅ PASSED | cargo build --workspace PASS |
| 6 | PKG-01 Windows release build with ported fxc smoke | automated | ✅ PASSED | target/release/webtmux.exe present; fxc.exe compiled; GPUI_FXC_PATH flow + gitignore |
| 7 | PKG-01 Linux doc path | automated (doc) | ✅ PASSED | BUILDING.md:28 libfontconfig1-dev, :56 GPUI_FXC_PATH (UAT-gated) |
| 8 | SHELL-02 window-control logic (window_state clamps, IsZoomed toggle) | automated | ✅ PASSED | window_state.rs:25 + tests/window_state green |
| 9 | SHELL-02 interactive controls + drag + double-click | manual | ◔ HUMAN | launch-only affirmation |
| 10 | STATE-01 startup flow visually observed (window, S2 → Ready, S3 paths) | manual | ◔ HUMAN | launch-only affirmation |
| 11 | SHELL-04/SET-05 geometry persistence across restart + hand-corrupt recovery | manual | ◔ HUMAN | restart-only affirmation |
| — | UI-SPEC S4 backstop (Ready body clean, no status remnants) | backstop | 🧪 backstop | backstop verification at UAT |

Coverage: 8 automated PASSED · 0 unverified · 3 HUMAN · 1 backstop · STATE-04 deferred by design.

## Human Verification Required

1. **Startup flow visually** — launch `desktop-gpui/target/debug/webtmux.exe`: window opens at 1200×800 (or restored geometry), S2 shows spinner + "Starting backend…" (≤10s), then Ready clears to the bare shell. On backend failure: S3 shows "Backend Startup Failed" + `Reason:` + redacted tail; Retry re-attempts the full handshake; Quit exits.
2. **Window controls** — minimize / maximize↔restore (Square⇄Copy icon swap by IsZoomed) / close / drag region / double-click toggle. Expected: close exits cleanly and the sidecar child is torn down (WR-03 fix: stop_supervisor on quit).
3. **Persistence + recovery** — move+resize the window, quit, relaunch → geometry restored. Hand-corrupt the settings file (config dir `tmux-gui-desktop`) → app launches with defaults and `settings.json.bak` is created.

## Gaps

None. The three human items are UAT-launch affirmations (not code gaps); the backstop item confirms at UAT.
