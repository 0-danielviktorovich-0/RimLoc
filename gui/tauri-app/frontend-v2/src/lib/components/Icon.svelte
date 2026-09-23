<script lang="ts">
  // Inline SVG icon set (Lucide-style strokes; spec §1.4: SVG only, no emoji).
  // One set, 16px default, stroke 1.5, currentColor — icons recolor with tokens.
  let { name, size = 16 }: { name: string; size?: number } = $props();

  const PATHS: Record<string, string> = {
    'file-plus':
      'M14 3v4a1 1 0 0 0 1 1h4 M17 21H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7l5 5v11a2 2 0 0 1-2 2z M12 12v6 M9 15h6',
    'folder-open':
      'M6 14l1.45-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5c0-1.1.9-2 2-2h3.93a2 2 0 0 1 1.66.9l.82 1.2a2 2 0 0 0 1.66.9H18a2 2 0 0 1 2 2v2',
    sun: 'M12 3v2 M12 19v2 M5.6 5.6l1.4 1.4 M17 17l1.4 1.4 M3 12h2 M19 12h2 M5.6 18.4 7 17 M17 7l1.4-1.4 M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0Z',
    moon: 'M12 3a6.36 6.36 0 0 0 9 9 9 9 0 1 1-9-9Z',
    monitor: 'M3 5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5Z M8 21h8 M12 16v5',
    search: 'M21 21l-4.35-4.35 M11 19a8 8 0 1 1 0-16 8 8 0 0 1 0 16Z',
    filter: 'M4 5h16 M7 12h10 M10 19h4',
    edit: 'M12 20h9 M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z',
    check: 'M20 6 9 17l-5-5',
    'circle-check': 'M12 22a10 10 0 1 1 0-20 10 10 0 0 1 0 20Z M8.5 12.2l2.3 2.3 4.7-4.7',
    warning:
      'M12 9v4 M12 17h.01 M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z',
    'arrow-right': 'M5 12h14 M13 6l6 6-6 6',
    info: 'M12 22a10 10 0 1 1 0-20 10 10 0 0 1 0 20Z M12 16v-4 M12 8h.01',
    close: 'M18 6 6 18 M6 6l12 12',
    palette:
      'M12 3a9 9 0 0 0 0 18c.83 0 1.5-.67 1.5-1.5 0-.39-.15-.74-.39-1.01a1.5 1.5 0 0 1 1.12-2.49H16a5 5 0 0 0 5-5c0-4.42-4.03-8-9-8Z M7.5 11.5a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z M10.5 8a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z M14.5 8a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z M17.5 11.5a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z'
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#if PATHS[name]}<path d={PATHS[name]} />{/if}
</svg>
