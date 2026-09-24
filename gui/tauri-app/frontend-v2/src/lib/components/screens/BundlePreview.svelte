<script lang="ts">
  // Support-bundle redaction preview (mandate §17, W4.5/W5 item 6): the three
  // explicit sections — Included / Redacted / Excluded. Shared by the Help
  // bug-report card and the #/diagnostics deep screen; the sanitized data
  // always comes from the diagnostics store, never recomputed here.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import type { BundlePreview as BundlePreviewData } from '../../mock/diagnostics';

  let { preview }: { preview: BundlePreviewData } = $props();

  const sections = $derived([
    {
      id: 'included' as const,
      items: preview.included,
      titleKey: 'diagnostics.bundle.included',
      icon: 'circle-check',
      count: preview.counts.included
    },
    {
      id: 'redacted' as const,
      items: preview.redacted,
      titleKey: 'diagnostics.bundle.redacted',
      icon: 'warning',
      count: preview.counts.redacted
    },
    {
      id: 'excluded' as const,
      items: preview.excluded,
      titleKey: 'diagnostics.bundle.excluded',
      icon: 'close',
      count: preview.counts.excluded
    }
  ]);
</script>

<div class="bundle" data-testid="diagnostics.bundle">
  <p class="intro">{t('diagnostics.bundle.intro')}</p>
  {#each sections as section (section.id)}
    <section class="sec" data-state={section.id} data-testid={`diagnostics.bundle.${section.id}`}>
      <h4 class="sec-title">
        <Icon name={section.icon} size={13} />
        {t(section.titleKey)}
        <span class="count mono">{section.count}</span>
      </h4>
      <ul class="items">
        {#each section.items as item (item.key)}
          <li class="item" data-testid={`diagnostics.bundle.${section.id}.${item.key}`}>
            <span class="key mono">{item.key}</span>
            <span class="value mono" class:redacted-value={item.state === 'redacted'}>{item.value}</span>
            {#if item.state === 'redacted' && item.reasonKey}
              <span class="badge">{t(item.reasonKey)}</span>
            {:else if item.state === 'excluded' && item.reasonKey}
              <span class="badge">{t(item.reasonKey)}</span>
            {:else if item.normalized}
              <span class="badge normalized">{t('bundle.reason.homePath')}</span>
            {/if}
          </li>
        {/each}
      </ul>
    </section>
  {/each}
</div>

<style>
  .bundle {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .intro {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-dense-size);
  }

  .sec {
    border: 1px solid var(--color-border);
    border-left-width: 3px;
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .sec[data-state='included'] {
    border-left-color: var(--color-success);
  }

  .sec[data-state='redacted'] {
    border-left-color: var(--color-warning);
  }

  .sec[data-state='excluded'] {
    border-left-color: var(--color-border-strong);
  }

  .sec-title {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    font-size: var(--text-dense-size);
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    background: var(--color-muted);
  }

  .sec[data-state='included'] .sec-title :global(svg) {
    color: var(--color-success);
  }

  .sec[data-state='redacted'] .sec-title :global(svg) {
    color: var(--color-warning);
  }

  .sec[data-state='excluded'] .sec-title :global(svg) {
    color: var(--color-muted-fg);
  }

  .count {
    margin-left: auto;
    color: var(--color-muted-fg);
    font-variant-numeric: tabular-nums;
  }

  .items {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .item {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    padding: var(--space-1) var(--space-3);
    border-top: 1px solid var(--color-border);
    font-size: var(--text-meta-size);
  }

  .item:first-child {
    border-top: none;
  }

  .key {
    color: var(--color-muted-fg);
    flex: none;
    min-width: 170px;
  }

  .value {
    flex: 1;
    min-width: 0;
    word-break: break-all;
  }

  .redacted-value {
    color: var(--color-warning);
    font-weight: 600;
  }

  .badge {
    flex: none;
    font-size: var(--text-meta-size);
    padding: 0 var(--space-1);
    border: 1px solid var(--color-warning);
    border-radius: var(--radius-sm);
    color: var(--color-warning);
  }

  .badge.normalized {
    border-color: var(--color-border-strong);
    color: var(--color-muted-fg);
  }
</style>
