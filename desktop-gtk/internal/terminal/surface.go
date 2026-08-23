package terminal

import (
	"context"
	"fmt"
	"math"
	"strings"
	"sync"
	"unicode/utf8"

	"github.com/diamondburned/gotk4/pkg/cairo"
	"github.com/diamondburned/gotk4/pkg/gdk/v4"
	"github.com/diamondburned/gotk4/pkg/gio/v2"
	"github.com/diamondburned/gotk4/pkg/glib/v2"
	"github.com/diamondburned/gotk4/pkg/gtk/v4"
	"github.com/diamondburned/gotk4/pkg/pango"
	"github.com/diamondburned/gotk4/pkg/pangocairo"
)

type SurfaceOptions struct {
	PaneID               string
	FontFamily           string
	FontSize, LineHeight float64
	Scrollback           int
	Palette              Palette
	TUIScroll            bool
	OnInput              func([]byte)
	OnResize             func(cols, rows int)
	OnActivate           func()
}

type queuedOp struct {
	replace bool
	data    []byte
}
type point struct{ row, col int }

type Surface struct {
	paneID                             string
	area                               *gtk.DrawingArea
	engine                             *Engine
	palette                            Palette
	fontFamily                         string
	fontSize, lineHeight, cellW, cellH float64
	fontAscent, fontDescent            float64
	fontMetricsReady                   bool
	tuiScroll                          bool
	onResize                           func(int, int)
	onActivate                         func()

	queueMu        sync.Mutex
	ops            []queuedOp
	drainScheduled bool
	disposed       bool

	scrollOffset                 int
	selectionStart, selectionEnd *point
	dragStart                    point
	blinkOn                      bool
}

func NewSurface(opts SurfaceOptions) (*Surface, error) {
	if opts.FontFamily == "" {
		opts.FontFamily = "JetBrains Mono, Menlo, Consolas, monospace"
	}
	if opts.FontSize <= 0 {
		opts.FontSize = 14
	}
	if opts.LineHeight <= 0 {
		opts.LineHeight = 1.35
	}
	if opts.Palette == (Palette{}) {
		opts.Palette = DefaultPalette()
	}
	s := &Surface{paneID: opts.PaneID, palette: opts.Palette, fontFamily: opts.FontFamily, fontSize: opts.FontSize, lineHeight: opts.LineHeight, tuiScroll: opts.TUIScroll, onResize: opts.OnResize, onActivate: opts.OnActivate, blinkOn: true}
	s.recalculateCellSize()
	eng, err := NewEngine(24, 80, opts.Scrollback, func(b []byte) {
		if opts.OnInput != nil {
			opts.OnInput(b)
		}
	}, func() { s.queueDraw() })
	if err != nil {
		return nil, err
	}
	s.engine = eng
	s.applyPalette()
	s.area = gtk.NewDrawingArea()
	s.area.SetHExpand(true)
	s.area.SetVExpand(true)
	s.area.SetFocusable(true)
	s.area.SetFocusOnClick(true)
	s.area.SetDrawFunc(s.draw)
	s.area.ConnectResize(s.resize)
	s.installKeyboard()
	s.installPointer()
	glib.TimeoutAdd(500, func() bool {
		if s.disposed {
			return false
		}
		s.blinkOn = !s.blinkOn
		s.area.QueueDraw()
		return true
	})
	return s, nil
}

func (s *Surface) Widget() gtk.Widgetter { return s.area }
func (s *Surface) PaneID() string        { return s.paneID }
func (s *Surface) Size() (int, int)      { return s.engine.Size() }

func (s *Surface) Feed(data []byte) {
	if len(data) == 0 {
		return
	}
	s.enqueue(queuedOp{data: append([]byte(nil), data...)})
}
func (s *Surface) ReplaceScreen(data string) {
	s.enqueue(queuedOp{replace: true, data: []byte(positionedFrame(data))})
}
func (s *Surface) ResetSnapshot() { s.selectionStart = nil; s.selectionEnd = nil }

