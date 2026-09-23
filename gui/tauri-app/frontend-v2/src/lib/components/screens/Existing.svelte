<script lang="ts">
  // Existing translation flow (mandate §13): pick the source mod and the
  // language pack you already have, RimLoc compares them with the current mod
  // version, then you choose [Review changes] or [Continue translation].
  // Deliberate safety note: nothing from the existing work is deleted —
  // obsolete entries are preserved for review. Mocks only.
  import Icon from '../Icon.svelte';
  import { t, i18n } from '../../../i18n/store.svelte';
  import { router } from '../../router.svelte';
  import { mockMods } from '../../mock/wizard';
  import { mockLanguagePacks, mockAnalysis } from '../../mock/existing';

  type Step = 'select' | 'analyzing' | 'result';
  const ANALYZE_MS = 1200;

  let step = $state<Step>('select');
  let modId = $state<string | null>(null);
  let packId = $state<string | null>(null);
  let analyzeTimer: ReturnType<typeof setTimeout> | undefined;

  const mod = $derived(modId ? (mockMods.find((m) => m.id === modId) ?? null) : null);
  const packs = $derived(modId ? mockLanguagePacks.filter((p) => p.modId === modId) : []);
  const pack = $derived(packId ? (mockLanguagePacks.find((p) => p.id === packId) ?? null) : null);
  const canAnalyze = $derived(Boolean(mod && pack));

  const RESULT: { key: string; value: number; icon: string; positive?: boolean }[] = [
    { key: 'reusable', value: mockAnalysis.reusable, icon: 'check', positive: true },
    { key: 'sourceChanged', value: mockAnalysis.sourceChanged, icon: 'alert' },
    { key: 'new', value: mockAnalysis.new, icon: 'file-plus' },
    { key: 'obsolete', value: mockAnalysis.obsolete, icon: 'clock' },
    { key: 'invalid', value: mockAnalysis.invalid, icon: 'warning' },
    { key: 'ambiguous', value: mockAnalysis.ambiguous, icon: 'info' }
  ];

  function fmt(n: number): string {
    return new Intl.NumberFormat(i18n.locale === 'ru' ? 'ru-RU' : 'en-US').format(n);
  }

  function pickMod(id: string) {
    modId = id;
    packId = null;
  }

  function analyze() {
    if (!canAnalyze) return;
    step = 'analyzing';
    clearTimeout(analyzeTimer);
    analyzeTimer = setTimeout(() => {
      step = 'result';
    }, ANALYZE_MS);
  }

  function back() {
    step = 'select';
    clearTimeout(analyzeTimer);
  }

  function reviewChanges() {
    router.navigate('review');
  }

  function continueTranslation() {
    router.navigate('workspace');
  }
</script>

