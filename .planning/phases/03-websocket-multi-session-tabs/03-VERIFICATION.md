---
phase: "03-websocket-multi-session-tabs"
verified: 2026-09-06T22:30:00Z
status: passed
score: 20/20 must-haves verified
behavior_unverified: 0
overrides_applied: 0
must_haves_total: 20
methodology: goal-backward (code inspection + commands actually executed by the verifier)
evidence_commands:
  - "cargo test --manifest-path desktop-gpui/Cargo.toml --workspace -j 2 -> PASS (39 passed / 0 failed: 11 webtmux + 12 backend-client + 7 settings + 9 supervisor)"
  - "file existence, grep wiring/prohibition/anti-pattern scans, Go/FE parity cross-checks via RESEARCH line refs"
human_verification_items: 4
gaps: 0
requirement_ids: [SHELL-01, SESS-02, SESS-04, SESS-05, STATE-04]
gaps_applied: []
---

# Phase 3: WebSocket Client & Multi-Session Tabs — Verification Report

**Phase Goal:** The user can hold several tmux sessions open as always-connected workspace tabs with race-free, correlated mutations
**Verified:** 2026-09-06T22:30:00Z
**Status:** passed
**Re-verification:** No — initial verification (no 03-VERIFICATION.md existed)

**Verifier:** gsd-verifier (goal-backward; evidence below produced by commands I actually ran on 2026-09-06, Windows / PowerShell, pwsh 7).
**Scope discipline:** changes limited to this VERIFICATION.md file only; no code re-implemented. Project-wide checks were NOT run beyond the desktop-gpui workspace per instruction.

---

## 1. Observable Truths (must-haves from 03-01-PLAN + 03-02-PLAN + roadmap success criteria)

Classification rule (Phase 1/2 precedent): straightforward wired call-chains are VERIFIED by code + wiring + driving tests (GPUI is not headless-renderable — see 03-VALIDATION.md manual-only rows). Interactive visuals (tab switching feel, chip pixels, context-menu open/dismiss, dialog typing/focus, live rename/kill against a backend) get HV advisory pointers but do not fail the truth when the code chain as written is present and wired. Only **concurrency/cancellation/cleanup/ordering invariants with no exercising test**, or plan `verification: backstop` truths, would park as ⚠️ PRESENT_BEHAVIOR_UNVERIFIED — Phase 3 plans carry no backstop truths, and every guard (generation equality, envelope match, tab-liveness, rename migration, kill gate) is driven by a unit/integration test, so the count is 0.

