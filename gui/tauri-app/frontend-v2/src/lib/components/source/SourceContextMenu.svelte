<script lang="ts">
  // Entry context menu (SOURCE_INSPECTOR_MANDATE §9): open original/effective
  // where meaningful, reveal, external editor, copy location, generated
  // output, candidates, compare. Every OS action is demo-honest: it reports
  // what WOULD happen and never claims a real launch. Actions that need a
  // real surface (viewer/browser/compare) open the W7 overlays.
  import { onMount } from 'svelte';
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { source } from '../../source/store.svelte';
  import { loadEditorChoice } from '../../source/editor';

  const menu = $derived(source.menu);
  const data = $derived(menu ? source.contextFor(menu.entryId) : null);
  const primary = $derived(data?.usages.find((u) => u.role === 'primary') ?? data?.usages[0] ?? null);
  const hasOriginal = $derived(
    data !== null && data.usages.some((u) => !u.effective || u.location.path !== primary?.location.path)
  );
  const hasGenerated = $derived(menu !== null && source.compareFor(menu.entryId) !== null);

  let el: HTMLDivElement | undefined = $state();

  function close() {
    source.closeMenu();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  onMount(() => {
    window.addEventListener('keydown', onKeydown);
    window.addEventListener('resize', close);
    return () => {
      window.removeEventListener('keydown', onKeydown);
      window.removeEventListener('resize', close);
    };
  });

  function mock(labelKey: string, params?: Record<string, string | number>) {
    source.noteMockAction(labelKey, params);
    close();
  }

  async function copyLoc() {
    if (!menu || !primary) return;
    await source.copyText(
      'menu-location',
      'source.copy.location',
      `${primary.location.displayPath}:${primary.location.line ?? '?'}`
    );
    close();
  }

  function openEditor() {
    if (!menu || !primary) return;
    const plan = source.planEditorLaunch(loadEditorChoice(), primary);
    if (!plan.ok) {
      source.noteMockAction(plan.reasonKey);
    } else {
      // honest demo: show the argv that WOULD be executed
      source.noteMockAction('source.action.wouldLaunch');
      void source.copyText('menu-argv', 'source.copy.argv', JSON.stringify(plan.argv));
    }
    close();
  }
</script>

{#if menu && data && primary}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div
    class="scrim"
    data-testid="source.menu.scrim"
    onclick={close}
    oncontextmenu={(e) => {
      e.preventDefault();
      close();
    }}
  >
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions, a11y_no_noninteractive_tabindex -->
    <div
      class="menu"
      role="menu"
      tabindex="-1"
      data-testid="source.menu"
      style="left:{Math.min(menu.x, window.innerWidth - 240)}px; top:{Math.min(menu.y, window.innerHeight - 300)}px;"
      bind:this={el}
      onclick={(e) => e.stopPropagation()}
    >
      <button type="button" role="menuitem" onclick={() => { source.openViewer(menu.entryId, 0); }}>
        <Icon name="file-code" size={13} />
        {t('source.menu.openEffective')}
      </button>
      {#if hasOriginal}
        <button
          type="button"
          role="menuitem"
          onclick={() => {
            const idx = data.usages.findIndex((u) => !u.effective);
            source.openViewer(menu.entryId, idx === -1 ? 0 : idx);
          }}
        >
          <Icon name="file-code" size={13} />
          {t('source.menu.openOriginal')}
        </button>
      {/if}
      <button type="button" role="menuitem" onclick={() => mock('source.action.wouldReveal', { path: primary.location.displayPath })}>
        <Icon name="external" size={13} />
        {t('source.menu.reveal')}
      </button>
      <button type="button" role="menuitem" onclick={openEditor}>
        <Icon name="edit" size={13} />
        {t('source.menu.editor')}
      </button>
      <button type="button" role="menuitem" onclick={copyLoc}>
        <Icon name="copy" size={13} />
        {t('source.menu.copyLocation')}
      </button>
      <button type="button" role="menuitem" onclick={() => mock('source.menu.wouldOpenGenerated')}>
        <Icon name="package" size={13} />
        {t('source.menu.generated')}
      </button>
      <button type="button" role="menuitem" onclick={() => source.openBrowser()}>
        <Icon name="layers" size={13} />
        {t('source.menu.candidates')}
      </button>
      {#if hasGenerated}
        <button type="button" role="menuitem" onclick={() => source.openCompare(menu.entryId)}>
          <Icon name="git-compare" size={13} />
          {t('source.menu.compare')}
        </button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 80;
  }
  .menu {
    position: fixed;
    display: flex;
    flex-direction: column;
    min-width: 220px;
    background: var(--surface, #1a1a1a);
    border: 1px solid var(--border, #ccc3);
    border-radius: 10px;
    padding: 4px;
    box-shadow: 0 10px 30px #0005;
  }
  .menu button {
    display: flex;
    align-items: center;
    gap: 8px;
    background: transparent;
    border: none;
    color: inherit;
    text-align: left;
    font-size: var(--font-sm, 12px);
    padding: 6px 8px;
    border-radius: 6px;
    cursor: pointer;
  }
  .menu button:hover {
    background: color-mix(in srgb, var(--accent, #4a8) 14%, transparent);
  }
</style>
