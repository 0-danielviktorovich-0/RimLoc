<script lang="ts">
  // Home per mandate §3: two equal-weight entry cards, first-run hint and
  // returning-user recent projects (name, source→target, progress bar,
  // sourceChanged/issues badges, [Continue]). No analytics dashboard.
  // W6: a bundled synthetic demo project card (marked, isolated, deterministic
  // reset) and the no-mods empty state with Try Demo / Choose folder /
  // Configure installation (MOCK_LIVE_ONBOARDING_MANDATE §6/§8/§9).
  import Icon from '../Icon.svelte';
  import { t, i18n } from '../../../i18n/store.svelte';
  import { ui, type SimpleState } from '../../stores/ui.svelte';
  import { router } from '../../router.svelte';
  import { mockProjects } from '../../mock/wizard';
  import { MOCK_ERROR_CODE, MOCK_ERROR_RAW } from '../../../lib/mock/data';
  import { demoProject } from '../../demo/demoProject.svelte';

  // NB: renamed to homeState — a prop named `state` makes the compiler parse
  // `$state(...)` as a legacy store subscription of that prop.
  let { state: homeState = 'ready' }: { state?: SimpleState } = $props();

  const firstRun = $derived(ui.homeMode === 'first-run');
  const noMods = $derived(ui.homeMode === 'no-mods');

  // Honest mock note for the folder picker: in this scaffold there is no OS
  // dialog and no backend — the button refuses to pretend otherwise.
  let folderNote = $state(false);

  function openDemo() {
    demoProject.seed();
    router.navigate('workspace');
  }

  function fmtDate(iso: string): string {
    try {
      return new Intl.DateTimeFormat(i18n.locale === 'ru' ? 'ru-RU' : 'en-US', {
        dateStyle: 'medium'
      }).format(new Date(iso));
    } catch {
      return iso;
    }
  }
</script>

