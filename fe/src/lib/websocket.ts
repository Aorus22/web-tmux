// WebSocket manager for one session (PRD §26-28, §48).
//
// Responsibilities:
//  - open /api/ws?session=<name> (relative URL; proxied or same-origin)
//  - exponential reconnect backoff: 250ms, 500ms, 1s, 2s, 5s, 10s
//  - dispatch typed messages to handlers; correlate command responses
//  - expose typed send helpers for every GUI action
//
// The tmuxStore owns live state; this module is transport only.

import { EV, MSG, type WsIncoming, type WsOutgoing } from './protocol'

export interface WsHandlers {
  onReady?: (session: string) => void
  onStateSnapshot?: (snap: unknown) => void
  onStateDelta?: (snap: unknown) => void
  onTerminalOutput?: (paneId: string, data: string) => void
  onTerminalSnapshot?: (paneId: string, data: string) => void
  onCommandResult?: (requestId: string, ok: boolean, message?: string) => void
  onReconnecting?: () => void
  onDisconnected?: () => void
  onServerError?: (message: string) => void
  onStateChange?: (state: 'connecting' | 'connected' | 'reconnecting' | 'disconnected') => void
}

const BACKOFF = [250, 500, 1000, 2000, 5000, 10000]

export class TmuxSocket {
  private ws: WebSocket | null = null
  private session: string
  private handlers: WsHandlers
  private closedByUser = false
  private attempt = 0
  private retryTimer: ReturnType<typeof setTimeout> | null = null
  private sendQueue: string[] = []

  constructor(session: string, handlers: WsHandlers) {
    this.session = session
    this.handlers = handlers
  }

  // setSession re-targets the socket (PRD §14: switch session = reconnect).
  setSession(session: string) {
    this.close()
    this.session = session
  }

  // setHandlers replaces the event handlers (used by useTmuxSocket on connect).
  setHandlers(handlers: WsHandlers) {
    this.handlers = handlers
  }

  connect() {
    this.closedByUser = false
    this.open()
  }

  private open() {
    const proto = window.location.protocol === 'https:' ? 'wss' : 'ws'
    const url = `${proto}://${window.location.host}/api/ws?session=${encodeURIComponent(this.session)}`
    this.setTransportState('connecting')
    const ws = new WebSocket(url)
    this.ws = ws

    ws.onopen = () => {
      this.attempt = 0
      // Flush anything queued while connecting.
      for (const raw of this.sendQueue) {
        ws.send(raw)
      }
      this.sendQueue = []
      this.setTransportState('connected')
    }

    ws.onmessage = (e) => {
      let msg: WsOutgoing
      try {
        msg = JSON.parse(String(e.data)) as WsOutgoing
      } catch {
        return
      }
      this.dispatch(msg)
    }

    ws.onclose = () => {
      if (this.closedByUser) return
      this.scheduleReconnect()
    }

    ws.onerror = () => {
      // onclose follows; nothing to do here.
    }
  }

  private scheduleReconnect() {
    const delay = BACKOFF[Math.min(this.attempt, BACKOFF.length - 1)]
    this.attempt++
    this.setTransportState('reconnecting')
    this.handlers.onReconnecting?.()
    this.retryTimer = setTimeout(() => this.open(), delay)
  }

  private dispatch(msg: WsOutgoing) {
    switch (msg.type) {
      case EV.connectionReady:
        this.handlers.onReady?.(msg.session ?? this.session)
        break
      case EV.stateSnapshot:
        this.handlers.onStateSnapshot?.(msg.snapshot)
        break
      case EV.stateDelta:
        this.handlers.onStateDelta?.(msg.snapshot)
        break
      case EV.terminalOutput:
        if (msg.paneId) this.handlers.onTerminalOutput?.(msg.paneId, msg.data ?? '')
        break
      case EV.terminalSnapshot:
        if (msg.paneId) this.handlers.onTerminalSnapshot?.(msg.paneId, msg.data ?? '')
        break
      case EV.commandSuccess:
        if (msg.requestId) this.handlers.onCommandResult?.(msg.requestId, true)
        break
      case EV.commandError:
        if (msg.requestId) this.handlers.onCommandResult?.(msg.requestId, false, msg.message)
        break
      case EV.tmuxReconnecting:
        this.handlers.onReconnecting?.()
        break
      case EV.tmuxDisconnected:
        this.handlers.onDisconnected?.()
        break
      case EV.serverError:
        this.handlers.onServerError?.(msg.message ?? 'server error')
        break
    }
  }

