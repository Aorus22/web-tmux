// PaneResizeHandle (PRD §19): draggable divider between panes. Converts pixel
// drags into tmux cell counts (pxToCells) and sends pane.resize with ~40ms
// throttling so a drag never produces hundreds of commands.

import { useEffect, useRef, type CSSProperties } from 'react'
import { toast } from 'sonner'
import { tmuxSocket } from '@/lib/socket'
import { runCommand } from '@/lib/commands'
import { pxToCells } from '@/lib/geometry'

interface Props {
  paneId: string
  direction: 'L' | 'R' | 'U' | 'D'
  style?: CSSProperties
}

export function PaneResizeHandle({ paneId, direction, style }: Props) {
  const dragRef = useRef<{
    start: number
    lastSent: number
  } | null>(null)

  useEffect(() => {
    const onMove = (e: PointerEvent) => {
      const d = dragRef.current
      if (!d) return
      const isVert = direction === 'U' || direction === 'D'
      const pos = isVert ? e.clientY : e.clientX
      const delta = pos - d.start
      const cells = pxToCells(delta, direction)
      if (cells === 0) return
      // Throttle to ~40ms (PRD §19: 30-60ms).
      const now = performance.now()
      if (now - d.lastSent < 40) return
      d.lastSent = now
      void runCommand(() => tmuxSocket.paneResize(paneId, direction, cells)).catch((err) =>
        toast.error((err as Error).message),
      )
    }

    const onUp = () => {
      dragRef.current = null
      document.body.style.cursor = ''
      window.removeEventListener('pointermove', onMove)
      window.removeEventListener('pointerup', onUp)
    }

    window.addEventListener('pointermove', onMove)
    window.addEventListener('pointerup', onUp)
    return () => {
      window.removeEventListener('pointermove', onMove)
      window.removeEventListener('pointerup', onUp)
    }
  }, [paneId, direction])

  return (
    <div
      role="separator"
      aria-orientation={direction === 'U' || direction === 'D' ? 'horizontal' : 'vertical'}
      className="absolute z-10 touch-none"
      style={{
        ...style,
        background: 'transparent',
      }}
      onPointerDown={(e) => {
        e.preventDefault()
        e.stopPropagation()
        const isVert = direction === 'U' || direction === 'D'
        const pos = isVert ? e.clientY : e.clientX
        dragRef.current = { start: pos, lastSent: 0 }
        document.body.style.cursor = isVert ? 'row-resize' : 'col-resize'
      }}
    >
      <div className="h-full w-full rounded-full bg-border/40 opacity-0 transition-opacity hover:opacity-100 active:opacity-100" />
    </div>
  )
}
