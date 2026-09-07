# Phase 7: Resilience, Packaging & Parity Audit - Context

**Gathered:** 2026-09-07
**Status:** Ready for planning
**Mode:** Auto-accepted (autonomous lane — user sleeping; D1–D7 from 07-RESEARCH.md treated as locked)

<domain>
## Phase Boundary

Resilience UX, distribution packaging, and the milestone-closing parity audit inside the
Phase-1–6 shell — no new mechanisms, only surfaces and scripts. Three workstreams:

1. **STATE-03 resilience surfaces** — split the conflated disconnect signal at the
   `backend-client` pump boundary (new local `transport.lost` ≠ server `tmux.disconnected`),
   render the reconnect banner + bounded toast overlay with the strict 4-way taxonomy,
   arm FE-verbatim BACKOFF auto-reconnect (generation-guarded), surface command errors as
   toasts. No backend changes (milestone rule).
2. **PKG-02/PKG-03 packaging + Makefile** — port web-term's 3-step bundle scripts verbatim
   (`package-gpui-windows.ps1`, `package-gpui-linux.sh` → `dist/tmux-gui-{windows,linux}-x64/`
   with adjacent Go sidecar), wire `desktop-gpui` / `dev-gpui` / `package-gpui-*` Makefile
   targets in the existing HOST_OS-dispatch pattern. No installers (MSI/deb/AppImage).
3. **SC4 parity audit** — side-by-side vs Electron over the 5 daily-driver scenarios plus ALL
   folded deferred-HV advisories (Phases 1/2/3/4/5/6); audit-gated D7 chrome-threading fixes
   only where the audit shows visible divergence under default-dark + 2 non-default themes.

Delivers STATE-03, PKG-02, PKG-03. Out: EXTRA-01/02/03 (v2), macOS, backend protocol
changes, installers, mobile changes.
</domain>

<decisions>
## Implementation Decisions (locked — 07-RESEARCH.md § Auto-Accepted, D1–D7)

- **D1 — Toasts = hand-rolled bounded overlay queue in `AppState` render** (`VecDeque`,
  cap 5, dedupe-by-key, autohide 5s errors / 3s success [A1-tunable]): NOT gpui-component
  `Root`/`push_notification` adoption (re-plumbs the render tree mid-milestone). Matches the
  hand-rolled-divs convention and the web-term `notification: Option<String>` precedent;
  headless-testable as a pure queue model.
- **D2 — Strict 4-way taxonomy with fixed copy:** (a) WS-drop → `"Connection lost —
  retrying… (attempt N)"` + auto-retry; (b) `tmux.reconnecting` → `"Reconnecting to
  tmux…"` (amber, transient); (c) `tmux.disconnected` → `"tmux disconnected — waiting for
  tmux"` + manual Reconnect; (d) `command.error`/timeout/`server.error` → error toast with
  backend-verbatim message. Copy mirrors FE `PaneWorkspace.tsx:217-250`, tmux- prefixed for
  server-originated states. FIRST task: the pump split — all copy keys off it.
- **D3 — Auto-reconnect backoff ports FE `BACKOFF = [250,500,1000,2000,5000,10_000]ms`
  verbatim** (`websocket.ts:27`), guarded by the existing per-tab generation counter
  (stale timers die on rename/close/re-resolve). Transport-lost ONLY — never for
  server-sent `tmux.disconnected` (the backend monitor owns that retry).
- **D4 — Dist layout `dist/tmux-gui-windows-x64/`** (`webtmux.exe` + `tmux-gui-server.exe`
  + README) **and `dist/tmux-gui-linux-x64/`** (`webtmux` + `tmux-gui-server` + README),
  plain zip/tarball; NO installer. Verbatim web-term 3-step port (`backend → release →
  assemble`); `bundle.rs` already resolves exactly this layout; font is `include_bytes!`
  (no asset-copy step).
- **D5 — Makefile target names:** `desktop-gpui` (release build), `dev-gpui` (cargo run),
  `package-gpui-windows`, `package-gpui-linux`, wired into `help`. Existing
  `desktop-gtk`/`package-gtk-windows` + HOST_OS/EXE_EXT pattern; Windows recipes delegate
  to `powershell -NoProfile -ExecutionPolicy Bypass -File ...` (never inline multi-line
  shell — Pitfall 6, the `E:\dev\null` lesson). Linux packaging documents "run on Linux".
