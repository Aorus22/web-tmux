---
phase: 03-websocket-multi-session-tabs
plan: "01"
subsystem: desktop-gpui
tags: [rust, gpui, websocket, tokio-tungstenite, flume, tabs, generation-guard, tmux]

# Dependency graph
requires:
  - phase: 02-rest-client-sidebar-session-management
    provides: Typed backend-client DTOs with null-slice discipline, AppState polling pump, workspace state routing
provides:
  - WS transport DTOs (WsIncoming/WsOutgoing/SessionSnapshot) plus connect_session with correlated send and tagged read pump in webtmux-backend-client
  - Per-session tab state (open_sessions + sessions map) with triple generation guard in AppState
  - Snapshot placeholder workspace panel proving the WS path end to end
affects: [03-02-PLAN, 03-03-PLAN, 04-terminal-io]

actuals:
  tokens: 16750
  tasks: 3
  commits: 6

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Flume split-pump WS handle: unbounded outbound queue plus tagged (session, generation) inbound events"
    - "Correlated send with pending[requestId] registered BEFORE enqueueing plus forget-timeout drop semantics"
    - "Triple generation guard at apply time: generation equality, envelope session match, tab-liveness"
    - "Rename re-resolution as entry migration old-to-new with generation+1 and stale socket drop"

key-files:
  created:
    - desktop-gpui/crates/backend-client/src/ws.rs
    - desktop-gpui/crates/backend-client/tests/ws_test.rs
    - desktop-gpui/crates/backend-client/tests/fixtures/ws_snapshot_full.json
    - desktop-gpui/crates/backend-client/tests/fixtures/ws_snapshot_null_slices.json
    - desktop-gpui/crates/backend-client/tests/fixtures/ws_delta.json
    - desktop-gpui/crates/webtmux/tests/tabs_test.rs
    - desktop-gpui/crates/webtmux/tests/ws_guard_test.rs
  modified:
    - desktop-gpui/crates/backend-client/src/lib.rs
    - desktop-gpui/crates/backend-client/Cargo.toml
    - desktop-gpui/Cargo.lock
    - desktop-gpui/crates/webtmux/src/app_state.rs
    - desktop-gpui/crates/webtmux/src/views/session_states.rs

key-decisions:
  - "Model Outgoing.replace as plain bool and seq as u64 with serde defaults (Go omits false/zero)"
  - "Read pump answers connection.ready with state.resync internally and never sends hello (D7)"
  - "Command success/error frames resolve pending correlations and are consumed, never forwarded as state events"
  - "Tab lifecycle and guard apply-function are pure AppState methods (no GPUI Context) so they stay headless-testable"

patterns-established:
  - "Pattern 1: SessionWsHandle with Drop-abort pump tasks giving teardown-on-close by entry removal"
  - "Pattern 2: connect_session_with_pending sharing one correlation table between AppState entry and read pump"
  - "Pattern 3: ensure_session_socket hops TOKIO_RT connect result back to the GPUI thread via oneshot plus weak upgrade"

requirements-completed:
  - SESS-02
  - STATE-04

