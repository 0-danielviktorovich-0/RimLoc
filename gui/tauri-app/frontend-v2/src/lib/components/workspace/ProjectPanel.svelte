<script lang="ts">
  // Project tab: identity, languages, RimWorld version, live status counts and
  // the lifecycle CTA (mandate §8, §19) — the "what remains" answer.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import { router } from '../../router.svelte';

  const counts = $derived(project.statusCounts());
  const total = $derived(project.entries.length);
  const problems = $derived(counts.pending_review + counts.sourceChanged);
</script>

<section class="about" aria-labelledby="about-heading" data-testid="workspace.project-panel">
  <h2 id="about-heading" class="title">{t('workspace.project.title')}</h2>

  <dl class="meta">
    <div class="row">
      <dt>{t('header.project')}</dt>
      <dd class="mono">{project.projectName}</dd>
    </div>
    <div class="row">
      <dt>{t('workspace.project.languages')}</dt>
      <dd class="mono">en → {project.targetLocale.toUpperCase()}</dd>
    </div>
    <div class="row">
      <dt>{t('workspace.project.version')}</dt>
      <dd>{t('workspace.meta.version')}</dd>
    </div>
    <div class="row">
      <dt>{t('workspace.project.entries')}</dt>
      <dd class="mono">{total}</dd>
    </div>
  </dl>

  <h3 class="section">{t('workspace.filter.label')}</h3>
  <ul class="counts">
    {#each Object.entries(counts) as [status, n] (status)}
      <li class="count-row">
        <span>{t(`workspace.filter.${status}`)}</span>
        <span class="mono">{n}</span>
      </li>
    {/each}
  </ul>

  <div class="cta-row">
    {#if problems > 0}
      <button type="button" class="btn btn-primary" data-testid="workspace.project.cta" onclick={() => router.navigate('review')}>
        <Icon name="clipboard-check" size={14} />
        {t('workspace.cta.reviewIssues', { count: problems })}
      </button>
    {:else}
      <button type="button" class="btn btn-primary" data-testid="workspace.project.cta" onclick={() => router.navigate('build')}>
        <Icon name="package" size={14} />
        {t('workspace.cta.build')}
      </button>
    {/if}
  </div>
</section>

<style>
  .about {
    padding: var(--space-4) var(--space-6);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    max-width: 520px;
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: 18px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .meta {
    margin: 0;
    display: flex;
    flex-direction: column;
  }

  .row {
    display: grid;
    grid-template-columns: 160px 1fr;
    gap: var(--space-3);
    padding: var(--space-1) 0;
    border-bottom: 1px solid var(--color-border);
    font-size: var(--text-base-size);
  }

  .row dt {
    color: var(--color-muted-fg);
  }

  .row dd {
    margin: 0;
  }

  .section {
    margin: var(--space-2) 0 0;
    font-size: var(--text-meta-size);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-muted-fg);
  }

  .counts {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .count-row {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-1) 0;
    border-bottom: 1px solid var(--color-border);
    font-size: var(--text-base-size);
  }

  .count-row:last-child {
    border-bottom: none;
  }

  .count-row .mono {
    color: var(--color-muted-fg);
    font-variant-numeric: tabular-nums;
  }

  .cta-row {
    margin-top: var(--space-2);
  }
</style>
