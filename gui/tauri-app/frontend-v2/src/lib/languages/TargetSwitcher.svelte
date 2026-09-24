<script lang="ts">
  // Target-language switcher for the Workspace header (W2): the project
  // language pair surfaced as `English → [Русский ▾] [+]`, with a searchable
  // dropdown (per-target progress/issues), a quick [+] for the add flow and
  // pinned quick tabs. The INTERFACE language (AppHeader RU/EN) stays
  // completely independent — this control only moves project.targetLocale
  // semantics (which dataset the editor shows), never i18n.locale.
  import { t } from '../../i18n/store.svelte';
  import { languages } from './store.svelte';
  import { registry } from './registry';
  import Icon from '../components/Icon.svelte';

  let open = $state(false);
  let query = $state('');

  const summaries = $derived(languages.summaries());
  const active = $derived(registry.resolve(languages.activeLocale));
  const activeSummary = $derived(summaries.find((s) => s.locale === languages.activeLocale));
  const pinnedSummaries = $derived(summaries.filter((s) => languages.isPinned(s.locale)));

  const filtered = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return summaries;
    return summaries.filter(
      (s) =>
        s.definition.nativeName.toLowerCase().includes(needle) ||
        s.definition.displayName.toLowerCase().includes(needle) ||
        s.locale.toLowerCase().includes(needle)
    );
  });

  function choose(locale: string) {
    open = false;
    query = '';
    languages.setActive(locale);
  }

  function openAdd() {
    open = false;
    languages.openManager('add');
  }

  function openManage() {
    open = false;
    languages.openManager('manage');
  }

  function onWindowKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) open = false;
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<span class="switcher-group" role="group" aria-label={t('languages.switcher.label')}>
  <span class="source mono">{t('languages.switcher.source')}: {registry.resolve(languages.sourceLocale).nativeName}</span>
  <span class="arrow" aria-hidden="true">→</span>

  <span class="anchor">
    <button
      type="button"
      class="btn switch-btn"
      aria-expanded={open}
      aria-haspopup="listbox"
      data-testid="languages.switcher.button"
      onclick={() => (open = !open)}
    >
      <span class="name" dir={active.direction}>{active.nativeName}</span>
      {#if activeSummary}
        <span class="pct mono">{activeSummary.progress}%</span>
      {/if}
      <Icon name="chevron-down" size={13} />
    </button>

    {#if open}
      <!-- Click-away surface closes the dropdown (FilterBar pattern). -->
      <button
        type="button"
        class="backdrop"
        tabindex="-1"
        aria-label={t('common.close')}
        onclick={() => (open = false)}
      ></button>
      <div class="menu" role="listbox" aria-label={t('languages.switcher.label')} data-testid="languages.switcher.menu">
        <div class="search-row">
          <Icon name="search" size={13} />
          <input
            type="search"
            bind:value={query}
            placeholder={t('languages.switcher.search')}
            aria-label={t('languages.switcher.search')}
            data-testid="languages.switcher.search"
          />
        </div>
        <div class="list">
          {#each filtered as s (s.locale)}
            <button
              type="button"
              role="option"
              aria-selected={s.locale === languages.activeLocale}
              class="opt"
              class:current={s.locale === languages.activeLocale}
              data-testid={`languages.switcher.target.${s.locale}`}
              onclick={() => choose(s.locale)}
            >
              <span class="opt-name" dir={s.definition.direction}>{s.definition.nativeName}</span>
              <span class="opt-meta mono">{s.definition.localeId}</span>
              <span class="opt-pct mono">{s.progress}%</span>
              {#if s.issueCount > 0}
                <span class="opt-issues" title={t('languages.manager.issues')}>
                  <Icon name="warning" size={12} />
                  {s.issueCount}
                </span>
              {/if}
              {#if s.locale === languages.activeLocale}
                <Icon name="circle-check" size={13} />
              {/if}
            </button>
          {:else}
            <p class="empty">{t('palette.noResults')}</p>
          {/each}
        </div>
        <div class="menu-actions">
          <button type="button" class="menu-action" data-testid="languages.switcher.add" onclick={openAdd}>
            <Icon name="file-plus" size={13} />
            {t('languages.switcher.add')}
          </button>
          <button type="button" class="menu-action" data-testid="languages.switcher.manage" onclick={openManage}>
            <Icon name="languages" size={13} />
            {t('languages.switcher.manage')}
          </button>
        </div>
      </div>
    {/if}
  </span>

  <button
    type="button"
    class="btn plus-btn"
    aria-label={t('languages.switcher.add')}
    title={t('languages.switcher.add')}
    data-testid="languages.switcher.plus"
    onclick={openAdd}
  >
    +
  </button>
</span>

{#if pinnedSummaries.length > 0}
  <span class="pins" role="group" aria-label={t('languages.pinned.label')}>
    {#each pinnedSummaries as s (s.locale)}
      <button
        type="button"
        class="pin"
        aria-pressed={s.locale === languages.activeLocale}
        data-testid={`languages.pinned.${s.locale}`}
        onclick={() => choose(s.locale)}
      >
        <span class="pin-name" dir={s.definition.direction}>{s.definition.nativeName}</span>
        <span class="pin-pct mono">{s.progress}%</span>
      </button>
    {/each}
  </span>
{/if}

<style>
  .switcher-group {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
  }

  .source {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    white-space: nowrap;
  }

  .arrow {
    color: var(--color-muted-fg);
  }

  .anchor {
    position: relative;
    display: inline-flex;
  }

  .switch-btn,
  .plus-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-height: var(--control-h);
    padding: 0 var(--space-2);
    font-size: var(--text-meta-size);
    color: var(--color-fg);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
  }

  .switch-btn:hover,
  .plus-btn:hover {
    background: var(--color-muted);
  }

  .switch-btn[aria-expanded='true'] {
    border-color: var(--color-primary);
    color: var(--color-primary-text);
  }

  .plus-btn {
    font-weight: 700;
    padding: 0 var(--space-2);
  }

  .pct {
    color: var(--color-muted-fg);
  }

  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: transparent;
    border: none;
    cursor: default;
  }

  .menu {
    position: absolute;
    top: calc(100% + var(--space-1));
    left: 0;
    z-index: 41;
    width: 300px;
    background: var(--color-surface);
    color: var(--color-surface-fg);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-overlay);
    overflow: hidden;
    animation: menu-in var(--motion-fast) var(--ease-out);
  }

  @keyframes menu-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .search-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    color: var(--color-muted-fg);
  }

  .search-row input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    color: inherit;
    padding: 0;
    font-size: var(--text-meta-size);
  }

  .search-row input:focus-visible {
    outline: none;
  }

  .list {
    max-height: 260px;
    overflow-y: auto;
    padding: var(--space-1);
  }

  .opt {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    min-height: var(--control-h);
    padding: 0 var(--space-2);
    border-radius: var(--radius-md);
    color: inherit;
    text-align: left;
    font-size: var(--text-meta-size);
  }

  .opt:hover,
  .opt.current {
    background: var(--nav-active-bg);
  }

  .opt-name {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .opt-meta {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .opt-pct {
    color: var(--color-muted-fg);
    min-width: 34px;
    text-align: right;
  }

  .opt-issues {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    color: var(--color-warning);
  }

  .empty {
    margin: 0;
    padding: var(--space-3);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .menu-actions {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--color-border);
    padding: var(--space-1);
  }

  .menu-action {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--control-h);
    padding: 0 var(--space-2);
    border-radius: var(--radius-md);
    color: var(--color-primary-text);
    font-size: var(--text-meta-size);
    text-align: left;
  }

  .menu-action:hover {
    background: var(--color-muted);
  }

  .pins {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .pin {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-height: 26px;
    padding: 0 var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: 999px;
    background: var(--color-bg);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .pin:hover {
    background: var(--color-muted);
    color: var(--color-fg);
  }

  .pin[aria-pressed='true'] {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: var(--color-primary-fg);
  }

  .pin-pct {
    opacity: 0.75;
  }
</style>
