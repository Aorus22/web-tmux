// PaneView — one pane: header + terminal, absolutely positioned.

import type { CSSProperties } from 'react'
import type { TmuxPane } from '@/lib/tmux-types'
import { cn } from '@/lib/utils'
import { TerminalView } from '@/features/terminal/TerminalView'
import { PaneHeader } from './PaneHeader'
import { PaneContextMenu } from './PaneContextMenu'
import { tmuxSocket } from '@/lib/socket'
import { useTmuxStore } from '@/stores/tmuxStore'
import { runCommand } from '@/lib/commands'
import { toast } from 'sonner'

// Stable empty array so the selector never returns a fresh reference.
const EMPTY: TmuxPane[] = []

interface Props {
  pane: TmuxPane
  isActive: boolean
  style?: CSSProperties
}

export function PaneView({ pane, isActive, style }: Props) {
  // Select the stable panes reference; filter outside the selector so the
  // selector result is referentially stable (avoids React #185 loop).
  const panes = useTmuxStore((s) => s.snapshot?.panes ?? EMPTY)
  const otherPanes = panes.filter(
    (p) => p.windowId === pane.windowId && p.id !== pane.id,
  )

  const zoom = async () => {
    try {
      await runCommand(() => tmuxSocket.paneZoom(pane.id))
    } catch (e) {
      toast.error((e as Error).message)
    }
  }

  return (
    <PaneContextMenu pane={pane} otherPanes={otherPanes}>
      <div
        className={cn(
          'absolute flex flex-col overflow-hidden rounded-sm border bg-[var(--term-bg)]',
          isActive ? 'border-border/70' : 'border-border/30',
        )}
        style={style}
        onMouseDown={() => {
          if (!pane.active) void tmuxSocket.paneSelect(pane.id)
        }}
      >
        <PaneHeader pane={pane} isActive={isActive} onZoom={() => void zoom()} />
        <div className="relative min-h-0 flex-1">
          <TerminalView paneId={pane.id} />
        </div>
      </div>
    </PaneContextMenu>
  )
}