coverage:
  - id: T1
    description: "WS envelopes round-trip rename/kill with requestId/newName/explicit session; snapshot/delta/command frames parse with session tags"
    requirement: STATE-04
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/backend-client/tests/ws_test.rs#test_ws_envelope_roundtrip"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/backend-client/tests/ws_test.rs#test_kill_by_name_serialization"
        status: pass
    human_judgment: false
  - id: T2
    description: "SessionSnapshot parses full payloads and normalizes windows:null/panes:null to empty vecs with replace/seq defaults"
    requirement: STATE-04
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/backend-client/tests/ws_test.rs#test_snapshot_null_normalization"
        status: pass
    human_judgment: false
  - id: T3
    description: "normalize_ws_url maps http(s) base to ws(s) api/ws with query-encoded session; request_id unique per call"
    requirement: SESS-02
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/backend-client/tests/ws_test.rs#test_normalize_ws_url"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/backend-client/tests/ws_test.rs#test_request_id_unique"
        status: pass
    human_judgment: false
  - id: T4
    description: "Mock-server bootstrap proves unsolicited ready/snapshot, resync answered, no hello, correct ?session= URL"
    requirement: SESS-02
    verification:
      - kind: integration
        ref: "desktop-gpui/crates/backend-client/tests/ws_test.rs#test_ws_bootstrap_flow"
        status: pass
    human_judgment: false
  - id: T5
    description: "Correlated session.rename round-trips via pending oneshot; orphan reply after forget-timeout ignored, pump survives"
    requirement: STATE-04
    verification:
      - kind: integration
        ref: "desktop-gpui/crates/backend-client/tests/ws_test.rs#test_ws_correlated_command"
        status: pass
    human_judgment: false
  - id: T6
    description: "Malformed text frame dropped without panic, next valid event still delivered"
    requirement: STATE-04
    verification:
      - kind: integration
        ref: "desktop-gpui/crates/backend-client/tests/ws_test.rs#test_ws_malformed_frame_dropped"
        status: pass
    human_judgment: false
  - id: T7
    description: "Tabs open/switch/close with neighbor min(idx,len-1) activation; switch never touches sockets; last close clears active"
    requirement: SESS-02
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/tabs_test.rs#test_tab_lifecycle"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/tabs_test.rs#test_tab_close_neighbor_middle"
        status: pass
    human_judgment: false
  - id: T8
    description: "Rename re-resolution migrates entry old-to-new with generation+1; stale old-generation events drop post-migration"
    requirement: SESS-02
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/tabs_test.rs#test_rename_reresolution"
        status: pass
    human_judgment: false
  - id: T9
    description: "Window-tabs model derives active window from snapshot; no-snapshot and empty-windows render empty without panic"
    requirement: SESS-02
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/tabs_test.rs#test_window_tabs_model"
        status: pass
    human_judgment: false
  - id: T10
    description: "Triple guard drops stale/future-generation, unknown-session, and envelope-mismatched events; absent session tag treated as belonging"
    requirement: STATE-04
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/ws_guard_test.rs#test_ws_generation_guard"
        status: pass
    human_judgment: false
  - id: T11
    description: "terminal.snapshot/terminal.output frames parse and are accepted but commit no state"
    requirement: STATE-04
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/ws_guard_test.rs#test_ws_terminal_frames_ignored"
        status: pass
    human_judgment: false

duration: 30min
completed: 2026-09-06
status: complete
---

# Phase 03 Plan 01: WS Transport, Tab State & Generation Guard (Tracer) Summary

**Tested `ws.rs` transport (DTOs plus connect/correlate path proven against an in-process mock WS server), `AppState` per-session tab lifecycle with the triple generation guard, and a snapshot placeholder workspace panel rendering live WS data.**

## Performance

- **Duration:** 30 min
- **Started:** 2026-09-06T15:12:30Z
- **Completed:** 2026-09-06T15:42:21Z
- **Tasks:** 3
- **Files modified:** 12

## Accomplishments

- Implemented `ws.rs` envelope DTOs verbatim from `be/internal/realtime/protocol.go` (`WsIncoming` with skip-serializing `Option` semantics, `WsOutgoing` with plain-`bool` replace and `u64` seq defaults, `SessionSnapshot` reusing Phase-2 tmux DTOs with null-slice discipline), `TransportState`, `CommandResult`, `request_id()` via pinned `rand 0.10.2`, `normalize_ws_url()`, and all MSG/EV string constants.
- Implemented `connect_session` / `connect_session_with_pending` with flume split pumps: write pump drains the outbound queue, read pump parses drop-and-continue (never unwraps), resolves `command.success/error` into pending oneshots, answers `connection.ready` with `state.resync`, and forwards everything else tagged `(session, generation)`.
- Proved mock-server interop: unsolicited ready/snapshot bootstrap with no hello sent, `?session=` URL assertion, correlated rename round-trip with forget-timeout orphan handling, malformed-frame resilience.
- Added `OpenSession` tab map (`open_sessions` ordered vec plus `sessions` map with generation, transport, snapshot, handle, shared pending, last_error) keeping `active_session` a view pointer; pure `open_session` / `set_active_session` / `close_session` (neighbor `min(idx,len-1)`) / `rename_session_entry` (migration with generation+1) / `apply_event` (triple guard) / `window_tabs` plus `ensure_session_socket` wiring connects on `TOKIO_RT` with mid-connect close/re-resolve drops.
- Extended the workspace body with a snapshot placeholder panel (session name plus window list from the live snapshot, Phase-4 note) falling back to existing routing when no snapshot is committed.
- Full workspace suite green: 36 tests pass across 5 crates (14 new).

