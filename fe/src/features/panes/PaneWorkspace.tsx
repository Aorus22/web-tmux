// PaneWorkspace (PRD §13): renders the active window's panes proportionally
// from tmux geometry. Zoomed panes fill the workspace; otherwise each pane is
// absolutely positioned via pixelRect and dividers are draggable (PRD §19).
// It also reports the container size to tmux as terminal.resize so the tmux
// window reflows with the browser window (PRD §26, §81).

import { useCallback, useEffect, useRef, useState } from 'react'
import type { CSSProperties } from 'react'
import { AlertTriangle, Loader2, RefreshCw } from 'lucide-react'
import type { TmuxPane, TmuxSnapshot } from '@/lib/tmux-types'
import { pixelRect, pxToColsRows, closePaneGaps, CELL_W, CELL_H } from '@/lib/geometry'
import { closeSocket, ensureSocket, getOrCreateSocket, getSocket } from '@/lib/sockets'
import { useTmuxStore, type TransportState } from '@/stores/tmuxStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { terminalRegistry } from '@/features/terminal/terminalRegistry'
import { terminalBackground } from '@/features/settings/data/terminal-themes'
import { resolvedTerminalTheme } from '@/features/settings/data/ui-themes'
import { Button } from '@/components/ui/button'
import { PaneView } from './PaneView'
import { PaneResizeHandle } from './PaneResizeHandle'

interface Props {
  session: string
  snapshot: TmuxSnapshot | null
  transport: TransportState
}

// Reconnect helper for the "disconnected" empty state: tears down the tab's
// socket and opens a fresh one. No backend changes needed — the socket layer
// already handles connect/reconnect.
function reconnectSession(session: string) {
  useTmuxStore.getState().setTransport(session, 'connecting')
  closeSocket(session)
  ensureSocket(session)
}

// Actual window viewport for terminal.resize (PRD §26): scale a visible pane's
// real xterm cols/rows (registry.size, set by fit) to the whole window via its
// tmux cell geometry. A full/zoomed pane yields exactly its xterm size. Nominal
// pxToColsRows is only the fallback before any terminal registers.
function actualViewport(
  pane: TmuxPane | undefined,
  windowWidth: number,
  windowHeight: number,
  widthPx: number,
  heightPx: number,
): { cols: number; rows: number } {
  if (!pane) return pxToColsRows(widthPx, heightPx)
  const s = terminalRegistry.size(pane.id)
  if (s && pane.width > 0 && pane.height > 0 && windowWidth > 0 && windowHeight > 0) {
    return {
      cols: Math.max(2, Math.round((s.cols * windowWidth) / pane.width)),
      rows: Math.max(1, Math.round((s.rows * windowHeight) / pane.height)),
    }
  }
  return pxToColsRows(widthPx, heightPx)
}

