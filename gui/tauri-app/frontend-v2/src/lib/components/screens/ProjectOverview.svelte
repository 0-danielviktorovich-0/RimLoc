<script lang="ts">
  // Project overview (mandate §7, W4.5/W5 item 7): the authoritative,
  // human-readable answer to "what is this project and where does it stand".
  // Content/mod identity · source locale → target locales (active
  // highlighted) · RimWorld version · source/output locations · translation
  // and review health · source-update state with rescan/review actions ·
  // links into the specialized tools · a clearly separated danger zone with
  // confirmed reset/archive. Deliberately NOT a clone of Settings: it reports
  // project state and routes into tools, it does not re-edit configuration.
  // Mock paths and states, zero backend.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import { languages, type TargetSummary } from '../../languages/store.svelte';
  import { registry } from '../../languages/registry';
  import { router } from '../../router.svelte';
  import { RW_VERSION } from '../../mock/diagnostics';
  import { SOURCE_LOCATION, OUTPUT_LOCATION } from '../../stores/diagnostics.svelte';
  import { capability, CAP_BUILD } from '../../client/capability.svelte';

  // ------------------------------------------------------------ lifecycle CTA
  const counts = $derived(project.statusCounts());
  const problems = $derived(counts.pending_review + counts.sourceChanged);
  const activeSummary = $derived(languages.summary(languages.activeLocale));
  // Audit P1-5: honest degradation on a REAL contract project while the
  // build slice has not landed (see Workspace for the same gate).
  const buildBlocked = $derived(
    project.source === 'contract' && capability.state(CAP_BUILD) === false
  );
  const buildBlockedTitle = $derived(
    buildBlocked
      ? t('capability.unsupported.title', { reason: capability.reason(CAP_BUILD) ?? '' })
      : undefined
  );

  // ------------------------------------------------------------ source update
  // Mock state machine for "the game/mod changed under the project":
  // needs-rescan → scanning → up-to-date. The affected count mirrors the real
  // sourceChanged entries so [Review changes] never lies.
  type UpdateState = 'needsRescan' | 'scanning' | 'upToDate';
  let updateState = $state<UpdateState>('needsRescan');
  let scanTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => () => {
    if (scanTimer) clearTimeout(scanTimer);
  });

  function rescan() {
    if (updateState === 'scanning') return;
    updateState = 'scanning';
    scanTimer = setTimeout(() => {
      updateState = 'upToDate';
    }, 1400);
  }

  // ------------------------------------------------------------ danger zone
  // Two-step confirmation: the destructive verb becomes "yes, <verb>" and
  // needs a second deliberate click. Reset re-uses the dev-panel semantics
  // (pristine mock dataset); archive is a mock-only state with a restore.
  let confirmReset = $state(false);
  let confirmArchive = $state(false);
  let archived = $state(false);

  function doReset() {
    project.reset();
    confirmReset = false;
    updateState = 'needsRescan';
  }

  function doArchive() {
    archived = true;
    confirmArchive = false;
  }

  function viaLabel(via: TargetSummary['addedVia']): string {
    return t(`languages.via.${via}`);
  }

  function modifiedLabel(iso: string): string {
    try {
      return new Date(iso).toLocaleDateString();
    } catch {
      return iso;
    }
  }
</script>

