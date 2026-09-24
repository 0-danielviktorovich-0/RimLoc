<script lang="ts">
  // Command palette (mandate §17, spec §18): power-user bridge over the same
  // routes as the toolbar. Fuzzy search is deliberately cheap — substring
  // beats subsequence, gap penalty breaks ties; no dependency. Rendered into
  // a body-level portal by palette-boot.svelte.ts, so styles here are scoped
  // but the element lives outside #app.
  import Icon from './Icon.svelte';
  import { t } from '../../i18n/store.svelte';
  import { router, PROJECT_ROUTES, type RouteId } from '../router.svelte';
  import { palette } from '../stores/palette.svelte';
  import { languages } from '../languages/store.svelte';
  import { project } from '../stores/project.svelte';
  import { source } from '../source/store.svelte';

  interface PaletteAction {
    id: string;
    labelKey: string;
    icon: string;
    route: RouteId;
    /** Interpolation params for the label. */
    params?: Record<string, string | number>;
    /** Custom runner (language actions): when set, no navigation happens. */
    onRun?: () => void;
  }

  /** Language manager lives in the Workspace; open it there from any route. */
  function openManagerInWorkspace(intent: 'manage' | 'add'): () => void {
    return () => {
      languages.openManager(intent);
      if (!(PROJECT_ROUTES as string[]).includes(router.route)) router.navigate('workspace');
    };
  }

  /** W7 source actions: they operate on the SELECTED workspace entry; the
   *  palette reports honestly when nothing is selected instead of guessing. */
  function requireSelectedEntry(run: (entryId: string) => void): () => void {
    return () => {
      const selected = project.selected;
      if (!selected) {
        source.noteMockAction('source.palette.noSelection');
        return;
      }
      if (!(PROJECT_ROUTES as string[]).includes(router.route)) router.navigate('workspace');
      run(selected.id);
    };
  }

  /** Static actions plus one live action per target locale (W2). */
  function buildActions(): PaletteAction[] {
    const base: PaletteAction[] = [
      { id: 'translate-selected', labelKey: 'palette.action.translateSelected', icon: 'sparkles', route: 'workspace' },
      { id: 'validate-project', labelKey: 'palette.action.validateProject', icon: 'circle-check', route: 'workspace' },
      { id: 'review-errors', labelKey: 'palette.action.reviewErrors', icon: 'warning', route: 'review' },
      { id: 'build-translation', labelKey: 'palette.action.buildTranslation', icon: 'package', route: 'build' },
      { id: 'open-project', labelKey: 'palette.action.openProject', icon: 'folder-open', route: 'home' },
      { id: 'compare-versions', labelKey: 'palette.action.compareVersions', icon: 'layers', route: 'workspace' },
      { id: 'open-settings', labelKey: 'palette.action.openSettings', icon: 'settings', route: 'settings' },
      { id: 'configure-ai', labelKey: 'palette.action.configureAI', icon: 'cpu', route: 'providers' },
      { id: 'open-glossary', labelKey: 'palette.action.openGlossary', icon: 'book', route: 'glossary' },
      { id: 'open-tm', labelKey: 'palette.action.openTM', icon: 'database', route: 'tm' },
      { id: 'open-about', labelKey: 'palette.action.openAbout', icon: 'info', route: 'help' },
      { id: 'run-diagnostics', labelKey: 'palette.action.runDiagnostics', icon: 'clipboard-check', route: 'help' },
      { id: 'show-shortcuts', labelKey: 'palette.action.showShortcuts', icon: 'lightbulb', route: 'help' },
      // W7 source actions (SOURCE_INSPECTOR_MANDATE §15): palette-driven,
      // remappable shortcut entries exist with unbound defaults.
      {
        id: 'source-open-location',
        labelKey: 'palette.action.openSourceLocation',
        icon: 'file-code',
        route: 'workspace',
        onRun: requireSelectedEntry((entryId) => source.openViewer(entryId, 0))
      },
      {
        id: 'source-browser',
        labelKey: 'palette.action.openSourceBrowser',
        icon: 'layers',
        route: 'workspace',
        onRun: () => {
          if (!(PROJECT_ROUTES as string[]).includes(router.route)) router.navigate('workspace');
          source.openBrowser();
        }
      },
      {
        id: 'source-compare',
        labelKey: 'palette.action.sourceCompare',
        icon: 'git-compare',
        route: 'workspace',
        onRun: requireSelectedEntry((entryId) => source.openCompare(entryId))
      }
    ];
    // Multi-target language actions (W2): switch → <lang>, add, manage.
    const languageActions: PaletteAction[] = languages
      .summaries()
      .map((s) => ({
        id: `switch-target-${s.locale}`,
        labelKey: 'palette.action.switchTarget',
        icon: 'languages',
        route: 'workspace' as RouteId,
        params: { name: s.definition.nativeName },
        onRun: () => languages.setActive(s.locale)
      }));
    languageActions.push(
      {
        id: 'add-target-language',
        labelKey: 'languages.switcher.add',
        icon: 'file-plus',
        route: 'workspace',
        onRun: openManagerInWorkspace('add')
      },
      {
        id: 'manage-languages',
        labelKey: 'languages.switcher.manage',
        icon: 'languages',
        route: 'workspace',
        onRun: openManagerInWorkspace('manage')
      }
    );
    return [...base, ...languageActions];
  }

  let query = $state('');
  let active = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);

  /** Cheap fuzzy score: exact substring > ordered subsequence (gap penalty); -1 = no match. */
  function score(text: string, q: string): number {
    const hay = text.toLowerCase();
    const needle = q.toLowerCase();
    if (!needle) return 0;
    const idx = hay.indexOf(needle);
    if (idx >= 0) return 200 - idx; // substring: strong hit, earlier = better
    let pos = 0;
    let gaps = 0;
    for (const ch of needle) {
      const found = hay.indexOf(ch, pos);
      if (found < 0) return -1;
      gaps += found - pos;
      pos = found + 1;
    }
    return 100 - gaps;
  }

  // Rebuilt reactively so freshly added targets appear in the palette.
  const actions = $derived(buildActions());

  const results = $derived.by(() => {
    const q = query.trim();
    if (!q) return actions;
    return actions.map((a) => ({ a, s: Math.max(score(t(a.labelKey, a.params), q), score(a.id, q) - 10) }))
      .filter((r) => r.s >= 0)
      .sort((x, y) => y.s - x.s)
      .map((r) => r.a);
  });

  function run(action: PaletteAction) {
    palette.hide();
    if (action.onRun) {
      action.onRun();
      return;
    }
    router.navigate(action.route);
  }

  function onInputKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      active = Math.min(active + 1, results.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      active = Math.max(active - 1, 0);
    } else if (e.key === 'Enter') {
      const chosen = results[active];
      if (chosen) run(chosen);
    }
  }

  // Fresh query and cursor on every open; keep focus in the input.
  $effect(() => {
    if (palette.open) {
      query = '';
      active = 0;
      queueMicrotask(() => inputEl?.focus());
    }
  });

  // Query changes move the cursor back to the best match.
  $effect(() => {
    void query;
    active = 0;
  });

  // Keep the active option visible while navigating with arrows.
  $effect(() => {
    if (!palette.open) return;
    document.getElementById(`palette-opt-${active}`)?.scrollIntoView({ block: 'nearest' });
  });

  // Escape closes even when focus left the input; list closes on navigation.
  $effect(() => {
    if (!palette.open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') palette.hide();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
  $effect(() => {
    void router.route;
    palette.hide();
  });
</script>

{#if palette.open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="overlay"
    data-testid="palette.overlay"
    onpointerdown={(e) => {
      if (e.target === e.currentTarget) palette.hide();
    }}
  >
    <div class="palette" role="dialog" aria-modal="true" aria-label={t('palette.placeholder')} data-testid="palette.dialog">
      <div class="search-row">
        <Icon name="search" size={16} />
        <input
          bind:this={inputEl}
          bind:value={query}
          onkeydown={onInputKey}
          type="text"
          role="combobox"
          aria-expanded={results.length > 0}
          aria-controls="palette-list"
          aria-activedescendant={results[active] ? `palette-opt-${active}` : undefined}
          aria-label={t('palette.placeholder')}
          placeholder={t('palette.placeholder')}
          data-testid="palette.input"
        />
      </div>

      <ul id="palette-list" class="list" role="listbox" aria-label={t('palette.placeholder')}>
        {#each results as action, i (action.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
          <li
            id={`palette-opt-${i}`}
            role="option"
            aria-selected={i === active}
            class="opt"
            class:active={i === active}
            data-testid={`palette.item.${action.id}`}
            onpointerenter={() => (active = i)}
            onclick={() => run(action)}
          >
            <Icon name={action.icon} size={15} />
            <span class="opt-label">{t(action.labelKey, action.params)}</span>
            <Icon name="arrow-right" size={13} />
          </li>
        {:else}
          <li class="empty">{t('palette.noResults')}</li>
        {/each}
      </ul>

      <p class="hint">{t('palette.hint')}</p>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgb(0 0 0 / 40%);
    animation: overlay-in var(--motion-fast) var(--ease-out);
  }

  .palette {
    width: min(560px, calc(100vw - var(--space-8)));
    margin: 10vh auto 0;
    background: var(--color-surface);
    color: var(--color-surface-fg);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-overlay);
    overflow: hidden;
    animation: palette-in var(--motion-emphasis) var(--ease-emphasis);
  }

  .search-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3);
    border-bottom: 1px solid var(--color-border);
    color: var(--color-muted-fg);
  }

  .search-row input {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--color-surface-fg);
    padding: 0;
    font-size: var(--text-base-size);
  }

  .search-row input:focus-visible {
    outline: none;
  }

  .search-row input::placeholder {
    color: var(--color-muted-fg);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: var(--space-1);
    max-height: 46vh;
    overflow-y: auto;
  }

  .opt {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    cursor: pointer;
    min-height: var(--control-h);
    color: var(--color-surface-fg);
  }

  .opt :global(svg:first-child) {
    color: var(--color-muted-fg);
    flex: none;
  }

  .opt :global(svg:last-child) {
    margin-left: auto;
    color: var(--color-muted-fg);
    opacity: 0;
  }

  .opt.active {
    background: var(--nav-active-bg);
  }

  .opt.active :global(svg:last-child) {
    opacity: 1;
    color: var(--color-primary-text);
  }

  .opt-label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .empty {
    padding: var(--space-4);
    color: var(--color-muted-fg);
    text-align: center;
  }

  .hint {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border-top: 1px solid var(--color-border);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  @keyframes overlay-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes palette-in {
    from {
      opacity: 0;
      transform: translateY(-6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
