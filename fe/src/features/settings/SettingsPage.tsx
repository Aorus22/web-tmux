// SettingsPage (dedicated page, not a modal): theme + terminal settings.
// Shown in the main area while session tabs stay open underneath.

import { UiThemeSettings } from './UiThemeSettings'
import { TerminalSettings } from './TerminalSettings'

export function SettingsPage() {
  return (
    <div className="flex h-full flex-col items-center overflow-y-auto">
      <div className="mx-auto w-full max-w-2xl space-y-6 px-6 py-6">
        <div className="border-b pb-4">
          <h2 className="text-lg font-semibold">Settings</h2>
          <p className="mt-0.5 text-sm text-muted-foreground">UI and terminal preferences.</p>
        </div>

        <UiThemeSettings />

        <div className="border-t pt-5">
          <TerminalSettings />
        </div>
      </div>
    </div>
  )
}
