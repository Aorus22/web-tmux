// WindowTabs (PRD §34): window-tab row, now rendered inside the title bar
// (web-term pattern) — one top bar. Click = select-window; right-click =
// context menu; trailing "+" creates a window. The root is the flex-1,
// horizontally-scrolling middle of the title bar; interactive children opt
// out of the Electron drag region via WebkitAppRegion 'no-drag'.

import { useState } from 'react'
import type { CSSProperties } from 'react'
import { Plus, X } from 'lucide-react'
import { toast } from 'sonner'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from '@/components/ui/context-menu'
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
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { selectActiveWindowId, selectWindows, useTmuxStore } from '@/stores/tmuxStore'
import { runCommand, shouldConfirm } from '@/lib/commands'
import { tmuxSocket } from '@/lib/socket'
import { isDesktop } from '@/lib/desktop-ipc'

// WebkitAppRegion isn't in React's CSSProperties; intersect to type it cleanly.
type DragStyle = CSSProperties & { WebkitAppRegion?: 'no-drag' }
const noDrag: DragStyle | undefined = isDesktop ? { WebkitAppRegion: 'no-drag' } : undefined

export function WindowTabs() {
  const windows = useTmuxStore(selectWindows)
  const activeWindowId = useTmuxStore(selectActiveWindowId)
  const [rename, setRename] = useState<{ id: string; name: string } | null>(null)
  const [kill, setKill] = useState<string | null>(null)
  const [renameValue, setRenameValue] = useState('')
  const [busy, setBusy] = useState(false)

  const newWindow = async () => {
    try {
      await runCommand(() => tmuxSocket.windowCreate())
      toast.success('Window created')
    } catch (e) {
      toast.error((e as Error).message)
    }
  }

  const doRename = async () => {
    if (!rename || !renameValue.trim()) return
    setBusy(true)
    try {
      await runCommand(() => tmuxSocket.windowRename(rename.id, renameValue.trim()))
      toast.success('Window renamed')
      setRename(null)
    } catch (e) {
      toast.error((e as Error).message)
    } finally {
      setBusy(false)
    }
  }

  const doKill = async (id: string) => {
    setBusy(true)
    try {
      await runCommand(() => tmuxSocket.windowKill(id))
      toast.success('Window closed')
      setKill(null)
    } catch (e) {
      toast.error((e as Error).message)
      setKill(null)
    } finally {
      setBusy(false)
    }
  }

  const move = async (id: string, offset: number) => {
    try {
      await runCommand(() => tmuxSocket.windowMove(id, offset))
    } catch (e) {
      toast.error((e as Error).message)
    }
  }

  const breakActivePane = async (id: string) => {
    try {
      await runCommand(() => tmuxSocket.windowBreakActive(id))
    } catch (e) {
      toast.error((e as Error).message)
    }
  }

  return (
    <div className="flex min-w-0 flex-1 items-center gap-0.5 overflow-x-auto no-scrollbar px-1">
      {windows.map((w) => {
        const active = w.id === activeWindowId
        return (
          <ContextMenu key={w.id}>
            <ContextMenuTrigger>
              <div
                role="button"
                tabIndex={0}
                style={noDrag}
                onClick={() => void tmuxSocket.windowSelect(w.id)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') void tmuxSocket.windowSelect(w.id)
                }}
                className={cn(
                  'group flex h-7 max-w-44 cursor-pointer items-center gap-1.5 rounded-md px-2.5 text-sm select-none',
                  active
                    ? 'bg-secondary text-secondary-foreground'
                    : 'text-muted-foreground hover:bg-muted/70 hover:text-foreground',
                )}
              >
                <span className="truncate">
                  {w.index}: {w.name}
                </span>
                <span
                  role="button"
                  tabIndex={-1}
                  style={noDrag}
                  aria-label={`Close ${w.name}`}
                  className="hidden rounded p-px hover:bg-foreground/10 group-hover:inline-flex"
                  onClick={(e) => {
                    e.stopPropagation()
                    if (shouldConfirm('window')) setKill(w.id)
                    else void doKill(w.id)
                  }}
                >
                  <X className="size-3.5" />
                </span>
              </div>
            </ContextMenuTrigger>
            <ContextMenuContent>
              <ContextMenuItem
                onClick={() => {
                  setRename({ id: w.id, name: w.name })
                  setRenameValue(w.name)
                }}
              >
                Rename Window
              </ContextMenuItem>
              <ContextMenuItem onClick={() => void move(w.id, -1)}>
                Move Left
              </ContextMenuItem>
              <ContextMenuItem onClick={() => void move(w.id, 1)}>
                Move Right
              </ContextMenuItem>
              <ContextMenuItem onClick={() => void breakActivePane(w.id)}>
                Break Active Pane
              </ContextMenuItem>
              <ContextMenuSeparator />
              <ContextMenuItem
                variant="destructive"
                onClick={() => {
                  if (shouldConfirm('window')) setKill(w.id)
                  else void doKill(w.id)
                }}
              >
                Kill Window
              </ContextMenuItem>
            </ContextMenuContent>
          </ContextMenu>
        )
      })}
      <Button
        variant="ghost"
        size="icon-xs"
        style={noDrag}
        aria-label="New window"
        onClick={() => void newWindow()}
      >
        <Plus className="size-3.5" />
      </Button>

      <Dialog open={!!rename} onOpenChange={(v) => !v && setRename(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rename Window</DialogTitle>
          </DialogHeader>
          <div className="space-y-2">
            <Label htmlFor="win-rename">New name</Label>
            <Input
              id="win-rename"
              value={renameValue}
              onChange={(e) => setRenameValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') void doRename()
              }}
              autoFocus
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setRename(null)}>
              Cancel
            </Button>
            <Button onClick={() => void doRename()} disabled={busy || !renameValue.trim()}>
              Rename
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog open={!!kill} onOpenChange={(v) => !v && setKill(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Close window?</AlertDialogTitle>
            <AlertDialogDescription>
              This closes the tmux window and all its panes.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction variant="destructive" onClick={() => kill && void doKill(kill)} disabled={busy}>
              Close
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
