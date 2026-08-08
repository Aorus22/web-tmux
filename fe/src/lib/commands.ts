// runCommand — runs a GUI action and awaits its command.success/error
// correlation (PRD §28). Components call `await runCommand(() => tmuxSocket.paneKill(id))`
// and errors surface as rejected promises (toast via sonner by the caller).
//
// A command is abandoned after `timeoutMs`: if the WebSocket is down the
// message sits in the send queue forever and the correlation never resolves,
// which would otherwise leave UI busy-flags stuck indefinitely (e.g. the
// Create Session button). On timeout the pending entry is forgotten so a late
// response is ignored.

import { tmuxSocket } from './socket'
import { useTmuxStore } from '@/stores/tmuxStore'
import { useSettingsStore } from '@/stores/settingsStore'

export { tmuxSocket }

const COMMAND_TIMEOUT_MS = 10_000

export function runCommand(action: () => string, timeoutMs: number = COMMAND_TIMEOUT_MS): Promise<void> {
  return new Promise((resolve, reject) => {
    const requestId = action()
    const timer = setTimeout(() => {
      useTmuxStore.getState().forgetCommand(requestId)
      reject(new Error('Command timed out (no response from tmux)'))
    }, timeoutMs)
    useTmuxStore.getState().trackCommand(requestId, {
      ok: () => {
        clearTimeout(timer)
        resolve()
      },
      error: (message) => {
        clearTimeout(timer)
        reject(new Error(message))
      },
    })
  })
}

// confirmKill — returns true unless the store has confirm disabled (PRD §42).
export function shouldConfirm(kind: 'pane' | 'window' | 'session'): boolean {
  const s = useSettingsStore.getState()
  switch (kind) {
    case 'pane':
      return s.confirmKillPane
    case 'window':
      return s.confirmKillWindow
    case 'session':
      return s.confirmKillSession
  }
}
