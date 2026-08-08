// AppSidebar (PRD §33): collapsible session → window → pane tree.
// Sessions come from the TanStack Query tree (polled 1.5s); clicking a session
// switches the active connection.

import { useState } from 'react'
import { ChevronRight, Plus, RefreshCw } from 'lucide-react'
import type { TmuxTree } from '@/lib/tmux-types'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { useAppStore } from '@/stores/appStore'
import { SessionContextMenu } from '@/features/sessions/SessionContextMenu'
import { useCreateSessionDialog } from '@/features/sessions/CreateSessionDialog'

interface Props {
  tree: TmuxTree
  activeSession: string | null
  onSelectSession: (name: string) => void
  onRefetch: () => void
}

export function AppSidebar({ tree, activeSession, onSelectSession, onRefetch }: Props) {
  const sidebarOpen = useAppStore((s) => s.sidebarOpen)
  const [expanded, setExpanded] = useState<Set<string>>(new Set())

  const toggle = (name: string) => {
    setExpanded((prev) => {
      const next = new Set(prev)
      if (next.has(name)) next.delete(name)
      else next.add(name)
      return next
    })
  }

  return (
    <aside
      className={cn(
        'shrink-0 overflow-hidden border-r bg-background transition-[width] duration-200',
        sidebarOpen ? 'w-60' : 'w-0',
      )}
    >
      <div className="flex h-full w-60 flex-col">
        <div className="flex h-9 items-center justify-between px-3">
          <span className="text-xs font-medium text-muted-foreground">Sessions</span>
          <div className="flex items-center gap-0.5">
            <Button
              variant="ghost"
              size="icon-xs"
              aria-label="Refresh"
              onClick={onRefetch}
            >
              <RefreshCw className="size-3" />
            </Button>
            <Button
              variant="ghost"
              size="icon-xs"
              aria-label="New Session"
              onClick={() => useCreateSessionDialog.getState().setOpen(true)}
            >
              <Plus className="size-3" />
            </Button>
          </div>
        </div>
        <ScrollArea className="min-h-0 flex-1">
          <div className="px-1.5 pb-3">
            {tree.sessions.length === 0 && (
              <p className="px-2 py-4 text-xs text-muted-foreground">No sessions</p>
            )}
            {tree.sessions.map((node) => {
              const isActive = node.session.name === activeSession
              const isExpanded = expanded.has(node.session.name)
              return (
                <SessionContextMenu
                  key={node.session.name}
                  sessionName={node.session.name}
                  onCreated={() => onSelectSession(node.session.name)}
                >
                  <div className="mb-0.5">
                    <button
                      type="button"
                      onClick={() => onSelectSession(node.session.name)}
                      className={cn(
                        'flex w-full items-center gap-1 rounded-md px-2 py-1.5 text-left text-sm transition-colors',
                        isActive
                          ? 'bg-accent text-accent-foreground'
                          : 'hover:bg-muted/60',
                      )}
                    >
                      <ChevronRight
                        className={cn(
                          'size-3.5 shrink-0 text-muted-foreground transition-transform',
                          isExpanded && 'rotate-90',
                        )}
                        onClick={(e) => {
                          e.stopPropagation()
                          toggle(node.session.name)
                        }}
                      />
                      <span className="truncate font-medium">{node.session.name}</span>
                      <span className="ml-auto rounded bg-muted px-1 text-[10px] text-muted-foreground">
                        {node.session.windows}
                      </span>
                    </button>
                    {isExpanded && (
                      <div className="ml-4 border-l border-border pl-2">
                        {node.windows.map((wn) => (
                          <div key={wn.window.id} className="py-0.5">
                            <div className="flex items-center gap-1 px-1 text-xs text-foreground/80">
                              <span className="truncate">
                                {wn.window.index}: {wn.window.name}
                              </span>
                            </div>
                            {(wn.panes ?? []).map((p) => (
                              <div
                                key={p.id}
                                className="flex items-center gap-1 px-1 py-px pl-3 text-[11px] text-muted-foreground"
                              >
                                <span className="size-1 rounded-full bg-muted-foreground/50" />
                                <span className="truncate">
                                  {p.currentCommand || p.title || p.id}
                                </span>
                              </div>
                            ))}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                </SessionContextMenu>
              )
            })}
          </div>
        </ScrollArea>
      </div>
    </aside>
  )
}
