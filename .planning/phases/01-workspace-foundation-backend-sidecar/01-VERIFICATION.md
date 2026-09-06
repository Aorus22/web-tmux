---
phase: "1"
created: 2026-09-06
verified: 2026-09-06T19:13:39+07:00
status: passed
must_haves_verified: 4/4
must_haves_total: 4
methodology: goal-backward (code inspection + commands actually executed by the verifier)
evidence_commands:
  - "cargo build --manifest-path desktop-gpui/Cargo.toml --workspace  ->  PASS (Finished dev profile; exit 0)"
  - "cargo test  --manifest-path desktop-gpui/Cargo.toml --workspace ->  PASS (16/16 core-pass tests; 5.14s integration incl. real Go sidecar)"
  - "git log / git ls-files, file existence, grep anti-pattern scan, vendored gpui-pre-windows-0.3.3/build.rs inspection"
human_verification_items: 4
gaps: 0
requirement_ids: [SHELL-02, SET-05, STATE-01, PKG-01]
---

# Phase 1: Workspace Foundation & Backend Sidecar — Verification Report

**Phase Goal:** The GPUI desktop app exists as a buildable Rust workspace (web-term pin set, committed Cargo.lock), launches its own Go backend sidecar on a dynamic port, and shows the startup path (Starting → Ready / Failed with reason).

**Verifier:** gsd-verifier (goal-backward; evidence below produced by commands I actually ran on 2026-09-06, Windows / PowerShell, pwsh 7).

---

## 1. Must-Haves Table (derived from roadmap success criteria M1..M4)