func (s *Surface) enqueue(op queuedOp) {
	s.queueMu.Lock()
	if s.disposed {
		s.queueMu.Unlock()
		return
	}
	if op.replace && len(s.ops) > 0 && s.ops[len(s.ops)-1].replace {
		s.ops[len(s.ops)-1] = op
	} else if !op.replace && len(s.ops) > 0 && !s.ops[len(s.ops)-1].replace {
		s.ops[len(s.ops)-1].data = append(s.ops[len(s.ops)-1].data, op.data...)
	} else {
		s.ops = append(s.ops, op)
	}
	if s.drainScheduled {
		s.queueMu.Unlock()
		return
	}
	s.drainScheduled = true
	s.queueMu.Unlock()
	glib.IdleAdd(func() bool { s.drain(); return false })
}

func (s *Surface) drain() {
	s.queueMu.Lock()
	ops := s.ops
	s.ops = nil
	s.drainScheduled = false
	s.queueMu.Unlock()
	for _, op := range ops {
		if op.replace {
			s.engine.Reset()
			s.scrollOffset = 0
		}
		s.engine.Feed(op.data)
	}
	s.area.QueueDraw()
}

func positionedFrame(data string) string {
	data = strings.ReplaceAll(data, "\r\n", "\n")
	data = strings.ReplaceAll(data, "\r", "\n")
	rows := strings.Split(data, "\n")
	// capture-pane terminates its output with a newline. Do not create an
	// extra row for it: that row would move the libvterm cursor away from the
	// prompt and make the caret appear detached from the text.
	if len(rows) > 1 && rows[len(rows)-1] == "" {
		rows = rows[:len(rows)-1]
	}
	var b strings.Builder
	// A replacement is a complete frame, not a stream append. Clear both the
	// visible screen and each line before writing so shorter rows cannot leave
	// stale glyphs behind from the previous capture.
	b.WriteString("\x1b[?25l\x1b[2J\x1b[H")
	cursorRow, cursorCol := -1, 0
	for i, row := range rows {
		row = strings.TrimRight(row, " \t")
		fmt.Fprintf(&b, "\x1b[%d;1H\x1b[2K%s", i+1, row)
		visible := visibleText(row)
		trimmed := strings.TrimRight(visible, " \t")
		if strings.TrimSpace(trimmed) != "" {
			cursorRow, cursorCol = i, displayWidth(trimmed)
		}
	}
	// Keep the cursor at the end of the captured prompt. capture-pane does not
	// expose cursor coordinates, and it normally includes blank rows after the
	// prompt; without this explicit position the caret would land at the bottom
	// of the pane instead of next to the text.
	if cursorRow >= 0 {
		fmt.Fprintf(&b, "\x1b[%d;%dH", cursorRow+1, cursorCol+1)
	} else {
		b.WriteString("\x1b[H")
	}
	b.WriteString("\x1b[?25h")
	return b.String()
}

func visibleText(s string) string {
	var out strings.Builder
	for i := 0; i < len(s); {
		if s[i] != '\x1b' {
			r, size := utf8.DecodeRuneInString(s[i:])
			if r == utf8.RuneError && size == 1 {
				i++
				continue
			}
			if r >= ' ' || r == '\t' {
				out.WriteRune(r)
			}
			i += size
			continue
		}
		i++
		if i >= len(s) {
			break
		}
		switch s[i] {
		case '[': // CSI: consume through the final byte.
			i++
			for i < len(s) {
				b := s[i]
				i++
				if b >= 0x40 && b <= 0x7e {
					break
				}
			}
		case ']': // OSC: consume through BEL or ST.
			i++
			for i < len(s) {
				if s[i] == '\a' {
					i++
					break
				}
				if s[i] == '\x1b' && i+1 < len(s) && s[i+1] == '\\' {
					i += 2
					break
				}
				i++
			}
		default:
			i++
		}
	}
	return out.String()
}

