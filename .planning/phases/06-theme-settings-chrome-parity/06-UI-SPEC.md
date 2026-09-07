---
phase: "6"
slug: "theme-settings-chrome-parity"
status: draft
shadcn_initialized: false
preset: none
created: "2026-09-07"
---

# Phase 6 — UI Design Contract (minimal)

> Chrome-parity contract for the Settings page, command palette, and preset-driven chrome.
> Minimal spec — executors follow 06-RESEARCH.md D3–D5/D8 and the FE sources verbatim.

**Parity ground truth:**
- `fe/src/features/settings/{SettingsPage,UiThemeSettings,TerminalSettings,ThemeCard}.tsx`
- `fe/src/features/palette/CommandPalette.tsx:48-115`
- `fe/src/features/settings/data/ui-themes.ts` + `terminal-themes.ts`
- Web-term reference `views/settings.rs:296-404` (card-grid shape), `app_state.rs:866-881` (live-apply)

---

## Design System

| Property | Value |
|----------|-------|
| Component library | gpui-component =0.6.0 (`command` palette module, `Switch::new(id)`, `InputState`, `Dialog`, `Theme::change`) |
| Icon library | Embedded lucide consts (`icons.rs`, `stroke-width="1.5"`); new: `CHECK_SVG` (card check), `SEARCH_SVG` (palette input), `PAINTBRUSH_SVG` (Appearance header) |
| Font | JetBrains Mono (embedded single family; free-text label reads `"Font family (embedded: JetBrains Mono)"`) |
| Hand-rolled rule | Card grid, filter row, status lines render as raw GPUI divs on preset-helper tokens (no caching — one preset read per render) |

---

## Surfaces

### SETTINGS-1: Appearance section (SET-01, THEME-01/02/03)
- Max-w-672 centered column; header with `PAINTBRUSH_SVG` + `"Appearance"` + FE sub-copy
  (`"The selected theme is applied to every pane and app surface."`).
- Filter row: `"Filter by dark or light appearance"` + all/dark/light toggle (reference
  `show_theme_mode_picker` pattern) + `"{n} themes available"` count.
- Card grid: flex-wrap 3-column `w(px(204))` cards; each card = 56px swatch + 3 dots + 2 bars
  + check overlay when active; click calls `set_theme_preset` (apply-then-save sync).

### SETTINGS-2: Terminal section (SET-02, SET-03)
- tmux-binary input + Check/Enter + status line (`Using {binary} ({version})` / backend error);
  FE copy `"Choose the same tmux installation…"`; no per-keystroke validation.
- Numeric inputs with FE clamps: font 8–32, line-height 1–2 step 0.05, scrollback 100–50000
  step 100; TUI-scroll-default switch; font label honest about single-family.

### SETTINGS-3: Kill switches (SET-04)
- Three `Switch` rows, FE labels verbatim (`"Confirm before killing pane/window/session"`);
  toggle persists via `save()` and gates the next kill flow immediately.

### PALETTE-1: Command palette (DLG-01)
- Modal `Command` dialog on `ctrl-shift-p` from any focus (terminal early-return included);
  `SEARCH_SVG` input; two groups (Actions: 7 FE items in FE order; Open Session: tree names);
  item actions reuse existing sends with FE guards; `Open Session` → `open_session`.

---

## Tokens

All chrome colors resolve through the active `UiThemePreset` row (15 `u32` fields) via
`theme.rs` helpers; terminal palettes resolve through the linked `TerminalThemePreset`
(19 `u32` fields) via `ColorPalette::from_rgb_u32`. Dark/Light bucketing uses the ported
`themeLuminance` rule (`> 0.5`), never eyeballing.

## Assumptions

- A1: `Command` render API fits a modal overlay (Wave-0 read of `command.rs`/`state.rs` confirms; else halt, never hand-roll).
- A2: `"ctrl-shift-p"` parses in gpui-pre 0.3.3 (compile-time check in Wave 1).
- A3: Light-chrome/dark-OS-frame mismatch is an accepted v1.0 deviation (Phase-7 audit).