- **D6 — Parity audit output = checklist table** (pass/fail + screenshot ref + deviation
  link) covering the 5 daily-driver scenarios × all folded HV advisories; single allowed
  pre-existing deviation = sidebar binary snap. Other historical deviations recorded once
  as accepted context, never re-argued.
- **D7 — Phase 6 deferred chrome-threading is audit-gated fix work:** audit first under
  default-dark + 2 non-default themes (one light, one saturated dark); thread preset tokens
  only where the audit shows visible divergence, using the established per-render preset
  pattern (55 reads precedent). NO blind re-theme of all files.
</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets (Phases 1–6, all verified in RESEARCH)
- `crates/backend-client/src/ws.rs` — `TransportState`, pump close/error arms
  (:437-443,485-491 — the conflation site), `connect_session_with_pending` for retries
- `crates/webtmux/src/app_state.rs` — `apply_event` (:925-936 transport mapping, :865-869
  absent-tag guard), `ensure_session_socket` Err arm (:3288-3294, no-retry today),
  `note_session_error` paths (:2563-2785, toast source), overlay stacking precedent
  (:3379-3381), `TOKIO_RT` spawn pattern (:3205-3215)
- `crates/webtmux/src/bundle.rs` — `resolve_backend_path` + pure
  `resolve_backend_path_with_current_exe` (headless-testable packaging contract)
- `crates/webtmux/src/views/window_toolbar.rs:71-75,163-164` — inline `last_error`
  surface (stays; toasts are additive)
- FE truth: `BACKOFF` (`websocket.ts:27`), banner copy (`PaneWorkspace.tsx:217-250`),
  10s command timeout (`commands.ts:17-19`), 13 `toast.error` call sites
- Makefile HOST_OS dispatch + gtk powershell-delegate pattern (:12-43, :80-100, :108-114)

### Established Patterns
- EXACT-pinned deps + committed Cargo.lock; every plan asserts empty
  `git diff Cargo.toml Cargo.lock` (zero-new-package gate — RESEARCH § Package Legitimacy)
- Generation-guard discipline (STATE-04) reused for retry timers
- GPUI pixels are human-UAT (Phases 2/4/5 precedent); headless tests guard models only
- Build env: run cargo from repo root with `--manifest-path desktop-gpui/Cargo.toml`,
  `CARGO_TARGET_DIR=C:\cargo-target\web-tmux`, `CARGO_BUILD_JOBS=2` (06-VERIFICATION §11)
- Toolchain: cargo 1.96 (gnu sysroot — scripts must NOT assume MSVC `cl.exe`), go 1.23.4,
  MSYS2 tmux present. NO Linux host on this box → Linux build/smoke are human_needed.

### Integration Points
- Backend emits all four events already (`protocol.go:86-97`, `hub.go:100-121`); monitor
  retry ladder matches FE (`monitor.go:332-341`) — client-side split only, no server work
- Web-term reference scripts: `E:\Coding Stuff\web-term\desktop-gpui\scripts\package-windows.ps1`
  + `package-linux.sh` (3-step port source, RESEARCH-read)
</code_context>

<specifics>
## Specific Ideas

- The pump-split grep-gate is the phase's load-bearing review check: after the split, zero
  `EV_TMUX_DISCONNECTED` references may remain in the pump close/error arms (Pitfall 1).
- Banner copy must NEVER render directly from `TransportState` without the origin bit —
  any plan doing so has reintroduced the conflation.
- Toast timings (5s/3s) are UAT-tunable guesses (A1); the audit may adjust them.
- `server.error` → error toast (cheap, reversible; FE only console.errors it — Open Q2 resolved).
- Reconnect retries forever on the 10s tail like FE + always offers manual Reconnect with
  attempt count (Open Q3 resolved: parity by construction).
</specifics>

<deferred>
## Specific Ideas / Deferred Ideas

- EXTRA-01 session-tab persistence, EXTRA-02 keyboard shortcuts, EXTRA-03 Linux
  window-control polish (all v2, STATE.md Deferred Items — NOT in any plan)
- macOS support, backend protocol changes, installers (MSI/deb/AppImage), mobile changes
- gpui-component `Root` notification adoption (revisit post-v1.0, D1 alternative)
- Linux cross-build from Windows via `--target` + zig/musl (OUT unless user asks — Pitfall 7)
- Bundled `.so` closure for Linux (escalate only if clean-distro smoke fails — A5)
</deferred>
