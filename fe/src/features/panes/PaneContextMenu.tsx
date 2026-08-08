// PaneContextMenu (PRD §39): Split Right / Split Down / Zoom / Swap / Break /
// Kill for one pane.

import { useState, type ReactNode } from 'react'
import { toast } from 'sonner'
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from '@/components/ui/context-menu'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ScrollArea } from '@/components/ui/scroll-area'
import type { TmuxPane } from '@/lib/tmux-types'
import { tmuxSocket } from '@/lib/socket'
import { runCommand, shouldConfirm } from '@/lib/commands'
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
  otherPanes: TmuxPane[]
  children: ReactNode
}

export function PaneContextMenu({ pane, otherPanes, children }: Props) {
  const [swapOpen, setSwapOpen] = useState(false)
  const [killOpen, setKillOpen] = useState(false)
  const [renameOpen, setRenameOpen] = useState(false)
  const [renameValue, setRenameValue] = useState('')
  const [busy, setBusy] = useState(false)

  const split = async (direction: 'horizontal' | 'vertical') => {
    try {
      await runCommand(() => tmuxSocket.paneSplit(pane.id, direction))
    } catch (e) {
      toast.error((e as Error).message)
    }
  }

  const zoom = async () => {
    try {
      await runCommand(() => tmuxSocket.paneZoom(pane.id))
    } catch (e) {
      toast.error((e as Error).message)
    }
  }

  const swap = async (otherId: string) => {
    setBusy(true)
    try {
      await runCommand(() => tmuxSocket.paneSwap(pane.id, otherId))
      setSwapOpen(false)
    } catch (e) {
      toast.error((e as Error).message)
    } finally {
      setBusy(false)
    }
  }

  const breakPane = async () => {
    try {
      await runCommand(() => tmuxSocket.paneBreak(pane.id))
    } catch (e) {
      toast.error((e as Error).message)
    }
  }

  const doKill = async () => {
    setBusy(true)
    try {
      await runCommand(() => tmuxSocket.paneKill(pane.id))
      setKillOpen(false)
    } catch (e) {
      toast.error((e as Error).message)
      setKillOpen(false)
    } finally {
      setBusy(false)
    }
  }

  const doRename = async () => {
    if (!renameValue.trim()) return
    setBusy(true)
    try {
      await runCommand(() => tmuxSocket.paneRename(pane.id, renameValue.trim()))
      toast.success('Pane renamed')
      setRenameOpen(false)
    } catch (e) {
      toast.error((e as Error).message)
    } finally {
      setBusy(false)
    }
  }

  const confirmKill = () => {
    if (shouldConfirm('pane')) setKillOpen(true)
    else void doKill()
  }

  return (
    <ContextMenu>
      <ContextMenuTrigger>{children}</ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem onClick={() => void split('horizontal')}>
          Split Right
        </ContextMenuItem>
        <ContextMenuItem onClick={() => void split('vertical')}>
          Split Down
        </ContextMenuItem>
        <ContextMenuItem
          onClick={() => {
            setRenameValue(pane.title || pane.currentCommand || '')
            setRenameOpen(true)
          }}
        >
          Rename Pane
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem onClick={() => void zoom()}>Zoom</ContextMenuItem>
        <ContextMenuItem onClick={() => setSwapOpen(true)} disabled={otherPanes.length === 0}>
          Swap
        </ContextMenuItem>
        <ContextMenuItem onClick={() => void breakPane()}>Break To Window</ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem variant="destructive" onClick={confirmKill}>
          Kill
        </ContextMenuItem>
      </ContextMenuContent>

      <Dialog open={swapOpen} onOpenChange={setSwapOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Swap pane {pane.id} with…</DialogTitle>
          </DialogHeader>
          <ScrollArea className="max-h-64">
            <div className="space-y-1">
              {otherPanes.map((p) => (
                <Button
                  key={p.id}
                  variant="outline"
                  className="w-full justify-start"
                  disabled={busy}
                  onClick={() => void swap(p.id)}
                >
                  <span className="font-mono text-xs">{p.id}</span>
                  <span className="ml-2 truncate text-xs text-muted-foreground">
                    {p.currentCommand || p.title || p.currentPath}
                  </span>
                </Button>
              ))}
              {otherPanes.length === 0 && (
                <p className="py-2 text-center text-xs text-muted-foreground">
                  No other panes in this window
                </p>
              )}
            </div>
          </ScrollArea>
        </DialogContent>
      </Dialog>

      <Dialog open={renameOpen} onOpenChange={setRenameOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rename Pane</DialogTitle>
          </DialogHeader>
          <div className="space-y-2">
            <Label htmlFor="pane-rename">New name</Label>
            <Input
              id="pane-rename"
              value={renameValue}
              onChange={(e) => setRenameValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') void doRename()
              }}
              autoFocus
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRenameOpen(false)}>
              Cancel
            </Button>
            <Button onClick={() => void doRename()} disabled={busy || !renameValue.trim()}>
              Rename
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

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
            <AlertDialogAction variant="destructive" onClick={() => void doKill()} disabled={busy}>
              Kill
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </ContextMenu>
  )
}
