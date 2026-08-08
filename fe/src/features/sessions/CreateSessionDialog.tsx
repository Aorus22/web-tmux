// CreateSessionDialog (PRD §14): Name* / Working directory / Initial command.
// Name is validated by the backend (tmux session name rules).

import { useEffect, useState } from 'react'
import { create } from 'zustand'
import { toast } from 'sonner'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { runCommand } from '@/lib/commands'
import { tmuxSocket } from '@/lib/socket'

// Internal open-state store: App.tsx renders the dialog always; AppSidebar and
// EmptyState open it via getState().setOpen(true).
export const useCreateSessionDialog = create<{
  open: boolean
  setOpen: (open: boolean) => void
}>((set) => ({ open: false, setOpen: (open) => set({ open }) }))

interface Props {
  onCreated: (name: string) => void
}

export function CreateSessionDialog({ onCreated }: Props) {
  const open = useCreateSessionDialog((s) => s.open)
  const setOpen = useCreateSessionDialog((s) => s.setOpen)
  const [name, setName] = useState('')
  const [cwd, setCwd] = useState('')
  const [command, setCommand] = useState('')
  const [busy, setBusy] = useState(false)

  const reset = () => {
    setName('')
    setCwd('')
    setCommand('')
    setBusy(false)
  }

  // A hung command (e.g. WS disconnected) must never leave the Create button
  // permanently disabled: clear the busy flag every time the dialog opens.
  useEffect(() => {
    if (open) setBusy(false)
  }, [open])

  const submit = async () => {
    if (!name.trim()) return
    setBusy(true)
    try {
      await runCommand(() =>
        tmuxSocket.sessionCreate(name.trim(), cwd.trim() || undefined, command.trim() || undefined),
      )
      toast.success(`Session "${name.trim()}" created`)
      const created = name.trim()
      reset()
      setOpen(false)
      onCreated(created)
    } catch (e) {
      toast.error((e as Error).message)
    } finally {
      setBusy(false)
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(v) => {
        setOpen(v)
        if (!v) reset()
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Create Session</DialogTitle>
          <DialogDescription>Start a new tmux session.</DialogDescription>
        </DialogHeader>
        <div className="space-y-3">
          <div className="space-y-1.5">
            <Label htmlFor="sess-name">Name *</Label>
            <Input
              id="sess-name"
              placeholder="dev"
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoFocus
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="sess-cwd">Working directory</Label>
            <Input
              id="sess-cwd"
              placeholder="~/projects (optional)"
              value={cwd}
              onChange={(e) => setCwd(e.target.value)}
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="sess-cmd">Initial command</Label>
            <Input
              id="sess-cmd"
              placeholder="e.g. nvim (optional)"
              value={command}
              onChange={(e) => setCommand(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') void submit()
              }}
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button onClick={() => void submit()} disabled={busy || !name.trim()}>
            Create
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