| ID  | Must-have (success criterion)                                                                                                                           | Verification evidence (command I ran / code inspected)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Status   | Source               |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- | -------------------- |
| M1  | Launching the app renders "Starting backend…" while the sidecar warms up, then Ready; on failure (10s port timeout or early exit) a Failed page shows reason with working Retry and Quit | **Code:** 3-variant `BackendStatus { Starting, Ready(BackendInfo), Failed { reason, stderr_tail } }` (`crates/supervisor/src/lib.rs:28-39`); env-only spawn `TMUXGUI_PORT=0` / `TMUXGUI_HOST=127.0.0.1`, `TMUX`/`TMUX_PANE` stripped, **argv empty**, `kill_on_drop(true)` (`lib.rs:117-143`); `BACKEND_PORT:<n>` stdout handshake with 10s timeout (`lib.rs:147-155, 251-303`); `/api/health` probe 250ms/1s with 10s readiness timeout (`lib.rs:327-364`); 2000-char newest-wins stderr ring buffer, char-boundary safe after WR-01 (`lib.rs:233-249`); `adopt_or_clear` (`lib.rs:441-463`).  **Wiring:** `main.rs` loads settings → `resolve_backend_path` → opens window → `start_supervisor`; `app_state.rs:58-143` watch→mpsc→GPUI pump (`weak.upgrade()`); `app_state.rs:146-173` render pipes `backend_status` → `views/status.rs` which renders S2 (animated 20px Loader2 + "Starting backend…"), S3 (card, "Backend Startup Failed", `Reason:`, redacted newest-10 stderr tail, **Quit → `stop_supervisor()` + `cx.quit()`, Retry → `start_supervisor`** after WR-03), S4 (bare `#1e1e1e` body + S1 title bar, no status leftovers).  **Behavior:** integration tests I ran all green: `reaches_ready`, `invalid_path`, `handshake_timeout`, `early_exit`, `adopt_or_clear`, `test_parse_handshake_unit`, `test_spawn_options_env_contract` — `reaches_ready` spawned a **real Go sidecar** (`tmux-gui-server.exe` fixture) end-to-end through handshake → `/api/health` → `BackendStatus::Ready` | VERIFIED | commands + code inspection |
| M2  | User can minimize, maximize/restore, and close via the custom title-bar window controls                                                                     | **Code:** `views/tab_strip.rs` — Minimize → `window.minimize_window()` (line 88-90), Maximize/Restore → `window_state::toggle_maximize` with Win32 `IsZoomed`/`ShowWindowAsync` + DWM dark-frame + redraw (window_state.rs:35-98) and Square↔Copy 14px icon swap on `app.is_maximized` (tab_strip.rs:104-117), Close → `window.remove_window()` (136-138), title-bar background `.window_control_area(WindowControlArea::Drag)` (52), 40x32 buttons with idle `#9d9d9d` / hover `#2d2d2d` / close-hover `#7F1D1D`.  Title bar wired into root render (`app_state.rs:148`). Builds clean → handlers attach on screen. Actual click/drag behavior is runtime-visual → listed as human-verification item HV-3 | VERIFIED (code inspection) | code inspection |
| M3  | Window position/size persist across an app restart, and a corrupted settings file recovers via .bak instead of breaking launch                               | **Code:** `settings/paths.rs` isolates config dir to `tmux-gui-desktop` (no `tmux-gui-gtk`/`webterm-desktop` collision); `main.rs:20-36` `DesktopSettings::load()` → `window_state::restore()` (clamped, min 800x500, ≤ −10000 coords filtered after WR-04) feeds `WindowOptions::window_bounds`; `window_state::observe` (177-187) flushes `extract_window_state` + `save()` on `on_window_should_close`.  **Behavior (tests I ran, all green):** `roundtrip` (real-file save/load of `WindowState` incl. 1400x900 geometry), `corrupt_recovery` (malformed JSON → renamed to `settings.json.bak` asserted on disk, defaults returned, second load succeeds), `first_run`, `atomic_save_leaves_no_temp_files_and_writes_file`.  Corrupt-file launch recovery is proven at store level; the visual "restart keeps geometry" check is HV-4.  **Noted deficiency (non-blocking):** the plan-declared `desktop-gpui/crates/webtmux/tests/window_state.rs` was never created; the substitute `store_test.rs::window_state_clamping_and_guard_logic` constructs structs and asserts on their own field values — it does NOT exercise `restore()`/`extract_window_state()`. The guard logic itself is present, substantive, and wired (`window_state.rs:101-174`), so the phase-level criterion holds; test-coverage of the clamp guard is not exercised by any test | VERIFIED (with noted deficiency) | commands + code inspection |
| M4  | `cargo build` succeeds on Windows (release with ported fxc shader tool); Linux dev-package prerequisites documented                                         | **Debug (I ran):** `cargo build --manifest-path desktop-gpui/Cargo.toml --workspace` → `Finished dev profile` exit 0 — debug needs no fxc, confirmed against the vendored dependency: `gpui-pre-windows-0.3.3/build.rs` gates shader compilation under `#[cfg(all(target_os = "windows", not(debug_assertions)))]` and reads `GPUI_FXC_PATH` as first-class override (lines 12, 114-117).  **Release (deferred evidence, not re-run):** `desktop-gpui/target/release/webtmux.exe` exists (21,767,624 bytes, last write 2026-09-06 18:40) from the executor's release run (~3m27s, 01-03-SUMMARY); `tools/fxc/main.rs` is a substantive 302-line zero-crate port (d3dcompiler_47.dll via LoadLibraryA/GetProcAddress → D3DCompile; parses `/T /E /Fh /Fo /Vn /nologo /O*`), and `tools/fxc/fxc.exe` exists locally (gitignored).  **Docs:** `desktop-gpui/docs/BUILDING.md` covers all 6 required items — Windows prereqs (Rust/MSVC/Go), debug + release-with-fxc instructions (`GPUI_FXC_PATH`), Linux apt list (`libfontconfig1-dev`, `libwayland-dev`, `libxkbcommon-dev`, `libxkbcommon-x11-dev`, `libx11-dev`, `libxcursor-dev`, `libxrandr-dev`, `libgl1-mesa-dev`, `vulkan-loader`, `tmux`), and `cargo test --workspace`.  `.gitignore` excludes `desktop-gpui/target/`, `tools/fxc/*.exe`, `test-support/`, `*.bak` | VERIFIED | commands + code inspection |

**Score: 4/4 must-haves verified.**

---

## 2. Workspace Contract (explicit in goal statement)

| Item | Expected | Status | Evidence |
| --- | --- | --- | --- |
| Web-term pin set | `resolver = "2"`, 5 members, exact `=` pins (gpui-pre 0.3.3, gpui-component 0.6.0, tokio 1.53.1, reqwest 0.12.28 rustls, dirs 6.0.0, thiserror 2.0.20, parking_lot 0.12.5, tempfile 3.27.0, alacritty_terminal 0.25.1, tokio-tungstenite 0.26.2) | VERIFIED | `desktop-gpui/Cargo.toml:1-56` matches 01-RESEARCH standard stack verbatim; app crate adds `gpui-pre-platform =0.3.3` + `raw-window-handle =0.6.2` (`crates/webtmux/Cargo.toml:19,29`) |
| Committed Cargo.lock | Lockfile tracked in git | VERIFIED | `git ls-files desktop-gpui/Cargo.lock` → tracked; workspace builds reproducibly |
| 5 crates exist | supervisor, settings, backend-client, terminal, webtmux | VERIFIED | all present; `backend-client`/`terminal` are intentional one-line stubs per locked CONTEXT decision ("compilable stubs filled by later phases") — not phase failures |
| Go sidecar on dynamic port | Spawn → `BACKEND_PORT:<n>` → dynamic `http://127.0.0.1:<n>` | VERIFIED behaviorally | `reaches_ready` integration test passed in my run (real spawn, dynamic port asserted `http://127.0.0.1:` prefix) |

