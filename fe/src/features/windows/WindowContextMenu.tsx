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
        <ContextMenuItem onSelect={onRename}>Rename Window</ContextMenuItem>
        <ContextMenuItem onSelect={onMoveLeft}>Move Left</ContextMenuItem>
        <ContextMenuItem onSelect={onMoveRight}>Move Right</ContextMenuItem>
        <ContextMenuItem onSelect={onBreakActive}>Break Active Pane</ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem variant="destructive" onSelect={onKill}>
          Kill Window
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  )
}
