// LayoutSelector (PRD §16): dropdown of layout presets for the active window.

import { toast } from 'sonner'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Button } from '@/components/ui/button'
import { LayoutGrid } from 'lucide-react'
import { runCommand } from '@/lib/commands'
import { tmuxSocket } from '@/lib/socket'
import { LAYOUTS } from './WindowToolbar'

interface Props {
  windowId: string
}

export function LayoutSelector({ windowId }: Props) {
  const apply = async (layout: string) => {
    try {
      await runCommand(() => tmuxSocket.windowLayout(windowId, layout))
    } catch (e) {
      toast.error((e as Error).message)
    }
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={<Button variant="outline" size="icon-sm" aria-label="Layout" />}
      >
        <LayoutGrid className="size-3.5" />
      </DropdownMenuTrigger>
      <DropdownMenuContent>
        <DropdownMenuLabel>Layout</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {LAYOUTS.map(({ id, label }) => (
          <DropdownMenuItem key={id} onSelect={() => void apply(id)}>
            {label}
          </DropdownMenuItem>
        ))}
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={() => void apply('next-layout')}>
          Next Layout
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