---

## 3. Behavioral Test Evidence (cargo test --workspace, run by me)

| Suite | Tests | Result |
| --- | --- | --- |
| `webtmux-settings` (store_test.rs) | roundtrip, corrupt_recovery, first_run, atomic_save_leaves_no_temp_files_and_writes_file, window_state_clamping_and_guard_logic | 5/5 PASS |
| `webtmux-supervisor` unit (lib.rs) | test_parse_handshake, test_spawn_options_env | 2/2 PASS |
| `webtmux-supervisor` integration (integration.rs) | test_parse_handshake_unit, test_spawn_options_env_contract, **reaches_ready (real Go sidecar)**, invalid_path, handshake_timeout, early_exit, **adopt_or_clear (live probe + stop → None)** | 7/7 PASS (5.14s; the two fixture-dependent tests did NOT skip — `tmux-gui-server.exe` was present at repo root) |

Note: `window_state_clamping_and_guard_logic` is vacuous (asserts struct fields it just constructed — see M3 noted deficiency); it does not exercise `window_state::restore()`/`extract_window_state()`.

---

## 4. Data-Flow Trace (Level 4)

| Artifact | Data variable | Source | Produces real data | Status |
| --- | --- | --- | --- | --- |
| `app_state.rs` render → `views/status.rs` | `backend_status` | Real child process: supervisor spawn → watch channel → mpsc `SupervisorEvent` → `AppState` | YES — integration tests prove spawn→Ready/Failed with a real Go backend | ✓ FLOWING |
| S3 Failed page reason / stderr tail | `reason`, `stderr_tail` | Actual child spawn errors / captured child stderr (ring buffer) | YES — `invalid_path` test asserts non-empty reason + tail from a real failed spawn | ✓ FLOWING |
| S2/S3/S4 pages | UI constants (colors, labels) | 01-UI-SPEC tokens (static by design) | Static-by-spec, not data | ✓ (by contract) |
| Geometry restore (main.rs) | `WindowBounds` | `settings.json` via `DesktopSettings` store | YES — store roundtrip test proves real-file persistence of `WindowState` | ✓ FLOWING |

## 5. Key Links (wiring) — all VERIFIED

| From | To | Via | Status |
| --- | --- | --- | --- |
| main.rs | settings crate | `DesktopSettings::load()` (main.rs:20) | WIRED |
| main.rs | window_state.rs | `restore()` (25) + `observe()` (86) | WIRED |
| main.rs | app_state.rs | `AppState::new` + `start_supervisor` (88-91) | WIRED |
| app_state.rs | supervisor crate | `Supervisor::new/subscribe/spawn` under `TOKIO_RT` (79-109) | WIRED |
| app_state.rs | views/status.rs + tab_strip.rs | root render (146-173) | WIRED |
| status.rs Retry | app_state.start_supervisor | retry closure in render (161-165) | WIRED |
| status.rs Quit | stop_supervisor + cx.quit | quit closure (166-169, after WR-03) | WIRED |
| tab_strip.rs | window_state.rs | `toggle_maximize` via cx.listener (113-117) | WIRED |
| window_state.observe | DesktopSettings.save | close-flush (178-186) | WIRED |
| tools/fxc/main.rs | d3dcompiler_47.dll | LoadLibraryA/GetProcAddress → D3DCompile (157-209) | WIRED |
| BUILDING.md | GPUI_FXC_PATH | documented release hook (sections §1/§2) | VERIFIED |

## 6. Review-Fix Regression Check (post-review commits re-verified in current code)

| Finding | Fix commit | Present in HEAD code |
| --- | --- | --- |
| WR-01 stderr buffer char safety | `cf3dbcd` | yes — `char_indices`-based trim (lib.rs:239-246) |
| WR-02 adopt_or_clear URL safety | `b0e7f69` | yes — `Url::parse` + `port_or_known_default` (lib.rs:451-455) |
| WR-03 quit teardown | `5384260` | yes — Quit → `stop_supervisor()` then `cx.quit()` (app_state.rs:166-169) |
| WR-04 −10000 threshold | `0386f07` | yes — extract/restore use −10000 (window_state.rs:113, 153) |

