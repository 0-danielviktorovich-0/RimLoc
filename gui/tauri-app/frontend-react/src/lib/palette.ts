// Command palette (Cmd+K): navigation commands. Minimal typed store —
// keyboard-first workflow support (mandate §45). The commands are built
// where the palette is mounted (App.tsx) and passed as props.
//
// Keyboard model (acceptance MUST-FIX #4, PALETTE_LM_ACCEPTANCE §3): the
// input keeps focus, the WINDOW-level keydown handler moves the active item
// — so real typing and synthetic WebDriver events (window.dispatchEvent)
// reach the same code path. ArrowDown/ArrowUp are cyclic; Home/End jump to
// the first/last item; Enter runs the active command; a fresh open and any
// filter change reset the cursor to the first match (Svelte canon:
// CommandPalette.svelte "fresh cursor on every open / query moves the
// cursor back"). Hover syncs the cursor via onMouseEnter on the items.
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Search } from 'lucide-react'

export interface PaletteCommand {
  id: string
  label: string
  action: () => void
}

export interface CommandPalette {
  open: boolean
  setOpen: (v: boolean) => void
  query: string
  setQuery: (v: string) => void
  filtered: PaletteCommand[]
  activeIndex: number
  setActiveIndex: (i: number) => void
  runCommand: (cmd: PaletteCommand) => void
}

export function useCommandPalette(commands: PaletteCommand[]): CommandPalette {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [activeIndex, setActiveIndex] = useState(0)

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase()
    if (!q) return commands
    return commands.filter((c) => c.label.toLowerCase().includes(q))
  }, [commands, query])

  // A new list (typing, commands change) moves the cursor back to the first
  // match — it must never point past the end of the list.
  useEffect(() => {
    setActiveIndex(0)
  }, [filtered])

  // Fresh cursor and CLEAN query on every open (acceptance MUST-FIX «запрос
  // переживает закрытие»: палитра открывалась предотфильтрованной — повторный
  // запуск команды вслепую рисковал выбрать не ту). Закрытие (Escape/toggle/
  // запуск команды) всегда возвращает палитру в исходное состояние.
  useEffect(() => {
    if (open) {
      setActiveIndex(0)
    } else {
      setQuery('')
      setActiveIndex(0)
    }
  }, [open])

  // Latest list/cursor/open for the stable window handler below.
  const filteredRef = useRef(filtered)
  filteredRef.current = filtered
  const activeRef = useRef(activeIndex)
  activeRef.current = activeIndex
  const openRef = useRef(open)
  openRef.current = open

  // Hide first, then navigate (Svelte canon: run() in CommandPalette.svelte).
  const runCommand = useCallback((cmd: PaletteCommand): void => {
    setOpen(false)
    cmd.action()
  }, [])

  useEffect(() => {
    const handler = (e: KeyboardEvent): void => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        setOpen((v) => !v)
        return
      }
      if (e.key === 'Escape') {
        setOpen(false)
        return
      }
      if (!openRef.current || e.metaKey || e.ctrlKey || e.altKey) return
      const list = filteredRef.current
      if (list.length === 0) return
      if (e.key === 'ArrowDown') {
        e.preventDefault()
        setActiveIndex((i) => (i + 1) % list.length) // cyclic
      } else if (e.key === 'ArrowUp') {
        e.preventDefault()
        setActiveIndex((i) => (i - 1 + list.length) % list.length) // cyclic
      } else if (e.key === 'Home') {
        e.preventDefault()
        setActiveIndex(0)
      } else if (e.key === 'End') {
        e.preventDefault()
        setActiveIndex(list.length - 1)
      } else if (e.key === 'Enter') {
        // preventDefault keeps a focused option button from ALSO firing its
        // native click — exactly one command runs per Enter.
        e.preventDefault()
        const cmd = list[activeRef.current] ?? list[0]
        if (cmd) runCommand(cmd)
      }
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [runCommand])

  return { open, setOpen, query, setQuery, filtered, activeIndex, setActiveIndex, runCommand }
}

export { Search }
