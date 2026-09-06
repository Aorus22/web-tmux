---
phase: 02-rest-client-sidebar-session-management
plan: "01"
subsystem: desktop-gpui
tags: [rust, gpui, rest, reqwest, serde, polling, sidebar, tmux]

# Dependency graph
requires:
  - phase: 01-workspace-foundation-backend-sidecar
    provides: 5-crate desktop workspace, supervisor sidecar lifecycle, and S1 title bar
provides:
  - Typed webtmux-backend-client crate with health, info, tree, create_session, and Go-matching validation
  - Generation-guarded 1.5s background polling pump in AppState
  - SB1 collapsible 3-level sidebar view (session->window->pane) with 240px binary collapse toggle
affects: [02-02-PLAN, 02-03-PLAN, 03-websocket-terminal-io]

actuals:
  tokens: 29800
  tasks: 3
  commits: 5

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Null-safe serde deserialization via deserialize_null_default helper"
    - "Generation / epoch counter guard on async REST requests discarding stale polling ticks"
    - "SB1 3-level tree layout using raw GPUI div composition matching Electron geometry"
    - "Binary sidebar width snap (240px <-> 0px) via PanelLeft title bar toggle"

key-files:
  created:
    - desktop-gpui/crates/backend-client/src/models.rs
    - desktop-gpui/crates/backend-client/src/rest.rs
    - desktop-gpui/crates/backend-client/src/validation.rs
    - desktop-gpui/crates/backend-client/tests/rest_test.rs
    - desktop-gpui/crates/backend-client/tests/validation_test.rs
    - desktop-gpui/crates/backend-client/tests/fixtures/health.json
    - desktop-gpui/crates/backend-client/tests/fixtures/tree_full.json
    - desktop-gpui/crates/backend-client/tests/fixtures/tree_empty.json
    - desktop-gpui/crates/backend-client/tests/fixtures/tree_null_slices.json
    - desktop-gpui/crates/backend-client/tests/fixtures/create_error_duplicate.json
    - desktop-gpui/crates/webtmux/src/views/sidebar.rs
    - desktop-gpui/crates/webtmux/tests/polling_test.rs
    - desktop-gpui/crates/webtmux/tests/sidebar_test.rs
  modified:
    - desktop-gpui/crates/backend-client/src/lib.rs
    - desktop-gpui/crates/webtmux/src/app_state.rs
    - desktop-gpui/crates/webtmux/src/icons.rs
    - desktop-gpui/crates/webtmux/src/views/mod.rs
    - desktop-gpui/crates/webtmux/src/views/tab_strip.rs

key-decisions:
  - "Use custom deserialize_null_default helper with Option::deserialize to map null Go slices into empty Vecs"
  - "Guard asynchronous polling with monotonically increasing poll_generation counter to discard out-of-order ticks"
  - "Render collapsible session tree using CHEVRON_RIGHT_SVG and CHEVRON_DOWN_SVG with 14px size"
  - "Implement binary snap between 240px and 0px width on sidebar toggle button click without layout animation"

patterns-established:
  - "Pattern 1: Typed REST client with error extraction mapping JSON {\"error\": \"...\"} to RestError::Api"
  - "Pattern 2: AppState trigger_poll with weak view handle capture and epoch generation checks"
  - "Pattern 3: SB1 36px header with 20x20 ghost icon buttons (RefreshCw, Plus) and 3-level tree indent"

requirements-completed:
  - SESS-01
  - SESS-06
  - SHELL-03

