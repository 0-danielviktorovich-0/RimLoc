<script lang="ts">
  // Translator Workspace (spec §2.4): three-panel layout, status chips,
  // SOURCE|TARGET table with inline editing, context panel. Mock states
  // (loading / empty / error) are forced from the dev panel.
  import { t } from '../../../i18n/store.svelte';
  import { ui } from '../../stores/ui.svelte';
  import { project } from '../../stores/project.svelte';
  import Navigator from '../workspace/Navigator.svelte';
  import StatusChips from '../workspace/StatusChips.svelte';
  import EntryTable from '../workspace/EntryTable.svelte';
  import ContextPanel from '../workspace/ContextPanel.svelte';
  import Icon from '../Icon.svelte';
  import { MOCK_ERROR_CODE, MOCK_ERROR_RAW } from '../../../lib/mock/data';

  const SKELETON_ROWS = 8; // spec §3: table loading shows 8 placeholder rows

  const counts = $derived(project.statusCounts());
  const kindCounts = $derived.by(() => {
    const acc: Record<string, number> = { all: project.entries.length };
    for (const e of project.entries) acc[e.kind] = (acc[e.kind] ?? 0) + 1;
    return acc;
  });
  const rows = $derived(project.filtered());

  // Below 1100px the context panel becomes a bottom sheet (spec §2.4).
  let contextOpen = $state(false);
  const narrowQuery =
    typeof window !== 'undefined' && window.matchMedia
      ? window.matchMedia('(max-width: 1100px)')
      : null;

  // Selecting a row on a narrow viewport surfaces the sheet automatically.
  $effect(() => {
    if (project.selectedId && narrowQuery?.matches) contextOpen = true;
  });

  function closeContext() {
    contextOpen = false;
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && contextOpen) closeContext();
  }

  // Search debounced at 300ms per spec §2.4 before it reaches the store.
  let searchInput = $state('');
  let searchTimer: ReturnType<typeof setTimeout> | undefined;

  function onSearchInput(event: Event) {
    searchInput = (event.currentTarget as HTMLInputElement).value;
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      project.search = searchInput;
    }, 300);
  }

  function closeProject() {
    // Spec principle 3 (no data loss): flush pending drafts before leaving.
    project.flushAll();
    ui.closeWorkspace();
  }

  function retry() {
    // Mock recovery path: in the real app this re-runs the service read.
    ui.wsState = 'ready';
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<section class="workspace" aria-label={t('workspace.title')}>
  <div class="toolbar">
    <h1 class="toolbar-title">{t('workspace.title')}</h1>
    <span class="toolbar-hint">
      <span class="toolbar-hint-icon" aria-hidden="true"><Icon name="arrow-right" size={12} /></span>
      {t('workspace.editor.hint')}
    </span>
    <button
      type="button"
      class="btn context-toggle"
      aria-expanded={contextOpen}
      aria-controls="workspace-context"
      data-testid="workspace.context-toggle"
      onclick={() => (contextOpen = !contextOpen)}
    >
      <Icon name="info" size={14} />
      {t('workspace.context.label')}
    </button>
    <button
      type="button"
      class="btn"
      data-testid="workspace.close-project"
      onclick={closeProject}
    >
      <Icon name="close" size={14} />
      {t('workspace.closeProject')}
    </button>
  </div>

  {#if ui.wsState === 'loading'}
    <div class="loading" role="status" aria-live="polite">
      <p class="loading-text">{t('workspace.state.loading')}</p>
      <div class="chips-skeleton" aria-hidden="true"></div>
      {#each Array(SKELETON_ROWS) as _, i (i)}
        <div class="skeleton-row" aria-hidden="true"></div>
      {/each}
    </div>
  {:else if ui.wsState === 'error'}
    <div class="state-card" role="alert" data-testid="workspace.error">
      <p class="state-title">{t('common.errorTitle')}</p>
      <p>{t('workspace.state.error')}</p>
      <p class="mono error-code">{MOCK_ERROR_CODE}</p>
      <button type="button" class="btn" onclick={retry}>{t('common.retry')}</button>
      <details class="raw">
        <summary>{t('common.details')}</summary>
        <pre class="mono">{MOCK_ERROR_RAW}</pre>
      </details>
    </div>
  {:else if ui.wsState === 'empty'}
    <div class="state-card" data-testid="workspace.empty">
      <p class="state-title">{t('workspace.state.empty.title')}</p>
      <p>{t('workspace.state.empty.desc')}</p>
      <button type="button" class="btn" onclick={() => (ui.wsState = 'ready')}>
        {t('common.reset')}
      </button>
    </div>
  {:else}
    <div class="layout">
      <div class="pane-left">
        <Navigator {kindCounts} />
      </div>
      <div class="pane-center">
        <div class="search-row">
          <span class="search-icon"><Icon name="search" size={14} /></span>
          <input
            class="search"
            type="search"
            placeholder={t('workspace.search.placeholder')}
            aria-label={t('workspace.search.label')}
            data-testid="workspace.search"
            value={searchInput}
            oninput={onSearchInput}
          />
        </div>
        <StatusChips {counts} />
        {#if rows.length === 0}
          <div class="no-results" data-testid="workspace.no-results">
            <p class="state-title">{t('workspace.noResults.title')}</p>
            <p>{t('workspace.noResults.desc')}</p>
            <button type="button" class="btn" onclick={() => project.clearFilters()}>
              {t('common.reset')}
            </button>
          </div>
        {:else}
          <EntryTable {rows} rowcount={rows.length} />
        {/if}
      </div>
      <div class="pane-right" class:open={contextOpen}>
        <div class="sheet-head">
          <span class="sheet-title">{t('workspace.context.label')}</span>
          <button
            type="button"
            class="btn sheet-close"
            aria-label={t('common.close')}
            data-testid="workspace.context-close"
            onclick={closeContext}
          >
            <Icon name="close" size={14} />
          </button>
        </div>
        <ContextPanel entry={project.selected} />
      </div>
    </div>
  {/if}
</section>

<style>
  .workspace {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .toolbar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-4);
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    flex: none;
  }

  .toolbar-title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
  }

  .toolbar-hint {
    margin-right: auto;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
  }

  .toolbar-hint-icon {
    display: inline-flex;
  }

  /* Context toggle + sheet chrome exist only on narrow viewports. */
  .context-toggle,
  .sheet-head {
    display: none;
  }

  .layout {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 240px minmax(0, 1fr) 320px;
  }

  .pane-left {
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .pane-left > :global(*) {
    flex: 1;
  }

  .pane-center {
    min-height: 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .pane-right {
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .pane-right > :global(*) {
    flex: 1;
  }

  .search-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    flex: none;
  }

  .search-icon {
    color: var(--color-muted-fg);
    display: inline-flex;
  }

  .search {
    flex: 1;
    min-height: var(--control-h);
  }

  /* States */
  .state-card {
    margin: var(--space-8) auto;
    max-width: 460px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    align-items: flex-start;
  }

  .state-card[role='alert'] {
    border-left: 3px solid var(--color-destructive);
  }

  .state-title {
    margin: 0;
    font-weight: 600;
  }

  .error-code {
    color: var(--color-muted-fg);
    margin: 0;
  }

  .raw {
    width: 100%;
  }

  .raw pre {
    background: var(--color-muted);
    border-radius: var(--radius-sm);
    padding: var(--space-2);
    overflow: auto;
  }

  .no-results {
    margin: var(--space-6) auto;
    max-width: 420px;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    align-items: flex-start;
    padding: var(--space-4);
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-lg);
  }

  .no-results .state-title {
    margin: 0;
  }

  /* Loading skeletons */
  .loading {
    padding: var(--space-2) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .loading-text {
    margin: 0;
    color: var(--color-muted-fg);
  }

  .chips-skeleton {
    height: 32px;
    width: 60%;
    border-radius: var(--radius-sm);
    background: var(--color-muted);
  }

  .skeleton-row {
    height: 40px;
    border-radius: var(--radius-sm);
    background: var(--color-muted);
    animation: pulse 1.4s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.55;
    }
  }

  /* Below 1100px the context panel folds into a bottom sheet (spec §2.4):
     fixed to the viewport bottom, slides with EMPHASIS motion, dismissed by
     the sheet close button or Escape. The grid keeps two panes. */
  @media (max-width: 1100px) {
    .layout {
      grid-template-columns: 240px minmax(0, 1fr);
    }

    .context-toggle {
      display: inline-flex;
    }

    .toolbar-hint {
      display: none;
    }

    .pane-right {
      position: fixed;
      left: 0;
      right: 0;
      bottom: 0;
      z-index: 30;
      max-height: 55vh;
      border-top: 1px solid var(--color-border-strong);
      border-left: none;
      box-shadow: var(--shadow-overlay);
      background: var(--color-surface);
      transform: translateY(100%);
      visibility: hidden;
      transition:
        transform var(--motion-emphasis) var(--ease-emphasis),
        visibility 0s linear var(--motion-emphasis);
    }

    .pane-right.open {
      transform: translateY(0);
      visibility: visible;
      transition: transform var(--motion-emphasis) var(--ease-emphasis);
    }

    .sheet-head {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: var(--space-2);
      padding: var(--space-2) var(--space-4);
      border-bottom: 1px solid var(--color-border);
      flex: none;
    }

    .sheet-title {
      font-weight: 600;
    }

    .sheet-close {
      min-height: var(--control-h);
      padding: 0 var(--space-2);
    }
  }
</style>
