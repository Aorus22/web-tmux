// Preload (PRD §51): contextIsolation + sandbox. Exposes only a minimal,
// audited API. Never exposes require/fs/child_process to the renderer.

const { contextBridge, ipcRenderer } = require('electron')

contextBridge.exposeInMainWorld('desktop', {
  platform: process.platform,
  isElectron: true,
  openExternal: (url) => ipcRenderer.invoke('open-external', url),
  pickDirectory: () => ipcRenderer.invoke('pick-directory'),
  window: {
    minimize: () => ipcRenderer.send('window-minimize'),
    maximize: () => ipcRenderer.send('window-maximize'),
    close: () => ipcRenderer.send('window-close'),
    getState: () => ipcRenderer.invoke('get-window-state'),
    onStateChange: (cb) => {
      const listener = (_e, state) => cb(state)
      ipcRenderer.on('window-state-change', listener)
      return () => ipcRenderer.removeListener('window-state-change', listener)
    },
  },
  getBackendPort: () => ipcRenderer.invoke('get-backend-port'),
  onBackendReady: (cb) => {
    const listener = (_e, port) => cb(port)
    ipcRenderer.on('backend-ready', listener)
    return () => ipcRenderer.removeListener('backend-ready', listener)
  },
})
