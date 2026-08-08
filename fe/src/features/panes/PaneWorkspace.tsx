// PaneWorkspace (PRD §13): renders the active window's panes proportionally
// from tmux geometry. Zoomed panes fill the workspace; otherwise each pane is
// absolutely positioned via pixelRect and dividers are draggable (PRD §19).
// It also reports the container size to tmux as terminal.resize so the tmux
// window reflows with the browser window (PRD §26, §81).

import { useCallback, useEffect, useRef, useState } from 'react'
import type { CSSProperties } from 'react'
import type { TmuxPane, TmuxSnapshot } from '@/lib/tmux-types'
import { pixelRect, pxToColsRows } from '@/lib/geometry'
import { tmuxSocket } from '@/lib/socket'
import { PaneView } from './PaneView'
import { PaneResizeHandle } from './PaneResizeHandle'

interface Props {
  snapshot: TmuxSnapshot | null
  transport: string
  onSelectSession?: (name: string) => void
}

export function PaneWorkspace({ snapshot }: Props) {
  const containerRef = useRef<HTMLDivElement>(null)
  const [size, setSize] = useState({ w: 0, h: 0 })
  const resizeTimer = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    const ro = new ResizeObserver((entries) => {
      const r = entries[0]?.contentRect
      if (r) setSize({ w: r.width, h: r.height })
    })
    ro.observe(el)
    return () => ro.disconnect()
  }, [])

  // Report the workspace viewport to tmux (debounced ~100ms so a window drag
  // never spams resize commands). Sent on connect and on every browser resize.
  const sessionName = snapshot?.session.name
  useEffect(() => {
    if (!sessionName || size.w <= 0 || size.h <= 0) return
    const { cols, rows } = pxToColsRows(size.w, size.h)
    if (resizeTimer.current) clearTimeout(resizeTimer.current)
    resizeTimer.current = setTimeout(() => {
      tmuxSocket.terminalResize(cols, rows)
    }, 100)
    return () => {
      if (resizeTimer.current) clearTimeout(resizeTimer.current)
    }
  }, [size.w, size.h, sessionName])

  const activeWindowId = snapshot?.activeWindow
  const windowObj = snapshot?.windows.find((w) => w.id === activeWindowId)
  const panes = (snapshot?.panes ?? []).filter((p) => p.windowId === activeWindowId)

  // Zoom: only the zoomed pane is visible, full-size.
  const zoomed = panes.find((p) => p.zoomed)
  const visible = zoomed ? [zoomed] : panes

  const renderPane = useCallback(
    (p: TmuxPane, style: CSSProperties) => (
      <PaneView key={p.id} pane={p} isActive={p.active} style={style} />
    ),
    [],
  )

  if (!snapshot || !windowObj || panes.length === 0) {
    return (
      <div ref={containerRef} className="relative min-h-0 flex-1 overflow-hidden bg-black/50">
        <div className="flex h-full items-center justify-center text-xs text-muted-foreground">
          Connecting to tmux...
        </div>
      </div>
    )
  }

  const ww = windowObj.width || panes[0].width
  const wh = windowObj.height || panes[0].height
  const { w, h } = size

  const positioned = visible.map((p) => ({
    pane: p,
    rect: pixelRect(p.left, p.top, p.width, p.height, w, h, ww, wh),
  }))

  // Build divider handles between horizontally/vertically adjacent panes.
  const dividers: {
    key: string
    paneId: string
    direction: 'L' | 'R' | 'U' | 'D'
    style: CSSProperties
  }[] = []
  for (const a of positioned) {
    for (const b of positioned) {
      if (a.pane.id === b.pane.id) continue
      // vertical divider: same top/height band, a left of b
      if (
        Math.abs(a.rect.top - b.rect.top) < 4 &&
        a.rect.height === b.rect.height &&
        b.rect.left > a.rect.left
      ) {
        dividers.push({
          key: `v-${a.pane.id}-${b.pane.id}`,
          paneId: a.pane.id,
          direction: 'R',
          style: {
            left: a.rect.left + a.rect.width - 2,
            top: a.rect.top,
            height: a.rect.height,
            width: 4,
            cursor: 'col-resize',
          },
        })
      }
      // horizontal divider: same left/width band, a above b
      if (
        Math.abs(a.rect.left - b.rect.left) < 4 &&
        a.rect.width === b.rect.width &&
        b.rect.top > a.rect.top
      ) {
        dividers.push({
          key: `h-${a.pane.id}-${b.pane.id}`,
          paneId: a.pane.id,
          direction: 'D',
          style: {
            top: a.rect.top + a.rect.height - 2,
            left: a.rect.left,
            width: a.rect.width,
            height: 4,
            cursor: 'row-resize',
          },
        })
      }
    }
  }

  return (
    <div
      ref={containerRef}
      className="relative min-h-0 flex-1 overflow-hidden bg-[var(--term-bg)]"
    >
      {positioned.map(({ pane, rect }) =>
        renderPane(pane, {
          left: rect.left,
          top: rect.top,
          width: rect.width,
          height: rect.height,
        }),
      )}
      {!zoomed &&
        dividers.map((d) => (
          <PaneResizeHandle
            key={d.key}
            paneId={d.paneId}
            direction={d.direction}
            style={d.style}
          />
        ))}
    </div>
  )
}