<section class="home" aria-labelledby="home-heading">
  <h1 id="home-heading" class="home-title">{t('home.title')}</h1>
  {#if firstRun}
    <p class="lead">{t('home.lead')}</p>
  {/if}

  {#if homeState === 'loading'}
    <p class="state-line" role="status">{t('home.state.loading')}</p>
    <div class="cards">
      <div class="card card-skeleton" aria-hidden="true"></div>
      <div class="card card-skeleton" aria-hidden="true"></div>
    </div>
  {:else if homeState === 'error'}
    <div class="error-card" role="alert">
      <p class="error-title">{t('common.errorTitle')}</p>
      <p>{t('home.state.error')}</p>
      <p class="mono error-code">{MOCK_ERROR_CODE}</p>
      <button type="button" class="btn" onclick={() => (ui.homeState = 'ready')}>
        {t('common.retry')}
      </button>
      <details class="raw">
        <summary>{t('common.details')}</summary>
        <pre class="mono">{MOCK_ERROR_RAW}</pre>
      </details>
    </div>
  {:else if noMods}
    <!-- W6 no-mods empty state (mandate §9): three real-way-out actions.
         Try Demo seeds the bundled synthetic project; the folder picker is an
         honest mock note, not a fake dialog; configuration is a real route. -->
    <div class="nomods" data-testid="home.nomods">
      <p class="nomods-title">{t('home.nomods.title')}</p>
      <p class="nomods-desc">{t('home.nomods.desc')}</p>
      <div class="nomods-actions">
        <button type="button" class="btn btn-primary" data-testid="home.nomods.demo" onclick={openDemo}>
          <Icon name="play" size={14} />
          {t('home.nomods.demo')}
        </button>
        <button type="button" class="btn" data-testid="home.nomods.folder" onclick={() => (folderNote = true)}>
          <Icon name="folder-open" size={14} />
          {t('home.nomods.folder')}
        </button>
        <button type="button" class="btn" data-testid="home.nomods.configure" onclick={() => router.navigate('settings')}>
          <Icon name="settings" size={14} />
          {t('home.nomods.configure')}
        </button>
      </div>
      {#if folderNote}
        <p class="nomods-note" role="note" data-testid="home.nomods.note">{t('home.nomods.note')}</p>
      {/if}
    </div>

    <div class="cards">
      <button type="button" class="card entry-card" data-testid="home.nomods.later" onclick={() => (ui.homeMode = 'returning')}>
        <span class="card-icon"><Icon name="arrow-left" size={28} /></span>
        <span class="card-title">{t('home.nomods.later')}</span>
        <span class="card-desc">{t('home.nomods.laterDesc')}</span>
      </button>
    </div>
  {:else}
    <div class="cards">
      <button
        type="button"
        class="card entry-card"
        data-testid="home.entry-new"
        onclick={() => router.navigate('wizard')}
      >
        <span class="card-icon"><Icon name="file-plus" size={28} /></span>
        <span class="card-title">{t('home.entryNew.title')}</span>
        <span class="card-desc">{t('home.entryNew.description')}</span>
      </button>

      <button
        type="button"
        class="card entry-card"
        data-testid="home.entry-existing"
        onclick={() => router.navigate('existing')}
      >
        <span class="card-icon"><Icon name="folder-open" size={28} /></span>
        <span class="card-title">{t('home.entryExisting.title')}</span>
        <span class="card-desc">{t('home.entryExisting.description')}</span>
      </button>
    </div>

    {#if firstRun}
      <p class="hint" data-testid="home.hint">{t('home.hint')}</p>
      <div class="secondary">
        <button type="button" class="link" data-testid="home.secondary-base">
          {t('home.secondaryBase')}
        </button>
        <span class="dot-sep" aria-hidden="true">·</span>
        <button type="button" class="link" data-testid="home.secondary-help" onclick={() => router.navigate('help')}>
          {t('home.secondaryHelp')}
        </button>
      </div>
    {/if}

    <!-- W6 demo project (mandate §6/§8): bundled, synthetic, RimLoc-owned and
         clearly marked so it is never confused with a real recent project.
         State is isolated and deterministically resettable. -->
    <section class="demo" aria-labelledby="demo-heading" data-testid="home.demo">
      <div class="demo-main">
        <span class="demo-name">
          {t('home.demo.title')}
          <span class="demo-mark">{t('home.demo.mark')}</span>
        </span>
        <span class="demo-desc">{t('home.demo.desc')}</span>
      </div>
      <div class="demo-actions">
        <button type="button" class="btn btn-primary" data-testid="home.demo.open" onclick={openDemo}>
          <Icon name="play" size={14} />
          {demoProject.active ? t('home.demo.again') : t('home.demo.open')}
        </button>
        {#if demoProject.active}
          <button type="button" class="btn" data-testid="home.demo.reset" onclick={() => demoProject.resetDemo()}>
            {t('home.demo.reset')}
          </button>
        {/if}
      </div>
    </section>

    {#if !firstRun && mockProjects.length > 0}
      <section class="recent" aria-labelledby="recent-heading" data-testid="home.recent">
        <h2 id="recent-heading" class="recent-title">{t('home.recent.title')}</h2>
        <ul class="recent-list">
          {#each mockProjects as p (p.id)}
            <li class="project">
              <div class="project-main">
                <span class="project-name">{p.name}</span>
                <span class="project-meta mono">{p.source} → {p.target.toUpperCase()}</span>
                <div
                  class="bar"
                  role="progressbar"
                  aria-label={t('home.recent.progress')}
                  aria-valuenow={p.progress}
                  aria-valuemin={0}
                  aria-valuemax={100}
                >
                  <div class="bar-fill" style="width: {p.progress}%"></div>
                </div>
                <div class="badges">
                  <span class="pct">{p.progress}%</span>
                  {#if p.sourceChanged > 0}
                    <span class="badge warn">
                      <Icon name="alert" size={12} />
                      {t('home.recent.sourceChanged', { count: p.sourceChanged })}
                    </span>
                  {/if}
                  {#if p.issues > 0}
                    <span class="badge issue">
                      <Icon name="warning" size={12} />
                      {t('home.recent.issues', { count: p.issues })}
                    </span>
                  {/if}
                  <span class="modified">{t('home.recent.modified')}: {fmtDate(p.modified)}</span>
                </div>
              </div>
              <button
                type="button"
                class="btn btn-primary continue"
                data-testid={`home.recent-continue.${p.id}`}
                onclick={() => router.navigate('workspace')}
              >
                {t('common.continue')}
                <Icon name="arrow-right" size={14} />
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  {/if}
</section>

<style>
  .home {
    width: min(860px, 100%);
    margin: 0 auto;
    padding: var(--space-8) var(--space-4);
    overflow-y: auto;
  }

  .home-title {
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
    margin: 0 0 var(--space-2);
  }

  .lead {
    margin: 0 0 var(--space-6);
    color: var(--color-muted-fg);
  }

  .state-line {
    color: var(--color-muted-fg);
  }

  .cards {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-4);
  }

  @media (max-width: 640px) {
    .cards {
      grid-template-columns: 1fr;
    }
  }

  .card {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    color: var(--color-surface-fg);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    text-align: left;
    min-height: 148px;
    box-shadow: var(--shadow-card);
    transition:
      border-color var(--motion-fast) var(--ease-out),
      box-shadow var(--motion-fast) var(--ease-out),
      transform var(--motion-fast) var(--ease-out);
  }

  .entry-card:hover {
    border-color: var(--color-primary);
    box-shadow: var(--shadow-hover);
    transform: var(--card-hover-transform);
  }

  .entry-card:active {
    transform: var(--btn-press-transform);
  }

  .card-icon {
    color: var(--card-icon-fg);
    background: var(--card-icon-bg);
    padding: var(--card-icon-pad);
    border-radius: var(--card-icon-radius);
    display: inline-flex;
    /* Keep the icon chip hug-content even when it carries a filled background. */
    align-self: flex-start;
    margin-bottom: var(--space-2);
  }

  .card-title {
    font-family: var(--font-heading);
    font-size: 16px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .card-desc {
    color: var(--color-muted-fg);
    font-size: var(--text-base-size);
  }

  .card-skeleton {
    background: var(--color-muted);
    border: none;
    min-height: 148px;
  }

  .hint {
    margin: var(--space-4) 0 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .secondary {
    margin-top: var(--space-2);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-meta-size);
  }

  .link {
    color: var(--color-primary-text);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .link:hover {
    color: var(--color-primary-hover-text, var(--color-primary-text));
  }

  .dot-sep {
    color: var(--color-muted-fg);
  }

  /* W6: demo project card + no-mods empty state */
  .demo {
    margin-top: var(--space-6);
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: var(--space-3);
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    padding: var(--space-3) var(--space-4);
  }

  .demo-main {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }

  .demo-name {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-weight: 600;
  }

  .demo-mark {
    font-size: var(--text-meta-size);
    font-weight: 500;
    padding: 0 var(--space-1);
    border: 1px dashed var(--color-warning);
    border-radius: var(--radius-sm);
    color: var(--color-warning);
    white-space: nowrap;
  }

  .demo-desc {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .demo-actions {
    display: flex;
    gap: var(--space-2);
    flex: none;
  }

  .nomods {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    align-items: flex-start;
    box-shadow: var(--shadow-card);
  }

  .nomods-title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
  }

  .nomods-desc {
    margin: 0 0 var(--space-2);
    color: var(--color-muted-fg);
  }

  .nomods-actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .nomods-note {
    margin: 0;
    color: var(--color-warning);
    font-size: var(--text-meta-size);
    border: 1px dashed var(--color-warning);
    border-radius: var(--radius-sm);
    padding: var(--space-1) var(--space-2);
  }

  /* Recent projects (returning users) */
  .recent {
    margin-top: var(--space-8);
  }

  .recent-title {
    font-family: var(--font-heading);
    font-size: 16px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
    margin: 0 0 var(--space-3);
  }

  .recent-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .project {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    padding: var(--space-3) var(--space-4);
    transition: border-color var(--motion-fast) var(--ease-out);
  }

  .project:hover {
    border-color: var(--color-border-strong);
  }

  .project-main {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
    flex: 1;
  }

  .project-name {
    font-weight: 600;
  }

  .project-meta {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .bar {
    height: 6px;
    border-radius: var(--radius-sm);
    background: var(--color-muted);
    overflow: hidden;
    max-width: 420px;
  }

  .bar-fill {
    height: 100%;
    background: var(--color-primary);
    border-radius: var(--radius-sm);
  }

  .badges {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
    font-size: var(--text-meta-size);
  }

  .pct {
    font-variant-numeric: tabular-nums;
  }

  .badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--color-warning);
  }

  .badge :global(svg) {
    flex: none;
  }

  .badge.issue {
    color: var(--color-destructive);
  }

  .modified {
    color: var(--color-muted-fg);
  }

  .continue {
    flex: none;
  }

  .error-card {
    border: 1px solid var(--color-border);
    border-left: 3px solid var(--color-destructive);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    align-items: flex-start;
  }

  .error-title {
    font-weight: 600;
    margin: 0;
  }

  .error-code {
    color: var(--color-muted-fg);
    margin: 0;
  }

  .raw {
    width: 100%;
  }

  .raw pre {
    background: var(--color-muted);
    border-radius: var(--radius-sm);
    padding: var(--space-2);
    overflow: auto;
  }
</style>
