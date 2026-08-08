// tmuxStore tests (PRD §76): per-session snapshot handling, view switching,
// command correlation, transport state, tab close / full clear.

import { beforeEach, describe, expect, it } from 'vitest'
import { useTmuxStore } from '@/stores/tmuxStore'
import type { TmuxSnapshot } from '@/lib/tmux-types'

function makeSnapshot(overrides: Partial<TmuxSnapshot> = {}): TmuxSnapshot {
  return {
    session: { name: 'dev', windows: 1, attached: 1, createdAt: 1, width: 80, height: 24 },
    windows: [
      { id: '@0', index: 0, name: 'editor', active: true, panes: 2, width: 80, height: 24, layout: 'x' },
    ],
    panes: [
      { id: '%0', index: 0, windowId: '@0', active: true, zoomed: false, left: 0, top: 0, width: 40, height: 24, pid: 1, currentCommand: 'bash', currentPath: '/', title: 'x' },
      { id: '%1', index: 1, windowId: '@0', active: false, zoomed: false, left: 40, top: 0, width: 40, height: 24, pid: 2, currentCommand: 'vim', currentPath: '/', title: 'y' },
    ],
    activeWindow: '@0',
    activePane: '%0',
    ...overrides,
  }
}

const LOGS_SESSION = { name: 'logs', windows: 1, attached: 1, createdAt: 1, width: 80, height: 24 }

describe('tmuxStore', () => {
  beforeEach(() => {
    useTmuxStore.getState().clear()
  })

  it('stores a snapshot per session and mirrors the view session', () => {
    const store = useTmuxStore.getState()
    store.setViewSession('dev')
    store.setSnapshot('dev', makeSnapshot())

    const s = useTmuxStore.getState()
    expect(s.snapshots['dev']).toBeDefined()
    expect(s.snapshot?.session.name).toBe('dev')
    expect(s.snapshot?.activePane).toBe('%0')
    expect(s.snapshot?.windows).toHaveLength(1)
    expect(s.snapshot?.panes).toHaveLength(2)
  })

  it('replaces the snapshot on a fresh state.delta', () => {
    const store = useTmuxStore.getState()
    store.setViewSession('dev')
    store.setSnapshot('dev', makeSnapshot())
    const snap2 = makeSnapshot({
      panes: [
        { id: '%0', index: 0, windowId: '@0', active: true, zoomed: false, left: 0, top: 0, width: 80, height: 24, pid: 1, currentCommand: 'bash', currentPath: '/', title: 'x' },
      ],
    })
    store.setSnapshot('dev', snap2)
    expect(useTmuxStore.getState().snapshot?.panes).toHaveLength(1)
  })

  it('keeps per-session snapshots isolated from the view', () => {
    const store = useTmuxStore.getState()
    store.setViewSession('dev')
    store.setSnapshot('dev', makeSnapshot())
    // A background tab's snapshot must not clobber the displayed one.
    store.setSnapshot('logs', makeSnapshot({ session: LOGS_SESSION }))
    expect(useTmuxStore.getState().snapshot?.session.name).toBe('dev')

    // Switching the view (tab switch) shows the other session's snapshot.
    store.setViewSession('logs')
    expect(useTmuxStore.getState().snapshot?.session.name).toBe('logs')
    expect(useTmuxStore.getState().viewSession).toBe('logs')
  })

  it('resolves commands with requestId correlation', async () => {
    const store = useTmuxStore.getState()
    const ok = new Promise<void>((resolve, reject) => {
      store.trackCommand('abc', { ok: () => resolve(), error: (m) => reject(new Error(m)) })
    })
    store.resolveCommand('abc', true)
    await expect(ok).resolves.toBeUndefined()
  })

  it('rejects commands on error with the message', async () => {
    const store = useTmuxStore.getState()
    const err = new Promise((_, reject) => {
      store.trackCommand('xyz', { ok: () => reject(new Error('should not succeed')), error: (m) => reject(new Error(m)) })
    })
    store.resolveCommand('xyz', false, 'pane not found')
    await expect(err).rejects.toThrow('pane not found')
  })

  it('ignores unknown requestIds', () => {
    const store = useTmuxStore.getState()
    expect(() => store.resolveCommand('nope', false, 'x')).not.toThrow()
  })

  it('tracks transport per session with the reconnecting flag', () => {
    const store = useTmuxStore.getState()
    store.setViewSession('dev')
    store.setTransport('dev', 'reconnecting')
    expect(useTmuxStore.getState().transport).toBe('reconnecting')
    expect(useTmuxStore.getState().reconnecting).toBe(true)

    // Background tab's transport does not affect the view.
    store.setTransport('logs', 'connected')
    expect(useTmuxStore.getState().transport).toBe('reconnecting')

    store.setTransport('dev', 'connected')
    expect(useTmuxStore.getState().transport).toBe('connected')
    expect(useTmuxStore.getState().reconnecting).toBe(false)
  })

  it('clears a closed tab session state (socket already disconnected)', () => {
    const store = useTmuxStore.getState()
    store.setViewSession('dev')
    store.setSnapshot('dev', makeSnapshot())
    store.setTransport('dev', 'connected')
    store.clearSession('dev')

    const s = useTmuxStore.getState()
    expect(s.snapshots['dev']).toBeUndefined()
    expect(s.transports['dev']).toBeUndefined()
    expect(s.snapshot).toBeNull()
    expect(s.transport).toBe('disconnected')
  })

  it('clears all state', () => {
    const store = useTmuxStore.getState()
    store.setViewSession('dev')
    store.setSnapshot('dev', makeSnapshot())
    store.setTransport('dev', 'connected')
    store.trackCommand('id', { ok: () => {}, error: () => {} })
    store.clear()

    const s = useTmuxStore.getState()
    expect(s.snapshots).toEqual({})
    expect(s.transports).toEqual({})
    expect(s.snapshot).toBeNull()
    expect(s.transport).toBe('disconnected')
    expect(s.pending).toEqual({})
  })
})
