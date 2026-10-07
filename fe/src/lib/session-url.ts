// Active session tab ↔ URL query sync (`?session=<name>`).
//
// The active tab rides the URL so a refresh (or a bookmarked/shared link)
// lands back on the same session instead of the empty select view. Writes
// use `history.replaceState` — the tab strip is app state, not navigation, so
// switching tabs must not spam browser history entries.

const PARAM = 'session'

export function readSessionFromUrl(): string | null {
  try {
    const value = new URLSearchParams(window.location.search).get(PARAM)
    return value && value.trim() !== '' ? value : null
  } catch {
    return null
  }
}

export function writeSessionToUrl(name: string | null): void {
  try {
    const url = new URL(window.location.href)
    if (name) {
      url.searchParams.set(PARAM, name)
    } else {
      url.searchParams.delete(PARAM)
    }
    window.history.replaceState(window.history.state, '', url)
  } catch {
    // Custom/opaque origins (some embedders) may reject the History API; the
    // query is a convenience, never load-bearing.
  }
}
