<script lang="ts">
  // Generic ARIA tablist (spec §5 semantics): role=tablist/tab, roving
  // tabindex, Arrow keys move focus and select. Used by the Workspace top
  // navigation and the entry detail panel.
  interface Tab {
    id: string;
    label: string;
    count?: number;
  }

  let {
    tabs,
    active,
    label,
    onSelect
  }: {
    tabs: Tab[];
    active: string;
    label: string;
    onSelect: (id: string) => void;
  } = $props();

  let listEl: HTMLDivElement | undefined = $state();

  function move(dir: 1 | -1) {
    const idx = tabs.findIndex((tb) => tb.id === active);
    if (idx === -1) return;
    const next = (idx + dir + tabs.length) % tabs.length;
    onSelect(tabs[next].id);
    const btns = listEl?.querySelectorAll<HTMLButtonElement>('[role="tab"]');
    btns?.[next]?.focus();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowRight') {
      e.preventDefault();
      move(1);
    } else if (e.key === 'ArrowLeft') {
      e.preventDefault();
      move(-1);
    }
  }
</script>

<!-- Focus lives on the tab buttons (roving tabindex per ARIA APG); the
     container only hosts the arrow-key handler. -->
<!-- svelte-ignore a11y_interactive_supports_focus -->
<div class="tabs" role="tablist" aria-label={label} bind:this={listEl} onkeydown={onKeydown}>
  {#each tabs as tb (tb.id)}
    <button
      type="button"
      role="tab"
      id={`tab-${tb.id}`}
      aria-selected={active === tb.id}
      tabindex={active === tb.id ? 0 : -1}
      data-testid={`tabs.${tb.id}`}
      onclick={() => onSelect(tb.id)}
    >
      {tb.label}
      {#if tb.count}
        <span class="count" aria-hidden="true">{tb.count}</span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    gap: var(--space-1);
    padding: 0 var(--space-4);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface);
    flex: none;
    overflow-x: auto;
  }

  [role='tab'] {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--control-h);
    padding: var(--space-1) var(--space-3);
    margin-bottom: -1px;
    border: 1px solid transparent;
    border-bottom: none;
    border-radius: var(--radius-md) var(--radius-md) 0 0;
    color: var(--color-muted-fg);
    font-size: var(--text-base-size);
    white-space: nowrap;
    transition:
      color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out);
  }

  [role='tab']:hover {
    color: var(--color-fg);
    background: var(--color-muted);
  }

  [role='tab'][aria-selected='true'] {
    color: var(--color-fg);
    font-weight: 600;
    border-color: var(--color-border);
    background: var(--color-bg);
    box-shadow: inset 0 -2px 0 var(--color-primary);
  }

  .count {
    font-size: var(--text-meta-size);
    font-weight: 400;
    color: var(--color-muted-fg);
    background: var(--color-muted);
    border-radius: var(--radius-sm);
    padding: 0 var(--space-1);
    font-variant-numeric: tabular-nums;
  }
</style>
