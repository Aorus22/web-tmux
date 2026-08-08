// SelectSessionView — empty state shown when no session tab is open (the app
// is NOT connecting to anything; the user just hasn't picked a session yet).
// Lists available sessions so one click opens a tab.

import { SquareTerminal } from 'lucide-react'
import { useAppStore } from '@/stores/appStore'
import { cn } from '@/lib/utils'
import type { TmuxTree } from '@/lib/tmux-types'

interface Props {
  tree: TmuxTree
}

export function SelectSessionView({ tree }: Props) {
  const openSession = useAppStore((s) => s.openSession)

  return (
    <div className="flex h-full flex-col items-center justify-center px-6 py-10 text-center">
      <div className="flex flex-col items-center gap-4">
        <div className="flex size-12 items-center justify-center rounded-xl border bg-muted/50 text-muted-foreground">
          <SquareTerminal className="size-6" />
        </div>
        <div>
          <h2 className="text-base font-medium text-foreground">No session open</h2>
          <p className="mx-auto mt-1 max-w-sm text-sm leading-relaxed text-muted-foreground">
            Pick a session to open it as a tab. Closing a tab never kills the
            tmux session — it keeps running in the background.
          </p>
        </div>

        {tree.sessions.length > 0 && (
          <div className="mt-2 w-full max-w-md">
            <p className="mb-2 text-left text-xs font-medium text-muted-foreground">
              Sessions
            </p>
            <div className="grid grid-cols-2 gap-2">
              {tree.sessions.map((node) => (
                <button
                  key={node.session.name}
                  type="button"
                  onClick={() => openSession(node.session.name)}
                  className={cn(
                    'flex items-center gap-2 rounded-lg border border-border/60 bg-background px-3 py-2.5 text-left',
                    'text-sm text-foreground transition-colors hover:border-border hover:bg-muted/60',
                  )}
                >
                  <span className="min-w-0 flex-1 truncate font-medium">
                    {node.session.name}
                  </span>
                  <span className="shrink-0 rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
                    {node.session.windows}w
                  </span>
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
