// PaneWorkspace (PRD §13): renders the active window's panes proportionally
// from tmux geometry. Zoomed panes fill the workspace; otherwise each pane is
// absolutely positioned via pixelRect and dividers are draggable (PRD §19).
// It also reports the container size to tmux as terminal.resize so the tmux
// window reflows with the browser window (PRD §26, §81).

import { useCallback, useEffect, useRef, useState } from 'react'
import type { CSSProperties } from 'react'
import { AlertTriangle, Loader2, RefreshCw } from 'lucide-react'
import type { TmuxPane, TmuxSnapshot } from '@/lib/tmux-types'
import { pixelRect, pxToColsRows } from '@/lib/geometry'
import { closeSocket, ensureSocket, getSocket } from '@/lib/sockets'
import { useTmuxStore, type TransportState } from '@/stores/tmuxStore'
import { useSettingsStore } from '@/stores/settingsStore'
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

export function PaneWorkspace({ session, snapshot, transport }: Props) {
  const containerRef = useRef<HTMLDivElement>(null)
  const [size, setSize] = useState({ w: 0, h: 0 })
  const resizeTimer = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Workspace background follows the selected terminal theme (xterm doesn't
  // paint its own background with allowTransparency, so this container is the
  // visible background). With no explicit terminal theme it follows the app
  // UI theme's mapped terminal preset; resolves to the CSS variable otherwise.
  const uiTheme = useSettingsStore((s) => s.uiTheme)
  const terminalTheme = useSettingsStore((s) => s.terminalTheme)
  const termBg = terminalBackground(resolvedTerminalTheme(uiTheme, terminalTheme))

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
  // never spams resize commands) — to THIS session's own socket, so every
  // open tab's control client tracks the window size.
  useEffect(() => {
    if (size.w <= 0 || size.h <= 0) return
    const { cols, rows } = pxToColsRows(size.w, size.h)
    if (resizeTimer.current) clearTimeout(resizeTimer.current)
    resizeTimer.current = setTimeout(() => {
      getSocket(session)?.terminalResize(cols, rows)
    }, 100)
    return () => {
      if (resizeTimer.current) clearTimeout(resizeTimer.current)
    }
  }, [size.w, size.h, session])

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
          />
        ))}
    </div>
  )
}
