package terminal

import (
	"fmt"
	"strings"
)

type Palette struct {
	Foreground, Background, Cursor RGB
	ANSI                           [16]RGB
}

func ParseHex(value string) RGB {
	var r, g, b uint8
	_, _ = fmt.Sscanf(strings.TrimPrefix(value, "#"), "%02x%02x%02x", &r, &g, &b)
	return RGB{r, g, b}
}

func DefaultPalette() Palette {
	colors := []string{"#000000", "#cd3131", "#0dbc79", "#e5e510", "#2472c8", "#bc3fbc", "#11a8cd", "#e5e5e5", "#666666", "#f14c4c", "#23d18b", "#f5f543", "#3b8eea", "#d670d6", "#29b8db", "#ffffff"}
	p := Palette{Foreground: ParseHex("#d4d4d4"), Background: ParseHex("#111111"), Cursor: ParseHex("#ffffff")}
	for i, s := range colors {
		p.ANSI[i] = ParseHex(s)
	}
	return p
}
