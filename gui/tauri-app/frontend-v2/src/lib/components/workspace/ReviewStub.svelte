<script lang="ts">
  // Review tab (mandate §12): the full QA experience. Overview counters sit on
  // top; below, an interactive queue over the entries loaded in this session —
  // select an issue to open its entry in context, then [Fix] / [Ignore with
  // reason] / [Mark reviewed].
  //
  // Honesty about the overview (night audit §7 follow-up): in fixture/demo mode
  // the counters are the mandate example values and the header carries the
  // demo badge. On a real contract project the numbers are computed from the
  // loaded snapshot — pending-review count and validation issues are real;
  // source-change tracking and the glossary check have no dimension in the v1
  // snapshot and are shown as an explicit "—" with the reason instead of a
  // plausible zero. No new contract commands are involved.
  import Icon from '../Icon.svelte';
  import { t, i18n } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import { review, type ReviewIssue } from '../../stores/review.svelte';
  import { router } from '../../router.svelte';
  import { mockReviewOverview, mockReviewCategories } from '../../mock/wizard';

  interface OverviewCard {
    key: 'needsReview' | 'errors' | 'sourceChanged' | 'glossaryConflicts';
    /** null = genuinely not computable in this build (never a faked zero). */
    value: number | null;
    icon: string;
  }

  const OVERVIEW_MOCK: OverviewCard[] = [
    { key: 'needsReview', value: mockReviewOverview.needsReview, icon: 'clipboard-check' },
    { key: 'errors', value: mockReviewOverview.errors, icon: 'warning' },
    { key: 'sourceChanged', value: mockReviewOverview.sourceChanged, icon: 'alert' },
    { key: 'glossaryConflicts', value: mockReviewOverview.glossaryConflicts, icon: 'book' }
  ];

  /** Contract mode: the counters the snapshot can actually express. Keys
   *  absent here are exactly the ones shown as an honest "—". */
  const liveOverview = $derived.by<Partial<Record<OverviewCard['key'], number>> | null>(() => {
    if (project.source !== 'contract') return null;
    const counts = project.statusCounts();
    return {
      needsReview: counts.pending_review,
      errors: project.entries.filter((e) => e.validation === 'issues').length
    };
  });

  const overview = $derived.by<OverviewCard[]>(() => {
    const live = liveOverview;
    if (!live) return OVERVIEW_MOCK;
    return OVERVIEW_MOCK.map((c) =>
      typeof live[c.key] === 'number' ? { ...c, value: live[c.key] as number } : { ...c, value: null }
    );
  });

  /** True on a real project: the overview is partial — say so under the cards. */
  const overviewIsPartial = $derived(liveOverview !== null);

  const KIND_ICONS: Record<string, string> = {
    placeholder_mismatch: 'warning',
    glossary: 'book',
    untranslated_suspect: 'languages',
    wordinfo: 'info',
    ambiguity: 'lightbulb',
    ai_concern: 'sparkles'
  };

  const selected = $derived(review.selected);
  const selectedEntry = $derived(selected ? project.byId(selected.entryId) ?? null : null);
  const resolvedCount = $derived(Object.keys(review.resolutions).length);

  // Drop session resolutions that no longer match a live issue (dev reset).
  $effect(() => {
    review.pruneStale(new Set(review.issues.map((i) => i.id)));
  });

  function fmt(n: number): string {
    return new Intl.NumberFormat(i18n.locale === 'ru' ? 'ru-RU' : 'en-US').format(n);
  }

  function snippet(text: string, max = 90): string {
    const one = text.replace(/\s+/g, ' ').trim();
    return one.length > max ? `${one.slice(0, max - 1)}…` : one;
  }

  function issueIcon(issue: ReviewIssue): string {
    return KIND_ICONS[issue.kind] ?? 'info';
  }

  function gotoBuild() {
    router.navigate('build');
  }

  // M-6 (UI audit 2026-09-29): the overview counters and the session queue
  // are two different truths — a "0 / 45" pair on one screen read as a
  // glitch. Each side now carries a visible link to the other; scrolling
  // honors prefers-reduced-motion.
  function reducedMotion(): boolean {
    return typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
  }
  function scrollToQueue() {
    document.getElementById('review-queue')?.scrollIntoView({ block: 'start', behavior: reducedMotion() ? 'auto' : 'smooth' });
  }
  function scrollToOverview() {
    document.getElementById('review-overview')?.scrollIntoView({ block: 'start', behavior: reducedMotion() ? 'auto' : 'smooth' });
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape' || !selected) return;
    if (review.editingId) review.cancelFix();
    else if (review.ignoringId) review.cancelIgnore();
    else review.closeContext();
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<section class="review" aria-labelledby="review-heading" data-testid="workspace.review">
  <div class="head">
    <h2 id="review-heading" class="title">{t('review.title')}</h2>
    <p class="scope">{t('review.scope')}</p>
  </div>

  <dl class="overview" id="review-overview">
    {#each overview as o (o.key)}
      <div class="card" data-testid={`review.overview.${o.key}`}>
        <dt>
          <Icon name={o.icon} size={14} />
          {t(`review.${o.key}`)}
        </dt>
        {#if o.value === null}
          <dd
            class="mono unavailable"
            title={t('review.overview.unavailable')}
            data-testid={`review.overview.${o.key}.unavailable`}
          >
            —
            <span class="visually-hidden">{t('review.overview.unavailable')}</span>
          </dd>
        {:else}
          <dd class="mono">{fmt(o.value)}</dd>
        {/if}
      </div>
    {/each}
  </dl>
  {#if overviewIsPartial}
    <p class="partial-note" data-testid="review.overview.partial">
      <Icon name="info" size={13} />
      {t('review.overview.partial')}
    </p>
  {/if}
  <!-- M-6: the visible bridge between the cards and the queue — the "0 next
       to 45" pair is two different counts, not a malfunction. -->
  <p class="queue-link" data-testid="review.overview.queuelink-note">
    <button type="button" class="linklike" data-testid="review.overview.queueLink" onclick={scrollToQueue}>
      {t('review.overview.queueLink')}
    </button>
    <span>{t('review.overview.queueExplainer')}</span>
  </p>

  <div class="chips" role="group" aria-label={t('review.categories')} data-testid="review.categories">
    <button
      type="button"
      class="chip"
      class:active={review.activeKind === 'all'}
      aria-pressed={review.activeKind === 'all'}
      data-testid="review.category.all"
      onclick={() => review.setKind('all')}
    >
      {t('review.categories.all')}
      <span class="mono count">{fmt(review.active.length)}</span>
    </button>
    {#each mockReviewCategories as c (c.id)}
      <button
        type="button"
        class="chip"
        class:active={review.activeKind === c.id}
        aria-pressed={review.activeKind === c.id}
        data-testid={`review.category.${c.id}`}
        onclick={() => review.setKind(c.id)}
      >
        {t(`issue.${c.id}`)}
        <span class="mono count">{fmt(review.countsByKind[c.id] ?? 0)}</span>
      </button>
    {/each}
  </div>

  <div class="layout">
    <div class="queue-pane" id="review-queue">
      {#if review.active.length === 0}
        <div class="clear-card" data-testid="review.queue-empty">
          <p class="clear-title">
            <Icon name="circle-check" size={16} />
            {t('review.queue.empty.title')}
          </p>
          <p class="clear-desc">{t('review.queue.empty.desc')}</p>
          <button type="button" class="btn btn-primary" data-testid="review.queue-empty.build" onclick={gotoBuild}>
            <Icon name="package" size={14} />
            {t('workspace.cta.build')}
          </button>
        </div>
      {:else}
        <p class="overview-link">
          <!-- M-6: the queue answers back to the overview counters. -->
          <button type="button" class="linklike" data-testid="review.queue.overviewLink" onclick={scrollToOverview}>
            {t('review.queue.overviewLink')}
          </button>
        </p>
        <ul class="queue" aria-label={t('review.queue')}>
          {#each review.active as issue (issue.id)}
            {@const entry = project.byId(issue.entryId)}
            <li>
              <button
                type="button"
                class="row"
                class:selected={review.selectedId === issue.id}
                class:sev-error={issue.severity === 'error'}
                aria-current={review.selectedId === issue.id ? 'true' : undefined}
                data-testid={`review.issue.${issue.id}`}
                onclick={() => review.open(issue)}
              >
                <span class="row-icon sev-{issue.severity}">
                  <Icon name={issue.severity === 'error' ? 'warning' : issueIcon(issue)} size={15} />
                </span>
                <span class="row-main">
                  <span class="row-kind">{t(`issue.${issue.kind}`)}</span>
                  <span class="row-key mono">{entry?.key}</span>
                  <span class="row-snippet">{snippet(issue.message ?? entry?.source ?? '')}</span>
                </span>
                <span class="row-loc mono">{entry?.file.split('/').pop()}:{entry?.line}</span>
                <Icon name="chevron-right" size={14} />
              </button>
            </li>
          {/each}
        </ul>
      {/if}
      {#if resolvedCount > 0}
        <p class="resolved-note" data-testid="review.resolved-note">
          <Icon name="circle-check" size={13} />
          {t('review.resolvedNote', { count: fmt(resolvedCount) })}
        </p>
      {/if}
    </div>

    {#if selected && selectedEntry}
      <aside class="context" aria-label={t('review.context.label')} data-testid="review.context">
        <div class="context-head">
          <div>
            <p class="context-kind">{t(`kind.${selectedEntry.kind}`)}</p>
            <h3 class="context-key mono">{selectedEntry.key}</h3>
          </div>
          <button
            type="button"
            class="btn context-close"
            aria-label={t('common.close')}
            data-testid="review.context-close"
            onclick={() => review.closeContext()}
          >
            <Icon name="close" size={14} />
          </button>
        </div>

        <p class="callout callout-{selected.severity}" data-testid="review.context-issue">
          <Icon name={selected.severity === 'error' ? 'warning' : issueIcon(selected)} size={14} />
          <span>
            <strong>{t(`issue.${selected.kind}`)}</strong>
            {selected.message ?? t('review.context.noDetails')}
          </span>
        </p>

        <dl class="blocks">
          <div class="block">
            <dt>{t('workspace.col.source')}</dt>
            <dd class="text">{selectedEntry.source}</dd>
          </div>
          <div class="block">
            <dt>{t('workspace.col.target')}</dt>
            <dd class="text" class:empty={!selectedEntry.target}>
              {selectedEntry.target || t('review.context.untranslated')}
            </dd>
          </div>
        </dl>

        {#if review.editingId === selected.id}
          <div class="fix-editor" data-testid="review.fix-editor">
            <label class="field-label" for="review-fix-text">{t('review.fix.hint')}</label>
            <textarea
              id="review-fix-text"
              rows="3"
              bind:value={review.fixText[selected.id]}
              data-testid="review.fix-text"
            ></textarea>
            <div class="form-actions">
              <button
                type="button"
                class="btn btn-primary"
                data-testid="review.fix-save"
                onclick={() => review.saveFix(selected)}
              >
                <Icon name="check" size={14} />
                {t('review.fix.save')}
              </button>
              <button type="button" class="btn" data-testid="review.fix-cancel" onclick={() => review.cancelFix()}>
                {t('common.cancel')}
              </button>
            </div>
          </div>
        {:else if review.ignoringId === selected.id}
          <div class="fix-editor" data-testid="review.ignore-form">
            <label class="field-label" for="review-ignore-reason">{t('review.ignore.reason')}</label>
            <input
              id="review-ignore-reason"
              type="text"
              bind:value={review.ignoreDraft}
              placeholder={t('review.ignore.placeholder')}
              data-testid="review.ignore-reason"
              onkeydown={(e) => {
                if (e.key === 'Enter') review.confirmIgnore(selected);
              }}
            />
            <div class="form-actions">
              <button
                type="button"
                class="btn btn-primary"
                disabled={!review.ignoreDraft.trim()}
                data-testid="review.ignore-confirm"
                onclick={() => review.confirmIgnore(selected)}
              >
                {t('review.ignore.confirm')}
              </button>
              <button type="button" class="btn" data-testid="review.ignore-cancel" onclick={() => review.cancelIgnore()}>
                {t('common.cancel')}
              </button>
            </div>
          </div>
        {:else}
          <div class="actions">
            <button
              type="button"
              class="btn btn-primary"
              data-testid="review.fix"
              onclick={() => review.startFix(selected)}
            >
              <Icon name="edit" size={14} />
              {t('review.fix')}
            </button>
            <button type="button" class="btn" data-testid="review.ignore" onclick={() => review.startIgnore(selected)}>
              {t('review.ignore')}
            </button>
            <button
              type="button"
              class="btn"
              data-testid="review.mark-reviewed"
              onclick={() => review.markReviewed(selected)}
            >
              <Icon name="clipboard-check" size={14} />
              {t('review.markReviewed')}
            </button>
          </div>
        {/if}

        {#if selectedEntry.usages && selectedEntry.usages.length > 0}
          <div class="meta-block">
            <p class="meta-label">{t('workspace.detail.usages')}</p>
            <ul class="usages">
              {#each selectedEntry.usages as u (u)}
                <li>{u}</li>
              {/each}
            </ul>
          </div>
        {/if}

        {#if selectedEntry.sourcePrev}
          <div class="meta-block">
            <p class="meta-label">{t('workspace.detail.sourcePrev')}</p>
            <p class="meta-value">{selectedEntry.sourcePrev}</p>
          </div>
        {/if}

        <div class="meta-block">
          <p class="meta-label">{t('workspace.detail.advanced')}</p>
          <p class="meta-value mono">
            {t('workspace.detail.file')}: {selectedEntry.file} · {t('workspace.detail.line')}: {selectedEntry.line}
          </p>
        </div>
      </aside>
    {/if}
  </div>
</section>

<style>
  .review {
    padding: var(--space-4) var(--space-6);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .head {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-4);
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: 18px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .scope {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  /* Overview counters (project-wide) */
  .overview {
    margin: 0;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--space-2);
  }

  @media (max-width: 900px) {
    .overview {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  .card {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .card dt {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .card dd {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  /* Not-computable counter (contract mode): explicit dash, never a faked zero. */
  .card dd.unavailable {
    color: var(--color-muted-fg);
    font-weight: 400;
  }

  .partial-note {
    margin: 0;
    display: inline-flex;
    align-items: flex-start;
    gap: var(--space-1);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .partial-note :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  /* M-6: the overview→queue bridge line (link + the 0-vs-N explainer). */
  .queue-link {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-1) var(--space-2);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  /* M-6: quiet text-link button used for both scroll bridges. */
  .linklike {
    border: none;
    background: none;
    padding: 0;
    color: var(--color-primary-text);
    font-size: inherit;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .linklike:hover {
    color: var(--color-fg);
  }

  .overview-link {
    margin: 0;
    font-size: var(--text-meta-size);
  }

  /* Category chips */
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--control-h);
    padding: 0 var(--space-3);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    transition:
      background var(--motion-fast) var(--ease-out),
      border-color var(--motion-fast) var(--ease-out);
  }

  .chip:hover {
    background: var(--color-muted);
  }

  .chip.active {
    border-color: var(--color-primary);
    background: var(--color-muted);
  }

  .chip .count {
    color: var(--color-muted-fg);
    font-variant-numeric: tabular-nums;
  }

  .chip.active .count {
    color: var(--color-fg);
  }

  /* Two-pane layout: queue + context */
  .layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 380px;
    gap: var(--space-4);
    align-items: start;
  }

  @media (max-width: 1100px) {
    .layout {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .queue-pane {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-width: 0;
  }

  .queue {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .row {
    width: 100%;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto auto;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    text-align: left;
    transition:
      border-color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out);
  }

  .row:hover {
    border-color: var(--color-border-strong);
    background: var(--color-muted);
  }

  .row.selected {
    border-color: var(--color-primary);
    background: var(--color-muted);
  }

  .row-icon {
    display: inline-flex;
  }

  .row-icon.sev-error {
    color: var(--color-destructive);
  }

  .row-icon.sev-warning {
    color: var(--color-warning);
  }

  .row-main {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .row-kind {
    font-weight: 600;
    font-size: var(--text-dense-size);
  }

  .row-key {
    color: var(--color-muted-fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-snippet {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-loc {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    white-space: nowrap;
  }

  .row :global(svg:last-child) {
    color: var(--color-muted-fg);
  }

  /* Queue empty state */
  .clear-card {
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-lg);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
  }

  .clear-title {
    margin: 0;
    font-weight: 600;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-success);
  }

  .clear-desc {
    margin: 0;
    color: var(--color-muted-fg);
  }

  .resolved-note {
    margin: 0;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  /* Context pane */
  .context {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-panel);
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    position: sticky;
    top: 0;
  }

  .context-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .context-kind {
    margin: 0;
    font-size: var(--text-meta-size);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-muted-fg);
  }

  .context-key {
    margin: 2px 0 0;
    font-size: var(--text-base-size);
    overflow-wrap: anywhere;
  }

  .context-close {
    flex: none;
    padding: 0 var(--space-2);
    min-height: var(--control-h);
  }

  .callout {
    margin: 0;
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    font-size: var(--text-dense-size);
    border: 1px solid var(--color-border);
    border-left: 3px solid var(--color-warning);
  }

  .callout strong {
    margin-right: var(--space-1);
  }

  .callout-error {
    border-left-color: var(--color-destructive);
  }

  .callout-warning {
    border-left-color: var(--color-warning);
  }

  /* Source/target blocks */
  .blocks {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .block dt {
    font-size: var(--text-meta-size);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-muted-fg);
    margin-bottom: 2px;
  }

  .block .text {
    margin: 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-muted);
    padding: var(--space-2);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .block .text.empty {
    color: var(--color-muted-fg);
    font-style: italic;
  }

  /* Fix editor + ignore form */
  .fix-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .field-label {
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
  }

  .fix-editor textarea,
  .fix-editor input {
    width: 100%;
  }

  .form-actions,
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  /* Context meta */
  .meta-block {
    border-top: 1px solid var(--color-border);
    padding-top: var(--space-2);
  }

  .meta-label {
    margin: 0 0 2px;
    font-size: var(--text-meta-size);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-muted-fg);
  }

  .meta-value {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .usages {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }

  .usages li {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 1px var(--space-2);
    font-size: var(--text-meta-size);
    background: var(--color-muted);
  }
</style>
