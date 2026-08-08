// PaneHeader (PRD §35): current path, pane ID, and an always-visible toolbar
// (Split right / Split down / Zoom / Kill) with tooltips. Zoom only shows when
// the window has another pane to toggle with. Double-click zooms (PRD §20).

import { Maximize2, SplitSquareHorizontal, SplitSquareVertical, X } from 'lucide-react'
import type { TmuxPane } from '@/lib/tmux-types'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { tmuxSocket } from '@/lib/socket'
import { runCommand } from '@/lib/commands'
import { toast } from 'sonner'
import { shouldConfirm } from '@/lib/commands'
import { useState } from 'react'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'

interface Props {
  pane: TmuxPane
  isActive: boolean
  // Zoom toggles between two+ panes; hide it for a single-pane window.
  canZoom?: boolean
  onZoom: () => void
}

export function PaneHeader({ pane, isActive, canZoom = true, onZoom }: Props) {
  const [killOpen, setKillOpen] = useState(false)

  const split = async (direction: 'horizontal' | 'vertical' = 'vertical') => {
    try {
      await runCommand(() => tmuxSocket.paneSplit(pane.id, direction))
    } catch (e) {
      toast.error((e as Error).message)
    }
  }

  const doKill = async () => {
    try {
      await runCommand(() => tmuxSocket.paneKill(pane.id))
      setKillOpen(false)
    } catch (e) {
      toast.error((e as Error).message)
      setKillOpen(false)
    }
  }

  const confirmKill = () => {
    if (shouldConfirm('pane')) setKillOpen(true)
    else void doKill()
  }

  return (
    <>
      <div
        onDoubleClick={onZoom}
        className={cn(
          'flex h-7 shrink-0 items-center gap-1.5 border-b px-2 text-[11px]',
          isActive ? 'bg-secondary/50' : 'bg-background/60',
        )}
      >
        <span className="min-w-0 flex-1 truncate text-muted-foreground">{pane.currentPath}</span>
        <span className="shrink-0 font-mono text-muted-foreground/70">
          {pane.id}
        </span>
        <span className="flex shrink-0 items-center gap-0.5">
          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  variant="ghost"
                  size="icon-xs"
                  aria-label="Split right"
                  onClick={(e) => {
                    e.stopPropagation()
                    void split('horizontal')
                  }}
                >
                  <SplitSquareHorizontal className="size-3" />
                </Button>
              }
            />
            <TooltipContent>Split right</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  variant="ghost"
                  size="icon-xs"
                  aria-label="Split down"
                  onClick={(e) => {
                    e.stopPropagation()
                    void split()
                  }}
                >
                  <SplitSquareVertical className="size-3" />
                </Button>
              }
            />
            <TooltipContent>Split down</TooltipContent>
          </Tooltip>
          {canZoom && (
            <Tooltip>
              <TooltipTrigger
                render={
                  <Button
                    variant="ghost"
                    size="icon-xs"
                    aria-label="Zoom"
                    onClick={(e) => {
                      e.stopPropagation()
                      onZoom()
                    }}
                  >
                    <Maximize2 className="size-3" />
                  </Button>
                }
              />
              <TooltipContent>Zoom</TooltipContent>
            </Tooltip>
          )}
          <Tooltip>
            <TooltipTrigger
              render={
                <Button
                  variant="ghost"
                  size="icon-xs"
                  aria-label="Kill pane"
                  className="hover:text-destructive"
                  onClick={(e) => {
                    e.stopPropagation()
                    confirmKill()
                  }}
                >
                  <X className="size-3" />
                </Button>
              }
            />
            <TooltipContent>Kill pane</TooltipContent>
          </Tooltip>
        </span>
      </div>

      <AlertDialog open={killOpen} onOpenChange={setKillOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Kill pane {pane.id}?</AlertDialogTitle>
            <AlertDialogDescription>
              This terminates the shell running in this pane.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction variant="destructive" onClick={() => void doKill()}>
              Kill
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  )
}
