---
phase: "5"
slug: "pane-grid-workspace-operations"
status: draft
shadcn_initialized: false
preset: none
created: "2026-09-07"
---

# Phase 5 — UI Design Contract (minimal)

> Visual and interaction contract for the pane grid, headers, dividers, toolbar, menus, and rename dialogs. Seeded from 05-RESEARCH.md Read-verified FE anchors; full checker pass deferred to execution.
>
> **Parity ground truth** (Read-verified in research, 2026-09-07):
> - `fe/src/lib/geometry.ts:23-146` (`CELL_W = 8`, `CELL_H = 18`, pixelRect, closePaneGaps, pxToCells, resizeDragStep)
> - `fe/src/features/panes/PaneWorkspace.tsx:116-200,261-370` (absolute layout, zoom filter, timers, divider handles)
> - `fe/src/features/panes/PaneResizeHandle.tsx:30-94` (FLIP, 40ms throttle)
> - `fe/src/features/panes/PaneHeader.tsx:41-167` (header contents, split mapping, tooltips)
> - `fe/src/features/panes/PaneContextMenu.tsx:123-206` (9-row menu, swap picker, prefill)
> - `fe/src/features/panes/PaneView.tsx:50-72` (absolute positioning, active border, canZoom)
> - `fe/src/features/windows/WindowToolbar.tsx:19-77` + `LayoutSelector.tsx:32-52` (six presets, h-8 bar)
> - `fe/src/features/windows/WindowTabs.tsx:56-218` + `WindowContextMenu.tsx:34-43` (Plus, tab menu, move ±1)
> - Palette: `default-dark` (`#1e1e1e` bg, `#2d2d2d` active, `#d4d4d4` fg, `#3c3c3c` border, `#808080` muted); font JetBrains Mono; icons lucide `stroke-width="1.5"`.

---

## Design System

| Property | Value |
|----------|-------|
| Component library | gpui-component =0.6.0 (Dialog + InputState for renames; ContextMenuExt + PopupMenuItem for menus) |
| Icon library | lucide inline SVG constants in `icons.rs`; Phase 5 adds SplitSquareHorizontal, SplitSquareVertical, Maximize2, Columns2, Rows2, SquareSplitHorizontal, SquareSplitVertical, LayoutGrid, ArrowRightLeft |
| Font | JetBrains Mono (embedded since Phase 1) |

Hand-rolled rule (locked): geometry-critical surfaces (grid, panes, headers, dividers, toolbar) render with raw GPUI divs using the tokens below; gpui-component behaviors (Dialog/Input/Menu/Tooltip) consumed directly.

---

## Phase Surfaces

### PG1 — Pane Grid (`views/pane_grid.rs`)

- **Container**: workspace div; panes absolutely positioned at `pixelRect` rects after `closePaneGaps`.
- **Zoom**: zoomed window renders only the zoomed pane full-size; dividers hidden.
- **Dividers**: from `divider_layout` — vertical handles `w=4` at `left = a.rect.left + a.rect.width - 2` (cursor col-resize), horizontal `h=4` at `top = ... - 2` (cursor row-resize); direction R/D on the trailing pane.
- **Empty/single-pane**: no dividers, no zoom button.

### PH1 — Pane Header (`views/pane_view.rs`)

- **Container**: h-7 bar: `currentPath` (truncated) + mono pane id (`%N`), TUI-scroll switch (bound to `tui_scroll` map, default ON), 4 icon actions with tooltips: Split right (`horizontal`), Split down (`vertical`), Zoom (hidden when single-pane), Kill (gated).
- **Interactions**: header double-click zooms; `!pane.active` click sends `pane.select`; active pane border highlight.

### TB1 — Window Toolbar (`views/window_toolbar.rs`)

- **Container**: h-8 bar with 5 preset icon buttons + separator + Next; sends `window.layout { paneId: @N, layout }`.
- **Presets**: `even-horizontal`, `even-vertical`, `main-horizontal`, `main-vertical`, `tiled`, `next-layout` (verbatim passthrough).

### PM1 — Pane Context Menu (`views/pane_context_menu.rs`)

9-row FE order verbatim (stable `pane-menu/%N` ids): Split right / Split down / Zoom / Rename / Swap (same-window picker, `currentCommand || title || currentPath`, disabled when empty) / Break pane / separator / Kill (destructive, `confirm_kill_pane` gate). **No Join row** (D8 backend gap — code comment cites it).

### WM1 — Window Tab Menu (`views/tab_strip.rs`)

Plus button (`window.create`, no args) + per-chip menu (stable `window-tab/@N` ids): Rename / Move Left (−1) / Move Right (+1) / Break active / separator / Kill (destructive, `confirm_kill_window` gate).

### DLG2 — Rename Dialogs (`views/rename_pane_dialog.rs`, `views/rename_window_dialog.rs`)

DLG1 clone: Title "Rename Pane" / "Rename Window", Label "New name", prefill pane `title || current_command || ''` / window `name`, autofocus, Enter submits, Rename disabled when empty/busy, inline `#7F1D1D` error line.

---

## Copywriting Contract

| Element | Copy |
|---------|------|
| Toolbar presets | Even Horizontal, Even Vertical, Main Horizontal, Main Vertical, Tiled, Next Layout |
| Rename pane | Title **Rename Pane**, Label **New name** |
| Rename window | Title **Rename Window**, Label **New name** |
| Header tooltips | Split right, Split down, Zoom, Kill (+ path/id text) |
| Swap picker | Pane labels `currentCommand \|\| title \|\| currentPath` |

---

## Interaction Contract

1. Split/zoom/menu/toolbar/tab actions send correlated commands; selects + drag steps fire-and-forget; snapshot is the sole truth (no optimistic flips).
2. Drags: 40ms throttle, `step==0` drop, negative→FLIP+abs, never cumulative, never `terminal.resize`/`hello`.
3. Kill entries branch on their own confirm flag; rename dialogs gate on trim + non-empty.

---

## Checker Sign-Off

- [ ] Copywriting: PASS
- [ ] Visuals: PASS
- [ ] Spacing: PASS
- [ ] Registry Safety: PASS

**Approval:** pending (minimal draft — full pass at execution)
