---
status: testing
phase: 01-workspace-foundation-backend-sidecar
source: [01-VERIFICATION.md]
started: 2026-09-06T18:55:00
updated: 2026-09-06T18:55:00
---

## Current Test

number: 1
name: Startup flow visually
expected: |
  Launch desktop-gpui/target/debug/webtmux.exe. Window opens at 1200x800 (or restored geometry).
  S2 shows spinner + "Starting backend…" for at most ~10s while the sidecar spawns, then Ready clears
  the body. On backend failure: "Backend Startup Failed" + Reason: + redacted stderr tail;
  Retry re-runs the full handshake; Quit exits the app cleanly.
awaiting: user response

## Tests

### 1. Startup flow visually
expected: window opens 1200x800 (or restored); S2 spinner + "Starting backend…" ≤10s → Ready bare shell; failure path shows S3 with Retry/Quit working
result: [pending]

### 2. Window controls
expected: minimize; maximize↔restore (Square⇄Copy icon swap by IsZoomed); close exits cleanly with sidecar teardown; drag region moves window; double-click toggles maximize
result: [pending]

### 3. Persistence + recovery
expected: move+resize then relaunch restores geometry; hand-corrupted settings.json (config dir tmux-gui-desktop) launches with defaults and creates settings.json.bak
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
