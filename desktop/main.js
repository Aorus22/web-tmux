// Electron main process (PRD §50-51, §66).
// Responsibilities: spawn the Go backend with a dynamic port, parse
// BACKEND_PORT from its stdout, open the window, forward native window
// controls through a minimal preload bridge, and kill the backend on quit —
// tmux sessions always stay alive.

const { app, BrowserWindow, ipcMain, shell, dialog, protocol, net } = require('electron')
const { spawn } = require('child_process')
const path = require('path')
const fs = require('fs')
const { pathToFileURL } = require('url')

const isDev = process.env.NODE_ENV === 'development'

// --- custom protocol: the desktop app serves its own FE bundle ---
// Register BEFORE app ready (required by Electron). The renderer loads
// `app://tmux-gui/index.html` — a STABLE origin that never changes between
// runs, so localStorage (UI settings) survives restarts regardless of which
// port the backend sidecar ends up on (web-term pattern: the shell serves the
// FE, the backend only provides the API).

const APP_SCHEME = 'app'
const APP_HOST = 'tmux-gui'

protocol.registerSchemesAsPrivileged([
  {
    scheme: APP_SCHEME,
    privileges: {
      standard: true,
      secure: true,
      supportFetchAPI: true,
      stream: true,
    },
  },
])

// FE dist location: unpackaged → desktop/resources/fe-dist; packaged →
// resources/fe-dist (extraResources).
function feDistDir() {
  const local = path.join(__dirname, 'resources', 'fe-dist')
  if (fs.existsSync(local)) return local
  return path.join(process.resourcesPath, 'fe-dist')
}

function serveFeDist() {
  const root = feDistDir()
  protocol.handle(APP_SCHEME, (request) => {
    const { hostname, pathname } = new URL(request.url)
    if (hostname !== APP_HOST) {
      return new Response('not found', { status: 404 })
    }
    // Map the URL path onto the dist directory; index.html is the fallback
    // (the FE is a single page).
    let rel = decodeURIComponent(pathname)
    if (rel === '/' || rel === '') rel = '/index.html'
    const file = path.normalize(path.join(root, rel))
    if (!file.startsWith(path.normalize(root))) {
      return new Response('forbidden', { status: 403 })
    }
    return net.fetch(pathToFileURL(file).toString())
  })
}

// --- backend lifecycle ---

let backend = null
let backendPort = null

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
  // Production: dynamic port 0 — the renderer origin is the stable app://
  // protocol (served by Electron itself), so the backend port no longer
  // matters for settings persistence; the FE learns the port via IPC.
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
      backendPort = port
      mainWindow?.webContents.send('backend-ready', port)
    }
  })

  backend.stderr.on('data', (chunk) => {
    process.stderr.write(`[backend] ${chunk}`)
  })

  backend.on('exit', (code) => {
    backendPort = null
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

  // Prod: the FE is served by Electron itself over the stable app:// origin
  // (see serveFeDist). No waiting on the backend — the renderer learns the
  // backend port via IPC and self-heals once it is up.
  if (isDev) {
    mainWindow.loadURL('http://127.0.0.1:14102')
  } else {
    mainWindow.loadURL(`${APP_SCHEME}://${APP_HOST}/index.html`)
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
ipcMain.handle('get-backend-port', () => backendPort)
ipcMain.handle('open-external', (_e, url) => {
  if (typeof url === 'string' && /^https?:\/\//.test(url)) {
    return shell.openExternal(url)
  }
})

// OS directory picker (PRD: new-session working directory). Returns the
// selected path or null when cancelled.
ipcMain.handle('pick-directory', async () => {
  if (!mainWindow) return null
  const result = await dialog.showOpenDialog(mainWindow, {
    title: 'Select working directory',
    properties: ['openDirectory'],
  })
  return result.canceled || result.filePaths.length === 0 ? null : result.filePaths[0]
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
    serveFeDist()
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