### 1a. Plan 03-01 truths (tracer: WS transport, tab state, generation guard)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| T1 | WsIncoming serializes session.rename (requestId/newName, no session field) and session.kill (explicit session); WsOutgoing parses connection.ready / state.snapshot / state.delta / command.success-error with session tags | ✓ VERIFIED | `test_ws_envelope_roundtrip` + `test_kill_by_name_serialization` green (ws_test.rs:14,118); DTO shapes read in ws.rs match protocol.go per RESEARCH §2 |
| T2 | SessionSnapshot parses full payloads; windows:null/panes:null normalize to empty vecs; replace/seq default false/0 | ✓ VERIFIED | `test_snapshot_null_normalization` green (ws_test.rs:80); `deserialize_null_default` on both vec fields + plain-bool/u64 defaults read in ws.rs |
| T3 | normalize_ws_url maps http(s) base to ws(s) /api/ws?session=<query-encoded> (slashes encoded, https→wss); request_id unique per call | ✓ VERIFIED | `test_normalize_ws_url` + `test_request_id_unique` green (ws_test.rs:133,157); helper read at ws.rs:165,202 |
| T4 | Client connects with ?session=NAME, receives unsolicited connection.ready + state.snapshot with no hello sent, answers ready with state.resync | ✓ VERIFIED | `test_ws_bootstrap_flow` green (ws_test.rs:181, in-process accept_async mock); read pump answers ready→resync (ws.rs:6-7 doc + pump code); MSG_HELLO exists as const only — grep shows zero send sites |
| T5 | Correlated session.rename round-trips via pending oneshot; pending registered BEFORE enqueue; late reply after forget-timeout ignored | ✓ VERIFIED | `test_ws_correlated_command` green (ws_test.rs:288); send path registers pending before enqueue (ws.rs:327-337) |
| T6 | Malformed text frame dropped without panic; next valid event still delivered | ✓ VERIFIED | `test_ws_malformed_frame_dropped` green (ws_test.rs:399); read pump `from_str` drop-and-continue, never unwraps (ws.rs:417,448-451,493) |
| T7 | Tabs open (append + activate), switch sets active only and never connects/closes a socket, close removes + drops entry with neighbor min(idx,len-1), last close clears active | ✓ VERIFIED | `test_tab_lifecycle` + `test_tab_close_neighbor_middle` green (tabs_test.rs:81,131); `set_active_session` body is a single assign + notify (app_state.rs:365-367) — zero socket calls by inspection |
| T8 | Rename re-resolution migrates entry old→new with generation+1; stale old-generation events drop post-migration | ✓ VERIFIED | `test_rename_reresolution` green (tabs_test.rs:144); migration read at app_state.rs:396-424 (generation bump, tab/expand/active moves) |
| T9 | Window-tabs model derives active window from snapshot; no-snapshot and empty-windows render empty without panic | ✓ VERIFIED | `test_window_tabs_model` green (tabs_test.rs:183); `render_window_tabs` fed by `window_tabs()` (tab_strip.rs:228,244), middle rendered ONLY when active set (:87-100) |
| T10 | Triple guard drops stale/future-generation, unknown-session, and envelope-mismatched events; absent session tag treated as belonging; terminal frames parse-and-ignore | ✓ VERIFIED | `test_ws_generation_guard` + `test_ws_terminal_frames_ignored` green (ws_guard_test.rs:37,86); all three layers read verbatim in `apply_event` (app_state.rs:436-466: (c) entry-existence, (a) generation equality, (b) session match) |
| T11 | Workspace body for an open session with a snapshot renders a real-data panel (session name + window list) proving the WS path | ✓ VERIFIED | Panel reads `sessions[name].snapshot` and renders name + window rows + "Terminal view arrives in Phase 4" (session_states.rs:405-451); falls back to existing routing without snapshot |

