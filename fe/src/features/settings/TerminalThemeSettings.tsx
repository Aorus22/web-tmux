// TerminalThemeSettings — pick a terminal color scheme from the curated
// presets (see data/terminal-themes.ts). Cards show a swatch preview; the
// selected preset is stored in settingsStore.terminalTheme and applied to the
// xterm theme object in useTerminal. "Default" (null) follows the app UI
// theme's terminal colors (see resolvedTerminalTheme).

import { useState } from 'react'
import { useSettingsStore } from '@/stores/settingsStore'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  terminalThemes,
  isLightTheme,
} from './data/terminal-themes'
import { ThemeCard } from './ThemeCard'

type Filter = 'all' | 'dark' | 'light'

// CSS-variable accents used for the Default card's swatch. The fallbacks keep
// the swatch visible when the --term-* variables aren't defined.
const DEFAULT_BACKGROUND = 'var(--term-bg, #1e1e1e)'
const DEFAULT_ACCENTS = [
  'var(--term-color-1, #f44747)',
  'var(--term-color-2, #6a9955)',
  'var(--term-color-3, #d7ba7d)',
  'var(--term-color-4, #569cd6)',
  'var(--term-color-5, #c586c0)',
  'var(--term-color-6, #4ec9b0)',
]

export function TerminalThemeSettings() {
  const terminalTheme = useSettingsStore((s) => s.terminalTheme)
  const set = useSettingsStore((s) => s.set)
  const [filter, setFilter] = useState<Filter>('all')

  const themes = terminalThemes.filter((t) => {
    if (filter === 'all') return true
    const light = isLightTheme(t)
    return filter === 'light' ? light : !light
  })

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-2">
        <span className="text-sm font-medium">Terminal theme</span>
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
        <ThemeCard
          label="Default (follow app theme)"
          background={DEFAULT_BACKGROUND}
          accents={DEFAULT_ACCENTS}
          selected={terminalTheme === null}
          onSelect={() => set({ terminalTheme: null })}
        />
        {themes.map((t) => (
          <ThemeCard
            key={t.name}
            label={t.label}
            background={t.colors.background}
            accents={[
              t.colors.red,
              t.colors.green,
              t.colors.yellow,
              t.colors.blue,
              t.colors.magenta,
              t.colors.cyan,
            ]}
            selected={terminalTheme === t.name}
            onSelect={() => set({ terminalTheme: t.name })}
          />
        ))}
      </div>
    </div>
  )
}
