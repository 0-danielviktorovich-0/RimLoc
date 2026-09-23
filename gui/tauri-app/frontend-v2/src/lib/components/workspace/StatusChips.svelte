<script lang="ts">
  // Status filter chips with live counters (spec §2.4).
  // Multi-select, aria-pressed; the accessible name stays stable because the
  // counter sits in an aria-hidden child node (spec §5).
  import { t } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import type { StatusCounts } from '../../stores/project.svelte';
  import type { EntryStatus } from '../../mock/types';

  let { counts }: { counts: StatusCounts } = $props();

  const STATUSES: EntryStatus[] = [
    'untranslated',
    'translated',
    'todo',
    'sourceChanged',
    'pending_review',
    'orphan'
  ];

  // Map status ids to their color token (--color-status-* from app.css).
  function dotColor(status: EntryStatus): string {
    const name = status === 'sourceChanged' ? 'source-changed' : status.replace('_', '-');
    return `background: var(--color-status-${name})`;
  }
</script>

<div class="chips" role="group" aria-label={t('workspace.filter.label')}>
  {#each STATUSES as status (status)}
    <button
      type="button"
      class="chip"
      aria-pressed={project.filters.includes(status)}
      data-testid={`workspace.filter.${status}`}
      onclick={() => project.toggleFilter(status)}
    >
      <span class="dot" style={dotColor(status)} aria-hidden="true"></span>
      <span class="chip-label">{t(`workspace.filter.${status}`)}</span>
      <span class="chip-count" aria-hidden="true">{counts[status]}</span>
    </button>
  {/each}
</div>

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    flex: none;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 32px;
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    font-size: var(--text-meta-size);
    color: var(--color-fg);
    transition: border-color var(--motion-fast), background var(--motion-fast);
  }

  .chip:hover {
    border-color: var(--color-border-strong);
    background: var(--color-muted);
  }

  .chip[aria-pressed='true'] {
    border-color: var(--color-primary);
    background: var(--color-muted);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }

  .chip-count {
    color: var(--color-muted-fg);
    font-variant-numeric: tabular-nums;
  }
</style>