  // send transmits a typed message with an optional requestId.
  send(msg: WsIncoming) {
    const raw = JSON.stringify(msg)
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(raw)
    } else {
      this.sendQueue.push(raw)
    }
  }

  // --- typed GUI actions (PRD §26, §87) ---

  private nextRequestId(): string {
    return Math.random().toString(36).slice(2, 10)
  }

  hello(cols: number, rows: number) {
    this.send({ type: MSG.hello, cols, rows })
  }

  terminalInput(paneId: string, data: string) {
    this.send({ type: MSG.terminalInput, paneId, data })
  }

  terminalResize(cols: number, rows: number) {
    this.send({ type: MSG.terminalResize, cols, rows })
  }

  terminalCapture(paneId: string) {
    this.send({ type: MSG.terminalCapture, paneId })
  }

  requestState() {
    this.send({ type: MSG.stateResync })
  }

  // Actions with requestId correlation. Returns the requestId.
  paneSelect(paneId: string): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.paneSelect, paneId, requestId: id })
    return id
  }

  paneSplit(paneId: string, direction: 'horizontal' | 'vertical'): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.paneSplit, paneId, direction, requestId: id })
    return id
  }

  paneResize(paneId: string, direction: 'L' | 'R' | 'U' | 'D', amount: number): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.paneResize, paneId, direction, amount, requestId: id })
    return id
  }

  paneKill(paneId: string): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.paneKill, paneId, requestId: id })
    return id
  }

  paneZoom(paneId: string): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.paneZoom, paneId, requestId: id })
    return id
  }

  paneBreak(paneId: string): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.paneBreak, paneId, requestId: id })
    return id
  }

  paneSwap(paneId: string, otherPaneId: string): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.paneSwap, paneId, otherPaneId, requestId: id })
    return id
  }

  windowSelect(windowId: string): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.windowSelect, paneId: windowId, requestId: id })
    return id
  }

  windowCreate(name?: string, cwd?: string, command?: string): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.windowCreate, name, cwd, command, requestId: id })
    return id
  }

  windowRename(windowId: string, name: string): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.windowRename, paneId: windowId, name, requestId: id })
    return id
  }

  windowKill(windowId: string): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.windowKill, paneId: windowId, requestId: id })
    return id
  }

  windowLayout(windowId: string, layout: string): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.windowLayout, paneId: windowId, layout, requestId: id })
    return id
  }

  windowMove(windowId: string, offset: number): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.windowMove, paneId: windowId, amount: offset, requestId: id })
    return id
  }

  windowBreakActive(windowId: string): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.windowBreakActive, paneId: windowId, requestId: id })
    return id
  }

  sessionCreate(name: string, cwd?: string, command?: string): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.sessionCreate, session: name, cwd, command, requestId: id })
    return id
  }

  sessionRename(newName: string): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.sessionRename, newName, requestId: id })
    return id
  }

  sessionKill(): string {
    const id = this.nextRequestId()
    this.send({ type: MSG.sessionKill, requestId: id })
    return id
  }

  close() {
    this.closedByUser = true
    if (this.retryTimer) clearTimeout(this.retryTimer)
    this.ws?.close()
  }

  private setTransportState(s: 'connecting' | 'connected' | 'reconnecting' | 'disconnected') {
    this.handlers.onStateChange?.(s)
  }
}
