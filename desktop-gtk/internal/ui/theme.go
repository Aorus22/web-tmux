package ui

import (
	"fmt"
	"math"
	"strings"

	"github.com/diamondburned/gotk4-adwaita/pkg/adw"
	"github.com/diamondburned/gotk4/pkg/gdk/v4"
	"github.com/diamondburned/gotk4/pkg/gtk/v4"
	"tmux-gui/desktop-gtk/internal/terminal"
)

var (
	currentTheme    *ThemePreset
	currentProvider *gtk.CSSProvider
)

func FindTheme(name string) *ThemePreset {
	for i := range Themes {
		if Themes[i].Name == name {
			return &Themes[i]
		}
	}
	return &Themes[0]
}

func ActiveTheme() *ThemePreset {
	if currentTheme == nil {
		return FindTheme(DefaultThemeName)
	}
	return currentTheme
}
func findTerminalTheme(name string) *TerminalThemePreset {
	for i := range TerminalThemes {
		if TerminalThemes[i].Name == name {
			return &TerminalThemes[i]
		}
	}
	return nil
}

func ApplyTheme(name string) *ThemePreset {
	t := FindTheme(name)
	display := gdk.DisplayGetDefault()
	if display == nil {
		return t
	}
	if currentProvider != nil {
		gtk.StyleContextRemoveProviderForDisplay(display, currentProvider)
		currentProvider = nil
	}
	provider := gtk.NewCSSProvider()
	provider.LoadFromData(themeCSS(t))
	gtk.StyleContextAddProviderForDisplay(display, provider, gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
	currentProvider = provider
	currentTheme = t
	if manager := adw.StyleManagerGetDefault(); manager != nil {
		if luminance(t.Colors.Background) < .5 {
			manager.SetColorScheme(adw.ColorSchemeForceDark)
		} else {
			manager.SetColorScheme(adw.ColorSchemeForceLight)
		}
	}
	return t
}

func PaletteForTheme(theme *ThemePreset) terminal.Palette {
	p := terminal.DefaultPalette()
	tt := findTerminalTheme(theme.TerminalTheme)
	if tt == nil {
		// Keep the terminal in lockstep with the app even if a generated theme
		// has no dedicated ANSI preset.
		p.Foreground = terminal.ParseHex(theme.Colors.Foreground)
		p.Background = terminal.ParseHex(theme.Colors.Background)
		p.Cursor = terminal.ParseHex(theme.Colors.Primary)
		return p
	}
	c := tt.Colors
	p.Foreground = terminal.ParseHex(c.Foreground)
	p.Background = terminal.ParseHex(c.Background)
	p.Cursor = terminal.ParseHex(c.Cursor)
	values := []string{c.Black, c.Red, c.Green, c.Yellow, c.Blue, c.Magenta, c.Cyan, c.White, c.BrightBlack, c.BrightRed, c.BrightGreen, c.BrightYellow, c.BrightBlue, c.BrightMagenta, c.BrightCyan, c.BrightWhite}
	for i, v := range values {
		p.ANSI[i] = terminal.ParseHex(v)
	}
	return p
}

func themeCSS(t *ThemePreset) string {
	c := &t.Colors
	var b strings.Builder
	define := func(name, value string) { fmt.Fprintf(&b, "@define-color %s %s;\n", name, value) }
	// Match libadwaita's full named-color surface, not only the terminal pane.
	define("window_bg_color", c.Background)
	define("window_fg_color", c.Foreground)
	define("view_bg_color", c.Background)
	define("view_fg_color", c.Foreground)
	define("dialog_bg_color", c.Card)
	define("dialog_fg_color", c.Foreground)
	define("sidebar_bg_color", c.Background)
	define("sidebar_fg_color", c.Foreground)
	define("headerbar_bg_color", c.Background)
	define("headerbar_fg_color", c.Foreground)
	define("headerbar_backdrop_bg_color", c.Background)
	define("headerbar_backdrop_fg_color", c.Foreground)
	define("card_bg_color", c.Card)
	define("card_fg_color", c.CardForeground)
	define("popover_bg_color", c.Card)
	define("popover_fg_color", c.CardForeground)
	define("thumbnail_bg_color", c.Secondary)
	define("accent_bg_color", c.Primary)
	define("accent_fg_color", c.PrimaryForeground)
	define("destructive_bg_color", c.Destructive)
	define("destructive_fg_color", c.DestructiveForeground)
	define("error_bg_color", c.Destructive)
	define("error_fg_color", c.DestructiveForeground)
	// App-specific aliases mirror wa-bot's stylesheet and keep custom widgets
	// on the same palette as libadwaita's standard surfaces.
	define("wa_border_color", c.Border)
	define("wa_muted_fg_color", c.MutedForeground)
	define("tmux_border", c.Border)
	define("tmux_muted", c.Muted)
	define("tmux_muted_fg", c.MutedForeground)
	b.WriteString(widgetCSS)
	return b.String()
}

func luminance(hex string) float64 {
	h := strings.TrimPrefix(hex, "#")
	if len(h) != 6 {
		return 0
	}
	var r, g, b int
	if _, err := fmt.Sscanf(strings.ToUpper(h), "%02X%02X%02X", &r, &g, &b); err != nil {
		return 0
	}
	linear := func(v int) float64 {
		x := float64(v) / 255
		if x <= .03928 {
			return x / 12.92
		}
		return math.Pow((x+.055)/1.055, 2.4)
	}
	return .2126*linear(r) + .7152*linear(g) + .0722*linear(b)
}
