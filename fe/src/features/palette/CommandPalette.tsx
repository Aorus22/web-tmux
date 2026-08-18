// CommandPalette (PRD §38): Ctrl+Shift+P opens a shadcn Command dialog with
// the common GUI actions. All actions remain available via mouse; the palette
// is optional convenience.

import { toast } from 'sonner'
import { useQuery } from '@tanstack/react-query'
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from '@/components/ui/command'
import { api } from '@/lib/api'
import { useAppStore } from '@/stores/appStore'
import { useTmuxStore } from '@/stores/tmuxStore'
import { runCommand } from '@/lib/commands'
import { tmuxSocket } from '@/lib/socket'
import { useCreateSessionDialog } from '@/features/sessions/CreateSessionDialog'

export function CommandPalette() {
  const open = useAppStore((s) => s.paletteOpen)
  const setOpen = useAppStore((s) => s.setPaletteOpen)
  const snapshot = useTmuxStore((s) => s.snapshot)
  const { data: tree } = useQuery({ queryKey: ['tree'], queryFn: api.tree })
  const sessions = tree?.sessions ?? []

  const activeWindow = snapshot?.windows.find((w) => w.id === snapshot.activeWindow)
  const activePane = snapshot?.panes.find((p) => p.id === snapshot.activePane)

  const run = async (action: () => string, label: string) => {
    try {
      await runCommand(action)
      toast.success(label)
      setOpen(false)
    } catch (e) {
      toast.error((e as Error).message)
    }
  }

  return (
    <CommandDialog open={open} onOpenChange={setOpen}>
      <CommandInput placeholder="Type a command or search…" />
      <CommandList>
        <CommandEmpty>No results found.</CommandEmpty>
        <CommandGroup heading="Actions">
          <CommandItem
            onSelect={() => {
              setOpen(false)
              useCreateSessionDialog.getState().setOpen(true)
            }}
          >
            New Session
          </CommandItem>
          <CommandItem
            onSelect={() => void run(() => tmuxSocket.windowCreate(), 'Window created')}
          >
            New Window
          </CommandItem>
          <CommandItem
            onSelect={() => {
              if (activePane) void run(() => tmuxSocket.paneSplit(activePane.id, 'horizontal'), 'Split right')
            }}
          >
            Split Right
          </CommandItem>
          <CommandItem
            onSelect={() => {
              if (activePane) void run(() => tmuxSocket.paneSplit(activePane.id, 'vertical'), 'Split down')
            }}
          >
            Split Down
          </CommandItem>
          <CommandItem
            onSelect={() => {
              if (activePane) void run(() => tmuxSocket.paneZoom(activePane.id), 'Zoomed')
            }}
          >
            Zoom Pane
          </CommandItem>
          <CommandItem
            onSelect={() => {
              if (activeWindow) void run(() => tmuxSocket.windowLayout(activeWindow.id, 'next-layout'), 'Next layout')
            }}
          >
            Next Layout
          </CommandItem>
          <CommandItem
            onSelect={() => {
              if (activePane) void run(() => tmuxSocket.paneKill(activePane.id), 'Pane killed')
            }}
          >
            Kill Pane
          </CommandItem>
        </CommandGroup>
        {sessions.length > 0 && (
          <>
            <CommandSeparator />
            <CommandGroup heading="Open Session">
              {sessions.map((node) => (
                <CommandItem
                  key={node.session.name}
                  onSelect={() => {
                    useAppStore.getState().openSession(node.session.name)
                    setOpen(false)
                  }}
                >
                  {node.session.name}
                </CommandItem>
              ))}
            </CommandGroup>
          </>
        )}
      </CommandList>
    </CommandDialog>
  )
}
