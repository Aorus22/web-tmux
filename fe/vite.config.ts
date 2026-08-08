import path from 'path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// PRD §61: production build goes straight into the backend's embed dir;
// development proxies /api to the Go backend on :14101 (no CORS needed).
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
    port: 14102,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:14101',
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
