<script lang="ts">
  // Build route (mandate §14): the final workflow. Pre-build summary over live
  // mock counters (coverage, validation, known warnings, destination), then the
  // [Build translation] action with a simulated run, and the success state with
  // [Open folder] / [Install translation] / [Test again]. Advanced details are
  // collapsed. All actions are mocks — nothing touches the filesystem.
  import Icon from '../Icon.svelte';
  import { t, i18n } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import { review } from '../../stores/review.svelte';
  import { router } from '../../router.svelte';
  // W6 (lead 027): the phase machine lives in ONE shared mock store so the
  // product tour presses the same engine the real button uses — and only a
  // completed run can count as completed anywhere.
  import { buildState } from '../../mock/buildState.svelte';

  const INSTALL_MS = 900;

  const phase = $derived(buildState.phase);
  let installState = $state<'idle' | 'installing' | 'installed'>('idle');
  let toast = $state('');
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  const counts = $derived(project.statusCounts());
  const total = $derived(project.entries.length);
  const done = $derived(total - counts.untranslated - counts.todo);
  const attention = $derived(counts.pending_review + counts.sourceChanged + counts.orphan);
  const pct = $derived(total > 0 ? Math.round((done / total) * 100) : 0);

  // Known warnings: group the still-open review issues by category.
  const warnings = $derived.by(() => {
    const acc: { kind: string; count: number }[] = [];
    for (const issue of review.active) {
      const hit = acc.find((w) => w.kind === issue.kind);
      if (hit) hit.count += 1;
      else acc.push({ kind: issue.kind, count: 1 });
    }
    return acc;
  });

  const LOCALE_NAMES: Record<string, string> = { ru: 'Russian', de: 'German', en: 'English' };
  const localeName = $derived(LOCALE_NAMES[project.targetLocale] ?? project.targetLocale);
  const outputPath = $derived(`…/RimWorld/Mods/${project.projectName}/Languages/${localeName}`);

  function fmt(n: number): string {
    return new Intl.NumberFormat(i18n.locale === 'ru' ? 'ru-RU' : 'en-US').format(n);
  }

  function showToast(message: string) {
    toast = message;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = ''), 3200);
  }

  function runBuild() {
    buildState.start();
    installState = 'idle';
  }

  function retest() {
    buildState.reset();
    installState = 'idle';
  }

  function install() {
    installState = 'installing';
    setTimeout(() => {
      installState = 'installed';
    }, INSTALL_MS);
  }

  function openFolder() {
    showToast(t('build.openedNote', { path: outputPath }));
  }
</script>

