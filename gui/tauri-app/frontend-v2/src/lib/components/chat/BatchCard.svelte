<script lang="ts">
  // One chat batch: stable id, status chip, counters and the per-status
  // actions of the external-AI loop (copy → waiting → import → apply/review),
  // plus the apply preview and the retry-split visualization. All writes go
  // through the chatbatch store — this component holds no state of its own.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { chat, type ChatBatch } from '../../stores/chatbatch.svelte';

  let { batch }: { batch: ChatBatch } = $props();

  const preview = $derived(chat.preview(batch));
  const hasCounts = $derived(batch.accepted + batch.problems + batch.stale + batch.skipped > 0);
  const canSplit = $derived(
    batch.split === null && (batch.status === 'needs_review' || batch.status === 'partial')
  );
</script>

<article class="batch" data-testid={`chat.batch.${batch.id}`}>
  <header class="head-row">
    <span class={`dot ${batch.status}`} aria-hidden="true"></span>
    <span class="id mono">{batch.id}</span>
    <span class={`status ${batch.status}`}>{t(`chat.status.${batch.status}`)}</span>
    <span class="size">{t('chat.summary.entries', { count: batch.size })}</span>
  </header>

  {#if hasCounts}
    <p class="counts">
      <span>{t('chat.counts.accepted', { count: batch.accepted })}</span>
      {#if batch.problems > 0}<span>{t('chat.counts.problems', { count: batch.problems })}</span>{/if}
      {#if batch.stale > 0}<span>{t('chat.counts.stale', { count: batch.stale })}</span>{/if}
      {#if batch.skipped > 0}<span>{t('chat.counts.skipped', { count: batch.skipped })}</span>{/if}
    </p>
  {/if}

  {#if batch.split}
    <!-- Retry-split: the whole batch re-sends in two halves, accepted stay. -->
    <div class="split" data-testid={`chat.split-viz.${batch.id}`}>
      <Icon name="layers" size={14} />
      <span class="split-line mono">
        {batch.size} → {batch.split.aSize} + {batch.split.bSize}
      </span>
      <span class="split-note">{t('chat.split.preserved', { count: batch.split.preserved })}</span>
    </div>
  {/if}

  {#if batch.status === 'imported' || batch.status === 'needs_review'}
    <!-- Apply preview: nothing is written until the human picks a side. -->
    <div
      class="preview"
      role="group"
      aria-label={t('chat.preview.title')}
      data-testid={`chat.preview.${batch.id}`}
    >
      <span class="preview-title">{t('chat.preview.title')}</span>
      <p class="preview-line">
        <span class="ok">{t('chat.preview.ready', { count: preview.ready })}</span>
        {#if preview.problems > 0}<span>{t('chat.preview.problems', { count: preview.problems })}</span>{/if}
        {#if preview.stale > 0}<span class="warn">{t('chat.preview.stale', { count: preview.stale })}</span>{/if}
        {#if preview.skipped > 0}<span>{t('chat.preview.skipped', { count: preview.skipped })}</span>{/if}
      </p>
      <div class="actions">
        <button
          type="button"
          class="btn btn-primary"
          disabled={preview.ready === 0}
          data-testid={`chat.apply.${batch.id}`}
          onclick={() => chat.apply(batch.id)}
        >
          <Icon name="circle-check" size={14} />
          {t('chat.apply', { count: preview.ready })}
        </button>
        <button
          type="button"
          class="btn"
          disabled={preview.review === 0}
          data-testid={`chat.review.${batch.id}`}
          onclick={() => chat.review(batch.id)}
        >
          <Icon name="clipboard-check" size={14} />
          {t('chat.reviewN', { count: preview.review })}
        </button>
      </div>
    </div>
  {/if}

  <div class="actions">
    {#if batch.status === 'not_started'}
      <button
        type="button"
        class="btn"
        data-testid={`chat.copy.${batch.id}`}
        onclick={() => chat.copy(batch.id)}
      >
        <Icon name="copy" size={14} />
        {t('chat.copy')}
      </button>
    {:else if batch.status === 'exported'}
      <button
        type="button"
        class="btn"
        data-testid={`chat.copy-again.${batch.id}`}
        onclick={() => chat.copyAgain(batch.id)}
      >
        <Icon name="copy" size={14} />
        {t('chat.copyAgain')}
      </button>
      <button
        type="button"
        class="btn"
        data-testid={`chat.waiting.${batch.id}`}
        onclick={() => chat.markWaiting(batch.id)}
      >
        <Icon name="clock" size={14} />
        {t('chat.markWaiting')}
      </button>
    {:else if batch.status === 'waiting'}
      <button
        type="button"
        class="btn"
        data-testid={`chat.import.${batch.id}`}
        onclick={() => chat.importReply(batch.id)}
      >
        <Icon name="download" size={14} />
        {t('chat.import')}
      </button>
    {:else if batch.status === 'partial'}
      <button
        type="button"
        class="btn"
        data-testid={`chat.copy-remainder.${batch.id}`}
        onclick={() => chat.copy(batch.id)}
      >
        <Icon name="copy" size={14} />
        {t('chat.copyRemainder', { count: chat.copyableCount(batch) })}
      </button>
    {/if}
    {#if canSplit}
      <button
        type="button"
        class="btn"
        data-testid={`chat.split.${batch.id}`}
        onclick={() => chat.split(batch.id)}
      >
        <Icon name="layers" size={14} />
        {t('chat.split.action')}
      </button>
    {/if}
  </div>
</article>

<style>
  .batch {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
  }

  .head-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
    background: var(--color-status-orphan);
  }

  .dot.exported,
  .dot.imported {
    background: var(--color-status-todo);
  }

  .dot.waiting {
    background: var(--color-status-untranslated);
  }

  .dot.partial {
    background: var(--color-status-source-changed);
  }

  .dot.needs_review {
    background: var(--color-status-pending-review);
  }

  .dot.done {
    background: var(--color-status-translated);
  }

  .id {
    font-weight: 600;
    white-space: nowrap;
  }

  .status {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    white-space: nowrap;
  }

  .status.done {
    color: var(--color-status-translated);
  }

  .status.needs_review {
    color: var(--color-status-pending-review);
  }

  .size {
    margin-left: auto;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    white-space: nowrap;
  }

  .counts {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-3);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    font-variant-numeric: tabular-nums;
  }

  .split {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-sm);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .split-line {
    font-weight: 600;
    color: var(--color-fg);
  }

  .preview {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
  }

  .preview-title {
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
    font-weight: 600;
  }

  .preview-line {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-3);
    font-variant-numeric: tabular-nums;
    font-size: var(--text-meta-size);
  }

  .ok {
    color: var(--color-status-translated);
  }

  .warn {
    color: var(--color-status-source-changed);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  button:disabled {
    opacity: 0.55;
    cursor: default;
  }
</style>
