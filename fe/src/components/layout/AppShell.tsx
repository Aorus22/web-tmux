// AppShell — minimal structural wrapper (PRD §32).

import type { ReactNode } from 'react'

export function AppShell({ children }: { children: ReactNode }) {
  return <div className="flex h-full w-full flex-col">{children}</div>
}
