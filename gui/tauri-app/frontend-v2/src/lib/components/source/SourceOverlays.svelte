<script lang="ts">
  // Source overlays host (W7): context menu, read-only viewer, advanced
  // browser + compare, clipboard fallback card and the mock-action toast.
  // Mounted once from the Workspace shell (project-scoped surfaces); the
  // external-change banner is exposed separately as SourceChangeBanner so
  // hosts can place it where it fits.
  import { t } from '../../../i18n/store.svelte';
  import Icon from '../Icon.svelte';
  import SourceContextMenu from './SourceContextMenu.svelte';
  import SourceViewer from './SourceViewer.svelte';
  import SourceBrowser from './SourceBrowser.svelte';
  import { source } from '../../source/store.svelte';

  const toast = $derived(source.lastMockAction);
</script>

<SourceContextMenu />
<SourceViewer />
<SourceBrowser />

{#if source.clipboardFallback}
  <div
    class="fallback-card"
    role="status"
    data-testid="source.clipboard.fallback"
  >
    <p>
      <Icon name="clipboard-check" size={13} />
      {t('source.copy.fallbackCard')}
    </p>
    <pre class="mono">{source.clipboardFallback.text}</pre>
    <button type="button" class="btn subtle" onclick={() => source.clearClipboardFallback()}>
      {t('common.close')}
    </button>
  </div>
{/if}

{#if toast}
  <div
    class="toast"
    role="status"
    data-testid="source.mock.toast"
  >
    <Icon name="info" size={13} />
    {t(toast.labelKey, toast.params)}
  </div>
{/if}

<style>
  .fallback-card {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 85;
    width: min(420px, 90vw);
    background: var(--surface, #1a1a1a);
    border: 1px solid var(--warning, #c90);
    border-radius: 10px;
    padding: 10px;
  }
  .fallback-card p {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 6px;
    font-size: var(--font-xs, 11px);
  }
  .fallback-card pre {
    max-height: 140px;
    overflow: auto;
    margin: 0 0 6px;
    padding: 8px;
    border: 1px dashed var(--border, #ccc3);
    border-radius: 8px;
    font-size: var(--font-xs, 11px);
    white-space: pre-wrap;
    word-break: break-all;
    user-select: text;
  }
  .toast {
    position: fixed;
    left: 50%;
    transform: translateX(-50%);
    bottom: 18px;
    z-index: 85;
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--surface, #1a1a1a);
    border: 1px solid var(--border, #ccc3);
    border-radius: 999px;
    padding: 6px 14px;
    font-size: var(--font-sm, 12px);
    box-shadow: 0 8px 24px #0004;
  }
  .mono {
    font-family: var(--font-mono, monospace);
  }
</style>
