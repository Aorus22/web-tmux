import path from 'path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

const backendPort = process.env.TMUXGUI_DEV_BACKEND_PORT ?? '4090'

// PRD §61: production build goes straight into the backend's embed dir;
// development proxies /api to the Go backend (no CORS needed). Electron's
// dev shell overrides this to its fixed :9001 sidecar port.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
  server: {
    host: '127.0.0.1', // bind IPv4 explicitly (Vite defaults to ::1, which
    // breaks the Electron dev flow and 127.0.0.1-based proxies)
    port: 9002,
    proxy: {
      '/api': {
        target: `http://127.0.0.1:${backendPort}`,
        changeOrigin: true,
        ws: true,
      },
    },
  },
  build: {
    outDir: '../be/internal/web/dist',
    emptyOutDir: true,
  },
})
