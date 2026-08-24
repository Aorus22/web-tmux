package terminal

import "testing"

func TestPositionedFrameClearsAndKeepsCursorWithLastRow(t *testing.T) {
	frame := positionedFrame("first   \r\nsecond\r\n", 24, 80)
	want := "\x1b[?25l\x1b[2J\x1b[H\x1b[1;1H\x1b[2Kfirst   \x1b[2;1H\x1b[2Ksecond\x1b[2;7H\x1b[?25h"
	if frame != want {
		t.Fatalf("positioned frame = %q, want %q", frame, want)
	}
}

func TestPositionedFrameNormalizesCarriageReturns(t *testing.T) {
	frame := positionedFrame("one\rtwo\rthree", 24, 80)
	if frame == "" || frame[len(frame)-len("\x1b[?25h"):] != "\x1b[?25h" {
		t.Fatalf("frame did not finish by restoring cursor visibility: %q", frame)
	}
	if want := "\x1b[3;1H\x1b[2Kthree"; !contains(frame, want) {
		t.Fatalf("frame missing normalized third row %q: %q", want, frame)
	}
}

func TestPositionedFrameClampsToGrid(t *testing.T) {
	// More rows than the grid: only the tail (visible screen) is kept.
	frame := positionedFrame("old1\nold2\nprompt $ ", 2, 80)
	if want := "\x1b[1;1H\x1b[2Kold2"; !contains(frame, want) {
		t.Fatalf("frame kept wrong rows: %q", frame)
	}
	if contains(frame, "old1") {
		t.Fatalf("frame leaked dropped row: %q", frame)
	}
	// Rows wider than the grid are clipped; escapes survive clipping.
	wide := "\x1b[31m" + string(make([]rune, 0)) + "0123456789abcdef"
	frame = positionedFrame(wide, 24, 8)
	if !contains(frame, "\x1b[31m01234567\x1b[") && !contains(frame, "\x1b[31m01234567") {
		t.Fatalf("wide row not clipped to 8 columns with escape kept: %q", frame)
	}
}

func contains(s, part string) bool {
	for i := 0; i+len(part) <= len(s); i++ {
		if s[i:i+len(part)] == part {
			return true
		}
	}
	return false
}
