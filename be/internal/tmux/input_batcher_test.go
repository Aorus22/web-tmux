package tmux

import (
	"encoding/hex"
	"strings"
	"sync"
	"testing"
	"time"
)

func TestBatcherFlushesAfterInterval(t *testing.T) {
	var mu sync.Mutex
	var flushed []string
	b := NewInputBatcher(20*time.Millisecond, 4096, func(pane, hexData string) {
		mu.Lock()
		flushed = append(flushed, pane+":"+hexData)
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
	if !strings.HasPrefix(flushed[0], "%1:") {
		t.Fatalf("pane id missing: %v", flushed[0])
	}
	got, _ := hex.DecodeString(strings.TrimPrefix(flushed[0], "%1:"))
	if string(got) != "hello" {
		t.Fatalf("hex round trip: %q", got)
	}
	b.Stop()
}

func TestBatcherUTF8(t *testing.T) {
	var mu sync.Mutex
	var flushed string
	b := NewInputBatcher(20*time.Millisecond, 4096, func(_pane, hexData string) {
		mu.Lock()
		flushed = hexData
		mu.Unlock()
	})
	b.Start()
	defer b.Stop()

	// UTF-8 bytes survive hex round trip.
	b.Write("%1", []byte("héllo✓"))
	time.Sleep(60 * time.Millisecond)

	mu.Lock()
	defer mu.Unlock()
	got, _ := hex.DecodeString(flushed)
	if string(got) != "héllo✓" {
		t.Fatalf("UTF-8 round trip: %q", got)
	}
}

func TestBatcherControlSequences(t *testing.T) {
	var mu sync.Mutex
	var flushed string
	b := NewInputBatcher(20*time.Millisecond, 4096, func(_pane, hexData string) {
		mu.Lock()
		flushed = hexData
		mu.Unlock()
	})
	b.Start()
	defer b.Stop()

	// ESC, arrow keys, Ctrl+C — all raw bytes, hex-encoded, no shell quoting.
	ctrlC := []byte{0x03}
	arrow := []byte{0x1b, 0x5b, 0x41}
	combined := append(append([]byte{}, ctrlC...), arrow...)
	b.Write("%1", combined)
	time.Sleep(60 * time.Millisecond)

	mu.Lock()
	defer mu.Unlock()
	got, _ := hex.DecodeString(flushed)
	if string(got) != string(combined) {
		t.Fatalf("control sequence round trip: %q", got)
	}
}

func TestBatcherMaxBytes(t *testing.T) {
	var mu sync.Mutex
	count := 0
	b := NewInputBatcher(time.Hour, 16, func(_pane, _hex string) {
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
	b := NewInputBatcher(50*time.Millisecond, 4096, func(_pane, hexData string) {
		mu.Lock()
		flushes = append(flushes, hexData)
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
	got, _ := hex.DecodeString(flushes[0])
	if string(got) != "aaaaa" {
		t.Fatalf("aggregated content: %q", got)
	}
}

func TestBatcherStopFlushes(t *testing.T) {
	var mu sync.Mutex
	flushed := false
	b := NewInputBatcher(time.Hour, 4096, func(_pane, _hex string) {
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
