---
phase: 01-workspace-foundation-backend-sidecar
plan: "03"
subsystem: infra
tags: [fxc, shader-compiler, windows-release, building-docs, gitignore]

requires:
  - phase: 01-01
    provides: workspace foundation and backend supervisor
  - phase: 01-02
    provides: settings store and window controls
provides:
  - Standalone zero-dependency Rust HLSL shader compiler (desktop-gpui/tools/fxc) emulating fxc.exe via d3dcompiler_47.dll
  - Verified Windows release build capability without requiring full Windows 10/11 SDK
  - Comprehensive building documentation for Windows and Linux in desktop-gpui/docs/BUILDING.md
  - Git ignore rules properly excluding desktop-gpui/target/, tools/fxc/*.exe, and test fixtures
affects: [packaging, release-pipeline, ci, documentation]

actuals:
  tokens: 5800
  tasks: 2
  commits: 2

tech-stack:
  added: [desktop-gpui/tools/fxc]
  patterns: [zero-dependency Win32 DLL loading for HLSL shader compilation, GPUI_FXC_PATH environment hook]

key-files:
  created:
    - desktop-gpui/tools/fxc/main.rs
    - desktop-gpui/tools/fxc/README.md
    - desktop-gpui/docs/BUILDING.md
  modified:
    - .gitignore

key-decisions:
  - "Port zero-dependency tools/fxc from web-term to wrap system d3dcompiler_47.dll rather than requiring the multi-gigabyte Windows SDK for release shader builds"
  - "Support both Windows debug/release (with GPUI_FXC_PATH) and documented Linux apt-get prerequisite environments"

patterns-established:
  - "Shader compilation hook: GPUI_FXC_PATH pointing to standalone fxc.exe enables cargo build --release on Windows"

requirements-completed: [PKG-01]

coverage:
  - id: D1
    description: "Standalone zero-dependency fxc.exe compiles cleanly and enables cargo build --release for webtmux"
    requirement: "PKG-01"
    verification:
      - kind: other
        ref: "cargo build --release --manifest-path desktop-gpui/Cargo.toml --package webtmux"
        status: pass
    human_judgment: false
  - id: D2
    description: "Comprehensive BUILDING.md documentation covering Windows and Linux build prerequisites and commands"
    requirement: "PKG-01"
    verification:
      - kind: other
        ref: "powershell -Command if (Test-Path desktop-gpui/docs/BUILDING.md) { Get-Content desktop-gpui/docs/BUILDING.md | Select-String -Pattern 'libfontconfig1-dev', 'GPUI_FXC_PATH' }"
        status: pass
    human_judgment: false

duration: 10min
completed: 2026-09-06
status: complete
---

# Phase 01 Plan 03: Tools FXC Port, Building Documentation, and Git Hygiene Summary

**Ported standalone zero-dependency fxc.exe shader compiler enabling Windows release builds, documented Windows and Linux build prerequisites in BUILDING.md, and validated workspace build/test pipelines.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-09-06T11:32:38Z
- **Completed:** 2026-09-06T11:42:38Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- Verified and finalized the standalone, zero-dependency `desktop-gpui/tools/fxc` tool (`main.rs` + `README.md`) which dynamically loads `d3dcompiler_47.dll` to satisfy `gpui-pre-windows` release shader compilation.
- Successfully verified `cargo build --release --manifest-path desktop-gpui/Cargo.toml --package webtmux` under `GPUI_FXC_PATH`, creating an optimized release binary in `desktop-gpui/target/release/webtmux.exe`.
- Created comprehensive `desktop-gpui/docs/BUILDING.md` detailing prerequisites and build instructions for Windows (debug & release with fxc) and Linux (`apt-get` packages including `libfontconfig1-dev`, `libwayland-dev`, `libxkbcommon-dev`, Vulkan loader, tmux).
- Configured `.gitignore` to prevent committing build artifacts (`desktop-gpui/target/`, `desktop-gpui/tools/fxc/*.exe`, `desktop-gpui/test-support/`, `*.bak`).
- Executed full workspace test battery with all 14 unit and integration tests passing.

## Task Commits

Each task was committed atomically:

1. **Task 1: Port tools/fxc HLSL shader compiler and verify Windows release build** - `cfc5335` (build)
2. **Task 2: Author BUILDING.md documentation for Windows and Linux environments** - `85063f1` (docs)

## Files Created/Modified
- `desktop-gpui/tools/fxc/main.rs` - Standalone Rust tool wrapping `d3dcompiler_47.dll` to compile HLSL shaders
- `desktop-gpui/tools/fxc/README.md` - Documentation and compilation instructions for `fxc.exe`
- `desktop-gpui/docs/BUILDING.md` - Complete build instructions and prerequisites for Windows and Linux
- `.gitignore` - Ignored `desktop-gpui/target/`, `tools/fxc/*.exe`, and test support artifacts

## Decisions Made
- Recovered draft state from previous executor run and verified that `desktop-gpui/tools/fxc/main.rs` cleanly matches the reference implementation from `web-term`.
- Compiled `fxc.exe` via `rustc -O` and verified release compilation of `webtmux` succeeds in ~3m27s.

## Deviations from Plan

### Auto-fixed Issues
None - plan executed and completed as specified.

---

**Total deviations:** 0
**Impact on plan:** None.

## Issues Encountered
- Continuation / Recovery mode: An initial uncommitted draft was present in the working tree. Inspected, verified against acceptance criteria, compiled with `rustc`, executed the release build with `GPUI_FXC_PATH`, and committed atomically according to GSD execution rules.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 01 (Workspace Foundation & Backend Sidecar) is fully complete across all 3 waves.
- Next phase (Phase 02 / REST API client) can proceed with the established GPUI workspace foundation, supervisor lifecycle, settings persistence, and build tooling.

## Self-Check: PASSED
- `desktop-gpui/tools/fxc/main.rs`: FOUND
- `desktop-gpui/tools/fxc/README.md`: FOUND
- `desktop-gpui/docs/BUILDING.md`: FOUND
- Commit `cfc5335`: FOUND
- Commit `85063f1`: FOUND
- Workspace test battery: 14/14 tests PASSED
