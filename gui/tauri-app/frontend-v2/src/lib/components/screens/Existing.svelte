<script lang="ts">
  // Existing translation flow (mandate §13, W2 closes the loop): pick the
  // source mod and the language pack you already have, RimLoc compares them
  // with the current mod version, then you choose [Review changes] or
  // [Continue translation]. Deliberate safety note: nothing from the
  // existing work is deleted — obsolete entries are preserved for review.
  //
  // W2 — two honest modes:
  // - LIVE (a contract project is open): the REAL contract flow — pick the
  //   existing pack directory (OS folder picker) → ANALYZE (dry-run
  //   `project_import_existing`, nothing is written) → a categories table
  //   with counts + capped sample lists → APPLY (`project_apply_existing`
  //   moves ONLY the reusable set; existing translations are never
  //   overwritten; ambiguous lines stay a list for a human decision).
  // - DEMO (no contract project): the original marked mock flow, unchanged.
  import Icon from '../Icon.svelte';
  import { t, i18n } from '../../../i18n/store.svelte';
  import { router } from '../../router.svelte';
  import { mockMods } from '../../mock/wizard';
  import { mockLanguagePacks, mockAnalysis } from '../../mock/existing';
  import { project } from '../../stores/project.svelte';
  import { existingPack } from '../../stores/existing.svelte';
  import {
    capability,
    CAP_APPLY_EXISTING,
    CAP_IMPORT_EXISTING
  } from '../../client/capability.svelte';
  import { clientInstance } from '../../client/instance.svelte';
  import { looksAbsolutePath } from '../../paths';

  // ---- shared ----
  const live = $derived(project.source === 'contract' && Boolean(project.contractProjectId));

  // ---- live flow ----
  let picking = $state(false);
  let pickError = $state<string | null>(null);

  const packStore = $derived(existingPack);
  const dirOk = $derived(looksAbsolutePath(packStore.existingDir));
  const canAnalyze = $derived(dirOk && !packStore.analyzing && capability.state(CAP_IMPORT_EXISTING) !== false);
  // Apply is bound to the ANALYZED directory: the numbers on screen must
  // be the numbers applied. Editing the dir after analyze keeps the
  // analysis visible but disables Apply until it is re-run.
  const dirChanged = $derived(
    Boolean(packStore.analysis) && packStore.analyzedDir !== packStore.existingDir.trim()
  );
  const canApply = $derived(
    Boolean(packStore.analysis) &&
      !dirChanged &&
      !packStore.applied &&
      !packStore.applying &&
      capability.state(CAP_APPLY_EXISTING) !== false
  );

  async function pickDir() {
    if (picking) return;
    picking = true;
    pickError = null;
    try {
      const dir = await clientInstance.getClient().pickDirectory(packStore.existingDir || undefined);
      if (dir && dir !== packStore.existingDir) {
        // A new folder resets the previous decision state (analysis/apply),
        // then becomes the selected dir.
        packStore.reset();
        packStore.existingDir = dir;
      }
    } catch (e) {
      pickError = e instanceof Error ? e.message : String(e);
    } finally {
      picking = false;
    }
  }

  function analyze() {
    if (!canAnalyze) return;
    void packStore.analyze();
  }

  function apply() {
    if (!canApply) return;
    void packStore.apply();
  }

  function backToSelect() {
    packStore.reset();
  }

  // Explicit row type (not `as const`): `positive` is optional styling, and
  // `class:positive={c.positive}` must typecheck for EVERY member.
  const CATEGORIES: {
    key: string;
    countKey: string;
    icon: string;
    positive?: boolean;
  }[] = [
    { key: 'reusable', countKey: 'reusable_count', icon: 'check', positive: true },
    { key: 'conflicts', countKey: 'conflict_count', icon: 'git-compare' },
    { key: 'new', countKey: 'new_count', icon: 'file-plus' },
    { key: 'obsolete', countKey: 'obsolete_count', icon: 'clock' },
    { key: 'invalid', countKey: 'invalid_count', icon: 'warning' },
    { key: 'ambiguous', countKey: 'ambiguous_count', icon: 'info' }
  ];

  function countOf(key: string): number {
    const a = packStore.analysis;
    if (!a) return 0;
    switch (key) {
      case 'reusable':
        return a.reusable_count;
      case 'conflicts':
        return a.conflict_count;
      case 'new':
        return a.new_count;
      case 'obsolete':
        return a.obsolete_count;
      case 'invalid':
        return a.invalid_count;
      case 'ambiguous':
        return a.ambiguous_count;
      default:
        return 0;
    }
  }

  function fmt(n: number): string {
    return new Intl.NumberFormat(i18n.locale === 'ru' ? 'ru-RU' : 'en-US').format(n);
  }

  // ---- demo flow (unchanged mock) ----
  type Step = 'select' | 'analyzing' | 'result';
  const ANALYZE_MS = 1200;

  let step = $state<Step>('select');
  let modId = $state<string | null>(null);
  let packId = $state<string | null>(null);
  let analyzeTimer: ReturnType<typeof setTimeout> | undefined;

  const mod = $derived(modId ? (mockMods.find((m) => m.id === modId) ?? null) : null);
  const packs = $derived(modId ? mockLanguagePacks.filter((p) => p.modId === modId) : []);
  const pack = $derived(packId ? (mockLanguagePacks.find((p) => p.id === packId) ?? null) : null);
  const canAnalyzeDemo = $derived(Boolean(mod && pack));

  const RESULT: { key: string; value: number; icon: string; positive?: boolean }[] = [
    { key: 'reusable', value: mockAnalysis.reusable, icon: 'check', positive: true },
    { key: 'sourceChanged', value: mockAnalysis.sourceChanged, icon: 'alert' },
    { key: 'new', value: mockAnalysis.new, icon: 'file-plus' },
    { key: 'obsolete', value: mockAnalysis.obsolete, icon: 'clock' },
    { key: 'invalid', value: mockAnalysis.invalid, icon: 'warning' },
    { key: 'ambiguous', value: mockAnalysis.ambiguous, icon: 'info' }
  ];

  function pickMod(id: string) {
    modId = id;
    packId = null;
  }

  function analyzeDemo() {
    if (!canAnalyzeDemo) return;
    step = 'analyzing';
    clearTimeout(analyzeTimer);
    analyzeTimer = setTimeout(() => {
      step = 'result';
    }, ANALYZE_MS);
  }

  function backDemo() {
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

  {#if live}
    <!-- ================= LIVE: contract flow (W2) ================= -->
    {#if packStore.error}
      <p class="ops-error" role="alert" data-testid="existing.live.error">
        <Icon name="warning" size={14} />
        {packStore.error}
      </p>
    {/if}
    {#if pickError}
      <p class="ops-error" role="alert" data-testid="existing.live.pick.error">
        <Icon name="warning" size={14} />
        {pickError}
      </p>
    {/if}

    <div class="card" data-testid="existing.live.select">
      <label class="field-label" for="existing-live-dir">{t('existing.live.dir')}</label>
      <div class="dir-row">
        <input
          id="existing-live-dir"
          class="dir-input mono"
          type="text"
          placeholder={t('existing.live.dirHint')}
          bind:value={packStore.existingDir}
          data-testid="existing.live.dir"
        />
        <button
          type="button"
          class="btn"
          onclick={pickDir}
          disabled={picking}
          data-testid="existing.live.pick"
        >
          <Icon name="folder-open" size={14} />
          {t('existing.live.pick')}
        </button>
      </div>
      <p class="safety" data-testid="existing.live.analyze-note">
        <Icon name="info" size={14} />
        {t('existing.live.analyzeNote')}
      </p>
      <div class="actions">
        {#if packStore.analyzing}
          <p class="status" role="status" aria-live="polite" data-testid="existing.live.analyzing">
            <Icon name="clock" size={14} />
            {t('existing.analyzing')}
          </p>
        {/if}
        <button
          type="button"
          class="btn btn-primary"
          data-testid="existing.live.analyze"
          disabled={!canAnalyze}
          onclick={analyze}
        >
          <Icon name="search" size={14} />
          {t('existing.analyze')}
        </button>
        <button
          type="button"
          class="btn"
          data-testid="existing.live.cancel"
          onclick={() => router.navigate('home')}
        >
          <Icon name="arrow-left" size={14} />
          {t('common.back')}
        </button>
      </div>
    </div>

    {#if packStore.analysis}
      <div class="card" data-testid="existing.live.result">
        <h2 class="result-title">{t('existing.result.title')}</h2>
        <p class="scope mono" data-testid="existing.live.scope">
          {t('existing.live.scanned', { files: fmt(packStore.analysis.scanned_files), keys: fmt(packStore.analysis.scanned_keys) })}
        </p>
        {#if dirChanged}
          <p class="safety" role="status" data-testid="existing.live.dir-changed">
            <Icon name="warning" size={14} />
            {t('existing.live.dirChanged')}
          </p>
        {/if}
        <dl class="stats">
          {#each CATEGORIES as c (c.key)}
            <div class="stat" class:positive={c.positive} data-testid={`existing.live.cat.${c.key}`}>
              <dt>
                <Icon name={c.icon} size={14} />
                {t(`existing.${c.key}`)}
              </dt>
              <dd class="mono">{fmt(countOf(c.key))}</dd>
            </div>
          {/each}
        </dl>

        {#if packStore.analysis.reusable.length > 0}
          <div class="list" data-testid="existing.live.list.reusable">
            <p class="list-title">{t('existing.live.list.reusable')}</p>
            <ul>
              {#each packStore.analysis.reusable as item (item.key)}
                <li class="mono">{item.key}</li>
              {/each}
            </ul>
          </div>
        {/if}
        {#if packStore.analysis.conflicts.length > 0}
          <div class="list" data-testid="existing.live.list.conflicts">
            <p class="list-title">{t('existing.live.list.conflicts')}</p>
            <ul>
              {#each packStore.analysis.conflicts as item (item.key)}
                <li class="mono">{item.key}</li>
              {/each}
            </ul>
          </div>
        {/if}
        {#if packStore.analysis.obsolete.length > 0}
          <div class="list" data-testid="existing.live.list.obsolete">
            <p class="list-title">{t('existing.live.list.obsolete')}</p>
            <ul>
              {#each packStore.analysis.obsolete as item (item.key)}
                <li class="mono">{item.key}</li>
              {/each}
            </ul>
          </div>
        {/if}
        {#if packStore.analysis.ambiguous.length > 0}
          <div class="list" data-testid="existing.live.list.ambiguous">
            <p class="list-title">{t('existing.live.list.ambiguous')}</p>
            <ul>
              {#each packStore.analysis.ambiguous as item (item.key)}
                <li class="mono">{item.key} → {item.candidates.join(' | ')}</li>
              {/each}
            </ul>
          </div>
        {/if}

        <p class="safety" data-testid="existing.live.apply-note">
          <Icon name="info" size={14} />
          {t('existing.live.applyNote')}
        </p>

        {#if packStore.applied}
          <p class="applied" role="status" data-testid="existing.live.applied">
            <Icon name="circle-check" size={14} />
            {t('existing.live.applied', {
              count: fmt(packStore.applied.applied),
              conflicts: fmt(packStore.applied.conflicts),
              obsolete: fmt(packStore.applied.unmatched),
              ambiguous: fmt(packStore.applied.ambiguous)
            })}
          </p>
        {/if}

        <div class="actions">
          <button
            type="button"
            class="btn btn-primary"
            data-testid="existing.live.apply"
            disabled={!canApply}
            onclick={apply}
          >
            <Icon name="circle-check" size={14} />
            {t('existing.live.apply', { count: fmt(countOf('reusable')) })}
          </button>
          <button type="button" class="btn" data-testid="existing.live.back" onclick={backToSelect}>
            <Icon name="arrow-left" size={14} />
            {t('existing.back')}
          </button>
        </div>
      </div>
    {/if}
  {:else}
    <!-- ================= DEMO: marked mock flow (unchanged) ================= -->
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
          <button type="button" class="btn" data-testid="existing.back" onclick={backDemo}>
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
            disabled={!canAnalyzeDemo || step === 'analyzing'}
            onclick={analyzeDemo}
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
  {/if}
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

  .ops-error {
    margin: 0;
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-left: 3px solid var(--color-danger, var(--color-warning));
    border-radius: var(--radius-md);
    font-size: var(--text-dense-size);
  }

  .ops-error :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  /* Directory picker row */
  .field-label {
    font-weight: 600;
  }

  .dir-row {
    display: flex;
    gap: var(--space-2);
    align-items: stretch;
  }

  .dir-input {
    flex: 1;
    min-width: 0;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    color: inherit;
    padding: var(--space-2) var(--space-3);
    font-size: var(--text-dense-size);
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

  /* Capped sample lists */
  .list {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-3);
  }

  .list-title {
    margin: 0 0 var(--space-1);
    font-weight: 600;
    font-size: var(--text-dense-size);
  }

  .list ul {
    margin: 0;
    padding: 0 0 0 var(--space-4);
    font-size: var(--text-dense-size);
    word-break: break-all;
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

  .applied {
    margin: 0;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-success);
    font-size: var(--text-dense-size);
  }

  .note {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }
</style>