IN-01/IN-02/IN-03 remain deferred INFO items with recorded rationale (01-REVIEW.md Fix Log) — no blocker impact.

## 7. Anti-Pattern / Debt-Marker Scan

`grep -r "TODO|FIXME|XXX|TBD|HACK|placeholder|coming soon|not yet implemented"` across `desktop-gpui/` → **zero matches**. No blockers, no warnings. Stub crates (`backend-client`, `terminal`) are one-liners **by locked design** (CONTEXT decision: "compilable stubs filled by later phases; Out: REST/WS clients Phases 2–3, terminal engine Phase 4") — correctly behind the phase fence, not stub-rot.

## 8. Requirement Traceability (Phase 1 IDs)

| Requirement | Source plan | Description | Status | Evidence |
| --- | --- | --- | --- | --- |
| STATE-01 | 01-01 | "Starting backend…" while sidecar warms up; failed-backend page with reason + Retry/Quit after 10s port timeout | SATISFIED | supervisor transitions test-verified (`reaches_ready`, `invalid_path`, `handshake_timeout`, `early_exit` — real fixture run, 7/7 green); views wired (`status.rs`, `app_state.rs`); screen rendering → HV-1/HV-2 |
| SET-05 | 01-02 | Settings persist across restarts (webterm settings-crate pattern) | SATISFIED | settings crate w/ atomic save + .bak recovery + `tmux-gui-desktop` isolation; `roundtrip`/`corrupt_recovery`/`first_run` green; wiring into boot (restore) and shutdown (observe→save) verified |
| SHELL-02 | 01-02 | Minimize/maximize/restore/close via custom title-bar window controls | SATISFIED | tab_strip.rs handlers + window_state Win32 toggle wired and building; drag region `WindowControlArea::Drag`; on-screen click loop → HV-3 |
| PKG-01 | 01-01, 01-03 | `cargo build` Windows (debug w/o fxc; release w/ fxc); Linux prereqs documented | SATISFIED | Debug workspace build PASS (run by me); release binary on disk + prior release run recorded; fxc tool substantive & matches vendored gpui contract; BUILDING.md complete (all Linux packages present) |

No orphaned Phase-1 requirements: REQUIREMENTS.md traceability maps exactly {SHELL-02, SET-05, STATE-01, PKG-01} → Phase 1; each appears in ≥1 plan's `requirements` list (01-01: STATE-01, PKG-01; 01-02: SET-05, SHELL-02; 01-03: PKG-01). Requirements completion checkboxes in REQUIREMENTS.md remain un-ticked, but per instructions STATE.md/REQUIREMENTS.md updates are owned by the orchestrator.

---

## 9. Human Verification Required (screen-level items — code paths verified, not exercised on a display)

Per verification policy: code-verified wiring + automated test evidence counts as verified; these four items are the residual on-screen judgment that only a human can observe:

