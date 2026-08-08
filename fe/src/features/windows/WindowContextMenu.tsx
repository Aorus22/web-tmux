// WindowContextMenu (PRD §39): shared menu items for window tabs.

import type { ReactNode } from 'react'
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from '@/components/ui/context-menu'

interface Props {
  windowId: string
  onRename: () => void
  onMoveLeft: () => void
  onMoveRight: () => void
  onBreakActive: () => void
  onKill: () => void
  children: ReactNode
}

export function WindowContextMenu({
  windowId: _windowId,
  onRename,
  onMoveLeft,
  onMoveRight,
  onBreakActive,
  onKill,
  children,
}: Props) {
  return (
    <ContextMenu>
      <ContextMenuTrigger>{children}</ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onClick={onRename}>Rename Window</ContextMenuItem>
        <ContextMenuItem onClick={onMoveLeft}>Move Left</ContextMenuItem>
        <ContextMenuItem onClick={onMoveRight}>Move Right</ContextMenuItem>
        <ContextMenuItem onClick={onBreakActive}>Break Active Pane</ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem variant="destructive" onClick={onKill}>
          Kill Window
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  )
}