coverage:
  - id: D1
    description: "webtmux-backend-client parses Go REST payloads, handles null slice normalization, validates session names, and performs health/tree/create queries"
    requirement: SESS-01
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/backend-client/tests/rest_test.rs#test_parse_sessions_tree"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/backend-client/tests/rest_test.rs#test_rest_client_health_and_tree"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/backend-client/tests/rest_test.rs#test_create_session_request"
        status: pass
      - kind: unit
        ref: "desktop-gpui/crates/backend-client/tests/validation_test.rs#test_session_name_validation"
        status: pass
    human_judgment: false
  - id: D2
    description: "AppState maintains generation-guarded 1.5s background polling pump discarding stale ticks, updating tree reactively for CLI-created sessions"
    requirement: SESS-06
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/polling_test.rs#test_polling_generation_guard"
        status: pass
      - kind: integration
        ref: "desktop-gpui/crates/webtmux/tests/polling_test.rs#test_cli_session_polling_integration"
        status: pass
    human_judgment: false
  - id: D3
    description: "Title bar toggle button snaps sidebar binary width between 240px and 0px, and SB1 renders collapsible 3-level session tree"
    requirement: SHELL-03
    verification:
      - kind: unit
        ref: "desktop-gpui/crates/webtmux/tests/sidebar_test.rs#test_sidebar_toggle_snap"
        status: pass
      - kind: other
        ref: "cargo build --manifest-path desktop-gpui/Cargo.toml --package webtmux"
        status: pass
    human_judgment: false

duration: 18min
completed: 2026-09-06
status: complete
---

# Phase 02 Plan 01: REST Client, Polling Pump & Collapsible Sidebar Summary

**Typed `webtmux-backend-client` crate with REST DTOs and Go-matching validation, generation-guarded 1.5s background polling in `AppState`, and SB1 collapsible 3-level sidebar view (240px width with binary collapse toggle).**

## Performance

- **Duration:** 18 min
- **Started:** 2026-09-06T12:35:43Z
- **Completed:** 2026-09-06T12:50:40Z
- **Tasks:** 3
- **Files modified:** 18

## Accomplishments

- Implemented `webtmux-backend-client` with `HealthResponse`, `TmuxInfo`, `TmuxTree`, `SessionTreeNode`, `WindowTreeNode`, `TmuxSession`, `TmuxWindow`, `TmuxPane`, `CreateSessionRequest`, `CreateSessionResponse`, and `ApiErrorResponse`.
- Added custom null deserializer `deserialize_null_default` ensuring Go `null` slices deserialize cleanly to empty `Vec`s.
- Implemented `validate_session_name` strictly mirroring Go `ValidateSessionName` (rejecting empty, >200 chars, colons, dots, leading `$`, and non-alphanumeric leading characters).
- Implemented `RestClient` with `health()`, `info()`, `tree()`, and `create_session()`, parsing backend error messages on non-2xx status codes.
- Added 5 JSON fixtures and in-process mock HTTP tests using `tokio::net::TcpListener`.
- Wired `poll_generation` generation guard and 1500ms `tokio::time::interval` polling loop into `AppState`.
- Added embedded lucide SVG constants: `PLUS_SVG`, `REFRESH_CW_SVG`, `CHEVRON_RIGHT_SVG`, `CHEVRON_DOWN_SVG`, `FOLDER_OPEN_SVG`, `ALERT_TRIANGLE_SVG`, `SQUARE_TERMINAL_SVG`, `PANEL_LEFT_SVG`, `SETTINGS_SVG`.
- Implemented `views/sidebar.rs` (SB1) matching 240px container, 36px header row with Refresh and Plus buttons, scrollable 3-level tree (session -> window -> pane) with count badge and chevron rotation, and Settings footer button.
- Wired sidebar toggle button (`PANEL_LEFT_SVG`) into title bar, providing binary snap between 240px and 0px width (`SHELL-03`).

## Task Commits

Each task was committed atomically following TDD and conventional commit guidelines:

1. **Task 1 (TDD RED):** `b46df7a` (test(02-01): add failing tests for backend-client models, validation, and REST client)
2. **Task 1 (TDD GREEN):** `3e3f660` (feat(02-01): implement typed models, validation, and REST client in webtmux-backend-client)
3. **Task 2 (TDD RED):** `287d7d9` (test(02-01): add failing tests for polling generation guard and sidebar toggle)
4. **Task 2 (TDD GREEN):** `fd6afb1` (feat(02-01): wire generation-guarded polling pump and sidebar toggle in AppState)
5. **Task 3 (Tracer Slice):** `de598a6` (feat(02-01): implement SB1 collapsible sidebar view and wire end-to-end rendering)

