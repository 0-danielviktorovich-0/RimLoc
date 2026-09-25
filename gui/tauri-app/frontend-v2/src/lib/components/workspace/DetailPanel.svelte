<script lang="ts">
  // Entry detail panel (mandate §11, spec §20, 320px): four tabs —
  // CONTEXT / SUGGESTIONS / VALIDATION / HISTORY — over the mock store.
  // CONTEXT keeps identity + TKey multi-contexts + note + review actions;
  // identity meta folds into a collapsed "technical details" block.
  import { t, i18n } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import Icon from '../Icon.svelte';
  import Tabs from './Tabs.svelte';
  import SourceTab from '../source/SourceTab.svelte';
  import type { Entry, HistoryEvent, IssueKind, Origin, Suggestion, TkeyStrategy } from '../../mock/types';

  const CONTEXT_REGION_ID = 'workspace-context';

  let { entry }: { entry: Entry | null } = $props();

  let tab = $state('context');
  // Keep the selected tab stable across entry changes; reset only when the
  // panel goes from empty → populated.
  $effect(() => {
    if (!entry) tab = 'context';
  });

  // Applied suggestion ids (mock: an applied suggestion disables its button).
  let applied = $state<Set<string>>(new Set());

  const detailTabs = $derived.by(() => {
    const issues = entry?.issues?.length ?? 0;
    const sugg = entry?.suggestions?.length ?? 0;
    const hist = entry?.history?.length ?? 0;
    return [
      { id: 'context', label: t('workspace.detail.tab.context') },
      { id: 'source', label: t('workspace.detail.tab.source') },
      { id: 'suggestions', label: t('workspace.detail.tab.suggestions'), count: sugg },
      { id: 'validation', label: t('workspace.detail.tab.validation'), count: issues },
      { id: 'history', label: t('workspace.detail.tab.history'), count: hist }
    ];
  });

  // Validation indicator. Audit P1-1: contract snapshots carry the REAL
  // validation dimension — it wins whenever present; fixture records without
  // contract validation keep the status proxy (mock phase).
  const validation = $derived.by(() => {
    if (!entry) return null;
    if (entry.validation === 'ok') return { state: 'pass', icon: 'circle-check' } as const;
    if (entry.validation === 'issues') return { state: 'fail', icon: 'warning' } as const;
    if (entry.status === 'translated') return { state: 'pass', icon: 'circle-check' } as const;
    if (entry.status === 'untranslated' || entry.status === 'sourceChanged')
      return { state: 'fail', icon: 'warning' } as const;
    return { state: 'na', icon: 'info' } as const;
  });

  function strategyLabel(strategy: TkeyStrategy): string {
    const key = strategy === '.value.slateRef' ? 'valueSlateRef' : strategy.replace('.', '');
    return t(`workspace.detail.strategy.${key}`);
  }

  function strategyClass(strategy: TkeyStrategy): string {
    return strategy === '.value.slateRef'
      ? 'value-slateref'
      : strategy === '.slateRef'
        ? 'slateref'
        : 'bare';
  }

  /** Glossary picks are human edits; TM and AI keep their provenance. */
  function suggestionOrigin(s: Suggestion): Origin {
    return s.source === 'glossary' ? 'human' : s.source;
  }

  function suggestionIcon(source: 'TM' | 'glossary' | 'LLM'): string {
    return source === 'TM' ? 'database' : source === 'glossary' ? 'book' : 'sparkles';
  }

  function historyIcon(e: HistoryEvent): string {
    switch (e.action) {
      case 'ai_draft':
        return 'sparkles';
      case 'tm_match':
        return 'database';
      case 'imported':
        return 'download';
      case 'source_changed':
        return 'alert';
      case 'edited':
        return 'edit';
      default:
        return 'clock';
    }
  }

  function issueIcon(kind: IssueKind): string {
    return kind === 'placeholder_mismatch' || kind === 'ambiguity' ? 'warning' : 'info';
  }

  function applySuggestion(entryId: string, text: string, origin: Origin, key: string) {
    project.applySuggestion(entryId, text, origin);
    applied = new Set([...applied, key]);
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
  aria-label={t('workspace.detail.label')}
  data-testid="workspace.context"
>
  {#if entry}
    <div class="context-head">
      <p class="context-kind">{t(`kind.${entry.kind}`)}</p>
      <h2 class="context-key mono">{entry.key}</h2>
    </div>

    <Tabs tabs={detailTabs} active={tab} label={t('workspace.detail.label')} onSelect={(id) => (tab = id)} />

    <div class="context-body" id={`panel-${tab}`} role="tabpanel" aria-labelledby={`tab-${tab}`}>
      {#if tab === 'context'}
        {#if validation}
          <p class="validation validation-{validation.state}" data-testid="workspace.context.validation">
            <Icon name={validation.icon} size={14} />
            <span>{t(`workspace.detail.validation.${validation.state}`)}</span>
          </p>
        {/if}

        {#if entry.usages && entry.usages.length > 0}
          <section>
            <h3 class="section-title">{t('workspace.detail.usages')}</h3>
            <ul class="usages">
              {#each entry.usages as u (u)}
                <li class="usage">{u}</li>
              {/each}
            </ul>
          </section>
        {/if}

        {#if entry.sourcePrev}
          <section>
            <h3 class="section-title">{t('workspace.detail.sourcePrev')}</h3>
            <p class="prev mono">«{entry.sourcePrev}»</p>
          </section>
        {/if}

        {#if entry.contexts && entry.contexts.length > 0}
          <section>
            <h3 class="section-title">{t('workspace.detail.nodes')}</h3>
            <ul class="nodes-list">
              {#each entry.contexts as ctx, i (i)}
                <li class="node">
                  <p class="node-line mono">{ctx.file}:{ctx.line}</p>
                  <p class="node-src">
                    <span class="node-label">{t('workspace.detail.nodeSource')}:</span>
                    <span class="mono">{ctx.node}</span>
                  </p>
                  <p class="node-value">«{ctx.sourceValue}»</p>
                  <p class="node-sibling">
                    <span class="node-label">{t('workspace.detail.sibling')}:</span>
                    <span class="mono">{ctx.sibling}</span>
                  </p>
                </li>
              {/each}
            </ul>
          </section>
        {:else if entry.kind === 'TKey'}
          <p class="hint">{t('workspace.detail.noContexts')}</p>
        {/if}

        <details class="advanced">
          <summary>{t('workspace.detail.advanced')}</summary>
          <dl class="meta">
            <div class="meta-row">
              <dt>{t('workspace.detail.file')}</dt>
              <dd class="mono">{entry.file}</dd>
            </div>
            <div class="meta-row">
              <dt>{t('workspace.detail.line')}</dt>
              <dd class="mono">{entry.line}</dd>
            </div>
            {#if entry.kind === 'TKey' && entry.strategy}
              <div class="meta-row">
                <dt>{t('workspace.detail.strategy')}</dt>
                <dd>
                  <span class="strategy strategy-{strategyClass(entry.strategy)}">
                    {strategyLabel(entry.strategy)}
                  </span>
                </dd>
              </div>
            {/if}
            <div class="meta-row">
              <dt>{t('workspace.detail.origin')}</dt>
              <dd class:origin class:origin-human={entry.origin === 'human'} class:origin-tm={entry.origin === 'TM'} class:origin-llm={entry.origin === 'LLM'}>
                {entry.origin ? t(`workspace.detail.origin.${entry.origin}`) : '—'}
              </dd>
            </div>
            <div class="meta-row">
              <dt>{t('workspace.detail.edited')}</dt>
              <dd>{formatDate(entry.editedAt)}</dd>
            </div>
          </dl>
        </details>

        <label class="note-label" for="context-note">{t('workspace.detail.note')}</label>
        <textarea
          id="context-note"
          class="note"
          rows="3"
          placeholder={t('workspace.detail.notePlaceholder')}
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
            {t('workspace.detail.markTranslated')}
          </button>
          <button
            type="button"
            class="btn"
            data-testid="workspace.context.mark-review"
            onclick={() => project.setStatus(entry.id, 'pending_review')}
          >
            {t('workspace.detail.markReview')}
          </button>
        </div>
      {:else if tab === 'source'}
        <SourceTab entry={entry} />
      {:else if tab === 'suggestions'}
        {#if entry.suggestions && entry.suggestions.length > 0}
          <ul class="suggestions">
            {#each entry.suggestions as s, i (i)}
              {@const key = `${entry.id}:${i}`}
              <li class="suggestion">
                <p class="suggestion-head">
                  <Icon name={suggestionIcon(s.source)} size={13} />
                  {t(`suggestion.${s.source === 'TM' ? 'tm' : s.source}`)}
                  {#if s.similarity}
                    <span class="sim">{t('suggestion.similarity', { n: s.similarity })}</span>
                  {/if}
                  {#if s.term}
                    <span class="term mono">{s.term}</span>
                  {/if}
                </p>
                <p class="suggestion-text">{s.text}</p>
                {#if applied.has(key)}
                  <span class="applied">
                    <Icon name="circle-check" size={13} />
                    {t('suggestion.applied')}
                  </span>
                {:else}
                  <button
                    type="button"
                    class="btn"
                    data-testid={`workspace.suggestion.apply.${i}`}
                    onclick={() => applySuggestion(entry.id, s.text, suggestionOrigin(s), key)}
                  >
                    {t('common.apply')}
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
        {:else}
          <p class="hint empty"><Icon name="info" size={14} /> {t('workspace.detail.noSelection')}</p>
        {/if}
      {:else if tab === 'validation'}
        {#if entry.issues && entry.issues.length > 0}
          <ul class="issues">
            {#each entry.issues as issue, i (i)}
              <li class="issue issue-{issue.severity}" data-testid={`workspace.issue.${i}`}>
                <p class="issue-head">
                  <Icon name={issue.severity === 'error' ? 'warning' : issueIcon(issue.kind)} size={13} />
                  {t(`issue.${issue.kind}`)}
                  <span class="sev">{t(`validation.severity.${issue.severity}`)}</span>
                </p>
                <p class="issue-msg">{issue.message}</p>
              </li>
            {/each}
          </ul>
        {:else if entry.validationIssues && entry.validationIssues.length > 0}
          <!-- Audit P1-1: canonical validation findings from the contract
               snapshot, surfaced verbatim until the validate slice lands. -->
          <ul class="issues">
            {#each entry.validationIssues as message, i (i)}
              <li class="issue issue-warning" data-testid={`workspace.validationIssue.${i}`}>
                <p class="issue-head">
                  <Icon name="warning" size={13} />
                  {t('workspace.detail.validation.fail')}
                </p>
                <p class="issue-msg">{message}</p>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="hint ok"><Icon name="circle-check" size={14} /> {t('validation.empty')}</p>
        {/if}
      {:else}
        {#if entry.history && entry.history.length > 0}
          <ul class="history">
            {#each entry.history as ev, i (i)}
              <li class="event">
                <span class="event-icon" aria-hidden="true"><Icon name={historyIcon(ev)} size={13} /></span>
                <div class="event-body">
                  <p class="event-line">
                    {t(`history.${ev.action}`)}
                    {#if ev.origin}
                      <span class="origin-badge">{t(`workspace.detail.origin.${ev.origin}`)}</span>
                    {/if}
                  </p>
                  {#if ev.model}
                    <p class="event-detail mono">{ev.model}</p>
                  {/if}
                  {#if ev.detail}
                    <p class="event-detail">{ev.detail}</p>
                  {/if}
                  <p class="event-time">{formatDate(ev.at)}</p>
                </div>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="hint empty"><Icon name="clock" size={14} /> {t('history.created')}</p>
        {/if}
      {/if}
    </div>
  {:else}
    <p class="hint no-selection">
      <span class="hint-icon" aria-hidden="true"><Icon name="info" size={14} /></span>
      {t('workspace.detail.noSelection')}
    </p>
  {/if}
</aside>

<style>
  .context {
    border-left: 1px solid var(--color-border);
    background: var(--color-surface);
    box-shadow: var(--shadow-panel);
    overflow-y: auto;
    padding: var(--space-3) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .context-head {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
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

  .context-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  /* The tabs strip inside the panel hugs the panel padding. */
  .context > :global(.tabs) {
    padding: 0;
    border-bottom: 1px solid var(--color-border);
  }

  /* Validation indicator: icon + color + text — color is never the only carrier. */
  .validation {
    margin: 0;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-meta-size);
  }

  .validation :global(svg) {
    flex: none;
  }

  .validation-pass {
    color: var(--color-success);
  }

  .validation-fail {
    color: var(--color-warning);
  }

  .validation-na {
    color: var(--color-muted-fg);
  }

  .section-title {
    margin: 0 0 var(--space-1);
    font-size: var(--text-meta-size);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-muted-fg);
  }

  .usages {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }

  .usage {
    font-size: var(--text-meta-size);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0 var(--space-1);
  }

  .prev {
    margin: 0;
    font-size: var(--text-meta-size);
    color: var(--color-warning);
    overflow-wrap: anywhere;
  }

  /* Provenance origin is tinted text on surface (AA pairs in tokens.css). */
  .origin-human {
    color: var(--color-origin-human);
  }

  .origin-tm {
    color: var(--color-origin-tm);
  }

  .origin-llm {
    color: var(--color-origin-llm);
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

  .advanced summary {
    display: inline-flex;
    align-items: center;
    min-height: var(--control-h);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    cursor: pointer;
    user-select: none;
    list-style: none;
  }

  .advanced summary::-webkit-details-marker {
    display: none;
  }

  .advanced summary:hover {
    color: var(--color-fg);
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

  /* Suggestions */
  .suggestions {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .suggestion {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: var(--space-2);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-meta-size);
  }

  .suggestion-head {
    margin: 0;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-muted-fg);
  }

  .suggestion-head :global(svg) {
    flex: none;
  }

  .sim,
  .term {
    color: var(--color-origin-tm);
  }

  .suggestion-text {
    margin: 0;
    font-size: var(--text-base-size);
  }

  .applied {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--color-success);
  }

  /* Validation issues */
  .issues {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .issue {
    border: 1px solid var(--color-border);
    border-left: 3px solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    padding: var(--space-2);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-meta-size);
  }

  .issue-error {
    border-left-color: var(--color-destructive);
  }

  .issue-warning {
    border-left-color: var(--color-warning);
  }

  .issue-head {
    margin: 0;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-weight: 600;
  }

  .issue-head :global(svg) {
    flex: none;
  }

  .issue-error .issue-head :global(svg) {
    color: var(--color-destructive);
  }

  .issue-warning .issue-head :global(svg) {
    color: var(--color-warning);
  }

  .sev {
    margin-left: auto;
    font-weight: 400;
    color: var(--color-muted-fg);
  }

  .issue-msg {
    margin: 0;
  }

  /* History */
  .history {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .event {
    display: flex;
    gap: var(--space-2);
    font-size: var(--text-meta-size);
  }

  .event-icon {
    color: var(--color-muted-fg);
    display: inline-flex;
    padding-top: 2px;
  }

  .event-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .event-line {
    margin: 0;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex-wrap: wrap;
  }

  .origin-badge {
    font-weight: 400;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0 var(--space-1);
    color: var(--color-muted-fg);
  }

  .event-detail {
    margin: 0;
    color: var(--color-muted-fg);
    overflow-wrap: anywhere;
  }

  .event-time {
    margin: 0;
    color: var(--color-muted-fg);
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
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .hint :global(svg) {
    flex: none;
  }

  .hint.ok {
    color: var(--color-success);
  }

  .hint-icon {
    display: inline-flex;
    margin-right: var(--space-1);
  }

  .no-selection {
    margin: var(--space-6) 0;
  }
</style>