func displayWidth(s string) int {
	col := 0
	for _, r := range s {
		if r == '\t' {
			col = (col/8 + 1) * 8
			continue
		}
		col++
	}
	return col
}

func (s *Surface) SetTUIScroll(enabled bool) { s.tuiScroll = enabled }
func (s *Surface) SetFont(family string, size, lineHeight float64) {
	if family != "" {
		s.fontFamily = family
	}
	if size > 0 {
		s.fontSize = size
	}
	if lineHeight > 0 {
		s.lineHeight = lineHeight
	}
	s.fontMetricsReady = false
	s.recalculateCellSize()
	w := s.area.AllocatedWidth()
	h := s.area.AllocatedHeight()
	s.resize(w, h)
	s.area.QueueDraw()
}
func (s *Surface) SetPalette(p Palette) { s.palette = p; s.applyPalette(); s.area.QueueDraw() }
func (s *Surface) applyPalette() {
	if s.engine != nil {
		s.engine.SetPalette(s.palette.Foreground, s.palette.Background, s.palette.ANSI)
	}
}
func (s *Surface) recalculateCellSize() {
	s.cellH = math.Max(8, math.Ceil(s.fontSize*s.lineHeight))
	s.cellW = math.Max(4, math.Ceil(s.fontSize*0.62))
	s.fontAscent = s.fontSize
	s.fontDescent = 0
}

// ensureFontMetrics resolves comma-separated fallback names and measures the
// actual Pango font used by Cairo. A guessed 0.62*font-size cell width is not
// stable across Windows font fallback and is what makes a fixed-cell terminal
// look like its cursor is drifting through the text.
func (s *Surface) ensureFontMetrics(cr *cairo.Context) bool {
	ctx := pangocairo.CreateContext(cr)
	family := resolveFontFamily(ctx, s.fontFamily)
	changedFamily := family != s.fontFamily
	if changedFamily {
		s.fontFamily = family
	}
	desc := s.fontDescription(false, false)
	metrics := ctx.Metrics(desc, nil)
	width := float64(metrics.ApproximateDigitWidth()) / pango.SCALE
	fontHeight := float64(metrics.Height()) / pango.SCALE
	ascent := float64(metrics.Ascent()) / pango.SCALE
	descent := float64(metrics.Descent()) / pango.SCALE
	if width < 1 {
		width = s.fontSize * 0.62
	}
	if fontHeight < 1 {
		fontHeight = s.fontSize
	}
	lineHeight := math.Max(fontHeight, s.fontSize) * s.lineHeight
	if lineHeight < 8 {
		lineHeight = 8
	}
	changedMetrics := math.Abs(width-s.cellW) > 0.01 || math.Abs(lineHeight-s.cellH) > 0.01
	s.cellW = width
	s.cellH = lineHeight
	s.fontAscent = ascent
	s.fontDescent = descent
	s.fontMetricsReady = true
	return changedFamily || changedMetrics
}

func (s *Surface) resize(width, height int) {
	if width <= 0 || height <= 0 {
		return
	}
	cols := max(2, int(float64(width)/s.cellW))
	rows := max(1, int(float64(height)/s.cellH))
	oldR, oldC := s.engine.Size()
	if oldR == rows && oldC == cols {
		return
	}
	s.engine.Resize(rows, cols)
	if s.onResize != nil {
		s.onResize(cols, rows)
	}
}

func (s *Surface) queueDraw() {
	if s.area != nil {
		s.area.QueueDraw()
	}
}

