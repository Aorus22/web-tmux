// App — composition root (PRD §32 UI layout, §46 state management).
// Wires the sidebar tree (TanStack Query polling), the active-session
// WebSocket, and the workspace shell together.

import { useEffect } from 'react'
import { useQuery } from '@tanstack/react-query'
import { api } from '@/lib/api'
import { useAppStore } from '@/stores/appStore'
import { useTmuxStore } from '@/stores/tmuxStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { useKeyboardShortcut } from '@/hooks/useKeyboardShortcut'
import { AppTitleBar } from '@/components/layout/AppTitleBar'
import { AppSidebar } from '@/components/layout/AppSidebar'
import { StatusBar } from '@/components/layout/StatusBar'
import { WindowTabs } from '@/features/windows/WindowTabs'
import { PaneWorkspace } from '@/features/panes/PaneWorkspace'
import { EmptyState } from '@/features/sessions/EmptyState'
import { ErrorState } from '@/features/sessions/ErrorState'
import { CreateSessionDialog } from '@/features/sessions/CreateSessionDialog'
import { SettingsDialog } from '@/features/settings/SettingsDialog'
import { CommandPalette } from '@/features/palette/CommandPalette'
import { useTmuxSocket } from '@/hooks/useTmuxSocket'

export default function App() {
  const activeSession = useAppStore((s) => s.activeSession)
  const setActiveSession = useAppStore((s) => s.setActiveSession)
  const setPaletteOpen = useAppStore((s) => s.setPaletteOpen)
  const transport = useTmuxStore((s) => s.transport)
  const snapshot = useTmuxStore((s) => s.snapshot)

  const theme = useSettingsStore((s) => s.theme)

  // Apply theme to <html> (dark-first).
  useEffect(() => {
    const root = document.documentElement
    root.classList.toggle('dark', theme === 'dark')
  }, [theme])

  // Sidebar tree: polled 1.5s fallback refresh (PRD §25). tmux is the source
  // of truth; polling only refreshes metadata, never terminal output.
  const { data: tree, refetch } = useQuery({
    queryKey: ['tree'],
    queryFn: api.tree,
    refetchInterval: 1500,
  })

  // tmux version for the status bar (PRD §36: "tmux 3.x").
  const { data: tmuxInfo } = useQuery({
    queryKey: ['tmuxInfo'],
    queryFn: api.tmuxInfo,
    staleTime: 60_000,
  })
  useEffect(() => {
    if (tmuxInfo?.version) {
      useTmuxStore.getState().setTmuxVersion(tmuxInfo.version)
    }
  }, [tmuxInfo])

  // Live session connection (PRD §14: switch session = reconnect socket).
  // useTmuxSocket configures handlers AND connects/disconnects the singleton
  // socket itself; no separate connectSocket call here (a second call would
  // open a second WS connection and duplicate every terminal output chunk).
  useTmuxSocket(activeSession)

  // Command palette shortcut (PRD §38): Ctrl+Shift+P.
  useKeyboardShortcut({ key: 'p', ctrl: true, shift: true }, () =>
    setPaletteOpen(true),
  )

  const tmuxInstalled = tree !== undefined
  const hasSessions = (tree?.sessions.length ?? 0) > 0

  return (
    <div className="flex h-full w-full flex-col bg-background text-foreground">
      <AppTitleBar />
      <div className="flex min-h-0 flex-1">
        {hasSessions && (
          <AppSidebar
            tree={tree ?? { sessions: [] }}
            activeSession={activeSession}
            onSelectSession={(name) => setActiveSession(name)}
            onRefetch={refetch}
          />
        )}
        <main className="flex min-w-0 flex-1 flex-col">
          {!tmuxInstalled ? (
            <ErrorState onRetry={refetch} />
          ) : !hasSessions ? (
            <EmptyState onCreate={() => setActiveSession(null)} />
          ) : (
            <>
              <WindowTabs />
              <PaneWorkspace
                snapshot={snapshot}
                transport={transport}
                onSelectSession={(name) => setActiveSession(name)}
              />
            </>
          )}
        </main>
      </div>
      <StatusBar />
      <CreateSessionDialog onCreated={(name) => setActiveSession(name)} />
      <SettingsDialog />
      <CommandPalette />
    </div>
  )
}
