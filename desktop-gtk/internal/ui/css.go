package ui

const widgetCSS = `
.tmux-sidebar { border-right: 1px solid @tmux_border; }
.tmux-tabbar { border-bottom: 1px solid @tmux_border; padding: 4px; }
.tmux-toolbar { border-bottom: 1px solid @tmux_border; padding: 3px 8px; }
.pane-frame { border: 1px solid @tmux_border; background: @window_bg_color; }
.pane-active { border-color: @accent_bg_color; }
.pane-header { background: @tmux_muted; padding: 2px 6px; }
.pane-title { font-size: 11px; color: @tmux_muted_fg; }
.statusbar { border-top: 1px solid @tmux_border; padding: 3px 8px; color: @tmux_muted_fg; }
.empty-title { font-size: 20px; font-weight: bold; }
.command-palette { min-width: 560px; min-height: 420px; }
`
