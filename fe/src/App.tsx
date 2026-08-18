// App — composition root (PRD §32 UI layout, §46 state management).
// Wires the sidebar tree (TanStack Query polling), one WebSocket per open
// session tab (multi-session, web-term style), and the workspace shell.
//
// All open session tabs stay mounted (inactive ones hidden) so terminal
// content keeps flowing while the user switches tabs. Sidebar pages
// (Settings) overlay the workspace when no tab is focused.

import { useEffect, useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Loader2 } from 'lucide-react'
import { api } from '@/lib/api'
import { useAppStore } from '@/stores/appStore'
import { useTmuxStore } from '@/stores/tmuxStore'
import { useSettingsStore } from '@/stores/settingsStore'
import { getUiTheme, isLightUiTheme } from '@/features/settings/data/ui-themes'
import { useKeyboardShortcut } from '@/hooks/useKeyboardShortcut'
import { desktop, isDesktop } from '@/lib/desktop-ipc'
import { AppTitleBar } from '@/components/layout/AppTitleBar'
import { AppSidebar } from '@/components/layout/AppSidebar'
import { PaneWorkspace } from '@/features/panes/PaneWorkspace'
import { EmptyState } from '@/features/sessions/EmptyState'
import { ErrorState } from '@/features/sessions/ErrorState'
import { SelectSessionView } from '@/features/sessions/SelectSessionView'
import { CreateSessionDialog } from '@/features/sessions/CreateSessionDialog'
import { SettingsPage } from '@/features/settings/SettingsPage'
import { CommandPalette } from '@/features/palette/CommandPalette'
import { ensureSocket, closeSocket } from '@/lib/sockets'
import { cn } from '@/lib/utils'

