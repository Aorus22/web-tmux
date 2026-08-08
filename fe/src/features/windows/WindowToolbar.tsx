// WindowToolbar (PRD §16): layout presets + next layout, above the workspace.

import { toast } from 'sonner'
import {
  ArrowRightLeft,
  Columns2,
  Rows2,
  SquareSplitHorizontal,
  SquareSplitVertical,
  LayoutGrid,
} from 'lucide-react'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { Button } from '@/components/ui/button'
import { selectActiveWindowId, useTmuxStore } from '@/stores/tmuxStore'
import { runCommand } from '@/lib/commands'
import { tmuxSocket } from '@/lib/socket'
import { cn } from '@/lib/utils'

export const LAYOUTS: { id: string; label: string; icon: typeof Columns2 }[] = [
  { id: 'even-horizontal', label: 'Even Horizontal', icon: Columns2 },
  { id: 'even-vertical', label: 'Even Vertical', icon: Rows2 },
  { id: 'main-horizontal', label: 'Main Horizontal', icon: SquareSplitHorizontal },
  { id: 'main-vertical', label: 'Main Vertical', icon: SquareSplitVertical },
  { id: 'tiled', label: 'Tiled', icon: LayoutGrid },
]

export function WindowToolbar() {
  const activeWindowId = useTmuxStore(selectActiveWindowId)

  const apply = async (layout: string) => {
    if (!activeWindowId) return
    try {
      await runCommand(() => tmuxSocket.windowLayout(activeWindowId, layout))
    } catch (e) {
      toast.error((e as Error).message)
    }
  }

  return (
    <div className="flex h-8 shrink-0 items-center gap-0.5 border-b bg-background/60 px-2">
      {LAYOUTS.map(({ id, label, icon: Icon }) => (
        <Tooltip key={id}>
          <TooltipTrigger
            render={
              <Button
                variant="ghost"
                size="icon-xs"
                aria-label={label}
                disabled={!activeWindowId}
                onClick={() => void apply(id)}
              >
                <Icon className="size-3.5" />
              </Button>
            }
          />
          <TooltipContent>{label}</TooltipContent>
        </Tooltip>
      ))}
      <div className="mx-1 h-4 w-px bg-border" />
      <Tooltip>
        <TooltipTrigger
          render={
            <Button
              variant="ghost"
              size="icon-xs"
              aria-label="Next Layout"
              disabled={!activeWindowId}
              onClick={() => void apply('next-layout')}
            >
              <ArrowRightLeft className={cn('size-3.5')} />
            </Button>
          }
        />
        <TooltipContent>Next Layout</TooltipContent>
      </Tooltip>
    </div>
  )
}