func (s *Surface) draw(_ *gtk.DrawingArea, cr *cairo.Context, width, height int) {
	if !s.fontMetricsReady {
		if s.ensureFontMetrics(cr) {
			// Recompute the vterm grid using measured metrics before reading cells.
			s.resize(width, height)
		}
	}
	source(cr, s.palette.Background)
	cr.Paint()
	rows, cols := s.engine.Size()
	history := s.engine.Scrollback()
	total := len(history) + rows
	end := total - s.scrollOffset
	if end < rows {
		end = rows
	}
	start := end - rows
	for vr := 0; vr < rows; vr++ {
		global := start + vr
		line := s.lineAt(global, history, rows, cols)
		for col := 0; col < cols && col < len(line); col++ {
			cell := line[col]
			if cell.Width == 0 {
				continue
			}
			x := float64(col) * s.cellW
			y := float64(vr) * s.cellH
			fg, bg := cell.Foreground, cell.Background
			if cell.Reverse {
				fg, bg = bg, fg
			}
			if s.selected(global, col) {
				bg = blend(bg, s.palette.Cursor, 0.45)
			}
			source(cr, bg)
			cr.Rectangle(x, y, s.cellW*float64(max(1, cell.Width)), s.cellH)
			cr.Fill()
			if cell.Conceal || cell.Text == "" || (cell.Blink && !s.blinkOn) {
				continue
			}
			source(cr, fg)
			desc := s.fontDescription(cell.Bold, cell.Italic)
			layout := pangocairo.CreateLayout(cr)
			layout.SetFontDescription(desc)
			layout.SetWidth(-1)
			layout.SetSingleParagraphMode(true)
			layout.SetText(cell.Text)
			// Clip each glyph to its terminal cell. This prevents a fallback glyph
			// with a different advance from painting into the next cell.
			cr.Save()
			cr.Rectangle(x, y, s.cellW*float64(max(1, cell.Width)), s.cellH)
			cr.Clip()
			textHeight := s.fontAscent + s.fontDescent
			cr.MoveTo(x, y+math.Max(0, (s.cellH-textHeight)/2))
			pangocairo.ShowLayout(cr, layout)
			cr.Restore()
			if cell.Underline > 0 {
				cr.SetLineWidth(1)
				cr.MoveTo(x, y+s.cellH-2)
				cr.LineTo(x+s.cellW*float64(max(1, cell.Width)), y+s.cellH-2)
				cr.Stroke()
			}
			if cell.Strike {
				cr.SetLineWidth(1)
				cr.MoveTo(x, y+s.cellH*.55)
				cr.LineTo(x+s.cellW*float64(max(1, cell.Width)), y+s.cellH*.55)
				cr.Stroke()
			}
		}
	}
	if s.scrollOffset == 0 && s.blinkOn {
		cursor := s.engine.Cursor()
		if cursor.Visible && cursor.Row >= 0 && cursor.Row < rows && cursor.Col >= 0 && cursor.Col < cols {
			x := float64(cursor.Col) * s.cellW
			y := float64(cursor.Row) * s.cellH
			sourceAlpha(cr, s.palette.Cursor, .65)
			switch cursor.Shape {
			case 2:
				cr.Rectangle(x, y+s.cellH-2, s.cellW, 2)
			case 3:
				cr.Rectangle(x, y, 2, s.cellH)
			default:
				cr.Rectangle(x, y, s.cellW, s.cellH)
			}
			cr.Fill()
		}
	}
	_ = width
	_ = height
}

func (s *Surface) fontDescription(bold, italic bool) *pango.FontDescription {
	d := pango.NewFontDescription()
	d.SetFamily(s.fontFamily)
	d.SetAbsoluteSize(s.fontSize * pango.SCALE)
	if bold {
		d.SetWeight(pango.WeightBold)
	}
	if italic {
		d.SetStyle(pango.StyleItalic)
	}
	return d
}

