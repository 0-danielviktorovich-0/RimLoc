<script lang="ts">
  // Context panel (spec §2.4, 320px): entry identity, TKey multi-contexts,
  // provenance, translator note and review actions. All reads/writes go through
  // the mock project store — no backend.
  import { t, i18n } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import type { Entry, TkeyStrategy } from '../../mock/types';

  const CONTEXT_REGION_ID = 'workspace-context';

  let { entry }: { entry: Entry | null } = $props();

  function strategyLabel(strategy: TkeyStrategy): string {
    const key = strategy === '.value.slateRef' ? 'valueSlateRef' : strategy.replace('.', '');
    return t(`workspace.context.strategy.${key}`);
  }

  function strategyClass(strategy: TkeyStrategy): string {
    return strategy === '.value.slateRef'
      ? 'value-slateref'
      : strategy === '.slateRef'
        ? 'slateref'
        : 'bare';
  }

  function formatDate(iso?: string | null): string {
    if (!iso) return '—';
    try {
      return new Intl.DateTimeFormat(i18nLocale(), { dateStyle: 'medium', timeStyle: 'short' }).format(
        new Date(iso)
      );
    } catch {
      return iso;
    }
  }

  // Locale as a function so the Intl formatter re-renders when it changes.
  function i18nLocale(): string {
    return i18n.locale === 'ru' ? 'ru-RU' : 'en-US';
  }
</script>

<aside
  class="context"
  id={CONTEXT_REGION_ID}
  role="region"
  aria-label={t('workspace.context.label')}
  data-testid="workspace.context"
>
  {#if entry}
    <div class="context-body">
      <p class="context-kind">{t(`kind.${entry.kind}`)}</p>
      <h2 class="context-key mono">{entry.key}</h2>

      <dl class="meta">
        <div class="meta-row">
          <dt>{t('workspace.context.file')}</dt>
          <dd class="mono">{entry.file}</dd>
        </div>
        <div class="meta-row">
          <dt>{t('workspace.context.line')}</dt>
          <dd class="mono">{entry.line}</dd>
        </div>
        {#if entry.kind === 'TKey' && entry.strategy}
          <div class="meta-row">
            <dt>{t('workspace.context.strategy')}</dt>
            <dd>
              <span class="strategy strategy-{strategyClass(entry.strategy)}">
                {strategyLabel(entry.strategy)}
              </span>
            </dd>
          </div>
        {/if}
        <div class="meta-row">
          <dt>{t('workspace.context.origin')}</dt>
          <dd>
            {entry.origin ? t(`workspace.context.origin.${entry.origin}`) : '—'}
          </dd>
        </div>
        <div class="meta-row">
          <dt>{t('workspace.context.edited')}</dt>
          <dd>{formatDate(entry.editedAt)}</dd>
        </div>
      </dl>

      {#if entry.contexts && entry.contexts.length > 0}
        <section class="nodes">
          <h3 class="nodes-title">{t('workspace.context.nodes')}</h3>
          <ul class="nodes-list">
            {#each entry.contexts as ctx, i (i)}
              <li class="node">
                <p class="node-line mono">{ctx.file}:{ctx.line}</p>
                <p class="node-src">
                  <span class="node-label">{t('workspace.context.nodeSource')}:</span>
                  <span class="mono">{ctx.node}</span>
                </p>
                <p class="node-value">«{ctx.sourceValue}»</p>
                <p class="node-sibling">
                  <span class="node-label">{t('workspace.context.sibling')}:</span>
                  <span class="mono">{ctx.sibling}</span>
                </p>
              </li>
            {/each}
          </ul>
        </section>
      {:else if entry.kind === 'TKey'}
        <p class="hint">{t('workspace.context.noContexts')}</p>
      {/if}

      <label class="note-label" for="context-note">{t('workspace.context.note')}</label>
      <textarea
        id="context-note"
        class="note"
        rows="3"
        placeholder={t('workspace.context.notePlaceholder')}
        value={entry.note ?? ''}
        oninput={(e) => project.setNote(entry.id, (e.currentTarget as HTMLTextAreaElement).value)}
      ></textarea>

      <div class="actions">
        <button
          type="button"
          class="btn"
          data-testid="workspace.context.mark-translated"
          onclick={() => project.setStatus(entry.id, 'translated')}
        >
          {t('workspace.context.markTranslated')}
        </button>
        <button
          type="button"
          class="btn"
          data-testid="workspace.context.mark-review"
          onclick={() => project.setStatus(entry.id, 'pending_review')}
        >
          {t('workspace.context.markReview')}
        </button>
      </div>
    </div>
  {:else}
    <p class="hint no-selection">{t('workspace.context.noSelection')}</p>
  {/if}
</aside>

<style>
  .context {
    border-left: 1px solid var(--color-border);
    background: var(--color-surface);
    overflow-y: auto;
    padding: var(--space-3) var(--space-4);
  }

  .context-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .context-kind {
    margin: 0;
    font-size: var(--text-meta-size);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-muted-fg);
  }

  .context-key {
    margin: 0;
    font-size: var(--text-base-size);
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .meta {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-meta-size);
  }

  .meta-row {
    display: grid;
    grid-template-columns: 96px 1fr;
    gap: var(--space-2);
  }

  .meta-row dt {
    color: var(--color-muted-fg);
  }

  .meta-row dd {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .strategy {
    display: inline-block;
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border-strong);
    font-family: var(--font-mono);
  }

  .strategy.bare {
    border-color: var(--color-status-translated);
  }

  .strategy.slateref {
    border-color: var(--color-status-todo);
  }

  .strategy.value-slateref {
    border-color: var(--color-status-pending-review);
  }

  .nodes-title {
    margin: 0 0 var(--space-1);
    font-size: var(--text-meta-size);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-muted-fg);
  }

  .nodes-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .node {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: var(--space-2);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-meta-size);
  }

  .node p {
    margin: 0;
  }

  .node-line {
    color: var(--color-muted-fg);
  }

  .node-label {
    color: var(--color-muted-fg);
    margin-right: var(--space-1);
  }

  .node-value {
    font-style: italic;
  }

  .note {
    width: 100%;
    resize: vertical;
  }

  .note-label {
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
  }

  .actions {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .hint {
    color: var(--color-muted-fg);
    font-size: var(--text-base-size);
  }

  .no-selection {
    margin: var(--space-6) 0;
  }
</style>
