// TerminalView — thin React wrapper around useTerminal for one pane.

import { forwardRef } from 'react'
import { useTerminal } from './useTerminal'
import type { Terminal } from '@xterm/xterm'

interface Props {
  paneId: string
  className?: string
  onResize?: (cols: number, rows: number) => void
  onReady?: (term: Terminal) => void
}

export const TerminalView = forwardRef<HTMLDivElement, Props>(function TerminalView(
  { paneId, className, onResize, onReady },
  ref,
) {
  const { containerRef } = useTerminal({ paneId, onResize, onOpen: onReady })

  return <div ref={ref} className={className} style={{ height: '100%', width: '100%' }} data-pane={paneId}>
    <div ref={containerRef} style={{ height: '100%', width: '100%' }} />
  </div>
})