### HV-1: Startup path on screen
**Test:** Run `cargo run --manifest-path desktop-gpui/Cargo.toml -p webtmux` and watch the window.
**Expected:** Window opens; while the Go sidecar warms up you see the 20px rotating Loader2 spinner above muted "Starting backend…"; within seconds the spinner is replaced by the ready shell (44px title bar with TerminalSquare icon + "Tmux GUI" over a bare #1e1e1e body, zero status-page leftovers).
**Why human:** GPUI pixel rendering, spinner animation, font shaping on a real display cannot be validated headlessly.

### HV-2: Failed-backend page and Retry/Quit loop
**Test:** Temporarily break the backend path (e.g. rename the sidecar or set `backend_path` in settings.json to a bogus file), launch the app, wait ≤10s.
**Expected:** Failed card appears (max-700px, #2d2d2d, red #7F1D1D border) with "Backend Startup Failed", the failure reason, and a redacted stderr tail if captured; **Retry** returns to the Starting page and re-runs the full teardown+respawn handshake; with the path still broken it re-fails with a fresh reason (no stale carry-over); **Quit** closes the app and leaves no orphaned Go backend process.
**Why human:** on-screen layout, button interaction, and process-leak inspection via Task Manager.

### HV-3: Title-bar controls + drag
**Test:** With the app running: drag the title bar, double-click it; click minimize, maximize (then restore), close; hover each control.
**Expected:** Native move + Aero snap work via the drag region; double-click toggles maximize; buttons minimize/toggle-maximize/close the window; the maximize icon swaps Square↔Copy (14px) matching the true maximized state; idle icons #9d9d9d, hover fill #2d2d2d with #d4d4d4 icon, close hover #7F1D1D with white icon.
**Why human:** OS-level window-manager interaction (HTCAPTION, DWM) and hover styling are visual/OS behaviors.

### HV-4: Geometry persistence across restart
**Test:** Move/resize (or maximize) the window, then close it; relaunch.
**Expected:** Window reopens at the saved position/size (clamped within sane bounds) and restores maximized state if it was maximized. Also verify corrupt-safety: hand-corrupt `%APPDATA%\...\tmux-gui-desktop\settings.json` (or `dirs::config_dir()` equivalent), launch again — app must start with defaults and leave a `settings.json.bak` behind.
**Why human:** requires physically closing/relaunching the GUI and inspecting window placement on screen.

---

## 10. Gaps

**None.** All four phase success criteria are verified. No response-blocking gap, no missing-or-stub artifact on the critical path, all key links wired, behavioral evidence present for the sidecar state machine.

## 11. Deferred Items

None applying — Phase 1 has no truth that is failed-now-but-covered-later (Step 9b): the phase's slice (startup path, controls, persistence, build) is complete for its boundary; later-phase deltas are additive features, not repairs.

## 12. Environment Caveats That Shaped the Verdict

1. **Release build not re-executed by the verifier.** Per instruction, the prior release run was accepted as evidence: `target/release/webtmux.exe` (21.7 MB, built 2026-09-06 18:40, same toolchain as this session) exists, `fxc.exe` is compiled, and the debug build I ran proves the workspace compiles with zero source changes since (`git status` clean except planning docs). Debug + tests were re-run and passed.
2. **Supervisor integration tests exercised real sidecar processes** (`tmux-gui-server.exe` fixture present at repo root; 5.14s wall time) — they spawn and stop child processes but modify no lasting state. On a machine without the fixture or Go toolchain these two tests would skip (guards in test code) — here they genuinely ran.
3. **UI rendering is inherently not headless-testable** in this project (GPUI is main-thread + window bound); the startup-path and title-bar items are therefore verified at code level per policy, with on-screen residuals listed in §9.
4. Minor non-blocking observation (see M3): the plan-declared `desktop-gpui/crates/webtmux/tests/window_state.rs` was never created, and its substitute test is vacuous. The clamp-guard code is present, substantive, and wired. If the team wants that uncovered path tightened, a 30-line test sweeping `restore()`/`extract_window_state()` edge cases would close it — non-blocking, does not gate phase completion.
5. `desktop-gpui/tools/fxc/fxc.exe` and `desktop-gpui/test-support/` exist locally but are correctly gitignored.

---

_Verified: 2026-09-06T19:13:39+07:00_
_Verifier: gsd-verifier agent_

---

## Re-verification Context

A previous verification report for this phase existed at this path in git (`81c1b57`, status `human_needed`: 8 automated PASSED, 0 unverified, 3 screen-level human items, gaps none) but was absent from the working tree when verification began. This report overwrote it per orchestrator instruction and re-verified every claim **independently** (own commands run this session). Reconciliation with the prior report:

| Prior report | This verification |
| --- | --- |
| 8 automated items passed | Re-run and confirmed: build PASS, 16/16 tests PASS, Cargo.lock tracked, release binary present, WR-01..04 fixes in HEAD, BUILDING.md content |
| Item 8 claimed "`tests/window_state` green" | **Refuted** — `crates/webtmux/tests/window_state.rs` does not exist (the crate's tests dir is empty); the only related test is `store_test.rs::window_state_clamping_and_guard_logic`, which passes but is vacuous — it never exercises `restore()`/`extract_window_state()` (see M3 noted deficiency) |
| Items 9-11 human (startup flow, window controls, persistence+recovery) | Confirmed as correct screen-level items — preserved here as HV-1..HV-4 with expanded instructions |
| STATE-04 deferred to Phase 3 | Agreed (WS generation guard is a Phase 3 requirement; also STATE-02/03 belong to Phases 2/7) |
| S4 backstop at UAT | Backstop item checked at code level: `BackendStatus::Ready(_)` renders bare `#1e1e1e` body, S1 title bar unconditionally; no conditional status widgets remain (status.rs:176-182, app_state.rs:146-173) — final confirmation on screen during HV-1 |