func resolveFontFamily(ctx *pango.Context, requested string) string {
	candidates := make([]string, 0, 4)
	for _, raw := range strings.Split(requested, ",") {
		if family := strings.TrimSpace(raw); family != "" {
			candidates = append(candidates, family)
		}
	}
	if len(candidates) == 0 {
		candidates = []string{"monospace"}
	}
	families := ctx.ListFamilies()
	available := make(map[string]string, len(families))
	for _, item := range families {
		family := pango.BaseFontFamily(item)
		if family == nil {
			continue
		}
		available[strings.ToLower(family.Name())] = family.Name()
	}
	for _, candidate := range candidates {
		if strings.EqualFold(candidate, "monospace") {
			for _, item := range families {
				family := pango.BaseFontFamily(item)
				if family != nil && family.IsMonospace() {
					return family.Name()
				}
			}
			continue
		}
		if resolved, ok := available[strings.ToLower(candidate)]; ok {
			return resolved
		}
	}
	// Keep an explicit first choice when Pango cannot enumerate a font map
	// (for example during early startup), but never pass the whole comma list
	// as one literal family name.
	return candidates[0]
}

func (s *Surface) lineAt(global int, history [][]Cell, rows, cols int) []Cell {
	if global < len(history) && global >= 0 {
		return history[global]
	}
	row := global - len(history)
	line := make([]Cell, cols)
	if row < 0 || row >= rows {
		return line
	}
	for col := 0; col < cols; col++ {
		if c, ok := s.engine.Cell(row, col); ok {
			line[col] = c
		}
	}
	return line
}

func source(cr *cairo.Context, c RGB) {
	cr.SetSourceRGB(float64(c.R)/255, float64(c.G)/255, float64(c.B)/255)
}
func sourceAlpha(cr *cairo.Context, c RGB, a float64) {
	cr.SetSourceRGBA(float64(c.R)/255, float64(c.G)/255, float64(c.B)/255, a)
}
func blend(a, b RGB, t float64) RGB {
	return RGB{uint8(float64(a.R)*(1-t) + float64(b.R)*t), uint8(float64(a.G)*(1-t) + float64(b.G)*t), uint8(float64(a.B)*(1-t) + float64(b.B)*t)}
}

func (s *Surface) installKeyboard() {
	keys := gtk.NewEventControllerKey()
	im := gtk.NewIMMulticontext()
	im.SetClientWidget(s.area)
	keys.SetIMContext(im)
	im.ConnectCommit(func(text string) {
		for _, r := range text {
			s.engine.Unichar(r, 0)
		}
	})
	keys.ConnectKeyPressed(func(keyval, keycode uint, state gdk.ModifierType) bool {
		_ = keycode
		mods := modsFromGDK(state)
		ctrl := state.Has(gdk.ControlMask) || state.Has(gdk.MetaMask)
		shift := state.Has(gdk.ShiftMask)
		if ctrl && shift && (keyval == gdk.KEY_C || keyval == gdk.KEY_c) {
			s.Copy()
			return true
		}
		if ctrl && shift && (keyval == gdk.KEY_V || keyval == gdk.KEY_v) {
			s.PasteClipboard()
			return true
		}
		if ctrl && (keyval == gdk.KEY_C || keyval == gdk.KEY_c) && s.HasSelection() {
			s.Copy()
			return true
		}
		if key, ok := specialKey(keyval); ok {
			s.engine.Key(key, mods)
			s.scrollOffset = 0
			return true
		}
		if mods&(ModCtrl|ModAlt) != 0 {
			if r := gdk.KeyvalToUnicode(keyval); r != 0 {
				s.engine.Unichar(rune(r), mods)
				s.scrollOffset = 0
				return true
			}
		}
		return false
	})
	s.area.AddController(keys)
	focus := gtk.NewEventControllerFocus()
	focus.ConnectEnter(func() {
		im.FocusIn()
		if s.onActivate != nil {
			s.onActivate()
		}
	})
	focus.ConnectLeave(func() { im.FocusOut() })
	s.area.AddController(focus)
}