## Task Commits

Each task was committed atomically following TDD and conventional commit guidelines:

1. **Task 1 (TDD RED):** `f2116cb` (test(03-01): add failing tests for WS envelopes, snapshot normalization, kill serialization, url helpers)
2. **Task 1 (TDD GREEN):** `650381d` (feat(03-01): implement WS envelope DTOs, snapshot, url and request-id helpers)
3. **Task 2 (TDD RED):** `d4b1b64` (test(03-01): add failing mock-server interop tests for bootstrap, correlation, malformed frames)
4. **Task 2 (TDD GREEN):** `0976394` (feat(03-01): implement connect_session with correlated send and tagged read pump)
5. **Task 3 (Tracer):** `4b0f48a` (feat(03-01): add per-session tab map, triple generation guard, and snapshot placeholder panel)
6. **Lockfile/fixture fixup:** `5db79d3` (chore(03-01): record pinned rand dep in lockfile and zoomed pane field in delta fixture)

## Files Created/Modified

- `desktop-gpui/crates/backend-client/src/ws.rs` - WS DTOs, helpers, SessionWsHandle, connect_session pumps
- `desktop-gpui/crates/backend-client/src/lib.rs` - WS surface re-exports
- `desktop-gpui/crates/backend-client/Cargo.toml` - pinned `rand` workspace dep (no new crates)
- `desktop-gpui/Cargo.lock` - lockfile entry for `rand 0.10.2` under backend-client
- `desktop-gpui/crates/backend-client/tests/ws_test.rs` - 8 envelope plus interop tests
- `desktop-gpui/crates/backend-client/tests/fixtures/ws_snapshot_full.json` - full state.snapshot envelope
- `desktop-gpui/crates/backend-client/tests/fixtures/ws_snapshot_null_slices.json` - null-slice envelope
- `desktop-gpui/crates/backend-client/tests/fixtures/ws_delta.json` - state.delta envelope with session tag and seq
- `desktop-gpui/crates/webtmux/src/app_state.rs` - OpenSession map, lifecycle, guard, ensure_socket wiring
- `desktop-gpui/crates/webtmux/src/views/session_states.rs` - snapshot placeholder panel
- `desktop-gpui/crates/webtmux/tests/tabs_test.rs` - lifecycle, neighbor, rename, window-tabs tests
- `desktop-gpui/crates/webtmux/tests/ws_guard_test.rs` - guard and terminal-ignore tests

## Decisions Made

- Read-pump answers `connection.ready` with `state.resync` internally so both the mock test and `AppState` get FE `sockets.ts` parity without extra wiring.
- Command ack frames are consumed by correlation and never forwarded as pump events, keeping `apply_event` purely about state.
- `SessionSnapshot` carries `replace`/`seq` with serde defaults so envelope-level assertions and future delta-steering have a home without breaking Go-shape parsing.
- `ensure_session_socket` is a no-op when a handle exists and drops mid-connect outcomes on generation mismatch, covering close/re-resolve races.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocker] Added pinned `rand` to backend-client dependencies**
- **Found during:** Task 1 (GREEN phase)
- **Issue:** `request_id()` needs `rand 0.10.2`, which was workspace-pinned but not yet a dependency of the backend-client crate.
- **Fix:** Added `rand = { workspace = true }` (exact pin, already in `Cargo.lock` via settings/supervisor) plus the one-line lockfile entry. Zero new packages.
- **Files modified:** `desktop-gpui/crates/backend-client/Cargo.toml`, `desktop-gpui/Cargo.lock`
- **Verification:** `cargo test -p webtmux-backend-client` green.
- **Committed in:** `650381d` / `5db79d3`