### 1b. Plan 03-02 truths (title bar, menus, rename + kill flows)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| T12 | confirm_kill_session/pane/window default true for fresh settings and legacy JSON lacking the keys; explicit false survives round-trip and save/load | ✓ VERIFIED | `test_kill_confirm_defaults` + `test_kill_confirm_legacy_json` green (store_test.rs:124,141); `#[serde(default = "default_true")]` on all three flags + Default impl true (settings/src/lib.rs:44,62-87) |
| T13 | Kill flow gate returns confirm-required when the session flag is true, direct-kill when false (pane/window flags do not steer it) | ✓ VERIFIED | `test_kill_flow_gate` green (kill_confirm_test.rs:11); `kill_requires_confirm` reads only the session flag (app_state.rs:370-373) |
| T14 | Title bar keeps S1 shell with divider + WindowTabs flex-1 middle only when active (h-7/max-w-44 chips, {index}: {name}, active #2d2d2d/#d4d4d4, idle #808080/hover #262626), Settings gear to honest Phase-6 placeholder with tabs left open; no Plus button, no chip menu | ✓ VERIFIED | Structure read in tab_strip.rs (divider + middle :87-100, gear sets showing_settings :111-132, chips h-7/max-w-44 + window.select click :225-269); placeholder page static copy (session_states.rs:351+); no `Plus` control and no `context_menu` in tab_strip.rs (grep). Pixel parity → HV-1 |
| T15 | Chip click sends fire-and-forget window.select on the active session socket (no await; state follows via delta) | ✓ VERIFIED | `send_window_select` uses `let _ = handle.send_command(msg)` — Receiver dropped (app_state.rs:521); click wired at tab_strip.rs:269. Live select against backend → HV-1 |
| T16 | Sidebar rows wrapped in ContextMenuExt with stable per-session ids; menu is Rename + separator + Kill Session (destructive) only; left-click opens-or-activates the tab (ensure-socket when new) | ✓ VERIFIED | Stable `session-row/{name}` ids + left-click open/ensure (sidebar.rs:146,168-170); menu builder Rename + separator + red Kill only (session_context_menu.rs:43-57); wrapped via `with_session_context_menu` (sidebar.rs:279). Right-click open/dismiss + poll-tick retention → HV-2 |
| T17 | Rename dialog (DLG1 tokens, prefilled, autofocus, Enter-submit, Rename disabled while empty/busy, inline #7F1D1D error) pre-flights validate_session_name and sends session.rename on the TARGET socket; success migrates old→new with generation+1 + reconnect + poll + close; error/timeout stays open, tab untouched | ✓ VERIFIED | Form entity + prefill + focus + Enter (rename_session_dialog.rs:28-79), disabled-while-empty/busy footer (:196-242), is_submitting guard (:259+), `submit_rename` resolves target handle + pends when unopened + 10s correlated await + migration/close (app_state.rs:528-630); pure migration half unit-locked (T8). Live typing + correlated rename → HV-3 |
| T18 | Kill honors the setting (confirm dialog with destructive Kill vs direct kill), rides victim / any-live-plus-explicit-session / ephemeral transports, closes socket + tab with neighbor activation + poll on success, renders inline destructive without closing on error | ✓ VERIFIED | Gate read at session_context_menu.rs:76-80; `KillRoute::{Victim, ViaOther, Ephemeral}` + `execute_kill` + ephemeral one-shot that commits nothing + `finish_kill_success` (app_state.rs:78,677-884); dialog DLG1 tokens + destructive Kill + inline error (session_context_menu.rs:86+). Live kill → HV-4 |
| T19 | Sidebar row click and SelectSessionView picker cards open-or-activate tabs (ensure-socket when new); kill of the last tab routes through existing workspace_state pages | ✓ VERIFIED | Row click open + ensure (sidebar.rs:168-170); picker cards open-or-activate per 03-02 deviation 5 (session_states.rs); `close_session` clears active when empty (app_state.rs:381-389) reusing workspace_state routing |
| T20 | Full workspace battery green with all 03-01 guards still passing — STATE-04 held, no regressions | ✓ VERIFIED | `cargo test --workspace -j 2`: 39 passed / 0 failed (see §5); carried guards (tab lifecycle, rename, window-tabs model, generation guard, terminal-ignore, bootstrap/correlation interop) all green in the same run |

**Score: 20/20 truths verified, 0 present-but-behavior-unverified, 0 failed.**

### 1c. Roadmap success criteria → truth mapping

| SC | Criterion | Status | Covering truths |
|----|-----------|--------|-----------------|
| SC1 | Multiple sessions as tabs, all connected, switching never disconnects/re-handshakes | ✓ VERIFIED | T4 (per-session sockets), T7 (switch never touches sockets), T19 (open-or-activate) |
| SC2 | Title bar: drag region, identity, active window tabs, Settings gear — Electron-identical | ✓ VERIFIED | T14, T15, T9 (code-level parity; pixel sign-off → HV-1) |
| SC3 | Rename via context menu with real text input, propagates to sidebar + tabs incl. mid-session re-resolution | ✓ VERIFIED | T16 (menu), T17 (dialog + target-socket + migration), T8 (migration guard) |
| SC4 | Kill via context menu confirms per setting, closes tab cleanly | ✓ VERIFIED | T12, T13 (setting + gate), T18 (flow), T7 (neighbor activation) |
| SC5 | Rapid switching while churning never applies another session's snapshot (generation guard) | ✓ VERIFIED | T10 (triple guard unit-driven), T5 (correlation), T6 (pump resilience); live stress → HV-4 residual |

---

## 2. Required Artifacts (exists → substantive → wired)

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `backend-client/src/ws.rs` | WsIncoming/WsOutgoing/SessionSnapshot DTOs, TransportState, CommandResult, SessionWsHandle, connect_session, normalize_ws_url, request_id | ✓ VERIFIED | 19 KB; all plan-listed exports present; msg/ev consts verbatim; drop-and-continue read pump; ready→resync; no hello send |
| `backend-client/src/lib.rs` | WS surface re-exports | ✓ VERIFIED | Re-exports WsIncoming/WsOutgoing/SessionSnapshot/TransportState/CommandResult/SessionWsHandle/connect_session/normalize_ws_url/request_id + MSG/EV consts (incl. MSG_HELLO const-only) |
| `backend-client/tests/ws_test.rs` + 3 fixtures | 8 envelope + interop tests | ✓ VERIFIED | 8/8 green; fixtures ws_snapshot_full / ws_snapshot_null_slices / ws_delta on disk with true Go envelope shapes |
| `webtmux/src/app_state.rs` | open_sessions + sessions map + triple guard + open/switch/close + rename/kill orchestration + ensure_socket | ✓ VERIFIED | 48 KB; OpenSession{generation, transport, snapshot, handle, pending, last_error}; lifecycle + apply_event + submit_rename + KillRoute/execute_kill/finish_kill_success + showing_settings + pending_rename flush paths all read |
| `webtmux/src/views/session_states.rs` | Snapshot placeholder panel + settings placeholder page + picker open-or-activate | ✓ VERIFIED | 18 KB; snapshot panel reads live entry snapshot (:415-451); settings placeholder static copy (:351+); picker opens-or-activates |
| `webtmux/src/views/tab_strip.rs` | WindowTabs middle + Settings gear in S1 bar | ✓ VERIFIED | 10 KB; divider + conditional middle + gear → showing_settings; chips wired to send_window_select; no Plus, no chip menu |
| `webtmux/src/views/sidebar.rs` | Stable row ids + context-menu wrap + open-or-activate click | ✓ VERIFIED | Stable `session-row/{name}` ids; left-click open/ensure; menu wrap; footer Settings routing |
| `webtmux/src/views/session_context_menu.rs` | Rename + separator + Kill menu + kill-confirm dialog + kill routing | ✓ VERIFIED | 10 KB; 2-item menu only; gate read; DLG1-token dialog with destructive Kill + inline error; routes via execute_kill |
| `webtmux/src/views/rename_session_dialog.rs` | RenameSessionForm entity + DLG1 dialog + target-socket submit | ✓ VERIFIED | 10 KB; InputState prefill/focus/Enter/disabled-states; submits via AppState::submit_rename |
| `settings/src/lib.rs` | confirm_kill_session/pane/window default-true flags | ✓ VERIFIED | default_true helper + serde defaults + Default true; atomic-write/.bak path untouched |
| Tests (tabs, ws_guard, kill_confirm, store ext) | lifecycle/rename/model/guard/terminal-ignore/gate/defaults | ✓ VERIFIED | tabs 4 + guard 2 + kill-gate 1 + store 7 (incl. 2 new) — all green |

Supporting edits verified present: `views/mod.rs` registers both new modules; `backend-client/Cargo.toml` pins workspace `rand` (no new crates — lockfile entry recorded in 03-01).

---

## 3. Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| app_state.rs | backend-client/ws.rs | ensure_session_socket spawns connect_session on TOKIO_RT; read pump forwards tagged (session, generation, WsEvent) | ✓ WIRED | ensure_session_socket pushes tab + bumps generation + spawns connect_session_with_pending with shared pending map (app_state.rs:911-978); generation-mismatch outcomes dropped |
| session_states.rs | app_state.rs | Open-session workspace panel renders active snapshot window list proving the WS path | ✓ WIRED | render_active_session_placeholder reads sessions[name].snapshot (session_states.rs:415-422) |
| tab_strip.rs | app_state.rs | WindowTabs reads active snapshot windows + active_window; chip click sends window.select on the active socket | ✓ WIRED | window_tabs() feed (tab_strip.rs:244) + send_window_select fire-and-forget (:269 → app_state.rs:521) |
| rename_session_dialog.rs | app_state.rs | Submit resolves the target handle, awaits correlated result, migrates old→new on success | ✓ WIRED | request_rename_submit → submit_rename(target…) (rename_session_dialog.rs:256-307 → app_state.rs:530-630) |
| session_context_menu.rs | sidebar.rs / app_state.rs | Rows wrapped with stable ids; Kill Session routes via execute_kill with D6 transport order | ✓ WIRED | with_session_context_menu at sidebar.rs:279; menu → execute_kill (session_context_menu.rs:69-80,270-304) |
| kill dialog / direct kill | app_state.rs | finish_kill_success drops entry + neighbor activation + poll | ✓ WIRED | execute_kill success paths → finish_kill_success → close_session (app_state.rs:721-884) |

Prohibitions: **P1** no hello sent — VERIFIED (MSG_HELLO const-only, zero send sites). **P2** no connect/close in set_active_session — VERIFIED (single-assign body). **P3** no unwrap in WS read loop — VERIFIED (drop-and-continue; the one `unwrap_or_default` is the serde null-default helper, not the loop). **P4** no new crate deps — VERIFIED (workspace-pinned rand/tokio-tungstenite/futures-util/flume only). **P5** no Plus button / chip menu / dead Phase-5 entries — VERIFIED (comment D8 + grep-empty). **P6** no rename via active socket for non-active target — VERIFIED (submit_rename resolves target handle, ensure_session_socket(target) + pending_rename when unopened). **P7** no backend protocol changes — VERIFIED (git status: be/ untouched; only planning-doc drift).

---

## 4. Data-Flow Trace (Level 4)

| Artifact | Data variable | Source | Produces real data | Status |
|----------|---------------|--------|--------------------|--------|
| app_state.sessions[name].snapshot | `SessionSnapshot` | Real WS state.snapshot/state.delta via connect_session bootstrap, committed through apply_event (mock-server proven) | YES | ✓ FLOWING |
| workspace snapshot panel | session name + window list | Live entry snapshot (session_states.rs:415-451) | YES (headless pixel output N/A by GPUI design) | ✓ FLOWING |
| WindowTabs chips | active snapshot windows + active_window | window_tabs() model (unit-proven incl. empty edge) | YES | ✓ FLOWING |
| rename submit | newName + requestId | Real InputState value; validate_session_name pre-flight; correlated command.success | YES (chain wired; live run → HV-3) | ✓ FLOWING |
| kill submit | session + requestId | Victim/via-other/ephemeral transports with explicit session field | YES (chain wired; live run → HV-4) | ✓ FLOWING |
| kill-confirm gate | confirm_kill_session | DesktopSettings (default-true, legacy-safe) | YES | ✓ FLOWING |
| ephemeral kill socket | one-shot success | Fresh pending map, never forwards events — commits nothing by construction | YES (phantom-tab caution documented) | ✓ FLOWING |

No hollow props, no static fallbacks, no mocks in the production path (mocks exist only inside test files).

---

## 5. Behavioral Spot-Checks (commands run by me)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full workspace suite | `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace -j 2` (from repo root) | **39 passed / 0 failed**: webtmux 11 (kill_confirm 1, polling 2, sidebar 1, state_view 1, tabs 4, ws_guard 2) + backend-client 12 (rest 3, validation 1, ws 8) + settings 7 + supervisor 9 (unit 2 + integration 7 incl. live Go sidecar `reaches_ready`, ~5.1s) | ✓ PASS |
| Test enumeration | same suite `-- --list` | 39 tests listed — matches the 39 passed; SUMMARY claim "39/39 green" **confirmed independently** | ✓ PASS |
| Named Phase-3 tests | enumerated via suite output (no extra full runs) | `test_ws_envelope_roundtrip`, `test_snapshot_null_normalization`, `test_kill_by_name_serialization`, `test_normalize_ws_url`, `test_request_id_unique`, `test_ws_bootstrap_flow`, `test_ws_correlated_command`, `test_ws_malformed_frame_dropped`, `test_tab_lifecycle`, `test_tab_close_neighbor_middle`, `test_rename_reresolution`, `test_window_tabs_model`, `test_ws_generation_guard`, `test_ws_terminal_frames_ignored`, `test_kill_confirm_defaults`, `test_kill_confirm_legacy_json`, `test_kill_flow_gate` — 17/17 ok | ✓ PASS |

(Cold first run compiled; two warm confirmation runs reused the cache. Tests spawn in-process mock listeners plus the Phase-1 sidecar fixture and mutate no lasting state. `-j 2` per the 03-02 disk-pressure deviation; no source touched.)

### Probe Execution

No probes declared: neither PLAN mentions `probe-*.sh`, and `scripts/*/tests/probe-*.sh` is unrelated to this phase. SKIPPED (no applicable probes).

---

## 6. Requirements Coverage

| Requirement | Source plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| SHELL-01 | 03-02 | Title bar with drag region, identity, active window tabs, Settings gear — Electron-identical | SATISFIED | T9/T14/T15 + tab_strip structure; pixel sign-off → HV-1 |
| SESS-02 | 03-01, 03-02 | Open sessions as workspaces; multiple stay connected; switching never tears down | SATISFIED | T3/T4/T7/T19 + bootstrap interop + open-or-activate wiring |
| SESS-04 | 03-02 | Rename via context menu (dialog with real text input) | SATISFIED | T8/T16/T17 + target-socket transport; typing/focus → HV-3 |
| SESS-05 | 03-01, 03-02 | Kill via context menu with confirmation (kill-confirm setting) | SATISFIED | T1(kill serialization)/T12/T13/T18 + KillRoute transports; live kill → HV-4 |
| STATE-04 | 03-01, 03-02 | WS generation guard: stale events never render into the wrong tab | SATISFIED | T1/T2/T5/T6/T10/T20 + triple guard unit-driven; live stress → HV-4 residual |

No orphaned Phase-3 requirements: REQUIREMENTS.md maps exactly {SHELL-01, SESS-02, SESS-04, SESS-05, STATE-04} → Phase 3, each claimed by ≥1 plan (03-01: SESS-02/STATE-04; 03-02: all five).

---

## 7. Anti-Patterns Found

`rg "TODO|FIXME|XXX|TBD|HACK|unimplemented|todo!|placeholder"` across `desktop-gpui/crates/{backend-client,webtmux,settings}/src` → **zero debt markers**. The only "placeholder" hits are: Input placeholder strings (`"dev"`, `"New name"` — real UX hint text), the doc comment on the settings flag, and the plan-mandated honest Phase-6 Settings page (`render_settings_placeholder` + `render_active_session_placeholder` doc names). The 03-02 SUMMARY's stub-scan claim **confirmed**. No blockers, no warnings — ℹ️ info only: the snapshot panel's "Terminal view arrives in Phase 4" note and the "Settings arrive in Phase 6" page are intentional phase seams, not stub-rot.

---

## 8. Human Verification Required

Residuals that code inspection + headless tests cannot close (GPUI needs a display; 03-VALIDATION.md lists the same five manual-only rows; 03-02 SUMMARY T3–T6 already flag human_judgment:true). Advisory per Phase 1/2 precedent — none is a gap: every truth as written holds in code.

### HV-1: Title-bar window tabs + gear pixel parity + chip click
**Test:** Launch against a backend with 2+ sessions; open two tabs; observe the title bar; click window chips; click the Settings gear; press Back.
**Expected:** Divider + flex-1 window chips `{index}: {name}` with active/idle/hover tokens per UI-SPEC; chip click selects the tmux window (state follows via delta, no flicker); gear shows the honest "Settings arrive in Phase 6" page with tabs still connected underneath; Back returns to the workspace; no Plus button, no chip menu.
**Why human:** Chip geometry/colors, gear behavior, and live window.select need eyes + a real backend (covers T14/T15 pixels, SC2, VALIDATION manual row 2).

### HV-2: Session context menu open/dismiss + poll-tick retention
**Test:** Right-click a sidebar session row; leave the menu open across a 1.5s poll tick; pick Rename / Kill Session; confirm left-click still selects.
**Expected:** Menu shows Rename + separator + Kill Session (destructive) only; stays open across poll ticks (stable ids); left-click selects without opening the menu.
**Why human:** Interactive GPUI menu behavior + poll-tick state retention (covers T16, SC3/SC4 menu half, VALIDATION manual rows 3–4).

### HV-3: Rename dialog end-to-end incl. mid-session re-resolution
**Test:** Right-click a row → Rename; confirm prefilled name + autofocus; type, Enter-submit; try empty name (Rename disabled); duplicate name (inline error, dialog stays open); rename the active tab while its session churns; rename an unopened sidebar session.
**Expected:** Prefill/focus/Enter/disabled states per DLG1; success migrates sidebar + tabs old→new with no ghost windows; error/timeout shows inline #7F1D1D with tab untouched; unopened-target path connects then renames (pending_rename flush — the one sub-path with no dedicated unit test, code-read only).
**Why human:** Dialog typing/focus and live correlated rename against tmux (covers T17, SC3, VALIDATION manual row 3).

### HV-4: Kill confirm per setting + clean tab close + switching stress
**Test:** With confirm on, Kill → confirm dialog → Kill (tab closes, neighbor activates); with confirm off, Kill kills directly; kill the last tab (workspace pages); kill an unopened session (ephemeral path); churn tmux windows while fast-switching tabs.
**Expected:** Confirm-vs-direct branches per flag; destructive copy; success drops entry + neighbor activation + poll; error renders inline without closing; last-tab kill routes to workspace pages; rapid switching never shows wrong-session windows.
**Why human:** Confirm branching, dialog copy, live kill, and race timing need a backend + eyes (covers T18, SC4/SC5, VALIDATION manual rows 4–5).

---

## 9. Gaps Summary

**No gaps.** All 20 truths verified against the codebase: transport DTOs + connect/correlate path proven by 8 interop-backed tests, tab lifecycle + triple guard + rename migration + kill gate driven by unit tests, title-bar/menu/dialog/kill flows present-substantive-wired with all seven prohibitions holding (no hello, no switch-path socket calls, no loop unwraps, no new deps, no Plus/chip-menu/dead entries, target-socket rename, backend untouched). Interactive/visual residuals are HV-1..HV-4 advisory, same class as the Phase 1/2 manual checks — they do not fail any plan truth as written.

## 10. Deferred Items

None — no failed truth exists to defer, and no later milestone phase repairs a Phase-3 concern (Step 9b checked against ROADMAP phases 4–7; terminal ingestion is Phase 4's new work, not a Phase-3 gap; reconnect UX is Phase 7's new work, not a Phase-3 gap).

## 11. Environment Caveats That Shaped the Verdict

1. Tests were run from the repo root per the task's minimum-verification instruction (`cargo test --manifest-path desktop-gpui/Cargo.toml --workspace -j 2`); no project-wide checks (lint/clippy/fmt/release build) were executed.
2. `-j 2` follows the 03-02 disk-pressure deviation (E: nearly full, 18 GB target dir); the durable fix (CARGO_TARGET_DIR on a roomier drive) remains flagged, unchanged by the verifier.
3. The workspace run re-executed Phase-1/2 suites as a regression sweep — all still green (polling 2, sidebar 1, state_view 1, rest 3, validation 1), so no regressions from Phase-3 edits.
4. `git status` shows only planning-doc drift (`M .planning/ROADMAP.md, STATE.md, config.json`, untracked `state.json`, `ui-reviews/`, phase dirs) — no source modifications by the verifier; the tree was clean for all phase files.
5. `pending_rename` flush/drop (rename of an unopened target) and the ephemeral-kill commit-nothing path are code-read + wired but have no dedicated unit test; they are covered by HV-3/HV-4 advisory, not by gaps — a future unit test would harden them.

---

_Verified: 2026-09-06T22:30:00Z_
_Verifier: gsd-verifier agent_
