---
phase: "3"
slug: "websocket-multi-session-tabs"
status: draft
created: "2026-09-06"
---

# Phase 3 — UI Design Contract (minimal)

> Lean chrome contract for the title-bar window tabs, session context menu, rename/kill
> dialogs, and snapshot placeholder panel. Rename/kill dialogs reuse DLG1 tokens verbatim
> (03-CONTEXT D1) — no new dialog spec. Extends 02-UI-SPEC tokens; parity ground truth is
> `fe/src/components/layout/AppTitleBar.tsx`, `fe/src/features/windows/WindowTabs.tsx`,
> `fe/src/features/sessions/SessionContextMenu.tsx`.

---

## Title Bar: WindowTabs + Gear (SHELL-01)

Bar shell unchanged (S1: `h(px(44.0))`, bg `#1e1e1e`, 1px `#3c3c3c` bottom border, px-3).
Order: sidebar toggle + TerminalSquare + "Tmux GUI" → divider (`ml-1 h-5 w-px border-l`,
`#3c3c3c`) → **WindowTabs flex-1 middle (ONLY when `active_session` set)** → Settings gear
(`Settings` lucide, ghost `icon-sm`, `#808080` idle / `#d4d4d4` hover on `#2d2d2d`) →
window controls. Interactive children stay out of the drag area (existing center filler kept).

| Token | Value | Source |
|-------|-------|--------|
| Chip height / max width | `h-7` (28px) / `max-w-44` (176px) | WindowTabs.tsx |
| Chip padding / radius | `px-2.5` (10px) / `rounded-md` (6px) | WindowTabs.tsx |
| Chip label | `{index}: {name}`, 14px/400, truncated | WindowTabs.tsx |
| Chip active | bg `#2d2d2d`, text `#d4d4d4` | 02-UI-SPEC derived blends |
| Chip idle / hover | text `#808080` / bg `#262626` | 02-UI-SPEC derived blends |
| Container | flex-1 horizontal scroll, no-scrollbar | WindowTabs.tsx |
| Plus new-window button | OMITTED (D8 — Phase 5) | 03-CONTEXT |

Chip click = fire-and-forget `window.select` (no await; state follows via delta).
Empty-windows snapshot → empty middle, no crash.

## Session Context Menu (sidebar rows)

Anchor: sidebar session rows wrapped in `ContextMenuExt::context_menu` with stable
`.id(("session-row", name))` (poll-tick state-loss pitfall). Items: `Rename` + separator +
`Kill Session` (destructive styling) only — no future-item dead entries. Left-click select
stays `MouseButton::Left`-only.

## Rename Dialog (D1 — DLG1 reuse, no new tokens)

440px, `#1e1e1e`, 1px `#3c3c3c`, radius 8, padding 20, dark overlay. Title "Rename Session",
label "New name", single `InputState` prefilled with current name, autofocus, Enter-submits,
`Rename` primary (`#d4d4d4` fill) disabled while empty/trim-empty or busy, Cancel outline,
inline `#7F1D1D` error line on validation/`command.error`/timeout.

## Kill Confirm Dialog (D2 — DLG1 tokens, destructive Kill)

Same DLG1 chrome. Title `Kill session "X"?`, body "This terminates the tmux session and
all processes inside it.", footer Cancel outline + Kill destructive (`#7F1D1D` fill, white
text — matches S1 close-button hover). Gated by `DesktopSettings.confirm_kill_session`.

## Snapshot Placeholder Panel (tracer, Phase-3-only)

Workspace body for an open session before Phase 4 terminals: session name (16px/500
`#d4d4d4`) + window list (`{index}: {name}` rows, 12px `#aaaaaa`) from the live snapshot,
muted note "Terminal view arrives in Phase 4" (12px `#808080`). Real WS data — proves the
path; replaced by terminal rendering in Phase 4, not polished further.

## Settings Placeholder Page (gear target, Phase-6-owned)

Centered muted copy: "Settings arrive in Phase 6" + sub-copy noting tabs stay connected
underneath. Static text only — no settings values displayed. Sidebar footer Settings entry
routes here too.
