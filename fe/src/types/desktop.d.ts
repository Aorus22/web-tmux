// Desktop IPC types (PRD §51: contextIsolation + sandbox; preload exposes a
// minimal API only). The renderer never gets require/fs/child_process.

export interface DesktopApi {
  platform: string
  isElectron: boolean
  openExternal: (url: string) => void
  pickDirectory: () => Promise<string | null>
  window: {
    minimize: () => void
    maximize: () => void
    close: () => void
    getState: () => Promise<'maximized' | 'restored'>
    onStateChange: (cb: (state: 'maximized' | 'restored') => void) => () => void
  }
  getBackendPort: () => Promise<number>
  onBackendReady: (cb: (port: number) => void) => () => void
}

declare global {
  interface Window {
    desktop?: DesktopApi
  }
}

export {}