<section class="existing" aria-labelledby="existing-heading" data-testid="existing.screen">
  <h1 id="existing-heading" class="title">{t('existing.title')}</h1>
  <p class="desc">{t('existing.desc')}</p>

  {#if step === 'result'}
    <p class="scope mono" data-testid="existing.scope">
      {mod?.name} · {pack?.name} v{pack?.version}
    </p>
    <div class="card" data-testid="existing.result">
      <h2 class="result-title">{t('existing.result.title')}</h2>
      <dl class="stats">
        {#each RESULT as r (r.key)}
          <div class="stat" class:positive={r.positive}>
            <dt>
              <Icon name={r.icon} size={14} />
              {t(`existing.${r.key}`)}
            </dt>
            <dd class="mono">{fmt(r.value)}</dd>
          </div>
        {/each}
      </dl>

      <p class="safety" data-testid="existing.safety">
        <Icon name="info" size={14} />
        {t('existing.safety')}
      </p>

      <div class="actions">
        <button
          type="button"
          class="btn btn-primary"
          data-testid="existing.review-changes"
          onclick={reviewChanges}
        >
          <Icon name="clipboard-check" size={14} />
          {t('existing.reviewChanges')}
        </button>
        <button
          type="button"
          class="btn btn-primary"
          data-testid="existing.continue"
          onclick={continueTranslation}
        >
          <Icon name="arrow-right" size={14} />
          {t('existing.continueTranslation')}
        </button>
        <button type="button" class="btn" data-testid="existing.back" onclick={back}>
          <Icon name="arrow-left" size={14} />
          {t('existing.back')}
        </button>
      </div>
    </div>
  {:else}
    <div class="card" data-testid="existing.select">
      <fieldset class="group">
        <legend>{t('existing.mod')}</legend>
        <div class="options" role="radiogroup" aria-label={t('existing.mod')}>
          {#each mockMods as m (m.id)}
            <button
              type="button"
              role="radio"
              class="option"
              class:active={modId === m.id}
              aria-checked={modId === m.id}
              data-testid={`existing.mod.${m.id}`}
              onclick={() => pickMod(m.id)}
            >
              <span class="option-name">{m.name}</span>
              <span class="option-meta mono">v{m.version} · {fmt(m.defs)} {t('wizard.w2.defs')}</span>
            </button>
          {/each}
        </div>
      </fieldset>

      <fieldset class="group" disabled={!modId}>
        <legend>{t('existing.pack')}</legend>
        {#if packs.length === 0}
          <p class="empty" data-testid="existing.packs-empty">
            <Icon name="info" size={14} />
            {t('existing.pack.none')}
          </p>
        {:else}
          <div class="options" role="radiogroup" aria-label={t('existing.pack')}>
            {#each packs as p (p.id)}
              <button
                type="button"
                role="radio"
                class="option"
                class:active={packId === p.id}
                aria-checked={packId === p.id}
                data-testid={`existing.pack.${p.id}`}
                onclick={() => (packId = p.id)}
              >
                <span class="option-name">{p.name}</span>
                <span class="option-meta mono">
                  v{p.version} · {p.locale.toUpperCase()} · {t(`existing.pack.origin.${p.origin}`)}
                </span>
              </button>
            {/each}
          </div>
        {/if}
      </fieldset>

      <div class="actions">
        {#if step === 'analyzing'}
          <p class="status" role="status" aria-live="polite" data-testid="existing.analyzing">
            <Icon name="clock" size={14} />
            {t('existing.analyzing')}
          </p>
        {/if}
        <button
          type="button"
          class="btn btn-primary"
          data-testid="existing.analyze"
          disabled={!canAnalyze || step === 'analyzing'}
          onclick={analyze}
        >
          <Icon name="search" size={14} />
          {t('existing.analyze')}
        </button>
        <button type="button" class="btn" data-testid="existing.cancel" onclick={() => router.navigate('home')}>
          <Icon name="arrow-left" size={14} />
          {t('common.back')}
        </button>
      </div>
    </div>
  {/if}

  <p class="note">{t('existing.note')}</p>
</section>

<style>
  .existing {
    width: min(680px, 100%);
    margin: 0 auto;
    padding: var(--space-8) var(--space-4);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
  }

  .desc {
    margin: 0;
    color: var(--color-muted-fg);
  }

  .scope {
    margin: 0;
    color: var(--color-muted-fg);
  }

  .card {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    padding: var(--space-4) var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  /* Selection groups */
  .group {
    border: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .group:disabled {
    opacity: 0.55;
  }

  legend {
    padding: 0;
    margin-bottom: var(--space-2);
    font-weight: 600;
  }

  .options {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .option {
    display: flex;
    flex-direction: column;
    gap: 2px;
    text-align: left;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    padding: var(--space-2) var(--space-3);
    transition:
      border-color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out);
  }

  .option:hover {
    background: var(--color-muted);
  }

  .option.active {
    border-color: var(--color-primary);
    background: var(--color-muted);
  }

  .option-name {
    font-weight: 600;
  }

  .option-meta {
    color: var(--color-muted-fg);
  }

  .empty {
    margin: 0;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-muted-fg);
    font-size: var(--text-dense-size);
  }

  /* Actions */
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .status {
    margin: 0;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-muted-fg);
    font-size: var(--text-dense-size);
  }

  /* Analysis result */
  .result-title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: 16px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .stats {
    margin: 0;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-2);
  }

  @media (max-width: 640px) {
    .stats {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  .stat {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-muted);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .stat dt {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .stat dd {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .stat.positive dd {
    color: var(--color-success);
  }

  .safety {
    margin: 0;
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-left: 3px solid var(--color-info);
    border-radius: var(--radius-md);
    font-size: var(--text-dense-size);
  }

  .safety :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  .note {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }
</style>
