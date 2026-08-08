// ErrorState (PRD §4.2): tmux missing — clear error page instead of a broken UI.

import { AlertTriangle } from 'lucide-react'
import { Button } from '@/components/ui/button'

export function ErrorState({ onRetry }: { onRetry: () => void }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
      <AlertTriangle className="size-10 text-destructive" />
      <div>
        <h2 className="text-base font-medium">Tmux is not installed</h2>
        <p className="mt-1 max-w-sm text-sm text-muted-foreground">
          Tmux GUI requires tmux (&gt;= 3.2) on Linux. Install it and try again.
        </p>
      </div>
      <Button variant="outline" onClick={onRetry}>
        Retry
      </Button>
    </div>
  )
}
