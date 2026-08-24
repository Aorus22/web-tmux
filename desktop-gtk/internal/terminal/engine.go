package terminal

/*
#cgo pkg-config: vterm
#include "vterm_bridge.h"
*/
import "C"

import (
	"errors"
	"runtime/cgo"
	"strings"
	"unsafe"
)

type RGB struct{ R, G, B uint8 }

type Cell struct {
	Text                                          string
	Width                                         int
	Bold, Italic, Blink, Reverse, Conceal, Strike bool
	Underline                                     int
	Foreground, Background                        RGB
}

type Cursor struct {
	Row, Col int
	Visible  bool
	Shape    int
}

type Engine struct {
	bridge          *C.VTBridge
	handle          cgo.Handle
	rows, cols      int
	onOutput        func([]byte)
	onDamage        func()
	scrollback      [][]Cell
	scrollbackLimit int
}

func NewEngine(rows, cols, scrollback int, onOutput func([]byte), onDamage func()) (*Engine, error) {
	if rows < 1 {
		rows = 1
	}
	if cols < 2 {
		cols = 2
	}
	e := &Engine{rows: rows, cols: cols, scrollbackLimit: scrollback, onOutput: onOutput, onDamage: onDamage}
	e.handle = cgo.NewHandle(e)
	e.bridge = C.vt_bridge_new(C.int(rows), C.int(cols), C.uintptr_t(e.handle))
	if e.bridge == nil {
		e.handle.Delete()
		return nil, errors.New("libvterm allocation failed")
	}
	return e, nil
}

func (e *Engine) Close() {
	if e.bridge != nil {
		C.vt_bridge_free(e.bridge)
		e.bridge = nil
		e.handle.Delete()
	}
}
func (e *Engine) Feed(data []byte) {
	if e.bridge == nil || len(data) == 0 {
		return
	}
	C.vt_bridge_feed(e.bridge, (*C.char)(unsafe.Pointer(&data[0])), C.size_t(len(data)))
}
func (e *Engine) Reset() {
	if e.bridge != nil {
		C.vt_bridge_reset(e.bridge, 1)
	}
}
func (e *Engine) Resize(rows, cols int) {
	if rows < 1 {
		rows = 1
	}
	if cols < 2 {
		cols = 2
	}
	if rows == e.rows && cols == e.cols {
		return
	}
	e.rows, e.cols = rows, cols
	C.vt_bridge_resize(e.bridge, C.int(rows), C.int(cols))
}
func (e *Engine) Size() (int, int) { return e.rows, e.cols }
func (e *Engine) MouseMode() int   { return int(C.vt_bridge_mouse_mode(e.bridge)) }

func (e *Engine) Cell(row, col int) (Cell, bool) {
	var c C.VTCell
	if C.vt_bridge_cell(e.bridge, C.int(row), C.int(col), &c) == 0 {
		return Cell{}, false
	}
	return cellFromC(&c), true
}
func (e *Engine) Cursor() Cursor {
	var r, c, v, s C.int
	C.vt_bridge_cursor(e.bridge, &r, &c, &v, &s)
	return Cursor{int(r), int(c), v != 0, int(s)}
}
func (e *Engine) Scrollback() [][]Cell { return e.scrollback }

// TrimScrollback keeps at most n history lines. Replacement frames must never
// grow history; this undoes any lines a synthetic frame pushed.
func (e *Engine) TrimScrollback(n int) {
	if n < 0 {
		n = 0
	}
	if len(e.scrollback) > n {
		e.scrollback = e.scrollback[:n]
	}
}

