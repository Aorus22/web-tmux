package geometry

import (
	"testing"

	"tmux-gui/desktop-gtk/internal/protocol"
)

func TestClosePaneGaps(t *testing.T) {
	in := []protocol.Pane{
		{ID: "%0", Left: 0, Top: 0, Width: 50, Height: 40},
		{ID: "%1", Left: 51, Top: 0, Width: 49, Height: 20},
		{ID: "%2", Left: 51, Top: 21, Width: 49, Height: 19},
	}
	out := ClosePaneGaps(in)
	if out[1].Left != 50 || out[1].Width != 50 || out[2].Top != 20 || out[2].Height != 20 {
		t.Fatalf("unexpected gap closing: %#v", out)
	}
	if in[1].Left != 51 {
		t.Fatal("input was mutated")
	}
}

func TestPixelRectAndViewport(t *testing.T) {
	r := PixelRect(protocol.Pane{Left: 50, Top: 20, Width: 50, Height: 20}, 1000, 800, 100, 40)
	if r != (Rect{Left: 500, Top: 400, Width: 500, Height: 400}) {
		t.Fatalf("unexpected rect: %#v", r)
	}
	c, rows := Viewport(800, 720)
	if c != 100 || rows != 40 {
		t.Fatalf("unexpected viewport: %dx%d", c, rows)
	}
}

func TestDragStepIsIncremental(t *testing.T) {
	if got := DragStep(32, 0, 2, 8); got != 2 {
		t.Fatalf("got %d, want 2", got)
	}
}
