// ThemeCard — shared swatch card for the settings theme grids (UI theme and
// terminal theme). Shows the theme's background with accent dots; the selected
// card gets a ring + check.

import { Check } from 'lucide-react'
import { cn } from '@/lib/utils'

export function ThemeCard({
  label,
  background,
  accents,
  selected,
  onSelect,
}: {
  label: string
  background: string
  accents: string[]
  selected: boolean
  onSelect: () => void
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onSelect}
      className={cn(
        'group relative flex flex-col overflow-hidden rounded-lg border text-left transition-colors',
        'border-border/60 bg-card hover:border-border/90',
        selected && 'border-ring ring-2 ring-ring/60',
      )}
    >
      <div
        className="flex h-14 items-end gap-1 p-2"
        style={{ background }}
      >
        {accents.map((color, i) => (
          <span
            key={i}
            className="size-3 rounded-[3px] ring-1 ring-black/20"
            style={{ background: color }}
          />
        ))}
      </div>
      <div className="flex items-center justify-between gap-1 px-2 py-1.5">
        <span className="min-w-0 truncate text-xs font-medium">{label}</span>
        {selected && <Check className="size-3.5 shrink-0 text-foreground" />}
      </div>
    </button>
  )
}