export function PaneWorkspace({ session, snapshot, transport }: Props) {
  const containerRef = useRef<HTMLDivElement>(null)
  const [size, setSize] = useState({ w: 0, h: 0 })
  const resizeTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
  // Resync state: the last seen layout key, plus the visible pane ids for the
  // current layout (read inside the debounced timer so it always sees the
  // settled layout even after intermediate renders).
  const prevLayoutKeyRef = useRef<string | null>(null)
  const visibleIdsRef = useRef<string[]>([])
  // Latest viewport inputs (visible pane, window dims, container px) read by
  // the debounced timers at fire time so they see settled xterm sizes.
  const viewportInputRef = useRef<{
    pane: TmuxPane | undefined
    ww: number
    wh: number
    sw: number
    sh: number
  }>({ pane: undefined, ww: 0, wh: 0, sw: 0, sh: 0 })

  // Workspace background follows the app theme's terminal colors (xterm doesn't
  // paint its own background with allowTransparency, so this container is the
  // visible background). Resolves to the CSS variable without a mapped preset.
  const uiTheme = useSettingsStore((s) => s.uiTheme)
  const termBg = terminalBackground(resolvedTerminalTheme(uiTheme))

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

  const activeWindowId = snapshot?.activeWindow
  const windowObj = snapshot?.windows.find((w) => w.id === activeWindowId)
  const panes = (snapshot?.panes ?? []).filter((p) => p.windowId === activeWindowId)

  // Zoom: only the zoomed pane is visible, full-size.
  const zoomed = panes.find((p) => p.zoomed)
  const visible = zoomed ? [zoomed] : panes

  // Window dims in tmux cells (used for the layout key below and the pixel
  // mapping further down).
  const ww = windowObj?.width || panes[0]?.width || 0
  const wh = windowObj?.height || panes[0]?.height || 0

  // Keep the debounced timers' inputs current (they recompute the viewport at
  // fire time, when the panes' xterm fit has settled).
  viewportInputRef.current = { pane: visible[0], ww, wh, sw: size.w, sh: size.h }

  // Report the actual workspace viewport to tmux (debounced ~100ms so a window
  // drag never spams resize commands) — to THIS session's own socket, so every
  // open tab's control client tracks the window size. Re-fires when the
  // viewport changes (e.g. the first xterm fit) so the real size lands.
  const viewport = actualViewport(visible[0], ww, wh, size.w, size.h)
  useEffect(() => {
    if (size.w <= 0 || size.h <= 0) return
    if (resizeTimer.current) clearTimeout(resizeTimer.current)
    resizeTimer.current = setTimeout(() => {
      const sock = getSocket(session) ?? getOrCreateSocket(session)
      if (!sock) return
      const { pane, ww, wh, sw, sh } = viewportInputRef.current
      const vp = actualViewport(pane, ww, wh, sw, sh)
      sock.terminalResize(vp.cols, vp.rows)
    }, 100)
    return () => {
      if (resizeTimer.current) clearTimeout(resizeTimer.current)
    }
  }, [size.w, size.h, session, viewport.cols, viewport.rows])

  // Stable layout key: active window, window dims/layout, and each visible
  // pane's id, cell geometry, and zoom state. Terminal output and active-pane
  // changes leave the key unchanged, so the resync below only fires on real
  // topology/geometry changes (zoom, window switch, layout rebalance).
  const layoutKey = visible.length
    ? `${activeWindowId}|${ww}x${wh}|${windowObj?.layout ?? ''}|${visible
        .map((p) => `${p.id}:${p.left},${p.top},${p.width},${p.height},${p.zoomed ? 1 : 0}`)
        .join(';')}`
    : ''
  visibleIdsRef.current = visible.map((p) => p.id)

  // Full-screen resync after a layout/topology change: after a zoom/unzoom,
  // window switch, or tmux layout change the panes only receive incremental
  // output, so the xterm buffer can keep stale/overlapping pixels from the
  // previous layout. On a layout-key change: wait ~150ms for the panes' xterm
  // fit to settle, report the actual viewport to tmux, then wait again before
  // force-capturing every visible pane so capture-pane reflects the new tmux
  // geometry. invalidateSnapshot is re-armed first so the capture replaces the
  // buffer (not dropped as a duplicate). All messages go to THIS session's own
  // socket, never the active-session proxy. Skipped on first mount: each
  // TerminalView already requests its own initial capture.
  useEffect(() => {
    if (!layoutKey) return
    if (prevLayoutKeyRef.current === null) {
      prevLayoutKeyRef.current = layoutKey
      return
    }
    if (layoutKey === prevLayoutKeyRef.current) return
    prevLayoutKeyRef.current = layoutKey
    const resizeT = setTimeout(() => {
      const sock = getSocket(session) ?? getOrCreateSocket(session)
      if (!sock) return
      const { pane, ww, wh, sw, sh } = viewportInputRef.current
      const vp = actualViewport(pane, ww, wh, sw, sh)
      sock.terminalResize(vp.cols, vp.rows)
    }, 150)
    const captureT = setTimeout(() => {
      const sock = getSocket(session) ?? getOrCreateSocket(session)
      if (!sock) return
      for (const paneId of visibleIdsRef.current) {
        if (!terminalRegistry.has(paneId)) continue
        terminalRegistry.invalidateSnapshot(paneId)
        sock.terminalCapture(paneId)
      }
    }, 325)
    return () => {
      clearTimeout(resizeT)
      clearTimeout(captureT)
    }
  }, [layoutKey, session])

  const renderPane = useCallback(
    (p: TmuxPane, style: CSSProperties) => (
      <PaneView key={p.id} pane={p} isActive={p.active} style={style} />
    ),
    [],
  )

  if (!snapshot || !windowObj || panes.length === 0) {
    return (
      <div
        ref={containerRef}
        className="relative min-h-0 flex-1 overflow-hidden bg-[var(--term-bg)]"
        style={{ background: termBg }}
      >
        <div className="flex h-full flex-col items-center justify-center gap-3 px-6 text-center">
          {transport === 'disconnected' ? (
            <>
              <AlertTriangle className="size-6 text-destructive" />
              <div>
                <p className="text-sm font-medium text-foreground">Connection lost</p>
                <p className="mt-0.5 text-xs text-muted-foreground">
                  The connection to {session} dropped. Reconnect to keep working.
                </p>
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={() => reconnectSession(session)}
              >
                Reconnect
              </Button>
            </>
          ) : transport === 'reconnecting' ? (
            <>
              <RefreshCw className="size-5 animate-pulse text-amber-500" />
              <p className="text-xs text-muted-foreground">
                Reconnecting to {session}...
              </p>
            </>
          ) : (
            <>
              <Loader2 className="size-5 animate-spin text-muted-foreground" />
              <p className="text-xs text-muted-foreground">
                {transport === 'connected'
                  ? `Waiting for ${session}...`
                  : `Connecting to ${session}...`}
              </p>
            </>
          )}
        </div>
      </div>
    )
  }

  const { w, h } = size

  // tmux leaves a 1-cell border strip between panes (e.g. pane A spans
  // [0,50), pane B starts at 51). Absorb those cells so panes tile
  // edge-to-edge instead of showing workspace background between them.
  const tiled = closePaneGaps(visible)

  const positioned = tiled.map((p) => ({
    pane: p,
    rect: pixelRect(p.left, p.top, p.width, p.height, w, h, ww, wh),
  }))

  // Build divider handles between panes that share an edge. A divider exists
  // for ANY pair whose ranges overlap along the shared edge — not just
  // same-height/same-width bands — so T layouts (full-height left pane next
  // to a split right side) get a working vertical handle too. Each handle
  // spans the overlapping portion of the edge. Adjacency is checked in CELL
  // space (exact integers after closePaneGaps): only immediate neighbors get
  // a divider — a looser `b.left > a.left` test also matched non-adjacent
  // pairs (e.g. %0 and %2 with %1 in between), stacking two handles.
  const dividers: {
    key: string
    paneId: string
    direction: 'L' | 'R' | 'U' | 'D'
    style: CSSProperties
    cellPx: { w: number; h: number }
  }[] = []
  for (const a of positioned) {
    for (const b of positioned) {
      if (a.pane.id === b.pane.id) continue
      // Vertical divider: b immediately right of a, vertical ranges overlap.
      if (
        b.pane.left === a.pane.left + a.pane.width &&
        a.pane.top < b.pane.top + b.pane.height &&
        b.pane.top < a.pane.top + a.pane.height
      ) {
        const top = Math.max(a.rect.top, b.rect.top)
        const bottom = Math.min(
          a.rect.top + a.rect.height,
          b.rect.top + b.rect.height,
        )
        dividers.push({
          key: `v-${a.pane.id}-${b.pane.id}`,
          paneId: a.pane.id,
          direction: 'R',
          style: {
            left: a.rect.left + a.rect.width - 2,
            top,
            height: bottom - top,
            width: 4,
            cursor: 'col-resize',
          },
          // The dragged pane's actual rendered cell size — makes the divider
          // track the pointer exactly regardless of zoom/font/theme.
          cellPx: {
            w: a.pane.width > 0 ? a.rect.width / a.pane.width : CELL_W,
            h: a.pane.height > 0 ? a.rect.height / a.pane.height : CELL_H,
          },
        })
      }
      // Horizontal divider: b immediately below a, horizontal ranges overlap.
      if (
        b.pane.top === a.pane.top + a.pane.height &&
        a.pane.left < b.pane.left + b.pane.width &&
        b.pane.left < a.pane.left + a.pane.width
      ) {
        const left = Math.max(a.rect.left, b.rect.left)
        const right = Math.min(
          a.rect.left + a.rect.width,
          b.rect.left + b.rect.width,
        )
        dividers.push({
          key: `h-${a.pane.id}-${b.pane.id}`,
          paneId: a.pane.id,
          direction: 'D',
          style: {
            top: a.rect.top + a.rect.height - 2,
            left,
            width: right - left,
            height: 4,
            cursor: 'row-resize',
          },
          cellPx: {
            w: a.pane.width > 0 ? a.rect.width / a.pane.width : CELL_W,
            h: a.pane.height > 0 ? a.rect.height / a.pane.height : CELL_H,
          },
        })
      }
    }
  }

  return (
    <div
      ref={containerRef}
      className="relative min-h-0 flex-1 overflow-hidden bg-[var(--term-bg)]"
      style={{ background: termBg }}
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
            cellPx={d.cellPx}
          />
        ))}
    </div>
  )
}
