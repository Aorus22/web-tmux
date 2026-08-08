// SettingsDialog (PRD §42): theme + terminal settings. Opened from the title
// bar via the internal useSettingsDialog store.

import { create } from 'zustand'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useSettingsStore } from '@/stores/settingsStore'
import { TerminalSettings } from './TerminalSettings'

export const useSettingsDialog = create<{
  open: boolean
  setOpen: (open: boolean) => void
}>((set) => ({ open: false, setOpen: (open) => set({ open }) }))

export function SettingsDialog() {
  const open = useSettingsDialog((s) => s.open)
  const setOpen = useSettingsDialog((s) => s.setOpen)
  const theme = useSettingsStore((s) => s.theme)
  const set = useSettingsStore((s) => s.set)

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
          <DialogDescription>UI and terminal preferences.</DialogDescription>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="theme">Theme</Label>
            <Select
              value={theme}
              onValueChange={(v) => set({ theme: v as 'dark' | 'light' })}
            >
              <SelectTrigger id="theme" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="dark">Dark</SelectItem>
                <SelectItem value="light">Light</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="border-t pt-3">
            <TerminalSettings />
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