func specialKey(k uint) (int, bool) {
	switch k {
	case gdk.KEY_Return, gdk.KEY_KP_Enter:
		return KeyEnter, true
	case gdk.KEY_Tab:
		return KeyTab, true
	case gdk.KEY_BackSpace:
		return KeyBackspace, true
	case gdk.KEY_Escape:
		return KeyEscape, true
	case gdk.KEY_Up:
		return KeyUp, true
	case gdk.KEY_Down:
		return KeyDown, true
	case gdk.KEY_Left:
		return KeyLeft, true
	case gdk.KEY_Right:
		return KeyRight, true
	case gdk.KEY_Insert:
		return KeyInsert, true
	case gdk.KEY_Delete:
		return KeyDelete, true
	case gdk.KEY_Home:
		return KeyHome, true
	case gdk.KEY_End:
		return KeyEnd, true
	case gdk.KEY_Page_Up:
		return KeyPageUp, true
	case gdk.KEY_Page_Down:
		return KeyPageDown, true
	}
	if k >= gdk.KEY_F1 && k <= gdk.KEY_F35 {
		return KeyFunction0 + int(k-gdk.KEY_F1) + 1, true
	}
	return 0, false
}
func modsFromGDK(s gdk.ModifierType) int {
	m := 0
	if s.Has(gdk.ShiftMask) {
		m |= ModShift
	}
	if s.Has(gdk.AltMask) {
		m |= ModAlt
	}
	if s.Has(gdk.ControlMask) {
		m |= ModCtrl
	}
	return m
}

func (s *Surface) installPointer() {
	click := gtk.NewGestureClick()
	click.SetButton(1)
	click.ConnectPressed(func(n int, x, y float64) {
		s.area.GrabFocus()
		if s.onActivate != nil {
			s.onActivate()
		}
		row, col := s.coords(x, y)
		mods := modsFromGDK(click.CurrentEventState())
		if s.engine.MouseMode() > 0 && !click.CurrentEventState().Has(gdk.ShiftMask) {
			s.engine.MouseMove(row, col, mods)
			s.engine.MouseButton(1, true, mods)
			return
		}
		p := s.globalPoint(row, col)
		s.selectionStart = &p
		s.selectionEnd = &p
		s.dragStart = p
		if n == 2 {
			s.selectWord(p)
		}
		s.area.QueueDraw()
	})
	click.ConnectReleased(func(_ int, x, y float64) {
		if s.engine.MouseMode() > 0 && !click.CurrentEventState().Has(gdk.ShiftMask) {
			row, col := s.coords(x, y)
			s.engine.MouseMove(row, col, modsFromGDK(click.CurrentEventState()))
			s.engine.MouseButton(1, false, modsFromGDK(click.CurrentEventState()))
		}
	})
	s.area.AddController(click)
	drag := gtk.NewGestureDrag()
	drag.SetButton(1)
	drag.ConnectDragBegin(func(x, y float64) { row, col := s.coords(x, y); s.dragStart = s.globalPoint(row, col) })
	drag.ConnectDragUpdate(func(dx, dy float64) {
		if s.engine.MouseMode() > 0 && !drag.CurrentEventState().Has(gdk.ShiftMask) {
			return
		}
		row, col := s.coords(float64(s.dragStart.col)*s.cellW+dx, float64(s.dragStart.row)*s.cellH+dy)
		p := s.globalPoint(row, col)
		if s.selectionStart == nil {
			q := s.dragStart
			s.selectionStart = &q
		}
		s.selectionEnd = &p
		s.area.QueueDraw()
	})
	s.area.AddController(drag)
	scroll := gtk.NewEventControllerScroll(gtk.EventControllerScrollBothAxes | gtk.EventControllerScrollDiscrete)
	scroll.ConnectScroll(func(dx, dy float64) bool {
		_ = dx
		if dy == 0 {
			return false
		}
		if s.tuiScroll {
			key := KeyPageDown
			if dy < 0 {
				key = KeyPageUp
			}
			s.engine.Key(key, 0)
			return true
		}
		if s.engine.MouseMode() > 0 && !scroll.CurrentEventState().Has(gdk.ShiftMask) {
			button := 5
			if dy < 0 {
				button = 4
			}
			s.engine.MouseButton(button, true, modsFromGDK(scroll.CurrentEventState()))
			s.engine.MouseButton(button, false, modsFromGDK(scroll.CurrentEventState()))
			return true
		}
		step := 3
		if dy < 0 {
			s.scrollOffset += step
		} else {
			s.scrollOffset -= step
		}
		maxOffset := len(s.engine.Scrollback())
		if s.scrollOffset < 0 {
			s.scrollOffset = 0
		}
		if s.scrollOffset > maxOffset {
			s.scrollOffset = maxOffset
		}
		s.area.QueueDraw()
		return true
	})
	s.area.AddController(scroll)
}