export default function App() {
  const activeSession = useAppStore((s) => s.activeSession)
  const openSessions = useAppStore((s) => s.openSessions)
  const sidebarPage = useAppStore((s) => s.sidebarPage)
  const backendPort = useAppStore((s) => s.backendPort)
  const setPaletteOpen = useAppStore((s) => s.setPaletteOpen)
  const uiTheme = useSettingsStore((s) => s.uiTheme)

  const snapshots = useTmuxStore((s) => s.snapshots)
  const transports = useTmuxStore((s) => s.transports)

  // Desktop: learn the backend sidecar port over IPC. Electron serves the FE
  // itself (app:// origin), so API/WS calls go to the absolute backend URL;
  // until the port arrives the data queries below stay disabled.
  useEffect(() => {
    if (!isDesktop) return
    const unsub = desktop.onBackendReady((port) => {
      if (port > 0) useAppStore.getState().setBackendPort(port)
    })
    void desktop.getBackendPort().then((port) => {
      if (port && port > 0) useAppStore.getState().setBackendPort(port)
    })
    return unsub
  }, [])

  // If the backend port never arrives (backend failed to start), stop the
  // loading state and surface an error instead of spinning forever.
  const [backendTimedOut, setBackendTimedOut] = useState(false)
  useEffect(() => {
    if (!isDesktop || backendPort > 0) return
    const t = setTimeout(() => setBackendTimedOut(true), 10_000)
    return () => clearTimeout(t)
  }, [backendPort])

  // Apply the UI theme to <html>: set every design-token CSS variable inline
  // (overriding the :root/.dark blocks in index.css) and toggle the `dark`
  // class so Tailwind `dark:` variants keep matching dark themes. index.css
  // maps --color-background: var(--background) etc., so setting the base
  // variables is enough for Tailwind's bg-background/text-foreground utilities.
  useEffect(() => {
    const root = document.documentElement
    const preset = getUiTheme(uiTheme)
    const c = preset.colors
    const set = (name: string, value: string) => root.style.setProperty(name, value)

    set('--background', c.background)
    set('--foreground', c.foreground)
    set('--card', c.card)
    set('--card-foreground', c.cardForeground)
    set('--popover', c.card)
    set('--popover-foreground', c.cardForeground)
    set('--primary', c.primary)
    set('--primary-foreground', c.primaryForeground)
    set('--secondary', c.secondary)
    set('--secondary-foreground', c.secondaryForeground)
    set('--muted', c.muted)
    set('--muted-foreground', c.mutedForeground)
    set('--accent', c.accent)
    set('--accent-foreground', c.accentForeground)
    set('--destructive', c.destructive)
    set('--destructive-foreground', c.destructiveForeground)
    set('--border', c.border)
    set('--input', c.input)
    set('--ring', c.ring)

    // Sidebar chrome follows the same theme (index.css has --sidebar-* too).
    set('--sidebar', c.background)
    set('--sidebar-foreground', c.foreground)
    set('--sidebar-primary', c.primary)
    set('--sidebar-primary-foreground', c.primaryForeground)
    set('--sidebar-accent', c.secondary)
    set('--sidebar-accent-foreground', c.foreground)
    set('--sidebar-border', c.border)
    set('--sidebar-ring', c.ring)

    root.classList.toggle('dark', !isLightUiTheme(preset))
  }, [uiTheme])

  // Sidebar tree: polled 1.5s fallback refresh (PRD §25). tmux is the source
  // of truth; polling only refreshes metadata, never terminal output.
  // Desktop: disabled until the backend sidecar port is known (IPC).
  const backendKnown = !isDesktop || backendPort > 0
  const { data: tree, refetch, isPending } = useQuery({
    queryKey: ['tree'],
    queryFn: api.tree,
    refetchInterval: 1500,
    enabled: backendKnown,
  })

  // tmux version for the status bar (PRD §36: "tmux 3.x").
  const { data: tmuxInfo } = useQuery({
    queryKey: ['tmuxInfo'],
    queryFn: api.tmuxInfo,
    staleTime: 60_000,
    enabled: backendKnown,
  })
  useEffect(() => {
    if (tmuxInfo?.version) {
      useTmuxStore.getState().setTmuxVersion(tmuxInfo.version)
    }
  }, [tmuxInfo])

  // Socket manager: ensure one live WebSocket per open tab and close sockets
  // of closed tabs. Each tab keeps its own connection + snapshot, so
  // switching tabs never tears anything down.
  const ensuredRef = useRef<Set<string>>(new Set())
  useEffect(() => {
    const store = useTmuxStore.getState()
    for (const name of openSessions) {
      if (!ensuredRef.current.has(name)) {
        ensuredRef.current.add(name)
        store.clearSession(name)
        store.setTransport(name, 'connecting')
        ensureSocket(name)
      }
    }
    for (const name of [...ensuredRef.current]) {
      if (!openSessions.includes(name)) {
        ensuredRef.current.delete(name)
        closeSocket(name)
        store.clearSession(name)
      }
    }
  }, [openSessions])

  // Display the active tab's snapshot/transport (components read `snapshot`).
  useEffect(() => {
    useTmuxStore.getState().setViewSession(activeSession)
  }, [activeSession])

  // Command palette shortcut (PRD §38): Ctrl+Shift+P.
  useKeyboardShortcut({ key: 'p', ctrl: true, shift: true }, () =>
    setPaletteOpen(true),
  )

  const tmuxInstalled = tree !== undefined
  const hasSessions = (tree?.sessions.length ?? 0) > 0
  const showingPage = !activeSession && sidebarPage === 'settings'
  const emptyTree = tree ?? { sessions: [] }

  // Startup windows must never flash the error page: (1) desktop — the
  // backend sidecar is still warming up and its port hasn't arrived over IPC
  // yet; (2) web/dev — the first tree fetch is still in flight. Both show a
  // loading state instead.
  const starting = isDesktop && !backendKnown && !backendTimedOut
  const loadingFirstFetch = !starting && isPending

  return (
    <div className="flex h-full w-full flex-col bg-background text-foreground">
      <AppTitleBar />
      <div className="flex min-h-0 flex-1">
        {hasSessions && (
          <AppSidebar
            tree={emptyTree}
            activeSession={activeSession}
            onSelectSession={(name) => useAppStore.getState().openSession(name)}
            onOpenSettings={() => {
              useAppStore.getState().setActiveSession(null)
              useAppStore.getState().setSidebarPage('settings')
            }}
            onRefetch={refetch}
          />
        )}
        <main className="flex min-w-0 flex-1 flex-col">
          {starting ? (
            <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
              <Loader2 className="size-5 animate-spin text-muted-foreground" />
              <p className="text-xs text-muted-foreground">Starting backend…</p>
            </div>
          ) : loadingFirstFetch ? (
            <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
              <Loader2 className="size-5 animate-spin text-muted-foreground" />
              <p className="text-xs text-muted-foreground">Loading…</p>
            </div>
          ) : !tmuxInstalled ? (
            <ErrorState onRetry={refetch} />
          ) : !hasSessions ? (
            <EmptyState onCreate={() => {}} />
          ) : (
            <>
              <div className="relative min-h-0 flex-1">
                {openSessions.map((name) => (
                  <div
                    key={name}
                    className={cn(
                      // flex flex-col so the PaneWorkspace (flex-1 root)
                      // stretches inside the absolutely-positioned wrapper.
                      'absolute inset-0 flex flex-col',
                      name !== activeSession && 'invisible pointer-events-none',
                    )}
                  >
                    <PaneWorkspace
                      session={name}
                      snapshot={snapshots[name] ?? null}
                      transport={transports[name] ?? 'disconnected'}
                    />
                  </div>
                ))}
                {showingPage && <SettingsPage />}
                {!activeSession && !showingPage && (
                  <SelectSessionView tree={emptyTree} />
                )}
              </div>
            </>
          )}
        </main>
      </div>
      <CreateSessionDialog onCreated={(name) => useAppStore.getState().openSession(name)} />
      <CommandPalette />
    </div>
  )
}
