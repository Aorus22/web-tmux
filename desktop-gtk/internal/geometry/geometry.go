package geometry

import (
	"math"

	"tmux-gui/desktop-gtk/internal/protocol"
)

const (
	CellWidth  = 8.0
	CellHeight = 18.0
)

type Rect struct {
	Left, Top, Width, Height int
}

type Divider struct {
	PaneID    string
	Direction string
	Rect      Rect
	CellPX    float64
}

func Viewport(width, height int) (cols, rows int) {
	return max(2, int(math.Round(float64(width)/CellWidth))), max(1, int(math.Round(float64(height)/CellHeight)))
}

func PixelRect(p protocol.Pane, width, height, windowWidth, windowHeight int) Rect {
	if windowWidth <= 0 || windowHeight <= 0 {
		return Rect{}
	}
	sx, sy := float64(width)/float64(windowWidth), float64(height)/float64(windowHeight)
	return Rect{
		Left:   int(math.Round(float64(p.Left) * sx)),
		Top:    int(math.Round(float64(p.Top) * sy)),
		Width:  int(math.Round(float64(p.Width) * sx)),
		Height: int(math.Round(float64(p.Height) * sy)),
	}
}

// ClosePaneGaps absorbs tmux's one-cell border strips into the pane on the
// right or below. This makes native pane frames tile without empty seams.
func ClosePaneGaps(panes []protocol.Pane) []protocol.Pane {
	out := append([]protocol.Pane(nil), panes...)
	for i := range out {
		for j := range out {
			if i == j {
				continue
			}
			p, q := &out[i], out[j]
			if q.Left+q.Width+1 == p.Left && p.Top < q.Top+q.Height && q.Top < p.Top+p.Height {
				p.Left--
				p.Width++
			}
			if q.Top+q.Height+1 == p.Top && p.Left < q.Left+q.Width && q.Left < p.Left+p.Width {
				p.Top--
				p.Height++
			}
		}
	}
	return out
}

func Dividers(panes []protocol.Pane, rects map[string]Rect) []Divider {
	var out []Divider
	for _, a := range panes {
		for _, b := range panes {
			if a.ID == b.ID {
				continue
			}
			ra, rb := rects[a.ID], rects[b.ID]
			if b.Left == a.Left+a.Width && a.Top < b.Top+b.Height && b.Top < a.Top+a.Height {
				top, bottom := max(ra.Top, rb.Top), min(ra.Top+ra.Height, rb.Top+rb.Height)
				out = append(out, Divider{PaneID: a.ID, Direction: "R", Rect: Rect{Left: ra.Left + ra.Width - 3, Top: top, Width: 6, Height: bottom - top}, CellPX: float64(ra.Width) / float64(max(1, a.Width))})
			}
			if b.Top == a.Top+a.Height && a.Left < b.Left+b.Width && b.Left < a.Left+a.Width {
				left, right := max(ra.Left, rb.Left), min(ra.Left+ra.Width, rb.Left+rb.Width)
				out = append(out, Divider{PaneID: a.ID, Direction: "D", Rect: Rect{Left: left, Top: ra.Top + ra.Height - 3, Width: right - left, Height: 6}, CellPX: float64(ra.Height) / float64(max(1, a.Height))})
			}
		}
	}
	return out
}

func DragStep(position, start float64, lastCells int, cellPX float64) int {
	if cellPX <= 0 {
		return 0
	}
	return int(math.Round((position-start)/cellPX)) - lastCells
}
