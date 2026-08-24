// UiThemeSettings — the app theme picker (web-term style). Cards show a swatch
// preview; the selected preset is stored in settingsStore.uiTheme and applied
// as CSS variables on <html> in App.tsx. One theme themes the whole app — the
// terminal follows the selected theme automatically (see resolvedTerminalTheme).

import { useState } from 'react'
import { Paintbrush } from 'lucide-react'
import { useSettingsStore } from '@/stores/settingsStore'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { uiThemes, isLightUiTheme } from './data/ui-themes'
import { ThemeCard } from './ThemeCard'

type Filter = 'all' | 'dark' | 'light'

export function UiThemeSettings() {
  const uiTheme = useSettingsStore((s) => s.uiTheme)
  const set = useSettingsStore((s) => s.set)
  const [filter, setFilter] = useState<Filter>('all')

  const themes = uiThemes.filter((t) => {
    if (filter === 'all') return true
    const light = isLightUiTheme(t)
    return filter === 'light' ? light : !light
  })

  return (
    <section className="space-y-4">
      <h2 className="text-sm font-medium uppercase tracking-wider text-muted-foreground">
        Appearance
      </h2>
      <div className="overflow-hidden rounded-lg border bg-card divide-y">
        <div className="flex items-center justify-between gap-3 px-4 py-3">
          <div className="flex items-center gap-3">
            <Paintbrush className="size-4 text-muted-foreground" />
            <div className="flex flex-col">
              <span className="text-sm font-medium">Theme mode</span>
              <span className="text-xs text-muted-foreground">
                Filter by dark or light appearance
              </span>
            </div>
          </div>
          <Select value={filter} onValueChange={(v) => setFilter(v as Filter)}>
            <SelectTrigger className="h-8 w-[110px] text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All themes</SelectItem>
              <SelectItem value="dark">Dark</SelectItem>
              <SelectItem value="light">Light</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="space-y-4 px-4 py-4">
          <div className="flex flex-col gap-0.5">
            <span className="text-sm font-medium">Color theme</span>
            <span className="text-xs text-muted-foreground">
              {themes.length} themes available
            </span>
          </div>
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
            {themes.map((t) => (
              <ThemeCard
                key={t.name}
                label={t.label}
                background={t.colors.background}
                accents={[t.colors.primary, t.colors.accent, t.colors.destructive]}
                selected={uiTheme === t.name}
                onSelect={() => set({ uiTheme: t.name })}
              />
            ))}
          </div>
        </div>
      </div>
    </section>
  )
}
