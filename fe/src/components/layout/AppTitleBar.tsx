// AppTitleBar (PRD §32): app name + Settings + custom window controls
// (minimize/maximize/close) for the frameless Electron window — web-term
// pattern. The bar is a drag region; interactive children opt out via
// WebkitAppRegion 'no-drag'. Harmless no-op in browser.

import { useEffect, useState } from 'react'
import type { CSSProperties } from 'react'
import { Minus, Square, Copy, X, Settings, TerminalSquare } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useSettingsDialog } from '@/features/settings/SettingsDialog'
import { desktop, isDesktop } from '@/lib/desktop-ipc'

// WebkitAppRegion isn't in React's CSSProperties; intersect to type it cleanly.
type DragStyle = CSSProperties & { WebkitAppRegion?: 'drag' | 'no-drag' }

const noDrag: DragStyle | undefined = isDesktop ? { WebkitAppRegion: 'no-drag' } : undefined
const dragRegion: DragStyle | undefined = isDesktop ? { WebkitAppRegion: 'drag' } : undefined

export function AppTitleBar() {
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
        <TerminalSquare className="size-4 text-muted-foreground" />
        <span>Tmux GUI</span>
      </div>
      <div className="flex items-center gap-1" data-no-drag style={noDrag}>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="Settings"
          onClick={() => useSettingsDialog.getState().setOpen(true)}
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
