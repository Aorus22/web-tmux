// Protocol tests (PRD §26-28): message constants and wire shapes.

import { describe, expect, it } from 'vitest'
import { EV, MSG } from '@/lib/protocol'

describe('protocol message types', () => {
  it('defines all client→server GUI actions', () => {
    expect(MSG.hello).toBe('hello')
    expect(MSG.terminalInput).toBe('terminal.input')
    expect(MSG.terminalResize).toBe('terminal.resize')
    expect(MSG.terminalCapture).toBe('terminal.capture')
    expect(MSG.paneSelect).toBe('pane.select')
    expect(MSG.paneSplit).toBe('pane.split')
    expect(MSG.paneResize).toBe('pane.resize')
    expect(MSG.paneKill).toBe('pane.kill')
    expect(MSG.paneZoom).toBe('pane.zoom')
    expect(MSG.paneBreak).toBe('pane.break')
    expect(MSG.paneSwap).toBe('pane.swap')
    expect(MSG.windowSelect).toBe('window.select')
    expect(MSG.windowCreate).toBe('window.create')
    expect(MSG.windowRename).toBe('window.rename')
    expect(MSG.windowKill).toBe('window.kill')
    expect(MSG.windowLayout).toBe('window.layout')
    expect(MSG.windowMove).toBe('window.move')
    expect(MSG.windowBreakActive).toBe('window.break-active')
    expect(MSG.sessionCreate).toBe('session.create')
    expect(MSG.sessionRename).toBe('session.rename')
    expect(MSG.sessionKill).toBe('session.kill')
    expect(MSG.stateResync).toBe('state.resync')
  })

  it('defines all server→client events', () => {
    expect(EV.connectionReady).toBe('connection.ready')
    expect(EV.stateSnapshot).toBe('state.snapshot')
    expect(EV.stateDelta).toBe('state.delta')
    expect(EV.terminalSnapshot).toBe('terminal.snapshot')
    expect(EV.terminalOutput).toBe('terminal.output')
    expect(EV.commandSuccess).toBe('command.success')
    expect(EV.commandError).toBe('command.error')
    expect(EV.tmuxReconnecting).toBe('tmux.reconnecting')
    expect(EV.tmuxDisconnected).toBe('tmux.disconnected')
    expect(EV.serverError).toBe('server.error')
  })
})