## Files Created/Modified

- `desktop-gpui/crates/backend-client/src/models.rs` - DTO structs with serde annotations and null deserialization helper
- `desktop-gpui/crates/backend-client/src/rest.rs` - RestClient implementation with error extraction
- `desktop-gpui/crates/backend-client/src/validation.rs` - Session name validation matching Go ValidateSessionName
- `desktop-gpui/crates/backend-client/src/lib.rs` - Public exports for backend-client crate
- `desktop-gpui/crates/backend-client/tests/rest_test.rs` - Unit and mock HTTP tests for REST endpoints
- `desktop-gpui/crates/backend-client/tests/validation_test.rs` - Unit tests for session name validation rules
- `desktop-gpui/crates/backend-client/tests/fixtures/*.json` - 5 JSON fixtures matching Go backend responses
- `desktop-gpui/crates/webtmux/src/app_state.rs` - AppState with polling loop, trigger_poll, and generation guard
- `desktop-gpui/crates/webtmux/src/icons.rs` - Phase 2 Lucide vector icons
- `desktop-gpui/crates/webtmux/src/views/sidebar.rs` - SB1 collapsible sidebar view
- `desktop-gpui/crates/webtmux/src/views/tab_strip.rs` - Title bar with sidebar toggle button
- `desktop-gpui/crates/webtmux/src/views/mod.rs` - Registered sidebar module
- `desktop-gpui/crates/webtmux/tests/polling_test.rs` - Tests for generation guard and polling integration
- `desktop-gpui/crates/webtmux/tests/sidebar_test.rs` - Tests for binary sidebar toggle snap

## Decisions Made

- Used `deserialize_null_default` helper for serde fields to avoid panic on Go `null` slices.
- Implemented `CHEVRON_DOWN_SVG` and `CHEVRON_RIGHT_SVG` for tree expander icons.
- Applied generation guard pattern checking `poll_generation == expected_gen` before committing tree updates in async callbacks.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added deserialize_null_default for Go null slice normalization**
- **Found during:** Task 1 (GREEN phase)
- **Issue:** Standard `#[serde(default)]` on `Vec<T>` panics with `invalid type: null, expected a sequence` when Go serializes `nil` slices as `null`.
- **Fix:** Implemented `deserialize_null_default` using `Option::deserialize` and `unwrap_or_default()`.
- **Files modified:** `desktop-gpui/crates/backend-client/src/models.rs`
- **Verification:** `test_parse_sessions_tree` passes for `tree_null_slices.json`.
- **Committed in:** `3e3f660`

**2. [Rule 3 - Blocker] Fixed GPUI async closure arguments in Context::spawn**
- **Found during:** Task 2 (GREEN phase)
- **Issue:** `cx.spawn` in `gpui-pre` takes a closure with `(WeakEntity<T>, &mut AsyncApp)` signature.
- **Fix:** Adjusted spawn closures in `app_state.rs` to receive `(view_weak, cx)` and added `move`.
- **Files modified:** `desktop-gpui/crates/webtmux/src/app_state.rs`
- **Verification:** `cargo test -p webtmux` compiles and passes.
- **Committed in:** `fd6afb1`

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocker)
**Impact on plan:** None. Fixes were required for correct deserialization and compilation against pinned GPUI bindings.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The typed REST client, polling pump, and sidebar rendering provide the live data backbone.
- Ready for Plan 02-02 (Create Session Dialog modal with native directory picker and input validation).

---
*Phase: 02-rest-client-sidebar-session-management*
*Completed: 2026-09-06*

## Self-Check: PASSED
- All 13 created files exist on disk.
- All 5 task commits verified in git log.
- All 20 workspace tests pass green across 5 crates.