func (s *Surface) coords(x, y float64) (int, int) {
	rows, cols := s.engine.Size()
	row := min(rows-1, max(0, int(y/s.cellH)))
	col := min(cols-1, max(0, int(x/s.cellW)))
	return row, col
}
func (s *Surface) globalPoint(row, col int) point {
	return point{len(s.engine.Scrollback()) + row - s.scrollOffset, col}
}
func ordered(a, b point) (point, point) {
	if a.row > b.row || (a.row == b.row && a.col > b.col) {
		return b, a
	}
	return a, b
}
func (s *Surface) selected(row, col int) bool {
	if s.selectionStart == nil || s.selectionEnd == nil {
		return false
	}
	a, b := ordered(*s.selectionStart, *s.selectionEnd)
	return row >= a.row && row <= b.row && (row != a.row || col >= a.col) && (row != b.row || col <= b.col)
}
func (s *Surface) HasSelection() bool {
	return s.selectionStart != nil && s.selectionEnd != nil && *s.selectionStart != *s.selectionEnd
}

func (s *Surface) selectWord(p point) {
	line := s.lineAt(p.row, s.engine.Scrollback(), func() int { r, _ := s.engine.Size(); return r }(), func() int { _, c := s.engine.Size(); return c }())
	if p.col >= len(line) {
		return
	}
	isWord := func(c Cell) bool { return c.Text != "" && !strings.ContainsAny(c.Text, " \t|,.;:()[]{}<>") }
	left, right := p.col, p.col
	for left > 0 && isWord(line[left-1]) {
		left--
	}
	for right+1 < len(line) && isWord(line[right+1]) {
		right++
	}
	a, b := point{p.row, left}, point{p.row, right}
	s.selectionStart = &a
	s.selectionEnd = &b
}

func (s *Surface) SelectionText() string {
	if s.selectionStart == nil || s.selectionEnd == nil {
		return ""
	}
	a, b := ordered(*s.selectionStart, *s.selectionEnd)
	rows, cols := s.engine.Size()
	history := s.engine.Scrollback()
	var lines []string
	for row := a.row; row <= b.row; row++ {
		line := s.lineAt(row, history, rows, cols)
		start, end := 0, len(line)-1
		if row == a.row {
			start = a.col
		}
		if row == b.row {
			end = b.col
		}
		var out strings.Builder
		for col := start; col <= end && col < len(line); col++ {
			if line[col].Width == 0 {
				continue
			}
			if line[col].Text == "" {
				out.WriteByte(' ')
			} else {
				out.WriteString(line[col].Text)
			}
		}
		lines = append(lines, strings.TrimRight(out.String(), " "))
	}
	return strings.Join(lines, "\n")
}
func (s *Surface) Copy() {
	if text := s.SelectionText(); text != "" {
		s.area.Clipboard().SetText(text)
	}
}
func (s *Surface) PasteClipboard() {
	cb := s.area.Clipboard()
	cb.ReadTextAsync(context.Background(), func(result gio.AsyncResulter) {
		text, err := cb.ReadTextFinish(result)
		if err == nil && text != "" {
			s.engine.Paste(text)
			s.scrollOffset = 0
		}
	})
}

func (s *Surface) Dispose() {
	s.queueMu.Lock()
	if s.disposed {
		s.queueMu.Unlock()
		return
	}
	s.disposed = true
	s.queueMu.Unlock()
	s.engine.Close()
}
