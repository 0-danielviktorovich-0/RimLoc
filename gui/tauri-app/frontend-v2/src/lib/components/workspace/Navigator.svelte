<script lang="ts">
  // Left navigator (spec §2.4, 240px): category list filtering the table.
  import { t } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import type { EntryKind } from '../../mock/types';

  const CATEGORIES: Array<'all' | EntryKind> = ['all', 'Keyed', 'DefInjected', 'TKey'];

  let { kindCounts }: { kindCounts: Record<string, number> } = $props();

  function label(kind: 'all' | EntryKind): string {
    return kind === 'all' ? t('workspace.navigator.all') : t(`kind.${kind}`);
  }
</script>

<nav class="navigator" aria-label={t('workspace.navigator.title')} data-testid="workspace.navigator">
  <p class="nav-title">{t('workspace.navigator.title')}</p>
  <ul class="nav-list">
    {#each CATEGORIES as kind (kind)}
      <li>
        <button
          type="button"
          class="nav-item"
          aria-pressed={project.category === kind}
          data-testid={`workspace.navigator.${kind}`}
          onclick={() => (project.category = kind)}
        >
          <span class="nav-label">{label(kind)}</span>
          <span class="nav-count" aria-hidden="true">{kindCounts[kind] ?? 0}</span>
        </button>
      </li>
    {/each}
  </ul>
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
  }
</style>