**2. [Rule 1 - Bug] Fixed WS fixtures to real envelope shapes with all required pane fields**
- **Found during:** Task 1 (GREEN phase)
- **Issue:** First-draft fixtures flattened the envelope (missing nested `snapshot` object) and omitted the required `zoomed` pane field, so parsing failed.
- **Fix:** Rewrote fixtures as true `Outgoing` envelopes (`type` plus `session` tag plus nested `snapshot`) with complete pane objects.
- **Files modified:** `desktop-gpui/crates/backend-client/tests/fixtures/*.json`
- **Verification:** `test_ws_envelope_roundtrip` and `test_snapshot_null_normalization` green.
- **Committed in:** `650381d` / `5db79d3`

**3. [Rule 3 - Blocker] Plan verify commands pass multiple filters; installed cargo accepts one**
- **Found during:** Task 1/3 verification
- **Issue:** `cargo test -p <pkg> <f1> <f2> ...` fails with `unexpected argument` on cargo 1.96; only a single `TESTNAME` positional is accepted.
- **Fix:** Ran each named filter as a separate invocation (all green), plus the full-package and full-workspace suites as the authoritative gate.
- **Verification:** `cargo test -p webtmux-backend-client`, per-filter `cargo test -p webtmux <name>`, and `cargo test --manifest-path desktop-gpui/Cargo.toml --workspace` all green.
- **Committed in:** N/A (verification procedure only)

**4. [Rule 1 - Bug] `SessionWsHandle::subscribe_events` added for the AppState forward pump**
- **Found during:** Task 3
- **Issue:** The forward pump needs a second handle on the inbound queue, but the events receiver had no clone accessor.
- **Fix:** Added a one-method clone accessor on the handle (flume MPMC receiver). Within plan `files_modified` (ws.rs).
- **Files modified:** `desktop-gpui/crates/backend-client/src/ws.rs`
- **Verification:** Full workspace suite green.
- **Committed in:** `4b0f48a`

---

**Total deviations:** 4 auto-fixed (1 bug-fix pair counted once, 2 blockers, 1 procedure adaptation)
**Impact on plan:** None. No scope expansion, no new crates, backend protocol untouched, all acceptance criteria met.

## Issues Encountered

- One transient `sidebar_test` link failure (`x86_64-w64-mingw32-gcc` exit 1) when running `cargo test -p webtmux <filter>` across all targets at once; rerun in isolation passed and the full workspace suite is green. Treated as parallel-linker resource contention, not a code issue.
- Pre-existing `be/test/tmux/service_integration_test.go:147` type error surfaced in LSP diagnostics; out of scope for this plan (Go backend, untouched).

## User Setup Required

None - no external service configuration required. Mock WS servers run in-process; no live backend needed.

## Next Phase Readiness

- The correlated command path (`send_command` plus requestId plumbing) is proven and ready for Plan 03-02 rename/kill dialogs to ride.
- `ensure_session_socket` plus `rename_session_entry` give the rename flow its target-socket resolution and re-resolution primitives.
- Title-bar window tabs (Plan 03-02/03-03) can consume `window_tabs()` directly; chip click maps to fire-and-forget `window.select` over the same correlated channel.

---
*Phase: 03-websocket-multi-session-tabs*
*Completed: 2026-09-06*

## Self-Check: PASSED
- All 7 created files exist on disk.
- All 6 task commits verified in git log.
- All 36 workspace tests pass green across 5 crates (14 new: 8 ws_test plus 4 tabs_test plus 2 ws_guard_test).
