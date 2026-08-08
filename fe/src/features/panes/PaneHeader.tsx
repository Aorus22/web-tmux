// PaneHeader (PRD §35): active dot, current command, cwd, pane ID, and a
// hover toolbar (Split / Zoom / Kill). Double-click zooms (PRD §20).

import { Maximize2, SplitSquareVertical, X } from 'lucide-react'
import type { TmuxPane } from '@/lib/tmux-types'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
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
  onZoom: () => void
}

export function PaneHeader({ pane, isActive, onZoom }: Props) {
  const [killOpen, setKillOpen] = useState(false)

  const split = async () => {
    try {
      await runCommand(() => tmuxSocket.paneSplit(pane.id, 'vertical'))
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
          'group/header flex h-7 shrink-0 items-center gap-1.5 border-b px-2 text-[11px]',
          isActive ? 'bg-secondary/50' : 'bg-background/60',
        )}
      >
        <span
          className={cn(
            'size-1.5 shrink-0 rounded-full',
            isActive ? 'bg-emerald-500' : 'bg-muted-foreground/40',
          )}
        />
        <span className="max-w-36 truncate font-medium text-foreground/90">
          {pane.currentCommand || pane.title || 'shell'}
        </span>
        <span className="max-w-40 truncate text-muted-foreground">{pane.currentPath}</span>
        <span className="ml-auto shrink-0 font-mono text-muted-foreground/70">
          {pane.id}
        </span>
        <span className="hidden shrink-0 items-center gap-0.5 group-hover/header:flex">
          <Button
            variant="ghost"
            size="icon-xs"
            aria-label="Split Down"
            onClick={(e) => {
              e.stopPropagation()
              void split()
            }}
          >
            <SplitSquareVertical className="size-3" />
          </Button>
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
