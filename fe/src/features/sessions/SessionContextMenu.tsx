// SessionContextMenu (PRD §39): New Window / Rename / Kill Session.

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
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { runCommand, shouldConfirm } from '@/lib/commands'
import { tmuxSocket } from '@/lib/socket'
import { useCreateSessionDialog } from './CreateSessionDialog'

interface Props {
  sessionName: string
  onCreated?: (name: string) => void
  children: ReactNode
}

export function SessionContextMenu({ sessionName, onCreated, children }: Props) {
  const [renameOpen, setRenameOpen] = useState(false)
  const [killOpen, setKillOpen] = useState(false)
  const [newName, setNewName] = useState('')
  const [busy, setBusy] = useState(false)

  const doRename = async () => {
    if (!newName.trim()) return
    setBusy(true)
    try {
      await runCommand(() => tmuxSocket.sessionRename(newName.trim()))
      toast.success(`Session renamed to "${newName.trim()}"`)
      setRenameOpen(false)
      onCreated?.(newName.trim())
    } catch (e) {
      toast.error((e as Error).message)
    } finally {
      setBusy(false)
    }
  }

  const doKill = async () => {
    setBusy(true)
    try {
      await runCommand(() => tmuxSocket.sessionKill())
      toast.success(`Session "${sessionName}" killed`)
      setKillOpen(false)
    } catch (e) {
      toast.error((e as Error).message)
      setKillOpen(false)
    } finally {
      setBusy(false)
    }
  }

  const confirmKill = () => {
    if (shouldConfirm('session')) setKillOpen(true)
    else void doKill()
  }

  return (
    <ContextMenu>
      <ContextMenuTrigger>{children}</ContextMenuTrigger>
      <ContextMenuContent>
        <ContextMenuItem
          onSelect={() => useCreateSessionDialog.getState().setOpen(true)}
        >
          New Session
        </ContextMenuItem>
        <ContextMenuItem
          onSelect={() => {
            setNewName(sessionName)
            setRenameOpen(true)
          }}
        >
          Rename
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem variant="destructive" onSelect={confirmKill}>
          Kill Session
        </ContextMenuItem>
      </ContextMenuContent>

      <Dialog open={renameOpen} onOpenChange={setRenameOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rename Session</DialogTitle>
          </DialogHeader>
          <div className="space-y-2">
            <Label htmlFor="rename-session">New name</Label>
            <Input
              id="rename-session"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
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
            <Button onClick={() => void doRename()} disabled={busy || !newName.trim()}>
              Rename
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog open={killOpen} onOpenChange={setKillOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Kill session "{sessionName}"?</AlertDialogTitle>
            <AlertDialogDescription>
              This terminates the tmux session and all processes inside it.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={() => void doKill()}
              disabled={busy}
            >
              Kill
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </ContextMenu>
  )
}
