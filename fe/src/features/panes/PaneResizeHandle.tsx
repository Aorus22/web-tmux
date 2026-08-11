// PaneResizeHandle (PRD §19): draggable divider between panes. Converts pixel
// drags into tmux cell counts (pxToCells) and sends pane.resize with ~40ms
// throttling so a drag never produces hundreds of commands.
//
// All pointer handling uses React props + pointer capture — deliberately NOT
// window.addEventListener from an effect: the effect listeners silently died
// after the first resize (cleanup ran without a re-attach), leaving the
// divider un-draggable on the second attempt. React's delegated handlers stay
// live for the element's lifetime, and pointer capture keeps the drag going
// when the pointer leaves the 4px handle.

import { useRef, type CSSProperties } from 'react'
import { toast } from 'sonner'
import { tmuxSocket } from '@/lib/socket'
import { runCommand } from '@/lib/commands'
import { resizeDragStep, CELL_H, CELL_W } from '@/lib/geometry'

interface Props {
  paneId: string
  direction: 'L' | 'R' | 'U' | 'D'
  style?: CSSProperties
  // Actual rendered cell size of the dragged pane (from its pixel rect ÷ cell
  // dimensions). Falls back to the nominal size when absent.
  cellPx?: { w: number; h: number }
}

// tmux rejects negative resize adjustments (`resize-pane -R -3` fails with
// "unknown flag -3"), so a negative step must flip the direction and keep the
// amount positive.
const FLIP: Record<'L' | 'R' | 'U' | 'D', 'L' | 'R' | 'U' | 'D'> = {
  L: 'R',
  R: 'L',
  U: 'D',
  D: 'U',
}

export function PaneResizeHandle({ paneId, direction, style, cellPx }: Props) {
  const dragRef = useRef<{
    pointerId: number
    start: number
    // Cumulative cells already sent — the NEXT command carries only the
    // difference, so the pane tracks the pointer instead of re-applying the
    // whole accumulated delta (tmux applies each resize on top of the current
    // size, which would otherwise overshoot).
    lastCells: number
    lastSent: number
  } | null>(null)

  const sendStep = (step: number) => {
    // Negative step → flipped direction with a positive amount (tmux rejects
    // negative adjustments: "unknown flag -3").
    const dir = step < 0 ? FLIP[direction] : direction
    const amount = Math.abs(step)
    void runCommand(() => tmuxSocket.paneResize(paneId, dir, amount)).catch((err) =>
      toast.error((err as Error).message),
    )
  }

  const isVert = direction === 'U' || direction === 'D'

  return (
    <div
      role="separator"
      aria-orientation={isVert ? 'horizontal' : 'vertical'}
      className="absolute z-10 touch-none"
      style={{
        ...style,
        background: 'transparent',
      }}
      onPointerDown={(e) => {
        e.preventDefault()
        e.stopPropagation()
        // Capture the pointer so pointermove/pointerup keep firing on this
        // element even when the pointer leaves the 4px handle.
        e.currentTarget.setPointerCapture(e.pointerId)
        const pos = isVert ? e.clientY : e.clientX
        dragRef.current = { pointerId: e.pointerId, start: pos, lastCells: 0, lastSent: 0 }
        document.body.style.cursor = isVert ? 'row-resize' : 'col-resize'
      }}
      onPointerMove={(e) => {
        const d = dragRef.current
        if (!d || e.pointerId !== d.pointerId || !(e.buttons & 1)) return
        const pos = isVert ? e.clientY : e.clientX
        const cell = isVert ? (cellPx?.h ?? CELL_H) : (cellPx?.w ?? CELL_W)
        const step = resizeDragStep(pos, d.start, d.lastCells, cell)
        if (step === 0) return
        // Throttle to ~40ms (PRD §19: 30-60ms). Throttled steps are NOT marked
        // sent, so the next command still carries the full pending difference.
        const now = performance.now()
        if (now - d.lastSent < 40) return
        d.lastSent = now
        d.lastCells += step
        sendStep(step)
      }}
      onPointerUp={() => {
        dragRef.current = null
        document.body.style.cursor = ''
      }}
      onPointerCancel={() => {
        dragRef.current = null
        document.body.style.cursor = ''
      }}
    >
      <div className="h-full w-full rounded-full bg-border/60 opacity-40 transition-opacity hover:opacity-100 active:opacity-100" />
    </div>
  )
}
