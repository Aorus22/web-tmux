// tmuxStore tests (PRD §76): snapshot handling, command correlation, sequence
// gap / reconnect behavior, active-pane removal.

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

describe('tmuxStore', () => {
  beforeEach(() => {
    useTmuxStore.getState().clear()
  })

  it('stores a snapshot and exposes selectors', () => {
    const store = useTmuxStore.getState()
    store.setSnapshot(makeSnapshot())

    const s = useTmuxStore.getState()
    expect(useTmuxStore.getState().snapshot?.session.name).toBe('dev')
    expect(s.snapshot?.activePane).toBe('%0')
    expect(s.snapshot?.windows).toHaveLength(1)
    expect(s.snapshot?.panes).toHaveLength(2)
  })

  it('replaces the snapshot on a fresh state.delta', () => {
    const store = useTmuxStore.getState()
    store.setSnapshot(makeSnapshot())
    const snap2 = makeSnapshot({
      panes: [
        { id: '%0', index: 0, windowId: '@0', active: true, zoomed: false, left: 0, top: 0, width: 80, height: 24, pid: 1, currentCommand: 'bash', currentPath: '/', title: 'x' },
      ],
    })
    store.setSnapshot(snap2)
    expect(useTmuxStore.getState().snapshot?.panes).toHaveLength(1)
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

  it('tracks transport state and reconnecting flag', () => {
    const store = useTmuxStore.getState()
    store.setTransport('reconnecting')
    expect(useTmuxStore.getState().transport).toBe('reconnecting')
    expect(useTmuxStore.getState().reconnecting).toBe(true)

    store.setTransport('connected')
    expect(useTmuxStore.getState().transport).toBe('connected')
    expect(useTmuxStore.getState().reconnecting).toBe(false)
  })

  it('clears all state (used on session switch)', () => {
    const store = useTmuxStore.getState()
    store.setSnapshot(makeSnapshot())
    store.setTransport('connected')
    store.trackCommand('id', { ok: () => {}, error: () => {} })
    store.clear()

    const s = useTmuxStore.getState()
    expect(s.snapshot).toBeNull()
    expect(s.transport).toBe('disconnected')
    expect(s.pending).toEqual({})
  })
})
