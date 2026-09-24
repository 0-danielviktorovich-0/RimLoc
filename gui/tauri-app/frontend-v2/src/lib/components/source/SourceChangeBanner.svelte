<script lang="ts">
  // External-change banner (SOURCE_INSPECTOR_MANDATE §12): mock transitions
  // only — [Rescan][Show changes][Ignore] flip store state; no watcher, no
  // real FS checks. Visible only when the active demo scenario carries a
  // change fixture and the user has not dismissed/rescanned it.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { source } from '../../source/store.svelte';

  const ec = $derived(source.scenario.externalChange);
  const visible = $derived(ec !== undefined && source.changeState === 'visible');
</script>

{#if visible && ec}
  <div class="banner" role="status" data-testid="source.changed.banner">
    <Icon name="warning" size={14} />
    <span class="text">{t(ec.summaryKey)}</span>
    <span class="file mono">{ec.displayPath}</span>
    <span class="spacer"></span>
    <button
      type="button"
      class="btn"
      data-testid="source.changed.rescan"
      onclick={() => source.changeRescan()}
    >
      {t('source.changed.rescan')}
    </button>
    <button
      type="button"
      class="btn"
      data-testid="source.changed.show"
      onclick={() => source.changeShow()}
    >
      {t('source.changed.show')}
    </button>
    <button
      type="button"
      class="btn subtle"
      data-testid="source.changed.ignore"
      onclick={() => source.changeIgnore()}
    >
      {t('source.changed.ignore')}
    </button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    border: 1px solid color-mix(in srgb, var(--warning, #c90) 55%, transparent);
    background: color-mix(in srgb, var(--warning, #c90) 12%, transparent);
    border-radius: 10px;
    padding: 6px 10px;
    margin: 0 0 8px;
    font-size: var(--font-sm, 12px);
  }
  .text {
    min-width: 0;
  }
  .file {
    font-size: var(--font-xs, 11px);
    color: var(--text-2, inherit);
    word-break: break-all;
  }
  .spacer {
    flex: 1;
  }
  .mono {
    font-family: var(--font-mono, monospace);
  }
</style>
