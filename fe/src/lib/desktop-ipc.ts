// desktop-ipc — runtime-agnostic adapter (web-term pattern). Detects Electron
// via window.desktop and branches; web mode is a no-op.

import type { DesktopApi } from '@/types/desktop.d'

const api: DesktopApi | undefined = window.desktop

export const isDesktop = !!api

export const desktop = {
  get platform(): string {
    return api?.platform ?? 'web'
  },
  openExternal(url: string) {
    api?.openExternal(url)
  },
  window: {
    minimize: () => api?.window.minimize(),
    maximize: () => api?.window.maximize(),
    close: () => api?.window.close(),
    async getState(): Promise<'maximized' | 'restored'> {
      if (!api) return 'restored'
      return api.window.getState()
    },
    onStateChange(cb: (state: 'maximized' | 'restored') => void): () => void {
      if (!api) return () => {}
      return api.window.onStateChange(cb)
    },
  },
  async getBackendPort(): Promise<number | null> {
    if (!api) return null
    return api.getBackendPort()
  },
  onBackendReady(cb: (port: number) => void): () => void {
    if (!api) return () => {}
    return api.onBackendReady(cb)
  },
}
