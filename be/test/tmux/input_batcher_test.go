package tmux_test

import (
	"sync"
	"testing"
	"time"

	"tmux-gui/be/internal/tmux"
)

func TestBatcherFlushesAfterInterval(t *testing.T) {
	var mu sync.Mutex
	var flushed []string
	b := tmux.NewInputBatcher(20*time.Millisecond, 4096, func(pane string, data []byte) {
		mu.Lock()
		flushed = append(flushed, pane+":"+string(data))
		mu.Unlock()
	})
	b.Start()

	b.Write("%1", []byte("hello"))
	time.Sleep(60 * time.Millisecond)

	mu.Lock()
	defer mu.Unlock()
	if len(flushed) != 1 {
		t.Fatalf("expected 1 flush, got %d: %v", len(flushed), flushed)
	}
	if flushed[0] != "%1:hello" {
		t.Fatalf("raw bytes round trip: %q", flushed[0])
	}
	b.Stop()
}

func TestBatcherUTF8(t *testing.T) {
	var mu sync.Mutex
	var flushed string
	b := tmux.NewInputBatcher(20*time.Millisecond, 4096, func(_pane string, data []byte) {
		mu.Lock()
		flushed = string(data)
		mu.Unlock()
	})
	b.Start()
	defer b.Stop()

	// UTF-8 bytes survive the batch untouched.
	b.Write("%1", []byte("héllo✓"))
	time.Sleep(60 * time.Millisecond)

	mu.Lock()
	defer mu.Unlock()
	if flushed != "héllo✓" {
		t.Fatalf("UTF-8 round trip: %q", flushed)
	}
}

func TestBatcherControlSequences(t *testing.T) {
	var mu sync.Mutex
	var flushed []byte
	b := tmux.NewInputBatcher(20*time.Millisecond, 4096, func(_pane string, data []byte) {
		mu.Lock()
		flushed = data
		mu.Unlock()
	})
	b.Start()
	defer b.Stop()

	// ESC, arrow keys, Ctrl+C — all raw bytes, no shell quoting.
	ctrlC := []byte{0x03}
	arrow := []byte{0x1b, 0x5b, 0x41}
	combined := append(append([]byte{}, ctrlC...), arrow...)
	b.Write("%1", combined)
	time.Sleep(60 * time.Millisecond)

	mu.Lock()
	defer mu.Unlock()
	if string(flushed) != string(combined) {
		t.Fatalf("control sequence round trip: %q", flushed)
	}
}

func TestBatcherMaxBytes(t *testing.T) {
	var mu sync.Mutex
	count := 0
	b := tmux.NewInputBatcher(time.Hour, 16, func(_pane string, _data []byte) {
		mu.Lock()
		count++
		mu.Unlock()
	})
	b.Start()
	defer b.Stop()

	// Multiple writes that each cross the 16-byte cap trigger immediate flushes.
	for i := 0; i < 5; i++ {
		b.Write("%1", make([]byte, 20))
	}
	time.Sleep(50 * time.Millisecond)

	mu.Lock()
	defer mu.Unlock()
	if count < 5 {
		t.Fatalf("expected multiple flushes, got %d", count)
	}
}

func TestBatcherAggregatesRapidTyping(t *testing.T) {
	var mu sync.Mutex
	var flushes []string
	b := tmux.NewInputBatcher(50*time.Millisecond, 4096, func(_pane string, data []byte) {
		mu.Lock()
		flushes = append(flushes, string(data))
		mu.Unlock()
	})
	b.Start()
	defer b.Stop()

	// Rapid keystrokes within one window should be batched into one flush.
	for i := 0; i < 5; i++ {
		b.Write("%1", []byte("a"))
		time.Sleep(2 * time.Millisecond)
	}
	time.Sleep(100 * time.Millisecond)

	mu.Lock()
	defer mu.Unlock()
	if len(flushes) != 1 {
		t.Fatalf("expected 1 aggregated flush, got %d", len(flushes))
	}
	if flushes[0] != "aaaaa" {
		t.Fatalf("aggregated content: %q", flushes[0])
	}
}

func TestBatcherStopFlushes(t *testing.T) {
	var mu sync.Mutex
	flushed := false
	b := tmux.NewInputBatcher(time.Hour, 4096, func(_pane string, _data []byte) {
		mu.Lock()
		flushed = true
		mu.Unlock()
	})
	b.Start()
	b.Write("%1", []byte("pending"))
	b.Stop() // must flush pending data before returning

	mu.Lock()
	defer mu.Unlock()
	if !flushed {
		t.Fatal("Stop() must flush pending input")
	}
}

// TestBatcherFlushPayloadIsRawBytes guards the Phase-3 contract: the flush
// callback receives RAW bytes (hex encoding lives at the send-keys fallback
// site in monitor.go, not here).
func TestBatcherFlushPayloadIsRawBytes(t *testing.T) {
	var mu sync.Mutex
	var got []byte
	b := tmux.NewInputBatcher(20*time.Millisecond, 4096, func(_pane string, data []byte) {
		mu.Lock()
		got = data
		mu.Unlock()
	})
	b.Start()
	defer b.Stop()

	payload := []byte("raw payload \x01\x02")
	b.Write("%1", payload)
	time.Sleep(60 * time.Millisecond)

	mu.Lock()
	defer mu.Unlock()
	if string(got) != string(payload) {
		t.Fatalf("flush handler got %q, want %q", got, payload)
	}
}
