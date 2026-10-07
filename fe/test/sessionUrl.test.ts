// Active session tab ↔ URL query sync (`?session=<name>`) contracts.

import { describe, expect, it, beforeEach } from 'vitest'
import { readSessionFromUrl, writeSessionToUrl } from '@/lib/session-url'

function setUrl(url: string) {
  window.history.replaceState(null, '', url)
}

describe('session-url', () => {
  beforeEach(() => {
    setUrl('/')
  })

  it('reads the session param', () => {
    setUrl('/?session=hris-main-dev')
    expect(readSessionFromUrl()).toBe('hris-main-dev')
  })

  it('treats a missing or empty param as no session', () => {
    setUrl('/')
    expect(readSessionFromUrl()).toBeNull()
    setUrl('/?session=')
    expect(readSessionFromUrl()).toBeNull()
  })

  it('ignores unrelated params', () => {
    setUrl('/?foo=bar')
    expect(readSessionFromUrl()).toBeNull()
  })

  it('writes the session param in place', () => {
    writeSessionToUrl('dev')
    expect(window.location.search).toBe('?session=dev')
    expect(readSessionFromUrl()).toBe('dev')
  })

  it('strips the param when no session is active', () => {
    setUrl('/?session=dev&other=1')
    writeSessionToUrl(null)
    expect(window.location.search).toBe('?other=1')
  })

  it('preserves unrelated params when writing', () => {
    setUrl('/?foo=1&bar=2')
    writeSessionToUrl('dev')
    expect(window.location.search).toContain('session=dev')
    expect(window.location.search).toContain('foo=1')
    expect(window.location.search).toContain('bar=2')
  })

  it('does not push history entries (tab switches are not navigation)', () => {
    const before = window.history.length
    writeSessionToUrl('dev')
    writeSessionToUrl('other')
    writeSessionToUrl(null)
    expect(window.history.length).toBe(before)
  })
})
