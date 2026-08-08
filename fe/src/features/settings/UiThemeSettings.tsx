// UiThemeSettings — the app theme picker (web-term style). Cards show a swatch
// preview; the selected preset is stored in settingsStore.uiTheme and applied
// as CSS variables on <html> in App.tsx. One theme themes the whole app — the
// terminal follows the selected theme automatically (see resolvedTerminalTheme).

import { useState } from 'react'
import { useSettingsStore } from '@/stores/settingsStore'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
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
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-2">
        <span className="text-sm font-medium">Theme</span>
        <Tabs value={filter} onValueChange={(v) => setFilter(v as Filter)}>
          <TabsList className="h-7">
            <TabsTrigger value="all" className="h-full px-2 text-xs">
              All
            </TabsTrigger>
            <TabsTrigger value="dark" className="h-full px-2 text-xs">
              Dark
            </TabsTrigger>
            <TabsTrigger value="light" className="h-full px-2 text-xs">
              Light
            </TabsTrigger>
          </TabsList>
        </Tabs>
      </div>

      <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4">
        {themes.map((t) => (
          <ThemeCard
            key={t.name}
            label={t.label}
            background={t.colors.background}
            accents={[
              t.colors.primary,
              t.colors.accent,
              t.colors.destructive,
            ]}
            selected={uiTheme === t.name}
            onSelect={() => set({ uiTheme: t.name })}
          />
        ))}
      </div>
    </div>
  )
}
