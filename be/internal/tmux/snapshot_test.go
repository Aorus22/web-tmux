package tmux

import (
	"strings"
	"testing"
)

func TestParseSessionLine(t *testing.T) {
	ses, ok := parseSession("dev|2|1|1786158000|120|30")
	if !ok {
		t.Fatal("parse failed")
	}
	if ses.Name != "dev" || ses.Windows != 2 || ses.Attached != 1 ||
		ses.CreatedAt != 1786158000 || ses.Width != 120 || ses.Height != 30 {
		t.Fatalf("bad session: %+v", ses)
	}
}

func TestParseSessionMalformed(t *testing.T) {
	if _, ok := parseSession("only-name"); ok {
		t.Fatal("should fail on short line")
	}
}

func TestParseWindowLine(t *testing.T) {
	w, ok := parseWindow("@0|0|bash|1|2|80|24|8f06,80x24,0,0")
	if !ok {
		t.Fatal("parse failed")
	}
	if w.ID != "@0" || w.Index != 0 || w.Name != "bash" || !w.Active ||
		w.Panes != 2 || w.Width != 80 || w.Height != 24 || w.Layout != "8f06,80x24,0,0" {
		t.Fatalf("bad window: %+v", w)
	}
}

func TestParsePaneLine(t *testing.T) {
	// Zoomed pane: window_zoomed_flag=1 and pane_active=1 (the zoomed pane is
	// always the active one).
	p, ok := parsePane("%12|1|@0|1|1|0|13|40|11|1234|bash|/home/user|user@host")
	if !ok {
		t.Fatal("parse failed")
	}
	if p.ID != "%12" || p.Index != 1 || p.WindowID != "@0" || !p.Active ||
		!p.Zoomed || p.Left != 0 || p.Top != 13 || p.Width != 40 || p.Height != 11 ||
		p.PID != 1234 || p.CurrentCommand != "bash" ||
		p.CurrentPath != "/home/user" || p.Title != "user@host" {
		t.Fatalf("bad pane: %+v", p)
	}
}

func TestParsePaneActive(t *testing.T) {
	p, ok := parsePane("%1|0|@0|1|0|0|0|80|24|1|zsh|/|x")
	if !ok || !p.Active || p.Zoomed {
		t.Fatalf("bad pane active parse: %+v", p)
	}
}

func TestFormatRoundTrip(t *testing.T) {
	// Each "#{...}" token is one field (pipes separate fields, not add one).
	if n := strings.Count(sessionFormat, "#{"); n != 6 {
		t.Fatalf("sessionFormat fields: %d", n)
	}
	if n := strings.Count(windowFormat, "#{"); n != 8 {
		t.Fatalf("windowFormat fields: %d", n)
	}
	if n := strings.Count(paneFormat, "#{"); n != 13 {
		t.Fatalf("paneFormat fields: %d", n)
	}
}