<section class="overview" aria-labelledby="project-overview-heading" data-testid="workspace.project.overview">
  {#if archived}
    <div class="archived-banner" role="status" data-testid="workspace.project.archived">
      <Icon name="package" size={14} />
      {t('workspace.project.archivedBanner')}
      <button type="button" class="btn" data-testid="workspace.project.restore" onclick={() => (archived = false)}>
        {t('workspace.project.restore')}
      </button>
    </div>
  {/if}

  <h2 id="project-overview-heading" class="title">{t('workspace.project.title')}</h2>

  <!-- Identity: content/mod + RimWorld version. -->
  <div class="identity">
    <span class="name mono" data-testid="workspace.project.name">{project.projectName}</span>
    <span class="chip">{t('workspace.project.contentKind')}</span>
    <span class="chip mono" data-testid="workspace.project.rwversion">{RW_VERSION}</span>
    {#if project.source === 'contract'}
      <span class="chip live-chip" data-testid="workspace.project.live">{t('workspace.project.live')}</span>
    {:else}
      <span class="chip fixture-chip" data-testid="workspace.project.fixture" title={t('workspace.project.fixtureNote')}>
        {t('workspace.project.fixtureDataset')}
      </span>
    {/if}
  </div>

  <!-- Lifecycle CTA (§15/§21): what remains → what next. -->
  <div class="cta-row">
    {#if problems > 0}
      <button type="button" class="btn btn-primary" data-testid="workspace.project.cta" onclick={() => router.navigate('review')}>
        <Icon name="clipboard-check" size={14} />
        {t('workspace.cta.reviewIssues', { count: problems })}
      </button>
    {:else}
      <button
        type="button"
        class="btn btn-primary"
        data-testid="workspace.project.cta"
        disabled={buildBlocked}
        title={buildBlockedTitle}
        aria-disabled={buildBlocked}
        onclick={() => router.navigate('build')}
      >
        <Icon name="package" size={14} />
        {t('workspace.cta.build')}
      </button>
    {/if}
  </div>

  <!-- Languages: one source → N targets, active highlighted (multi-target). -->
  <div class="block" data-testid="workspace.project.languages-block">
    <h3 class="block-title">
      {registry.resolve(languages.sourceLocale).nativeName}
      <span class="mono muted">({languages.sourceLocale})</span>
      <Icon name="arrow-right" size={13} />
      {t('workspace.project.targets')}
    </h3>
    <ul class="targets">
      {#each languages.summaries() as summary (summary.locale)}
        <li>
          <button
            type="button"
            class="target"
            class:active={summary.locale === languages.activeLocale}
            data-testid={`workspace.project.target.${summary.locale}`}
            aria-pressed={summary.locale === languages.activeLocale}
            onclick={() => languages.setActive(summary.locale)}
          >
            <span class="target-name">
              {summary.definition.nativeName}
              <span class="mono muted">({summary.locale})</span>
              {#if summary.locale === languages.activeLocale}
                <span class="badge-active">{t('workspace.project.active')}</span>
              {/if}
            </span>
            <span class="bar" aria-hidden="true">
              <span class="bar-fill" style={`width:${summary.progress}%`}></span>
            </span>
            <span class="mono muted target-meta">
              {summary.progress}% · {t('languages.manager.issues')}: {summary.issueCount} · {viaLabel(summary.addedVia)} · {modifiedLabel(summary.lastModified)}
            </span>
          </button>
        </li>
      {/each}
    </ul>
    <button type="button" class="btn" data-testid="workspace.project.addLanguage" onclick={() => languages.openManager('add')}>
      <Icon name="file-plus" size={13} />
      {t('languages.switcher.add')}
    </button>
  </div>

  <!-- Locations: where the source lives and where the build lands. -->
  <dl class="locations" data-testid="workspace.project.locations">
    <div class="loc-row">
      <dt>{t('workspace.project.sourceLocation')}</dt>
      <dd class="mono">{SOURCE_LOCATION}</dd>
    </div>
    <div class="loc-row">
      <dt>{t('workspace.project.outputLocation')}</dt>
      <dd class="mono">{OUTPUT_LOCATION}</dd>
    </div>
    <div class="loc-row">
      <dt>{t('workspace.project.entries')}</dt>
      <dd class="mono">{project.entries.length}</dd>
    </div>
  </dl>

  <!-- Health: translation progress + review state (progress + issues). -->
  <div class="block" data-testid="workspace.project.health">
    <h3 class="block-title">{t('workspace.project.health')}</h3>
    <div class="health-row">
      <span>{t('workspace.project.health.progress', { name: languages.activeDefinition.nativeName })}</span>
      <span class="mono">{activeSummary?.progress ?? 0}%</span>
    </div>
    <div class="bar large" aria-hidden="true">
      <span class="bar-fill" style={`width:${activeSummary?.progress ?? 0}%`}></span>
    </div>
    <div class="health-issues">
      <button type="button" class="btn" data-testid="workspace.project.health.review" onclick={() => router.navigate('review')} disabled={problems === 0}>
        <Icon name="clipboard-check" size={13} />
        {t('workspace.project.health.issues', { count: problems })}
      </button>
      <span class="mono muted">
        {t('workspace.filter.orphan')}: {counts.orphan} · {t('workspace.filter.todo')}: {counts.todo}
      </span>
    </div>
  </div>

  <!-- Source-update state: rescan / review changes. -->
  <div class="block" data-testid="workspace.project.source-update">
    <h3 class="block-title">{t('workspace.project.sourceUpdate')}</h3>
    <div class="update-row">
      {#if updateState === 'scanning'}
        <span class="state scanning" role="status">
          <Icon name="clock" size={13} />
          {t('workspace.project.sourceUpdate.scanning')}
        </span>
      {:else if updateState === 'upToDate'}
        <span class="state ok" data-testid="workspace.project.uptodate">
          <Icon name="circle-check" size={13} />
          {t('workspace.project.sourceUpdate.upToDate')}
        </span>
      {:else}
        <span class="state warn" data-testid="workspace.project.needsrescan">
          <Icon name="warning" size={13} />
          {t('workspace.project.sourceUpdate.needsRescan')}
        </span>
      {/if}
      <div class="update-actions">
        {#if updateState !== 'scanning'}
          <button type="button" class="btn" data-testid="workspace.project.rescan" onclick={rescan}>
            <Icon name="search" size={13} />
            {t('workspace.project.rescan')}
          </button>
        {/if}
        {#if counts.sourceChanged > 0}
          <button type="button" class="btn" data-testid="workspace.project.review-changes" onclick={() => router.navigate('review')}>
            <Icon name="layers" size={13} />
            {t('workspace.project.reviewChanges', { count: counts.sourceChanged })}
          </button>
        {/if}
      </div>
    </div>
  </div>

  <!-- Links into the specialized tools (capability parity in product language). -->
  <div class="block" data-testid="workspace.project.tools">
    <h3 class="block-title">{t('workspace.project.tools')}</h3>
    <div class="tool-grid">
      <button type="button" class="btn" data-testid="workspace.project.tool.glossary" onclick={() => router.navigate('glossary')}>
        <Icon name="book" size={13} /> {t('workspace.tab.glossary')}
      </button>
      <button type="button" class="btn" data-testid="workspace.project.tool.tm" onclick={() => router.navigate('tm')}>
        <Icon name="database" size={13} /> {t('workspace.tab.tm')}
      </button>
      <button type="button" class="btn" data-testid="workspace.project.tool.review" onclick={() => router.navigate('review')}>
        <Icon name="clipboard-check" size={13} /> {t('workspace.tab.review')}
      </button>
      <button type="button" class="btn" data-testid="workspace.project.tool.build" onclick={() => router.navigate('build')}>
        <Icon name="package" size={13} /> {t('workspace.cta.build')}
      </button>
    </div>
  </div>

  <!-- Danger zone: separated, confirmed, explained. -->
  <div class="danger" data-testid="workspace.project.danger">
    <h3 class="block-title danger-title">
      <Icon name="warning" size={13} />
      {t('workspace.project.danger')}
    </h3>
    <p class="muted-text">{t('workspace.project.dangerNote')}</p>

    <div class="danger-action">
      <div class="danger-text">
        <span class="danger-name">{t('workspace.project.reset')}</span>
        <span class="muted-text">{t('workspace.project.resetDesc')}</span>
      </div>
      {#if confirmReset}
        <div class="confirm-row">
          <span class="confirm-text">{t('workspace.project.resetConfirm')}</span>
          <button type="button" class="btn btn-destructive" data-testid="workspace.project.resetConfirm" onclick={doReset}>
            {t('workspace.project.resetYes')}
          </button>
          <button type="button" class="btn" onclick={() => (confirmReset = false)}>{t('common.cancel')}</button>
        </div>
      {:else}
        <button type="button" class="btn" data-testid="workspace.project.reset" disabled={archived} onclick={() => (confirmReset = true)}>
          {t('workspace.project.reset')}
        </button>
      {/if}
    </div>

    <div class="danger-action">
      <div class="danger-text">
        <span class="danger-name">{t('workspace.project.archive')}</span>
        <span class="muted-text">{t('workspace.project.archiveDesc')}</span>
      </div>
      {#if confirmArchive}
        <div class="confirm-row">
          <span class="confirm-text">{t('workspace.project.archiveConfirm')}</span>
          <button type="button" class="btn btn-destructive" data-testid="workspace.project.archiveConfirm" onclick={doArchive}>
            {t('workspace.project.archiveYes')}
          </button>
          <button type="button" class="btn" onclick={() => (confirmArchive = false)}>{t('common.cancel')}</button>
        </div>
      {:else}
        <button type="button" class="btn" data-testid="workspace.project.archive" disabled={archived} onclick={() => (confirmArchive = true)}>
          {t('workspace.project.archive')}
        </button>
      {/if}
    </div>
  </div>
</section>

<style>
  .overview {
    padding: var(--space-4) var(--space-6);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-width: 640px;
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: 18px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .archived-banner {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-warning);
    border-radius: var(--radius-md);
    color: var(--color-warning);
    font-size: var(--text-dense-size);
  }

  .archived-banner .btn {
    margin-left: auto;
  }

  .identity {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .name {
    font-weight: 600;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    padding: 2px var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-muted);
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
  }

  .cta-row {
    display: flex;
  }

  .fixture-chip,
  .live-chip {
    border-style: dashed;
  }

  .fixture-chip {
    color: var(--color-warning);
    border-color: var(--color-warning);
  }

  .live-chip {
    color: var(--color-success);
    border-color: var(--color-success);
  }

  .block {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .block-title {
    margin: 0;
    display: flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-meta-size);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-muted-fg);
  }

  .block-title :global(svg) {
    color: var(--color-muted-fg);
  }

  .muted {
    color: var(--color-muted-fg);
  }

  .muted-text {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .targets {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .target {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    cursor: pointer;
    text-align: left;
    transition: border-color var(--motion-fast) var(--ease-out);
  }

  .target:hover {
    border-color: var(--color-border-strong);
  }

  .target.active {
    border-color: var(--color-primary);
    box-shadow: inset 3px 0 0 var(--color-primary);
  }

  .target-name {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-base-size);
  }

  .badge-active {
    font-size: var(--text-meta-size);
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
    background: var(--color-primary);
    color: var(--color-primary-fg);
  }

  .bar {
    height: 6px;
    border-radius: var(--radius-sm);
    background: var(--color-muted);
    overflow: hidden;
  }

  .bar.large {
    height: 8px;
  }

  .bar-fill {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--color-primary);
    transition: width var(--motion-standard) var(--ease-out);
  }

  .target-meta {
    font-size: var(--text-meta-size);
  }

  .locations {
    margin: 0;
    display: flex;
    flex-direction: column;
  }

  .loc-row {
    display: grid;
    grid-template-columns: 170px 1fr;
    gap: var(--space-3);
    padding: var(--space-1) 0;
    border-bottom: 1px solid var(--color-border);
    font-size: var(--text-dense-size);
  }

  .loc-row:last-child {
    border-bottom: none;
  }

  .loc-row dt {
    color: var(--color-muted-fg);
  }

  .loc-row dd {
    margin: 0;
    word-break: break-all;
  }

  .health-row {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    font-size: var(--text-dense-size);
  }

  .health-issues {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: var(--space-2);
    font-size: var(--text-meta-size);
  }

  .update-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .state {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-dense-size);
  }

  .state.ok {
    color: var(--color-success);
  }

  .state.warn {
    color: var(--color-warning);
  }

  .state.scanning {
    color: var(--color-muted-fg);
  }

  .update-actions {
    display: flex;
    gap: var(--space-2);
  }

  .tool-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: var(--space-2);
  }

  .danger {
    border: 1px solid var(--color-destructive);
    border-radius: var(--radius-lg);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .danger-title {
    color: var(--color-destructive);
  }

  .danger-title :global(svg) {
    color: var(--color-destructive);
  }

  .danger-action {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: var(--space-3);
  }

  .danger-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 200px;
  }

  .danger-name {
    font-weight: 600;
    font-size: var(--text-dense-size);
  }

  .confirm-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .confirm-text {
    font-size: var(--text-meta-size);
  }

  /* Scoped destructive accent for the danger-zone confirmations (global btn
     classes stay untouched — this is a presentation-only accent). */
  .btn-destructive {
    border-color: var(--color-destructive);
    color: var(--color-destructive);
  }

  .btn-destructive:hover {
    background: var(--color-destructive);
    color: var(--color-surface-fg);
  }
</style>
