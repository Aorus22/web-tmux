package ui

const widgetCSS = `
/* Theme the panes and shell directly as well as through Adwaita named colors.
   This is the same overlay approach used by wa-bot: every app surface keeps
   the active preset instead of leaving a default-gray libadwaita background. */
.sidebar-pane, .content-pane {
  --sidebar-bg-color: @window_bg_color;
  --sidebar-fg-color: @window_fg_color;
  --sidebar-shade-color: alpha(@window_fg_color, 0.06);
  --sidebar-border-color: @tmux_border;
  --sidebar-backdrop-color: @window_bg_color;
  background-color: @window_bg_color;
  color: @window_fg_color;
}
.background, window, .view, headerbar, .titlebar {
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
.settings-page, preferencespage, preferencesgroup { background-color: @window_bg_color; color: @window_fg_color; }
.tmux-tabbar { border-bottom: 1px solid @tmux_border; padding: 4px; }
.tmux-toolbar { border-bottom: 1px solid @tmux_border; padding: 3px 8px; }
.terminal-workspace { background-color: @window_bg_color; color: @window_fg_color; }
.pane-frame { border: 1px solid @tmux_border; background: @window_bg_color; }
.pane-active { border-color: @accent_bg_color; }
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
dialog, popover, menu, .card, .boxed-list { background-color: @card_bg_color; color: @card_fg_color; }
/* No explicit "dropdown" rule: the DropDown button and its popover popup are
   themed through the named colors above. Painting the outer node directly
   double-paints behind the internal toggle button and shows a stray edge. */
entry, spinbutton, scale trough { background-color: @view_bg_color; color: @view_fg_color; }
button.suggested-action { background-color: @accent_bg_color; color: @accent_fg_color; }
`
