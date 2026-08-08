// useTerminal — xterm.js lifecycle for one pane (PRD §21-23, §46).
// Xterm is only a renderer/input collector: no PTY, no shell spawning. Output
// arrives via terminalRegistry.write (from the WebSocket); input leaves via
// tmuxSocket.terminalInput. Fit handles resize and reports cols/rows.

import { useCallback, useEffect, useRef } from 'react'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { WebLinksAddon } from '@xterm/addon-web-links'
import { terminalRegistry } from './terminalRegistry'
import { tmuxSocket } from '@/lib/socket'
import { useSettingsStore } from '@/stores/settingsStore'

export interface UseTerminalOptions {
  paneId: string
  onResize?: (cols: number, rows: number) => void
  onOpen?: (term: Terminal) => void
}

export function useTerminal({ paneId, onResize, onOpen }: UseTerminalOptions) {
  const containerRef = useRef<HTMLDivElement>(null)
  const termRef = useRef<Terminal | null>(null)
  const fitRef = useRef<FitAddon | null>(null)
  const resizeObserverRef = useRef<ResizeObserver | null>(null)
  const cbRef = useRef({ onResize, onOpen })
  cbRef.current = { onResize, onOpen }

  // Settings that require a terminal rebuild (font family/size).
  const fontFamily = useSettingsStore((s) => s.fontFamily)
  const fontSize = useSettingsStore((s) => s.fontSize)
  const lineHeight = useSettingsStore((s) => s.lineHeight)
  const theme = useSettingsStore((s) => s.theme)
  const scrollbackLines = useSettingsStore((s) => s.scrollbackLines)

  const createTerminal = useCallback(() => {
    if (!containerRef.current || termRef.current) return
    const term = new Terminal({
      fontFamily,
      fontSize,
      lineHeight,
      scrollback: scrollbackLines,
      cursorBlink: true,
      allowTransparency: true,
      theme: {
        background: 'var(--term-bg)',
        foreground: 'var(--term-fg)',
        cursor: 'var(--term-cursor)',
        black: 'var(--term-color-0)',
        red: 'var(--term-color-1)',
        green: 'var(--term-color-2)',
        yellow: 'var(--term-color-3)',
        blue: 'var(--term-color-4)',
        magenta: 'var(--term-color-5)',
        cyan: 'var(--term-color-6)',
        white: 'var(--term-color-7)',
        brightBlack: 'var(--term-color-8)',
        brightRed: 'var(--term-color-9)',
        brightGreen: 'var(--term-color-10)',
        brightYellow: 'var(--term-color-11)',
        brightBlue: 'var(--term-color-12)',
        brightMagenta: 'var(--term-color-13)',
        brightCyan: 'var(--term-color-14)',
        brightWhite: 'var(--term-color-15)',
      },
    })

    const fit = new FitAddon()
    term.loadAddon(fit)
    term.loadAddon(new WebLinksAddon((_e, uri) => window.open(uri, '_blank')))

    term.open(containerRef.current)
    fit.fit()

    // Register so the WS handler can write output straight to this terminal.
    const { cols, rows } = term
    terminalRegistry.register(paneId, term, cols, rows)

    // Input → backend (PRD §22). No per-character tmux command: the backend
    // InputBatcher aggregates (8-16ms / 4KB).
    term.onData((data) => {
      tmuxSocket.terminalInput(paneId, data)
    })

    // Clipboard (PRD §40, §81): Ctrl+Shift+C copies the xterm selection,
    // Ctrl+C copies when there is a selection (else passes through as
    // interrupt), Ctrl+Shift+V pastes the clipboard as terminal input.
    term.attachCustomKeyEventHandler((event) => {
      if (event.type !== 'keydown') return true
      const ctrl = event.ctrlKey || event.metaKey
      const shift = event.shiftKey
      const isCtrlC = ctrl && !shift && event.key.toLowerCase() === 'c'
      const isCtrlShiftC = ctrl && shift && event.key.toLowerCase() === 'c'
      const isCtrlShiftV = ctrl && shift && event.key.toLowerCase() === 'v'

      if (isCtrlShiftC || isCtrlC) {
        const selection = term.getSelection()
        if (selection) {
          void navigator.clipboard?.writeText(selection)
          return false // consume; we handled it
        }
        if (isCtrlC) return true // no selection: let Ctrl+C through (interrupt)
        return false // Ctrl+Shift+C with no selection: nothing to copy
      }

      if (isCtrlShiftV) {
        void navigator.clipboard?.readText().then((text) => {
          if (text) tmuxSocket.terminalInput(paneId, text)
        })
        return false // consume; paste handled as terminal input
      }
      return true
    })

    term.onResize(({ cols, rows }) => {
      terminalRegistry.setSize(paneId, cols, rows)
      cbRef.current.onResize?.(cols, rows)
    })

    // Initial snapshot for scrollback (PRD §24).
    tmuxSocket.terminalCapture(paneId)

    termRef.current = term
    fitRef.current = fit
    cbRef.current.onOpen?.(term)
  }, [paneId, fontFamily, fontSize, lineHeight, scrollbackLines, theme])

  useEffect(() => {
    createTerminal()
    return () => {
      resizeObserverRef.current?.disconnect()
      if (termRef.current) {
        terminalRegistry.unregister(paneId)
        termRef.current.dispose()
        termRef.current = null
      }
    }
  }, [createTerminal, paneId])

  // Keep the terminal sized to its container (web-term pattern: ResizeObserver
  // with ~80ms debounce to avoid resize storms).
  useEffect(() => {
    const el = containerRef.current
    if (!el) return
    let timer: ReturnType<typeof setTimeout> | null = null
    const ro = new ResizeObserver(() => {
      if (timer) clearTimeout(timer)
      timer = setTimeout(() => {
        fitRef.current?.fit()
      }, 80)
    })
    ro.observe(el)
    resizeObserverRef.current = ro
    return () => {
      if (timer) clearTimeout(timer)
      ro.disconnect()
    }
  }, [])

  return { containerRef, termRef, fitRef }
}
