// SettingsPage (dedicated page, not a modal): theme + terminal settings.
// Shown in the main area while session tabs stay open underneath.

import { UiThemeSettings } from './UiThemeSettings'
import { TerminalSettings } from './TerminalSettings'
import { TerminalSquare } from 'lucide-react'

export function SettingsPage() {
  return (
    <div className="flex min-h-full flex-col items-center overflow-y-auto bg-background p-4 pt-10 text-foreground md:p-8 md:pt-12">
      <div className="w-full max-w-2xl space-y-8 pb-12">
        <h1 className="text-2xl font-bold tracking-tight">Settings</h1>
        <UiThemeSettings />

        <section className="space-y-4">
          <h2 className="text-sm font-medium uppercase tracking-wider text-muted-foreground">
            Terminal
          </h2>
          <div className="overflow-hidden rounded-lg border bg-card px-4 py-4">
            <div className="mb-4 flex items-center gap-3">
              <TerminalSquare className="size-4 text-muted-foreground" />
              <div className="flex flex-col">
                <span className="text-sm font-medium">Terminal preferences</span>
                <span className="text-xs text-muted-foreground">
                  The selected theme is applied to every pane and app surface.
                </span>
              </div>
            </div>
            <TerminalSettings />
          </div>
        </section>
      </div>
    </div>
  )
}
