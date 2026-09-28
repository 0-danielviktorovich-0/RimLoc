<script lang="ts">
  // Status filter bar (mandate §10): the most useful quick chips stay visible;
  // the remaining dimensions (full status list, record kind, origin) live in a
  // filter popover. Dimensions combine with AND, statuses with OR. The filter
  // button carries the active-filters badge. Meaning is never color-only:
  // every chip has a text label (spec §1.2).
  import { t } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import type { StatusCounts } from '../../stores/project.svelte';
  import type { EntryStatus, Origin } from '../../mock/types';
  import Icon from '../Icon.svelte';

  let { counts }: { counts: StatusCounts } = $props();

  // Quick filters (mandate §10: keep the useful ones visible). M-3 (UI audit
  // 2026-09-29): the chips are the ONE filter surface — the left navigator
  // mirrors this exact set, so every status is reachable from both and the
  // sets can never diverge again ('orphan' used to be navigator-only).
  const QUICK: EntryStatus[] = [
    'untranslated',
    'pending_review',
    'sourceChanged',
    'translated',
    'todo',
    'orphan'
  ];
  const ALL_STATUSES: EntryStatus[] = [
    'untranslated',
    'translated',
    'todo',
    'sourceChanged',
    'pending_review',
    'orphan'
  ];
  const KINDS = ['Keyed', 'DefInjected', 'TKey'] as const;
  const ORIGINS: Array<Origin | 'any'> = ['any', 'human', 'TM', 'LLM', 'imported'];

  let open = $state(false);

  const activeCount = $derived(project.activeFilterCount());

  function dotColor(status: EntryStatus): string {
    const name = status === 'sourceChanged' ? 'source-changed' : status.replace('_', '-');
    return `background: var(--color-status-${name})`;
  }

  function onWindowKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) open = false;
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<div class="chips" role="group" aria-label={t('workspace.filter.label')}>
  <span class="chips-icon" aria-hidden="true"><Icon name="filter" size={14} /></span>
  {#each QUICK as status (status)}
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

  <div class="popover-anchor">
    <button
      type="button"
      class="chip filter-btn"
      aria-expanded={open}
      aria-haspopup="true"
      aria-label="{t('workspace.filters.button')}{activeCount > 0 ? `, ${activeCount}` : ''}"
      data-testid="workspace.filters.button"
      onclick={() => (open = !open)}
    >
      <span class="filter-icon" aria-hidden="true"><Icon name="sliders" size={14} /></span>
      {t('workspace.filters.button')}
      {#if activeCount > 0}
        <span class="active-badge" aria-hidden="true">{activeCount}</span>
      {/if}
    </button>

    {#if open}
      <!-- Click-away surface: a full-screen button that closes the popover. -->
      <button
        type="button"
        class="backdrop"
        tabindex="-1"
        aria-label={t('common.close')}
        onclick={() => (open = false)}
      ></button>
      <div class="popover" role="group" aria-label={t('workspace.filters.title')} data-testid="workspace.filters.popover">
        <p class="pop-title">{t('workspace.filters.title')}</p>

        <p class="pop-section">{t('workspace.filters.status')}</p>
        <div class="pop-chips">
          {#each ALL_STATUSES as status (status)}
            <button
              type="button"
              class="chip"
              aria-pressed={project.filters.includes(status)}
              data-testid={`workspace.filters.status.${status}`}
              onclick={() => project.toggleFilter(status)}
            >
              <span class="dot" style={dotColor(status)} aria-hidden="true"></span>
              {t(`workspace.filter.${status}`)}
              <span class="chip-count" aria-hidden="true">{counts[status]}</span>
            </button>
          {/each}
        </div>

        <p class="pop-section">{t('workspace.filters.kind')}</p>
        <div class="pop-chips">
          {#each KINDS as kind (kind)}
            <button
              type="button"
              class="chip"
              aria-pressed={project.category === kind}
              data-testid={`workspace.filters.kind.${kind}`}
              onclick={() => (project.category = project.category === kind ? 'all' : kind)}
            >
              <span class="mono">{t(`kind.${kind}`)}</span>
            </button>
          {/each}
        </div>

        <p class="pop-section">{t('workspace.filters.origin')}</p>
        <div class="pop-chips">
          {#each ORIGINS as o (o)}
            <button
              type="button"
              class="chip"
              aria-pressed={project.originFilter === o}
              data-testid={`workspace.filters.origin.${o}`}
              onclick={() => (project.originFilter = o)}
            >
              {o === 'any' ? t('workspace.filters.origin.any') : t(`workspace.filters.origin.${o}`)}
            </button>
          {/each}
        </div>

        <div class="pop-actions">
          <button type="button" class="btn" data-testid="workspace.filters.clear" onclick={() => project.clearFilters()}>
            {t('workspace.filters.clear')}
          </button>
          <button type="button" class="btn btn-primary" onclick={() => (open = false)}>
            {t('workspace.filters.done')}
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    flex: none;
  }

  .chips-icon {
    display: inline-flex;
    color: var(--color-muted-fg);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--control-h);
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    font-size: var(--text-meta-size);
    color: var(--color-fg);
    transition:
      border-color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out),
      transform var(--motion-fast) var(--ease-out);
  }

  .chip:hover {
    border-color: var(--color-border-strong);
    background: var(--color-muted);
  }

  /* Press feedback on chips — microinteraction, instant enough to feel direct. */
  .chip:active {
    transform: var(--btn-press-transform);
  }

  .chip[aria-pressed='true'] {
    border-color: var(--color-primary);
    background: var(--nav-active-bg);
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

  .popover-anchor {
    position: relative;
    display: inline-flex;
    margin-left: auto;
  }

  .filter-btn {
    color: var(--color-muted-fg);
  }

  .filter-btn[aria-expanded='true'],
  .filter-btn:hover {
    color: var(--color-fg);
  }

  .filter-icon {
    display: inline-flex;
  }

  .active-badge {
    min-width: 18px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 9px;
    background: var(--color-primary);
    color: var(--color-primary-fg);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    padding: 0 4px;
  }

  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    cursor: default;
    background: transparent;
  }

  .popover {
    position: absolute;
    top: calc(100% + var(--space-1));
    right: 0;
    z-index: 21;
    width: 320px;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-overlay);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    animation: pop-in var(--motion-standard) var(--ease-out);
  }

  @keyframes pop-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .popover {
      animation: none;
    }
  }

  .pop-title {
    margin: 0;
    font-weight: 600;
  }

  .pop-section {
    margin: var(--space-1) 0 0;
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .pop-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }

  .pop-actions {
    display: flex;
    justify-content: space-between;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
</style>
