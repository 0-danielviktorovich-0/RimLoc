<script lang="ts">
  // Build route (mandate §14) — phase-2 stub: pre-build summary (coverage,
  // validation, warnings, output destination) over live mock counters. The
  // build itself and its result screen arrive in GUI sprint phase 2.
  import Icon from '../Icon.svelte';
  import { t, i18n } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import { router } from '../../router.svelte';

  const counts = $derived(project.statusCounts());
  const total = $derived(project.entries.length);
  const done = $derived(total - counts.untranslated - counts.todo);
  const attention = $derived(counts.pending_review + counts.sourceChanged + counts.orphan);

  function fmt(n: number): string {
    return new Intl.NumberFormat(i18n.locale === 'ru' ? 'ru-RU' : 'en-US').format(n);
  }
</script>

<section class="build" aria-labelledby="build-heading">
  <h1 id="build-heading" class="title">{t('build.title')}</h1>

  <div class="card" data-testid="build.summary">
    <div class="row">
      <span class="label">{t('build.coverage')}</span>
      <span class="value" data-testid="build.coverage">
        {t('build.coverage.value', { done: fmt(done), total: fmt(total) })}
      </span>
    </div>
    <div class="bar" role="progressbar" aria-label={t('build.coverage')} aria-valuenow={Math.round((done / total) * 100)} aria-valuemin={0} aria-valuemax={100}>
      <div class="bar-fill" style="width: {(done / total) * 100}%"></div>
    </div>
    <div class="row">
      <span class="label">{t('build.validation')}</span>
      <span class="value" class:warn={attention > 0}>
        {t('build.validation.value', { count: fmt(attention) })}
      </span>
    </div>
    <div class="row">
      <span class="label">{t('build.dest')}</span>
      <span class="value mono">…/Translations/{project.projectName}/{project.targetLocale}</span>
    </div>

    <div class="actions">
      <button type="button" class="btn btn-primary" data-testid="build.run">
        <Icon name="package" size={14} />
        {t('build.button')}
      </button>
      <button type="button" class="btn" data-testid="build.back-editor" onclick={() => router.navigate('workspace')}>
        <Icon name="edit" size={14} />
        {t('wizard.w7.openEditor')}
      </button>
    </div>
  </div>

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
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-2);
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