func (e *Engine) SetPalette(fg, bg RGB, palette [16]RGB) {
	C.vt_bridge_set_default_colors(e.bridge, C.uint8_t(fg.R), C.uint8_t(fg.G), C.uint8_t(fg.B), C.uint8_t(bg.R), C.uint8_t(bg.G), C.uint8_t(bg.B))
	for i, c := range palette {
		C.vt_bridge_set_palette(e.bridge, C.int(i), C.uint8_t(c.R), C.uint8_t(c.G), C.uint8_t(c.B))
	}
}
func (e *Engine) Key(key, mods int)        { C.vt_bridge_key(e.bridge, C.int(key), C.int(mods)) }
func (e *Engine) Unichar(r rune, mods int) { C.vt_bridge_unichar(e.bridge, C.uint32_t(r), C.int(mods)) }
func (e *Engine) Paste(text string) {
	C.vt_bridge_paste_start(e.bridge)
	for _, r := range text {
		e.Unichar(r, 0)
	}
	C.vt_bridge_paste_end(e.bridge)
}
func (e *Engine) MouseMove(row, col, mods int) {
	C.vt_bridge_mouse_move(e.bridge, C.int(row), C.int(col), C.int(mods))
}
func (e *Engine) MouseButton(button int, pressed bool, mods int) {
	p := 0
	if pressed {
		p = 1
	}
	C.vt_bridge_mouse_button(e.bridge, C.int(button), C.int(p), C.int(mods))
}

func cellFromC(c *C.VTCell) Cell {
	var b strings.Builder
	chars := []C.uint32_t{c.chars[0], c.chars[1], c.chars[2], c.chars[3], c.chars[4], c.chars[5]}
	for _, ch := range chars {
		if ch == 0 {
			break
		}
		b.WriteRune(rune(ch))
	}
	return Cell{Text: b.String(), Width: int(c.width), Bold: c.bold != 0, Underline: int(c.underline), Italic: c.italic != 0, Blink: c.blink != 0, Reverse: c.reverse != 0, Conceal: c.conceal != 0, Strike: c.strike != 0, Foreground: RGB{uint8(c.fg_r), uint8(c.fg_g), uint8(c.fg_b)}, Background: RGB{uint8(c.bg_r), uint8(c.bg_g), uint8(c.bg_b)}}
}

//export goVTermOutput
func goVTermOutput(handle C.uintptr_t, data unsafe.Pointer, length C.size_t) {
	e := cgo.Handle(handle).Value().(*Engine)
	if e.onOutput != nil {
		e.onOutput(C.GoBytes(data, C.int(length)))
	}
}

//export goVTermDamage
func goVTermDamage(handle C.uintptr_t) {
	e := cgo.Handle(handle).Value().(*Engine)
	if e.onDamage != nil {
		e.onDamage()
	}
}

//export goVTermScrollbackPush
func goVTermScrollbackPush(handle C.uintptr_t, cells unsafe.Pointer, count C.int) C.int {
	e := cgo.Handle(handle).Value().(*Engine)
	line := make([]Cell, int(count))
	for i := range line {
		var c C.VTCell
		C.vt_bridge_cell_from_array(e.bridge, cells, C.int(i), &c)
		line[i] = cellFromC(&c)
	}
	e.scrollback = append(e.scrollback, line)
	if e.scrollbackLimit > 0 && len(e.scrollback) > e.scrollbackLimit {
		copy(e.scrollback, e.scrollback[len(e.scrollback)-e.scrollbackLimit:])
		e.scrollback = e.scrollback[:e.scrollbackLimit]
	}
	return 1
}

//export goVTermScrollbackClear
func goVTermScrollbackClear(handle C.uintptr_t) {
	e := cgo.Handle(handle).Value().(*Engine)
	e.scrollback = nil
}

const (
	ModShift     = 1
	ModAlt       = 2
	ModCtrl      = 4
	KeyEnter     = 1
	KeyTab       = 2
	KeyBackspace = 3
	KeyEscape    = 4
	KeyUp        = 5
	KeyDown      = 6
	KeyLeft      = 7
	KeyRight     = 8
	KeyInsert    = 9
	KeyDelete    = 10
	KeyHome      = 11
	KeyEnd       = 12
	KeyPageUp    = 13
	KeyPageDown  = 14
	KeyFunction0 = 256
)
