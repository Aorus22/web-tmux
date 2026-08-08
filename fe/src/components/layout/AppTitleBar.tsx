// AppTitleBar (PRD §32): one top bar — sidebar toggle, app identity, window
// tabs (web-term pattern) and Settings + custom window controls
// (minimize/maximize/close) for the frameless Electron window. The bar is a
// drag region; interactive children (toggle, tabs, controls) opt out via
// WebkitAppRegion 'no-drag'. Harmless no-op in browser.

import { useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { Minus, Square, Copy, X, Settings, TerminalSquare, PanelLeft } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useAppStore } from '@/stores/appStore'
import { WindowTabs } from '@/features/windows/WindowTabs'
import { desktop, isDesktop } from '@/lib/desktop-ipc'

// WebkitAppRegion isn't in React's CSSProperties; intersect to type it cleanly.
type DragStyle = CSSProperties & { WebkitAppRegion?: 'drag' | 'no-drag' }

const noDrag: DragStyle | undefined = isDesktop ? { WebkitAppRegion: 'no-drag' } : undefined
const dragRegion: DragStyle | undefined = isDesktop ? { WebkitAppRegion: 'drag' } : undefined

export function AppTitleBar() {
  const activeSession = useAppStore((s) => s.activeSession)
  const [windowState, setWindowState] = useState<'maximized' | 'restored'>('restored')

  useEffect(() => {
    if (!isDesktop) return
    let alive = true
    desktop.window.getState().then((s) => alive && setWindowState(s))
    const unsubscribe = desktop.window.onStateChange(setWindowState)
    return () => {
      alive = false
      unsubscribe()
    }
  }, [])

  return (
    <header
      data-drag-region
      style={dragRegion}
      className="flex h-11 shrink-0 items-center justify-between border-b bg-background/80 px-3 backdrop-blur"
    >
      <div className="flex items-center gap-2 text-sm font-medium">
        {/* Sidebar toggle stays clickable in the frameless window: the title
            bar is a drag region, so the button opts out via no-drag. */}
        <div data-no-drag style={noDrag}>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label="Toggle sidebar"
            onClick={() => useAppStore.getState().toggleSidebar()}
          >
            <PanelLeft className="size-4" />
          </Button>
        </div>
        <TerminalSquare className="size-4 text-muted-foreground" />
        <span>Tmux GUI</span>
      </div>
      {/* Divider before the window tabs (matches the separator used before the
          window controls). Window tabs take the flex-1 middle when a session
          tab is open; the header stays just identity + controls otherwise. */}
      {activeSession && (
        <>
          <div
            className="ml-1 h-5 w-px shrink-0 border-l border-border pl-1"
            aria-hidden
          />
          <WindowTabs />
        </>
      )}
      <div className="flex items-center gap-1" data-no-drag style={noDrag}>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Settings"
          onClick={() => {
            // Dedicated Settings page (not a modal): unfocus the current tab
            // and show the page; tabs stay open underneath.
            useAppStore.getState().setActiveSession(null)
            useAppStore.getState().setSidebarPage('settings')
          }}
        >
          <Settings className="size-4" />
        </Button>
        {isDesktop && (
          <div className="ml-1 flex items-center border-l border-border pl-1">
            <button
              type="button"
              title="Minimize"
              aria-label="Minimize"
              onClick={() => desktop.window.minimize()}
              className="flex h-8 w-10 items-center justify-center rounded-md text-foreground/70 transition-colors hover:bg-muted hover:text-foreground"
            >
              <Minus className="size-4 stroke-[1.5]" />
            </button>
            <button
              type="button"
              title={windowState === 'maximized' ? 'Restore' : 'Maximize'}
              aria-label={windowState === 'maximized' ? 'Restore' : 'Maximize'}
              onClick={() => desktop.window.maximize()}
              className="flex h-8 w-10 items-center justify-center rounded-md text-foreground/70 transition-colors hover:bg-muted hover:text-foreground"
            >
              {windowState === 'maximized' ? (
                <Copy className="size-3.5 stroke-[1.5]" />
              ) : (
                <Square className="size-3.5 stroke-[1.5]" />
              )}
            </button>
            <button
              type="button"
              title="Close"
              aria-label="Close"
              onClick={() => desktop.window.close()}
              className="flex h-8 w-10 items-center justify-center rounded-md text-foreground/70 transition-colors hover:bg-destructive hover:text-destructive-foreground"
            >
              <X className="size-4 stroke-[1.5]" />
            </button>
          </div>
        )}
      </div>
    </header>
  )
}
