// REST API client (PRD §9: read-only queries). Consumed via TanStack Query.
// The frontend always calls relative /api paths; Vite proxies in dev, Go
// serves in production — no CORS, no hardcoded URLs (PRD §61).

import type { HealthResponse, TmuxInfo, TmuxSnapshot, TmuxTree } from './tmux-types'

async function get<T>(path: string): Promise<T> {
  const res = await fetch(path)
  if (!res.ok) {
    throw new Error(`${path}: ${res.status} ${res.statusText}`)
  }
  return res.json() as Promise<T>
}

export const api = {
  health: () => get<HealthResponse>('/api/health'),

  tmuxInfo: () => get<TmuxInfo>('/api/tmux/info'),

  tree: () => get<TmuxTree>('/api/sessions'),

  snapshot: (session: string) =>
    get<TmuxSnapshot>(`/api/sessions/${encodeURIComponent(session)}/snapshot`),
}
