<script lang="ts">
  // Left navigator, user-oriented (mandate §9): "All entries" + statuses come
  // first; technical record kinds live collapsed under "Structure (advanced)".
  // M-3 (UI audit 2026-09-29): this panel is NAVIGATION OVER the table chips,
  // not a second filter set — status items toggle the SAME project.filters
  // array the chips above the table do, and the item set mirrors FilterBar's
  // QUICK exactly ('translated' used to be chip-only, 'orphan' panel-only).
  import { t } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import Icon from '../Icon.svelte';
  import type { EntryStatus } from '../../mock/types';

  // Same set as FilterBar QUICK — keep the two lists in lockstep by design.
  const STATUS_ITEMS: EntryStatus[] = [
    'untranslated',
    'pending_review',
    'sourceChanged',
    'translated',
    'todo',
    'orphan'
  ];

  const KINDS = ['Keyed', 'DefInjected', 'TKey'] as const;

  let { kindCounts }: { kindCounts: Record<string, number> } = $props();

  const statusCounts = $derived(project.statusCounts());
  const total = $derived(project.entries.length);
</script>

<nav class="navigator" aria-label={t('workspace.navigator.title')} data-testid="workspace.navigator">
  <p class="nav-title">{t('workspace.navigator.title')}</p>
  <ul class="nav-list">
    <li>
      <button
        type="button"
        class="nav-item"
        aria-pressed={project.filters.length === 0}
        data-testid="workspace.navigator.all"
        onclick={() => project.setStatusFilter(null)}
      >
        <span class="nav-label">{t('workspace.navigator.all')}</span>
        <span class="nav-count" aria-hidden="true">{total}</span>
      </button>
    </li>
  </ul>

  <p class="nav-section">{t('workspace.navigator.status')}</p>
  <ul class="nav-list">
    {#each STATUS_ITEMS as status (status)}
      <li>
        <button
          type="button"
          class="nav-item"
          aria-pressed={project.filters.includes(status)}
          data-testid={`workspace.navigator.status.${status}`}
          onclick={() => project.toggleFilter(status)}
        >
          <span class="nav-label">{t(`workspace.filter.${status}`)}</span>
          <span class="nav-count" aria-hidden="true">{statusCounts[status]}</span>
        </button>
      </li>
    {/each}
  </ul>

  <details class="advanced">
    <summary data-testid="workspace.navigator.advanced">
      <Icon name="layers" size={12} />
      {t('workspace.navigator.advanced')}
    </summary>
    <ul class="nav-list">
      {#each KINDS as kind (kind)}
        <li>
          <button
            type="button"
            class="nav-item"
            aria-pressed={project.category === kind}
            data-testid={`workspace.navigator.${kind}`}
            onclick={() => (project.category = project.category === kind ? 'all' : kind)}
          >
            <span class="nav-label mono">{t(`kind.${kind}`)}</span>
            <span class="nav-count" aria-hidden="true">{kindCounts[kind] ?? 0}</span>
          </button>
        </li>
      {/each}
    </ul>
  </details>
</nav>

<style>
  .navigator {
    border-right: 1px solid var(--color-border);
    padding: var(--space-3);
    overflow-y: auto;
    background: var(--color-surface);
  }

  .nav-title {
    margin: 0 0 var(--space-2);
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .nav-section {
    margin: var(--space-3) 0 var(--space-1);
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .nav-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .nav-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    width: 100%;
    min-height: var(--control-h);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    color: var(--color-fg);
    text-align: left;
    transition:
      background var(--motion-fast) var(--ease-out),
      color var(--motion-fast) var(--ease-out),
      transform var(--motion-fast) var(--ease-out);
  }

  .nav-item:hover {
    background: var(--color-muted);
  }

  .nav-item:active {
    transform: var(--btn-press-transform);
  }

  .nav-item[aria-pressed='true'] {
    background: var(--nav-active-bg);
    color: var(--color-primary-text);
    font-weight: 600;
  }

  .nav-count {
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
    font-variant-numeric: tabular-nums;
  }

  .advanced {
    margin-top: var(--space-3);
    border-top: 1px solid var(--color-border);
    padding-top: var(--space-2);
  }

  .advanced summary {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-height: var(--control-h);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    cursor: pointer;
    user-select: none;
    list-style: none;
  }

  .advanced summary::-webkit-details-marker {
    display: none;
  }

  .advanced summary:hover {
    background: var(--color-muted);
    color: var(--color-fg);
  }

  .advanced ul {
    margin-top: var(--space-1);
  }
</style>
