<script lang="ts">
  // Minimal inline SVG icon set (Lucide-style strokes; spec §1.4: SVG only, no emoji).
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
    x: 'M18 6 6 18 M6 6l12 12',
    alert:
      'M12 9v4 M12 17h.01 M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0Z',
    check: 'M20 6 9 17l-5-5'
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#if PATHS[name]}<path d={PATHS[name]} />{/if}
</svg>
