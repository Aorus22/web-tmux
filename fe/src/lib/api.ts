// REST API client (PRD §9: read-only queries, §10: bootstrap create).
// Consumed via TanStack Query. The frontend always calls relative /api paths;
// Vite proxies in dev, Go serves in production — no CORS, no hardcoded URLs
// (PRD §61).

import type { HealthResponse, TmuxInfo, TmuxSnapshot, TmuxTree } from './tmux-types'

async function get<T>(path: string): Promise<T> {
  const res = await fetch(path)
  if (!res.ok) {
    throw new Error(`${path}: ${res.status} ${res.statusText}`)
  }
  return res.json() as Promise<T>
}

async function post<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  })
  if (!res.ok) {
    // Surface the backend error message (e.g. "duplicate session") instead of
    // a bare status code.
    let message = `${path}: ${res.status} ${res.statusText}`
    try {
      const data = (await res.json()) as { error?: string }
      if (data.error) message = data.error
    } catch {
      // non-JSON error body: keep the generic message
    }
    throw new Error(message)
  }
  return res.json() as Promise<T>
}

export const api = {
  health: () => get<HealthResponse>('/api/health'),

  tmuxInfo: () => get<TmuxInfo>('/api/tmux/info'),

  tree: () => get<TmuxTree>('/api/sessions'),

  snapshot: (session: string) =>
    get<TmuxSnapshot>(`/api/sessions/${encodeURIComponent(session)}/snapshot`),

  // createSession goes over REST, not the per-session WebSocket: it must work
  // with zero sessions (no socket exists to send the command on).
  createSession: (name: string, cwd?: string, initialCommand?: string) =>
    post<{ name: string }>('/api/sessions', { name, cwd, initialCommand }),
}
