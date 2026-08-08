// StatusBar (PRD §36): connection status, active session/window, pane count,
// tmux version. Shows "Reconnecting to tmux..." while transport is down.

import { useAppStore } from '@/stores/appStore'
import { useTmuxStore } from '@/stores/tmuxStore'

export function StatusBar() {
  const activeSession = useAppStore((s) => s.activeSession)
  const snapshot = useTmuxStore((s) => s.snapshot)
  const transport = useTmuxStore((s) => s.transport)
  const tmuxVersion = useTmuxStore((s) => s.tmuxVersion)

  const activeWindow = snapshot?.windows.find((w) => w.id === snapshot.activeWindow)
  const paneCount = snapshot?.panes.length ?? 0

  const status = (() => {
    switch (transport) {
      case 'connected':
        return { dot: 'bg-emerald-500', text: 'Tmux Connected' }
      case 'reconnecting':
        return { dot: 'bg-amber-500 animate-pulse', text: 'Reconnecting to tmux...' }
      case 'connecting':
        return { dot: 'bg-muted-foreground', text: 'Connecting...' }
      default:
        return { dot: 'bg-muted-foreground', text: 'Disconnected' }
    }
  })()

  return (
    <footer className="flex h-7 shrink-0 items-center gap-4 border-t px-3 text-xs text-muted-foreground">
      <span className="flex items-center gap-1.5">
        <span className={`size-1.5 rounded-full ${status.dot}`} />
        {status.text}
      </span>
      {activeSession && (
        <>
          <span className="hidden sm:inline">Session: {activeSession}</span>
          {activeWindow && <span className="hidden md:inline">Window: {activeWindow.name}</span>}
          <span>
            {paneCount} pane{paneCount === 1 ? '' : 's'}
          </span>
        </>
      )}
      <span className="ml-auto">{tmuxVersion ? `tmux ${tmuxVersion}` : ''}</span>
    </footer>
  )
}
