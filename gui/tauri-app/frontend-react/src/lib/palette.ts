// Command palette (Cmd+K): navigation commands. Minimal typed store —
// keyboard-first workflow support (mandate §45). The commands are built
// where the palette is mounted (App.tsx) and passed as props.
import { useEffect, useState, useMemo } from 'react'
import { Search } from 'lucide-react'

export interface PaletteCommand {
  id: string
  label: string
  action: () => void
}

export function useCommandPalette(commands: PaletteCommand[]): { open: boolean; setOpen: (v: boolean) => void; query: string; setQuery: (v: string) => void; filtered: PaletteCommand[] } {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')

  useEffect(() => {
    const handler = (e: KeyboardEvent): void => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        setOpen((v) => !v)
      }
      if (e.key === 'Escape') setOpen(false)
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [])

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase()
    if (!q) return commands
    return commands.filter((c) => c.label.toLowerCase().includes(q))
  }, [commands, query])

  return { open, setOpen, query, setQuery, filtered }
}

export { Search }