<section class="build" aria-labelledby="build-heading">
  <h1 id="build-heading" class="title">{t('build.title')}</h1>

  {#if phase !== 'done'}
    <div class="card" data-testid="build.summary">
      <div class="row">
        <span class="label">{t('build.coverage')}</span>
        <span class="value" data-testid="build.coverage">
          {t('build.coverage.value', { done: fmt(done), total: fmt(total) })}
          <span class="pct mono">{pct}%</span>
        </span>
      </div>
      <div
        class="bar"
        role="progressbar"
        aria-label={t('build.coverage')}
        aria-valuenow={pct}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div class="bar-fill" class:run={phase === 'building'} style="width: {phase === 'building' ? 100 : pct}%"></div>
      </div>

      <div class="row">
        <span class="label">{t('build.validation')}</span>
        <span class="value" class:warn={attention > 0} data-testid="build.validation">
          {t('build.validation.value', { count: fmt(attention) })}
        </span>
      </div>

      <div class="warnings" data-testid="build.warnings">
        {#if warnings.length === 0}
          <p class="warnings-ok"><Icon name="circle-check" size={14} /> {t('build.warnings.none')}</p>
        {:else}
          <p class="warnings-label">{t('build.warnings')}</p>
          <ul class="warnings-list">
            {#each warnings as w (w.kind)}
              <li>
                <span class="mono count">{fmt(w.count)}</span>
                {t(`issue.${w.kind}`)}
              </li>
            {/each}
          </ul>
          <button type="button" class="link" data-testid="build.warnings.review" onclick={() => router.navigate('review')}>
            {t('build.warnings.review')}
            <Icon name="arrow-right" size={13} />
          </button>
        {/if}
      </div>

      <div class="row">
        <span class="label">{t('build.dest')}</span>
        <span class="value mono" data-testid="build.dest">{outputPath}</span>
      </div>

      <div class="actions">
        <button
          type="button"
          class="btn btn-primary"
          data-testid="build.run"
          disabled={phase === 'building'}
          onclick={runBuild}
        >
          <Icon name="package" size={14} />
          {phase === 'building' ? t('build.building') : t('build.button')}
        </button>
        {#if phase === 'idle'}
          <button type="button" class="btn" data-testid="build.back-editor" onclick={() => router.navigate('workspace')}>
            <Icon name="edit" size={14} />
            {t('wizard.w7.openEditor')}
          </button>
        {/if}
      </div>

      {#if phase === 'building'}
        <p class="status" role="status" aria-live="polite" data-testid="build.building">
          <Icon name="clock" size={14} />
          {t('build.buildingNote')}
        </p>
      {/if}
    </div>
  {:else}
    <div class="card success" data-testid="build.done">
      <p class="success-head">
        <Icon name="circle-check" size={20} />
        {t('build.success')}
      </p>

      <div class="row">
        <span class="label">{t('build.output')}</span>
        <span class="value mono" data-testid="build.output">{outputPath}</span>
      </div>

      <div class="actions">
        <button type="button" class="btn" data-testid="build.open-folder" onclick={openFolder}>
          <Icon name="external" size={14} />
          {t('build.openFolder')}
        </button>
        <button
          type="button"
          class="btn btn-primary"
          data-testid="build.install"
          disabled={installState !== 'idle'}
          onclick={install}
        >
          {#if installState === 'installed'}
            <Icon name="circle-check" size={14} />
            {t('build.installed')}
          {:else}
            <Icon name="download" size={14} />
            {t('build.install')}
          {/if}
        </button>
        <button type="button" class="btn" data-testid="build.retest" onclick={retest}>
          <Icon name="clipboard-check" size={14} />
          {t('build.retest')}
        </button>
      </div>

      {#if installState === 'installing'}
        <p class="status" role="status" aria-live="polite" data-testid="build.installing">
          <Icon name="clock" size={14} />
          {t('build.installingNote')}
        </p>
      {:else if installState === 'installed'}
        <p class="status ok" role="status" data-testid="build.installed-note">
          <Icon name="info" size={14} />
          {t('build.installedNote')}
        </p>
      {/if}

      <p class="toast" role="status" aria-live="polite" data-testid="build.toast">
        {#if toast}<Icon name="external" size={13} /> {toast}{/if}
      </p>

      <details class="advanced">
        <summary data-testid="build.advanced">{t('build.advanced')}</summary>
        <dl class="adv-grid">
          <div class="adv">
            <dt>{t('build.advanced.keys')}</dt>
            <dd class="mono">{fmt(total)}</dd>
          </div>
          <div class="adv">
            <dt>{t('build.advanced.files')}</dt>
            <dd class="mono">14</dd>
          </div>
          <div class="adv">
            <dt>{t('build.advanced.gameVersion')}</dt>
            <dd class="mono">{t('workspace.meta.version')}</dd>
          </div>
          <div class="adv">
            <dt>{t('build.advanced.time')}</dt>
            <dd class="mono">2.4 s</dd>
          </div>
          <div class="adv">
            <dt>{t('build.advanced.pkg')}</dt>
            <dd class="mono">{project.projectName.toLowerCase()}.translations.{project.targetLocale}</dd>
          </div>
          <div class="adv">
            <dt>{t('build.validation')}</dt>
            <dd class="mono">{t('build.validation.value', { count: fmt(attention) })}</dd>
          </div>
        </dl>
      </details>
    </div>
  {/if}

  <p class="note">{t('build.note')}</p>
</section>

<style>
  .build {
    width: min(680px, 100%);
    margin: 0 auto;
    padding: var(--space-8) var(--space-4);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .title {
    margin: 0 0 var(--space-2);
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
  }

  .card {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    padding: var(--space-4) var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .success {
    border-color: var(--color-success);
  }

  .success-head {
    margin: 0;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-family: var(--font-heading);
    font-size: 16px;
    font-weight: 600;
    color: var(--color-success);
  }

  .row {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: var(--space-3);
    font-size: var(--text-base-size);
    align-items: baseline;
  }

  .label {
    color: var(--color-muted-fg);
  }

  .value {
    overflow-wrap: anywhere;
  }

  .value.warn {
    color: var(--color-warning);
  }

  .pct {
    color: var(--color-muted-fg);
    margin-left: var(--space-2);
  }

  .bar {
    height: 6px;
    border-radius: var(--radius-sm);
    background: var(--color-muted);
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    background: var(--color-primary);
    border-radius: var(--radius-sm);
    transition: width var(--motion-emphasis) var(--ease-emphasis);
  }

  .bar-fill.run {
    transition: width 1.3s linear;
  }

  /* Known warnings block */
  .warnings {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .warnings-ok {
    margin: 0;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-success);
    font-size: var(--text-dense-size);
  }

  .warnings-label {
    margin: 0;
    font-size: var(--text-meta-size);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-muted-fg);
  }

  .warnings-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: var(--text-dense-size);
  }

  .warnings-list .count {
    display: inline-block;
    min-width: 3ch;
    color: var(--color-warning);
    font-variant-numeric: tabular-nums;
  }

  .link {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    align-self: flex-start;
    color: var(--color-primary-text);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .link:hover {
    color: var(--color-primary-hover, var(--color-primary-text));
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-2);
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

  .status.ok {
    color: var(--color-success);
  }

  .toast {
    margin: 0;
    min-height: 1em;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  /* Advanced details */
  .advanced summary {
    cursor: pointer;
    color: var(--color-muted-fg);
    font-size: var(--text-dense-size);
    padding: var(--space-1) 0;
  }

  .advanced summary:hover {
    color: var(--color-fg);
  }

  .adv-grid {
    margin: var(--space-2) 0 0;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-3);
  }

  @media (max-width: 560px) {
    .adv-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  .adv dt {
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
  }

  .adv dd {
    margin: 2px 0 0;
    overflow-wrap: anywhere;
  }

  .note {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  @media (max-width: 560px) {
    .row {
      grid-template-columns: 1fr;
    }
  }
</style>
