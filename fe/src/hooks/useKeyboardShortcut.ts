// useKeyboardShortcut — global shortcut registration (PRD §38).

import { useEffect } from 'react'

interface Shortcut {
  key: string
  ctrl?: boolean
  shift?: boolean
  alt?: boolean
  meta?: boolean
}

function matches(e: KeyboardEvent, s: Shortcut): boolean {
  const wantCtrl = !!s.ctrl
  const wantShift = !!s.shift
  const wantAlt = !!s.alt
  const wantMeta = !!s.meta
  return (
    (e.ctrlKey || e.metaKey) === (wantCtrl || wantMeta) &&
    !!e.shiftKey === wantShift &&
    !!e.altKey === wantAlt &&
    e.key.toLowerCase() === s.key.toLowerCase()
  )
}

export function useKeyboardShortcut(shortcut: Shortcut, handler: () => void) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!matches(e, shortcut)) return
      e.preventDefault()
      handler()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [shortcut.key, shortcut.ctrl, shortcut.shift, shortcut.alt, shortcut.meta, handler])
}
