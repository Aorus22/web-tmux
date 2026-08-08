// Electron main process (PRD §50-51, §66).
// Responsibilities: spawn the Go backend with a dynamic port, parse
// BACKEND_PORT from its stdout, open the window, forward native window
// controls through a minimal preload bridge, and kill the backend on quit —
// tmux sessions always stay alive.

const { app, BrowserWindow, ipcMain, shell } = require('electron')
const { spawn } = require('child_process')
const path = require('path')
const fs = require('fs')

const isDev = process.env.NODE_ENV === 'development'

// --- backend lifecycle ---

let backend = null

function backendBinary() {
  // dev: use the compiled binary in desktop/resources (make dev-desktop builds it)
  const local = path.join(__dirname, 'resources', 'tmux-gui-server')
  if (fs.existsSync(local)) return local
  // packaged: extraResources copies it to resources/tmux-gui-server
  return path.join(process.resourcesPath, 'tmux-gui-server')
}

function startBackend() {
  const bin = backendBinary()
  // Dev mode (PRD §64): Electron spawns Go on the FIXED :14101 port, and the
  // window loads the Vite dev server on :14102, which proxies /api → :14101.
  // Production (PRD §50): dynamic port 0; Electron parses BACKEND_PORT.
  const env = {
    ...process.env,
    TMUXGUI_HOST: '127.0.0.1',
    TMUXGUI_PORT: isDev ? '14101' : '0',
  }
  // If Electron itself runs inside a tmux session, TMUX/TMUX_PANE point at
  // that session's socket — every `tmux` call the backend makes would attach
  // to the wrong server. Clear them so the backend uses the user's default
  // tmux socket (the source of truth).
  delete env.TMUX
  delete env.TMUX_PANE
  backend = spawn(bin, [], { env, stdio: ['ignore', 'pipe', 'pipe'] })

  let portBuf = ''
  backend.stdout.on('data', (chunk) => {
    portBuf += chunk.toString()
    const m = portBuf.match(/BACKEND_PORT:(\d+)/)
    if (m) {
      const port = Number(m[1])
      mainWindow.webContents.send('backend-ready', port)
      if (!isDev) {
        // prod: load once the dynamic backend port is known
        mainWindow.loadURL(`http://127.0.0.1:${port}`)
      }
    }
  })

  backend.stderr.on('data', (chunk) => {
    process.stderr.write(`[backend] ${chunk}`)
  })

  backend.on('exit', (code) => {
    if (!app.isQuiting) {
      console.error('backend exited', code)
    }
  })
}

function stopBackend() {
  if (backend && !backend.killed) {
    backend.kill('SIGTERM')
  }
  backend = null
}

// --- window ---

let mainWindow = null

function createWindow() {
  // Shown immediately, like the web-term reference app. No `show:false` +
  // `ready-to-show` gating: on machines where first paint is slow or broken
  // (Wayland+Vulkan), `ready-to-show` never fires and the window would stay
  // hidden forever. The window appears right away and content fills in when
  // the renderer is ready.
  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 800,
    minHeight: 500,
    frame: false, // frameless (web-term style): the FE titlebar provides drag + min/max/close
    backgroundColor: '#111111',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  })

  mainWindow.on('closed', () => {
    mainWindow = null
  })

  // Push maximize state to the renderer so the custom titlebar can swap the
  // maximize/restore icon (web-term pattern).
  mainWindow.on('maximize', () => {
    mainWindow?.webContents.send('window-state-change', 'maximized')
  })
  mainWindow.on('unmaximize', () => {
    mainWindow?.webContents.send('window-state-change', 'restored')
  })

  // Dev (PRD §64): load the Vite dev server directly, like the web-term
  // reference app — no waiting on the backend; Vite proxies /api → :14101.
  if (isDev) {
    mainWindow.loadURL('http://127.0.0.1:14102')
  }
  startBackend()
}

// --- IPC: native window controls + openExternal (PRD §41, §51) ---

ipcMain.on('window-minimize', () => mainWindow?.minimize())
ipcMain.on('window-maximize', () => {
  if (!mainWindow) return
  mainWindow.isMaximized() ? mainWindow.unmaximize() : mainWindow.maximize()
})
ipcMain.on('window-close', () => mainWindow?.close())
ipcMain.handle('get-window-state', () =>
  mainWindow ? (mainWindow.isMaximized() ? 'maximized' : 'restored') : 'restored',
)
ipcMain.handle('open-external', (_e, url) => {
  if (typeof url === 'string' && /^https?:\/\//.test(url)) {
    return shell.openExternal(url)
  }
})

// --- app lifecycle ---

const gotLock = app.requestSingleInstanceLock()
if (!gotLock) {
  app.quit()
} else {
  app.on('second-instance', () => {
    if (mainWindow) {
      if (mainWindow.isMinimized()) mainWindow.restore()
      mainWindow.focus()
    }
  })

  app.whenReady().then(() => {
    createWindow()
    app.on('activate', () => {
      if (BrowserWindow.getAllWindows().length === 0) createWindow()
    })
  })

  app.on('before-quit', () => {
    app.isQuiting = true
    stopBackend() // tmux sessions are never killed (PRD §49)
  })

  app.on('window-all-closed', () => {
    app.quit()
  })
}
