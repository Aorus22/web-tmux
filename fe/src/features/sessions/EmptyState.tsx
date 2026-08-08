// EmptyState (PRD §37): no tmux sessions — prompt to create the first one.
// Never auto-creates sessions silently.

import { TerminalSquare } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useCreateSessionDialog } from './CreateSessionDialog'

export function EmptyState({ onCreate }: { onCreate: () => void }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
      <TerminalSquare className="size-10 text-muted-foreground" />
      <div>
        <h2 className="text-base font-medium">No tmux sessions</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Create your first session to get started.
        </p>
      </div>
      <Button
        variant="default"
        onClick={() => {
          useCreateSessionDialog.getState().setOpen(true)
          onCreate()
        }}
      >
        Create Session
      </Button>
    </div>
  )
}
