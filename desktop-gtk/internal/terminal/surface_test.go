package terminal

import "testing"

func TestPositionedFrameClearsAndKeepsCursorWithLastRow(t *testing.T) {
	frame := positionedFrame("first   \r\nsecond\r\n")
	want := "\x1b[?25l\x1b[2J\x1b[H\x1b[1;1H\x1b[2Kfirst\x1b[2;1H\x1b[2Ksecond\x1b[2;7H\x1b[?25h"
	if frame != want {
		t.Fatalf("positioned frame = %q, want %q", frame, want)
	}
}

func TestPositionedFrameNormalizesCarriageReturns(t *testing.T) {
	frame := positionedFrame("one\rtwo\rthree")
	if frame == "" || frame[len(frame)-len("\x1b[?25h"):] != "\x1b[?25h" {
		t.Fatalf("frame did not finish by restoring cursor visibility: %q", frame)
	}
	if want := "\x1b[3;1H\x1b[2Kthree"; !contains(frame, want) {
		t.Fatalf("frame missing normalized third row %q: %q", want, frame)
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
