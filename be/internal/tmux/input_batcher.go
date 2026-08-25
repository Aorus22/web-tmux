package tmux

import (
	"sync"
	"time"
)

// InputBatcher accumulates terminal input bytes per pane and delivers them as
// one raw batch every ~6ms or when the buffer exceeds 512 bytes (PRD §22).
// Delivery goes through the pane's pipe socket when its stream is active
// (zero process spawns) and falls back to `send-keys -H` otherwise; hex
// encoding at the fallback site avoids every shell-quoting problem and
// handles UTF-8, control sequences, ESC, function keys and mouse escapes.
// The low cap keeps the fallback's per-byte hex argv bounded (~512 argv
// elements — well under Windows cmdline limits even after bash quoting).
// Large pastes split across more flushes, which is acceptable: pastes are not
// latency-critical, while typical keystroke bursts are only a few bytes.
type InputBatcher struct {
	interval time.Duration
	maxBytes int

	mu      sync.Mutex
	buffers map[string][]byte

	// flush is invoked with the pane ID and the raw accumulated bytes.
	flush func(paneID string, data []byte)

	stopCh   chan struct{}
	doneCh   chan struct{}
	stopOnce sync.Once
}

func NewInputBatcher(interval time.Duration, maxBytes int, flush func(paneID string, data []byte)) *InputBatcher {
	if interval <= 0 {
		interval = 6 * time.Millisecond
	}
	if maxBytes <= 0 {
		maxBytes = 512
	}
	return &InputBatcher{
		interval: interval,
		maxBytes: maxBytes,
		buffers:  make(map[string][]byte),
		flush:    flush,
		stopCh:   make(chan struct{}),
		doneCh:   make(chan struct{}),
	}
}

// Start launches the flush ticker. Safe to call once.
func (b *InputBatcher) Start() {
	go func() {
		defer close(b.doneCh)
		t := time.NewTicker(b.interval)
		defer t.Stop()
		for {
			select {
			case <-b.stopCh:
				b.flushAll()
				return
			case <-t.C:
				b.flushAll()
			}
		}
	}()
}

// Stop flushes any remaining input and stops the ticker. Idempotent.
func (b *InputBatcher) Stop() {
	b.stopOnce.Do(func() {
		close(b.stopCh)
		<-b.doneCh
	})
}

// Write appends bytes for a pane; flushes immediately if over the cap.
func (b *InputBatcher) Write(paneID string, data []byte) {
	b.mu.Lock()
	buf := b.buffers[paneID]
	buf = append(buf, data...)
	over := len(buf) >= b.maxBytes
	b.buffers[paneID] = buf
	b.mu.Unlock()
	if over {
		b.flushAll()
	}
}

func (b *InputBatcher) flushAll() {
	b.mu.Lock()
	buffers := b.buffers
	b.buffers = make(map[string][]byte)
	b.mu.Unlock()

	for pane, buf := range buffers {
		if len(buf) == 0 {
			continue
		}
		b.flush(pane, buf)
	}
}
