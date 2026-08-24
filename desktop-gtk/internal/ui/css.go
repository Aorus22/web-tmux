package ui

// widgetCSS holds app-specific widget styles layered over (themed) Adwaita
// named colors. It is concatenated after the @define-color overrides built by
// themeCSS. Generic widgets (dropdowns, entries, popovers, boxed lists,
// preference rows) are intentionally NOT restyled here — libadwaita already
// themes them from the named colors, exactly like wa-bot.
const widgetCSS = `
.sidebar-pane, .content-pane {
  --sidebar-bg-color: @window_bg_color;
  --sidebar-fg-color: @window_fg_color;
  --sidebar-shade-color: alpha(@window_fg_color, 0.06);
  --sidebar-border-color: @tmux_border;
  --sidebar-backdrop-color: @window_bg_color;
  background-color: @window_bg_color;
  color: @window_fg_color;
}
.navigation-sidebar { background-color: @window_bg_color; }
.navigation-sidebar > row.activatable:hover { background-color: alpha(@window_fg_color, 0.07); }
.navigation-sidebar > row.activatable:active { background-color: alpha(@window_fg_color, 0.12); }
.navigation-sidebar > row:selected {
  background-color: alpha(@accent_bg_color, 0.30);
  color: @window_fg_color;
}
.tmux-sidebar { border-right: 1px solid @tmux_border; background-color: @window_bg_color; color: @window_fg_color; }
.tmux-tabbar { border-bottom: 1px solid @tmux_border; padding: 4px; }
.tmux-toolbar { border-bottom: 1px solid @tmux_border; padding: 3px 8px; }
.terminal-workspace { background-color: @window_bg_color; color: @window_fg_color; }
/* Pane frames mirror the web PaneView: the same theme border at different
   opacity for active vs inactive — never a bright accent ring (default-dark's
   accent is near-white and reads as a focus flash). */
.pane-frame { border: 1px solid alpha(@tmux_border, 0.30); background: @window_bg_color; }
.pane-frame.pane-active { border-color: alpha(@tmux_border, 0.85); }
/* The terminal surface takes keyboard focus; suppress Adwaita's bright focus
   ring in every state so panes keep only the theme-colored frame border. */
.pane-frame, .pane-terminal,
.pane-frame:focus, .pane-frame:focus-visible, .pane-frame:focus-within,
.pane-terminal:focus, .pane-terminal:focus-visible, .pane-terminal:focus-within {
  outline-width: 0;
  outline-color: transparent;
}
.pane-terminal { background-color: @window_bg_color; }
.pane-header { background: @tmux_muted; padding: 2px 4px; }
.pane-title { font-size: 11px; color: @tmux_muted_fg; }
/* Compact per-pane controls; kill turns destructive on hover like the web. */
.pane-header button { min-width: 20px; min-height: 20px; padding: 1px 2px; }
.pane-header button image { -gtk-icon-transform: scale(0.75); }
.pane-header button.kill-pane:hover { background-color: @destructive_bg_color; color: @destructive_fg_color; }
.pane-divider { background-color: @tmux_border; }
.statusbar { border-top: 1px solid @tmux_border; padding: 3px 8px; color: @tmux_muted_fg; }
.empty-title { font-size: 20px; font-weight: bold; }
.command-palette { min-width: 560px; min-height: 420px; }
button.suggested-action { background-color: @accent_bg_color; color: @accent_fg_color; }
`
