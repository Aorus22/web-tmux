package tmux

import (
	"strings"
	"testing"
)

func TestParseOutput(t *testing.T) {
	p := NewParser()

	ev, ok := p.ParseLine("%output %12 hello")
	if !ok || ev.Kind != "output" || ev.PaneID != "%12" || ev.Data != "hello" {
		t.Fatalf("bad output parse: %+v ok=%v", ev, ok)
	}
}

func TestParseOutputSpaces(t *testing.T) {
	p := NewParser()

	// A lone space typed in the terminal: the whole %output payload is the
	// data. It must survive verbatim — trimming it dropped the event, so the
	// space worked in the shell but never rendered.
	ev, ok := p.ParseLine("%output %12  ")
	if !ok || ev.Kind != "output" || ev.PaneID != "%12" {
		t.Fatalf("standalone space: bad parse: %+v ok=%v", ev, ok)
	}
	if ev.Data != " " {
		t.Fatalf("standalone space: data %q, want a single space", ev.Data)
	}

	// Trailing space after a word (e.g. echoing "hello ").
	ev, ok = p.ParseLine("%output %12 hello ")
	if !ok || ev.Kind != "output" || ev.PaneID != "%12" || ev.Data != "hello " {
		t.Fatalf("trailing space: bad parse: %+v ok=%v", ev, ok)
	}

	// Leading space in the data.
	ev, ok = p.ParseLine("%output %12  hi")
	if !ok || ev.Kind != "output" || ev.PaneID != "%12" || ev.Data != " hi" {
		t.Fatalf("leading space: bad parse: %+v ok=%v", ev, ok)
	}
}

func TestParseOutputWithEscape(t *testing.T) {
	p := NewParser()
	// tmux control mode escapes special chars as \ooo octal and \\ for backslash.
	raw := "%output %3 \\033[32mok\\033[0m"
	ev, ok := p.ParseLine(raw)
	if !ok || ev.Kind != "output" || ev.PaneID != "%3" {
		t.Fatalf("bad parse: %+v", ev)
	}
	decoded := DecodeOutput(ev.Data)
	want := "\x1b[32mok\x1b[0m"
	if string(decoded) != want {
		t.Fatalf("decode mismatch: got %q want %q", decoded, want)
	}
}

func TestDecodeOutputBackslash(t *testing.T) {
	if got := string(DecodeOutput("a\\\\b")); got != "a\\b" {
		t.Fatalf("backslash decode: got %q", got)
	}
	if got := string(DecodeOutput("\\377")); got != "\xff" {
		t.Fatalf("octal decode: got %q", got)
	}
}

func TestParseMarkers(t *testing.T) {
	p := NewParser()

	ev, ok := p.ParseLine("%begin 4 0")
	if !ok || ev.Marker != "begin" || ev.Command != 4 {
		t.Fatalf("bad begin: %+v", ev)
	}
	ev, ok = p.ParseLine("%end 4 0")
	if !ok || ev.Marker != "end" || ev.Command != 4 {
		t.Fatalf("bad end: %+v", ev)
	}
	ev, ok = p.ParseLine("%error 5 pane not found")
	if !ok || ev.Marker != "error" || ev.Command != 5 || ev.Error != "pane not found" {
		t.Fatalf("bad error: %+v", ev)
	}
}

func TestParseWindowPaneChanged(t *testing.T) {
	p := NewParser()
	// tmux emits %window-pane-changed when the active pane changes (pane
	// click / select-pane), including when select-pane crosses into another
	// window (which also moves the current window).
	ev, ok := p.ParseLine("%window-pane-changed @1 %2")
	if !ok || ev.Kind != "window-pane-changed" {
		t.Fatalf("bad window-pane-changed: %+v ok=%v", ev, ok)
	}
	if ev.Data != "@1 %2" {
		t.Fatalf("bad payload: %q", ev.Data)
	}
}

func TestParseSessionWindowChanged(t *testing.T) {
	p := NewParser()
	// tmux emits %session-window-changed when the current window of a session
	// changes — the frontend relies on it to resync after a tab click.
	ev, ok := p.ParseLine("%session-window-changed $0 @1")
	if !ok || ev.Kind != "session-window-changed" {
		t.Fatalf("bad session-window-changed: %+v ok=%v", ev, ok)
	}
	if ev.Data != "$0 @1" {
		t.Fatalf("bad payload: %q", ev.Data)
	}
}

func TestParseLayoutChange(t *testing.T) {
	p := NewParser()
	ev, ok := p.ParseLine("%layout-change @3 8f06,80x24,0,0{...}")
	if !ok || ev.Kind != "layout-change" {
		t.Fatalf("bad layout-change: %+v", ev)
	}
}

func TestParseUnknownEventDoesNotPanic(t *testing.T) {
	p := NewParser()
	inputs := []string{
		"%some-future-event payload",
		"%exit",
		"%%literal-percent-line",
		"",
		"%output %1",
		"%output %broken-no-pane",
	}
	for _, in := range inputs {
		func() {
			defer func() {
				if r := recover(); r != nil {
					t.Fatalf("panic on %q: %v", in, r)
				}
			}()
			ev, ok := p.ParseLine(in)
			_ = ev
			_ = ok
		}()
	}
}

func TestSplitOutputMalformed(t *testing.T) {
	_, _, ok := splitOutput("no-percent")
	if ok {
		t.Fatal("expected split failure")
	}
	_, _, ok = splitOutput("%12") // no space
	if ok {
		t.Fatal("expected split failure for pane-only")
	}
}

func TestParserRoundTripOutput(t *testing.T) {
	p := NewParser()
	ev, ok := p.ParseLine("%output %5 echo hi")
	if !ok {
		t.Fatal("parse failed")
	}
	got := string(DecodeOutput(ev.Data))
	if !strings.Contains(got, "echo") {
		t.Fatalf("output content: %q", got)
	}
}
