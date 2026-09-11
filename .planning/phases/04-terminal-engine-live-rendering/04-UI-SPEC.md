---
phase: "4"
slug: "terminal-engine-live-rendering"
status: draft
created: "2026-09-06"
---

# Phase 4 — UI Design Contract (minimal)

> Visual and interaction contract for live pane terminals. Phase 4 renders
> terminals inside the existing Phase-3 workspace body — no grid geometry
> (Phase 5), no theme page (Phase 6). Tokens below are the FE defaults the
> plans implement (D1/D2); chrome around panes is unchanged.
>
> **Parity ground truth** (read 2026-09-06 in 04-RESEARCH):
> - FE: `fe/src/features/terminal/useTerminal.ts` (xterm opts, wheel, keys,
>   capture), `terminalRegistry.ts` (replay), `PaneWorkspace.tsx:131-200`
>   (debounce + layout-key), `settingsStore.ts:33-45` (defaults), `App.tsx`
>   (hidden workspaces).
> - GPUI reference: `web-term/desktop-gpui/crates/terminal/src/view.rs`
>   (canvas + handlers), `render.rs` (paint order), `mouse.rs` (geometry).

---

## Terminal Grid Tokens

| Token | Value | Source |
|-------|-------|--------|
| Font family | JetBrains Mono (embedded Phase 1) | FE settings + reference view |
| Font size | 14px | FE `settingsStore.ts:38` (D2) |
| Line height | 1.35 | FE `settingsStore.ts:39` (D2, wins over reference 1.2) |
| Cell measure | "M"-advance per paint | reference `measure_cell` |
| Palette | Fixed dark default (reference `dark_default`) | D2; 78 presets arrive Phase 6 |
| Scrollback | 2000 lines | FE default (D1; reference 10_000 NOT used) |
| Paint order | bg quads → selection quads → text runs; whitespace-run skip; skip WIDE_CHAR_SPACER; INVERSE/DIM/HIDDEN handling | reference `render.rs:paint` verbatim |
| Grid sizing | cols/rows = floor(avail/cell), clamped ≥ 1; local `term.resize` immediate on change | reference canvas shape |
| Cursor | Block cursor at the alacritty grid cursor; any keypress scrolls to bottom | FE `useTerminal` + reference view |

---

## Selection / Clipboard

| Behavior | Binding | Notes |
|----------|---------|-------|
| Drag select | Mouse left-drag (pixel→cell geometry; click-count: char/word/line) | ported `mouse.rs` geometry |
| Copy | Ctrl+Shift+C, Cmd+C (with non-empty selection) | `selection_to_string` → system clipboard |
| Interrupt | Ctrl+C with EMPTY selection → passes through to pane | FE parity, test-locked |
| Paste | Ctrl+Shift+V, Cmd+V | clipboard → one `terminal.input` message, no client chunking |
| Mouse apps | Click/drag/scroll SGR passthrough when app enabled mouse reporting | verbatim `mouse_button_report` |

---

## Resize Behavior (user-visible)

- Window drags never visibly disturb other frontends: exactly one
  `terminal.resize` fires ~100ms after the drag settles (FE debounce parity).
- Layout changes (window switch, pane set change) resettle in two steps:
  quick resize (~150ms) then scrollback resync (~325ms); first mount skips
  the pair (initial capture already covers it).
- Hidden session tabs show no resize activity of their own; their grids are
  current when reshown (ingest continues off-screen).
- There is no handshake spinner or dimension flash on tab open: the first
  paint shows replayed history as soon as the capture reply lands.

---

## Non-Goals (explicitly NOT in this spec)

- Pane dividers, split/zoom controls, layout presets, pane headers,
  context menus — Phase 5.
- Theme switching, font/scrollback preference UI — Phase 6 (constants only).
- Reconnect banners/toasts — Phase 7 (silent re-capture only).
