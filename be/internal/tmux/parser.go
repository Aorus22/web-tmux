package tmux

import (
	"bytes"
	"fmt"
	"strconv"
	"strings"
)

// ControlEvent is a parsed line from tmux control mode.
type ControlEvent struct {
	Kind    string // "output", "layout-change", "window-add", ... ; "" for begin/end/error
	Marker  string // "begin" | "end" | "error" when Kind == ""
	PaneID  string // set for %output
	Data    string // raw payload (escaped octal sequences still present for output)
	Command int    // %begin/%end/%error command number
	Flags   string // %begin flags
	Error   string // %error message
}

// Parser reads tmux control-mode protocol lines.
//
// Supported events (PRD §29): %begin, %end, %error, %output, %layout-change,
// %window-add, %window-close, %window-renamed, %session-changed,
// %sessions-changed, %pane-mode-changed, %client-session-changed.
// Unknown events are returned with Kind "unknown" — they must not crash the backend.
type Parser struct{}

func NewParser() *Parser { return &Parser{} }

// ParseLine parses a single raw control-mode line. Lines begin with '%' (event)
// or '%%' (literal '%' — returned as output data). Empty lines are skipped.
func (p *Parser) ParseLine(raw string) (ControlEvent, bool) {
	if raw == "" {
		return ControlEvent{}, false
	}
	if !strings.HasPrefix(raw, "%") {
		// Literal text line (shouldn't normally appear outside %output).
		return ControlEvent{Kind: "output", Data: raw}, true
	}
	if strings.HasPrefix(raw, "%%") {
		return ControlEvent{Kind: "output", Data: raw[1:]}, true
	}

	rest := raw[1:]
	space := strings.IndexByte(rest, ' ')
	kind := rest
	payload := ""
	if space >= 0 {
		kind = rest[:space]
		payload = strings.TrimSpace(rest[space+1:])
	}

	switch kind {
	case "begin", "end", "error":
		return parseMarker(kind, payload), true
	case "output":
		pane, data, ok := splitOutput(payload)
		if !ok {
			return ControlEvent{Kind: "unknown"}, true
		}
		return ControlEvent{Kind: "output", PaneID: pane, Data: data}, true
	case "layout-change":
		// %layout-change <window-id> <window-layout> — index 1 carries the window id.
		fields := strings.Fields(payload)
		if len(fields) >= 2 {
			return ControlEvent{Kind: "layout-change", PaneID: fields[1], Data: fields[0]}, true
		}
		return ControlEvent{Kind: "layout-change", Data: payload}, true
	case "window-add", "window-close", "window-renamed", "session-changed",
		"sessions-changed", "pane-mode-changed", "client-session-changed",
		"client-detached", "client-attached", "session-renamed":
		return ControlEvent{Kind: kind, Data: payload}, true
	default:
		return ControlEvent{Kind: "unknown", Data: payload}, true
	}
}

// parseMarker parses %begin <num> <flags>, %end <num> <flags>, %error <num> <message>.
func parseMarker(kind, payload string) ControlEvent {
	ev := ControlEvent{Kind: "", Marker: kind}
	fields := strings.Fields(payload)
	if len(fields) > 0 {
		ev.Command, _ = strconv.Atoi(fields[0])
	}
	switch kind {
	case "begin":
		if len(fields) > 1 {
			ev.Flags = fields[1]
		}
	case "error":
		if len(fields) > 1 {
			ev.Error = strings.Join(fields[1:], " ")
		} else {
			ev.Error = payload
		}
	}
	return ev
}

// splitOutput splits "%output %<pane> <data>" payloads. tmux output data uses
// %<pane-id> prefix followed by a space, then escaped data.
func splitOutput(payload string) (pane, data string, ok bool) {
	if !strings.HasPrefix(payload, "%") {
		return "", "", false
	}
	rest := payload[1:]
	space := strings.IndexByte(rest, ' ')
	if space < 0 {
		return "", "", false
	}
	pane = "%" + rest[:space]
	data = rest[space+1:]
	return pane, data, true
}

// DecodeOutput converts escaped control-mode output data back into raw bytes.
// Control mode escapes characters as \ooo (octal) and \\ for literal backslash.
func DecodeOutput(data string) []byte {
	var buf bytes.Buffer
	buf.Grow(len(data))
	for i := 0; i < len(data); i++ {
		c := data[i]
		if c == '\\' && i+1 < len(data) {
			if data[i+1] == '\\' {
				buf.WriteByte('\\')
				i++
				continue
			}
			if i+3 < len(data) {
				o := data[i+1 : i+4]
				if n, err := strconv.ParseUint(o, 8, 8); err == nil {
					buf.WriteByte(byte(n))
					i += 3
					continue
				}
			}
		}
		buf.WriteByte(c)
	}
	return buf.Bytes()
}

// FormatOutput renders a decoded byte slice for logging/validation.
func FormatOutput(b []byte) string {
	if len(b) > 200 {
		return fmt.Sprintf("%s...(%d bytes)", string(b[:200]), len(b))
	}
	return string(b)
}
