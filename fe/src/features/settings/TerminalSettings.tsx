// TerminalSettings (PRD §42): font family, size, line height, scrollback.
// Live-updates the settings store; terminals rebuild or fit accordingly.

import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { useSettingsStore } from '@/stores/settingsStore'

export function TerminalSettings() {
  const settings = useSettingsStore()
  const set = useSettingsStore((s) => s.set)

  return (
    <div className="space-y-4">
      <div className="space-y-1.5">
        <Label htmlFor="ts-font-family">Font family</Label>
        <Input
          id="ts-font-family"
          value={settings.fontFamily}
          onChange={(e) => set({ fontFamily: e.target.value })}
        />
      </div>
      <div className="grid grid-cols-3 gap-3">
        <div className="space-y-1.5">
          <Label htmlFor="ts-font-size">Font size</Label>
          <Input
            id="ts-font-size"
            type="number"
            min={8}
            max={32}
            value={settings.fontSize}
            onChange={(e) => set({ fontSize: Number(e.target.value) || 14 })}
          />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="ts-line-height">Line height</Label>
          <Input
            id="ts-line-height"
            type="number"
            min={1}
            max={2}
            step={0.05}
            value={settings.lineHeight}
            onChange={(e) => set({ lineHeight: Number(e.target.value) || 1.35 })}
          />
        </div>
        <div className="space-y-1.5">
          <Label htmlFor="ts-scrollback">Scrollback lines</Label>
          <Input
            id="ts-scrollback"
            type="number"
            min={100}
            max={50000}
            step={100}
            value={settings.scrollbackLines}
            onChange={(e) => set({ scrollbackLines: Number(e.target.value) || 2000 })}
          />
        </div>
      </div>
      <div className="space-y-2 border-t pt-3">
        <span className="text-xs font-medium text-muted-foreground">Safety</span>
        <ConfirmRow
          label="Confirm before killing pane"
          checked={settings.confirmKillPane}
          onChange={(v) => set({ confirmKillPane: v })}
        />
        <ConfirmRow
          label="Confirm before killing window"
          checked={settings.confirmKillWindow}
          onChange={(v) => set({ confirmKillWindow: v })}
        />
        <ConfirmRow
          label="Confirm before killing session"
          checked={settings.confirmKillSession}
          onChange={(v) => set({ confirmKillSession: v })}
        />
      </div>
    </div>
  )
}

function ConfirmRow({
  label,
  checked,
  onChange,
}: {
  label: string
  checked: boolean
  onChange: (v: boolean) => void
}) {
  return (
    <div className="flex items-center justify-between">
      <Label className="text-sm">{label}</Label>
      <Switch checked={checked} onCheckedChange={onChange} />
    </div>
  )
}
