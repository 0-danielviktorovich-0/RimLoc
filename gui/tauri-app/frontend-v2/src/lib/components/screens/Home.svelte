<script lang="ts">
  // Home per mandate §3: first-run hint and returning-user recent projects
  // (name, source→target, progress bar, sourceChanged/issues badges,
  // [Continue]). No analytics dashboard. W6: a bundled synthetic demo project
  // card (marked, isolated, deterministic reset) and the no-mods empty state
  // with Try Demo / Choose folder / Configure installation
  // (MOCK_LIVE_ONBOARDING_MANDATE §6/§8/§9).
  //
  // Design synthesis (jury 2026-09-29): the design-system skeleton — the
  // FIRST content block is the "continue working" hero card (project, path,
  // pair, progress, counters, actions), the recents list is a semantic table
  // (scope=col headers, sr-only actions column), status chips are soft
  // fg/bg/border token triples (see tokens.css). Identity stays the app's
  // indigo — one accent color, no second hue. From minimalism: human dates
  // ("modified today at 14:32") and inline-code path chips in the helper.
  // From the brutalism lens: a status vocabulary readable as
  // marker-pattern + label + note (color-blind safe). The featured project
  // is EXCLUDED from the table, so "continue" and "recents" never duplicate.
  import Icon from '../Icon.svelte';
  import { t, i18n } from '../../../i18n/store.svelte';
  import { ui, type SimpleState } from '../../stores/ui.svelte';
  import { router } from '../../router.svelte';
  import { mockProjects, type RecentProject } from '../../mock/wizard';
  import { MOCK_ERROR_CODE, MOCK_ERROR_RAW } from '../../../lib/mock/data';
  import { demoProject } from '../../demo/demoProject.svelte';
  import { project } from '../../stores/project.svelte';
  import { clientInstance, type ResolvedClientMode } from '../../client/instance.svelte';
  import type { ProjectSummaryDto } from '../../client/types';
  import { openSelflocProject } from '../../selfloc';

  // NB: renamed to homeState — a prop named `state` makes the compiler parse
  // `$state(...)` as a legacy store subscription of that prop.
  let { state: homeState = 'ready' }: { state?: SimpleState } = $props();

  // W-built: mode resolution at Home mount gates the whole fixture surface.
  // 'tauri' = real client (contract panel); 'mock' = explicit demo/dev;
  // 'none' = honest configuration error, no silent mock.
  const mode: ResolvedClientMode = clientInstance.resolveMode();
  let contractBusy = $state(false);
  let contractError = $state<string | null>(null);
  let modPath = $state('');
  let contractRecents = $state<ProjectSummaryDto[]>([]);
  let picking = $state(false);

  async function createFromPath() {
    if (contractBusy || !modPath.trim()) return;
    contractBusy = true;
    contractError = null;
    const ok = await project.createContractProject(modPath.trim());
    contractBusy = false;
    if (ok) router.navigate('workspace');
    else contractError = project.contractError;
  }

  // Native folder dialog (tauri mode only — the panel above is tauri-only).
  // Cancellation (null) is silent: closing the dialog is a normal outcome,
  // not a failure. A REAL failure surfaces verbatim in the shared alert.
  async function pickModFolder() {
    if (picking || contractBusy) return;
    picking = true;
    contractError = null;
    try {
      const dir = await clientInstance.getClient().pickDirectory(modPath.trim() || undefined);
      if (dir) modPath = dir;
    } catch (e) {
      contractError = e instanceof Error ? e.message : String(e);
    } finally {
      picking = false;
    }
  }

  async function openContract(id: string) {
    if (contractBusy) return;
    contractBusy = true;
    contractError = null;
    const ok = await project.openContractProject(id);
    contractBusy = false;
    if (ok) router.navigate('workspace');
    else contractError = project.contractError;
  }

  // M-1 (UI audit 2026-09-29): look-alike recents ("RimLoc UI (en)" ×7 +
  // a raw numeric name) become distinguishable — target version, revision
  // and the short id suffix straight from the summary DTO, no new contract
  // fields. Data notation (v/r/#) is locale-independent.
  function recentMeta(rp: ProjectSummaryDto): string {
    const parts: string[] = [];
    if (rp.target_version) parts.push(`v${rp.target_version}`);
    parts.push(`r${rp.revision}`);
    parts.push(`#${rp.project_id.slice(-6)}`);
    return parts.join(' · ');
  }

  // Selfloc entry (mandate D): the flow itself lives in lib/selfloc.ts —
  // shared with the Help screen card (wave 5), one implementation, the dedup
  // by 'RimLoc UI (en)' included. Home owns only the busy/error UI state;
  // every failure surfaces verbatim in the section alert; in mock mode the
  // typed refusal IS the honest outcome (no fake catalog dir, no fake
  // success, no dead-end create).
  let selflocBusy = $state(false);
  let selflocError = $state<string | null>(null);

  async function openSelfloc() {
    if (contractBusy || selflocBusy) return;
    selflocBusy = true;
    selflocError = null;
    const res = await openSelflocProject();
    selflocBusy = false;
    if (!res.ok) selflocError = res.error;
  }

  $effect(() => {
    if (mode !== 'tauri') return;
    // Recent real projects for the returning-user view.
    project.listContractProjects().then((list) => (contractRecents = list));
  });

  const firstRun = $derived(ui.homeMode === 'first-run');
  const noMods = $derived(ui.homeMode === 'no-mods');

  // Honest mock note for the folder picker: in this scaffold there is no OS
  // dialog and no backend — the button refuses to pretend otherwise.
  let folderNote = $state(false);

  function openDemo() {
    demoProject.seed();
    router.navigate('workspace');
  }

  // --- Recents presentation helpers (fixtures are module constants) ---

  // Featured = the most recent mock project; the table lists the REST, so
  // the hero "continue" card and the recents table never duplicate a row.
  const featured: RecentProject | undefined = mockProjects[0];
  const restProjects: RecentProject[] = mockProjects.slice(1);

  // Hero lifecycle pill: working/ready — issue and source-change counts sit
  // in their own warn/bad pills next to it.
  const heroStatus = featured && featured.progress >= 100 ? 'ready' : 'working';

  type HomeProjectStatus = 'working' | 'ready' | 'problems';

  // Row status vocabulary: marker pattern (color chip + icon) + label + note.
  function statusOf(p: RecentProject): HomeProjectStatus {
    if (p.issues > 0) return 'problems';
    if (p.progress >= 100) return 'ready';
    return 'working';
  }

  function statusIcon(s: HomeProjectStatus): string {
    return s === 'ready' ? 'circle-check' : s === 'problems' ? 'warning' : 'edit';
  }

  function statusLabel(s: HomeProjectStatus): string {
    if (s === 'ready') return t('home.status.ready');
    if (s === 'problems') return t('home.status.problems');
    return t('home.status.working');
  }

  // The note under a row pill: the next concrete step with its number.
  function rowNote(p: RecentProject): string {
    if (p.issues > 0) return t('home.recent.issues', { count: p.issues });
    if (p.sourceChanged > 0) return t('home.recent.sourceChanged', { count: p.sourceChanged });
    return linesNote(p) ?? '';
  }

  // Line counters ("1086 of 1248 lines translated"), only when the fixture
  // carries them — never invented for data that has none.
  function linesNote(p: RecentProject): string | null {
    if (p.linesDone === undefined || p.linesTotal === undefined) return null;
    const done = t('home.continue.linesDone', {
      done: fmtNum(p.linesDone),
      total: fmtNum(p.linesTotal)
    });
    const left = t('home.continue.linesLeft', { count: fmtNum(p.linesTotal - p.linesDone) });
    return `${done} · ${left}`;
  }

  function fmtNum(n: number): string {
    try {
      return new Intl.NumberFormat(i18n.locale === 'ru' ? 'ru-RU' : 'en-US').format(n);
    } catch {
      return String(n);
    }
  }

  // Human dates ("modified today at 14:32", "yesterday at 21:05", else a
  // medium calendar date) — locale-aware via Intl.
  function relDate(iso: string): string {
    const tag = i18n.locale === 'ru' ? 'ru-RU' : 'en-US';
    try {
      const d = new Date(iso);
      const now = new Date();
      const yesterday = new Date(now);
      yesterday.setDate(now.getDate() - 1);
      const sameDay = (a: Date, b: Date) =>
        a.getFullYear() === b.getFullYear() &&
        a.getMonth() === b.getMonth() &&
        a.getDate() === b.getDate();
      const time = new Intl.DateTimeFormat(tag, { hour: '2-digit', minute: '2-digit' }).format(d);
      if (sameDay(d, now)) return t('home.recent.when.today', { time });
      if (sameDay(d, yesterday)) return t('home.recent.when.yesterday', { time });
      return new Intl.DateTimeFormat(tag, { dateStyle: 'medium' }).format(d);
    } catch {
      return iso;
    }
  }
</script>

<section class="home" aria-labelledby="home-heading">
  <header class="page-head">
    <h1 id="home-heading" class="home-title">{t('home.title')}</h1>
    {#if firstRun}
      <p class="lead">{t('home.lead')}</p>
    {:else}
      <!-- Trust line (design-system statusbar): files are never mutated. -->
      <p class="lead">
        <span class="trust-ic"><Icon name="shield" size={14} /></span>
        {t('home.locals')}
      </p>
    {/if}
  </header>

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
        {#if mode !== 'tauri'}
          <button type="button" class="btn btn-primary" data-testid="home.nomods.demo" onclick={openDemo}>
            <Icon name="play" size={14} />
            {t('home.nomods.demo')}
          </button>
        {/if}
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
  {:else if mode === 'tauri'}
    <!-- W-built: the REAL client surface — reopen a recent project (main
         column) or create one from a mod folder (side column). Demo fixtures
         stay behind the explicit dev/demo mode. -->
    <div class="grid">
      <div class="col-main">
        <section class="card" aria-labelledby="recents-heading" data-testid="home.recent">
          <div class="list-head">
            <h2 id="recents-heading" class="card-title">{t('home.recent.title')}</h2>
          </div>
          {#if contractRecents.length === 0}
            <p class="recents-empty">{t('home.recent.empty')}</p>
          {:else}
            <ul class="contract-list">
              {#each contractRecents as rp (rp.project_id)}
                <li class="contract-recent">
                  <span class="contract-main">
                    <span class="name">{rp.name}</span>
                    <span class="rev mono">{recentMeta(rp)}</span>
                  </span>
                  <button
                    type="button"
                    class="btn-ghost"
                    data-testid={`home.contract.open.${rp.project_id}`}
                    disabled={contractBusy}
                    onclick={() => openContract(rp.project_id)}
                  >
                    {t('home.contract.open')}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      </div>

      <div class="col-side">
        <section class="card create" aria-labelledby="contract-heading" data-testid="home.contract">
          <h2 id="contract-heading" class="card-title">{t('home.contract.title')}</h2>
          <p class="card-desc">{t('home.contract.desc')}</p>
          <div class="field">
            <label class="field-label" for="home-mod-path">{t('home.contract.pathLabel')}</label>
            <div class="path-row">
              <input
                id="home-mod-path"
                class="path-input"
                type="text"
                placeholder={t('home.contract.pathPlaceholder')}
                aria-describedby="home-mod-help"
                autocomplete="off"
                spellcheck="false"
                data-testid="home.contract.path"
                bind:value={modPath}
                disabled={contractBusy}
              />
              <button
                type="button"
                class="btn"
                data-testid="home.contract.pick"
                disabled={contractBusy || picking}
                onclick={pickModFolder}
              >
                <Icon name="folder-open" size={14} />
                {t('home.contract.pick')}
              </button>
            </div>
            <!-- Path hint as inline-code chips (minimalism lens): About /
                 Languages / example path stay literal data. -->
            <p class="helper" id="home-mod-help">
              {t('home.contract.helperLead')}
              <code>About</code>
              {t('home.contract.helperMid')}
              <code>Languages</code>
              {t('home.contract.helperTail')}
              <code>{t('home.contract.helperExample')}</code>
            </p>
          </div>
          <button
            type="button"
            class="btn btn-primary create-btn"
            data-testid="home.contract.create"
            disabled={contractBusy || !modPath.trim()}
            onclick={createFromPath}
          >
            <Icon name="file-plus" size={14} />
            {t('home.contract.create')}
          </button>
          {#if contractError}
            <p class="note-error" role="alert" data-testid="home.contract.error">{contractError}</p>
          {/if}
          <!-- H-1 (UI audit 2026-09-29): the seven-step wizard is a real,
               implemented screen that was unreachable on the live Home. This
               secondary entry is honestly labeled a DEMO tour — the wizard
               walks the bundled demo dataset; the live create flow above
               stays the only way to make a real project. -->
          <button
            type="button"
            class="btn tour-btn"
            data-testid="home.contract.wizardTour"
            onclick={() => router.navigate('wizard')}
          >
            <Icon name="play" size={14} />
            {t('home.wizard.tour')}
          </button>
          <p class="helper tour-note">{t('home.wizard.tourNote')}</p>
        </section>

        <!-- Selfloc entry (mandate D, first wave): the app's own UI catalog
             as an ordinary project. Visible in BOTH modes — in mock the click
             surfaces the honest typed refusal instead of pretending a catalog
             exists. -->
        <section class="card flag" aria-labelledby="selfloc-heading" data-testid="home.selfloc">
          <div class="flag-head">
            <span class="flag-ic"><Icon name="languages" size={18} /></span>
            <h2 id="selfloc-heading" class="flag-title">{t('home.selfloc.title')}</h2>
            <span class="mark">{t('home.selfloc.beta')}</span>
          </div>
          <p class="flag-desc">{t('home.selfloc.desc')}</p>
          <div class="flag-actions">
            <button
              type="button"
              class="btn"
              data-testid="home.selfloc.open"
              disabled={contractBusy || selflocBusy}
              onclick={openSelfloc}
            >
              <Icon name="book" size={14} />
              {t('home.selfloc.open')}
            </button>
          </div>
          {#if selflocError}
            <p class="note-error" role="alert" data-testid="home.selfloc.error">{selflocError}</p>
          {/if}
        </section>
      </div>
    </div>
  {:else if firstRun}
    <!-- First run: the two entry ways lead; hint and secondary links follow. -->
    <div class="grid grid-first">
      <div class="card create">
        <button
          type="button"
          class="entry-card"
          data-testid="home.entry-new"
          onclick={() => router.navigate('wizard')}
        >
          <span class="card-icon"><Icon name="file-plus" size={28} /></span>
          <span class="card-title">{t('home.entryNew.title')}</span>
          <span class="card-desc">{t('home.entryNew.description')}</span>
        </button>
        <div class="steps">
          <p class="steps-title">{t('home.create.stepsTitle')}</p>
          <ol>
            <li><span class="n" aria-hidden="true">1</span><span>{t('home.create.step1')}</span></li>
            <li><span class="n" aria-hidden="true">2</span><span>{t('home.create.step2')}</span></li>
            <li><span class="n" aria-hidden="true">3</span><span>{t('home.create.step3')}</span></li>
          </ol>
        </div>
      </div>

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

    <div class="firstrun-flags">
      <section class="card flag" aria-labelledby="selfloc-heading" data-testid="home.selfloc">
      <div class="flag-head">
        <span class="flag-ic"><Icon name="languages" size={18} /></span>
        <h2 id="selfloc-heading" class="flag-title">{t('home.selfloc.title')}</h2>
        <span class="mark">{t('home.selfloc.beta')}</span>
      </div>
      <p class="flag-desc">{t('home.selfloc.desc')}</p>
      <div class="flag-actions">
        <button
          type="button"
          class="btn"
          data-testid="home.selfloc.open"
          disabled={contractBusy || selflocBusy}
          onclick={openSelflocProject}
        >
          <Icon name="book" size={14} />
          {t('home.selfloc.open')}
        </button>
      </div>
      {#if selflocError}
        <p class="note-error" role="alert" data-testid="home.selfloc.error">{selflocError}</p>
      {/if}
    </section>

    <!-- W6 demo project (mandate §6/§8): bundled, synthetic, RimLoc-owned and
         clearly marked so it is never confused with a real recent project. -->
    <section class="card flag" aria-labelledby="demo-heading" data-testid="home.demo">
      <div class="flag-head">
        <span class="flag-ic"><Icon name="play" size={18} /></span>
        <h2 id="demo-heading" class="flag-title">{t('home.demo.title')}</h2>
        <span class="mark">{t('home.demo.mark')}</span>
      </div>
      <p class="flag-desc">{t('home.demo.desc')}</p>
      <div class="flag-actions">
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
    </div>
  {:else}
    <!-- Returning (mock): the featured project is the first content block;
         the table lists the REST — no duplicate "main" list (jury warning). -->
    <div class="grid">
      <div class="col-main">
        {#if featured}
          <section class="card hero" aria-labelledby="hero-heading" data-testid="home.recent.hero">
            <div class="kicker">
              <span id="hero-heading">{t('home.continue.kicker')}</span>
              <span class="when">{t('home.recent.modifiedWhen', { when: relDate(featured.modified) })}</span>
            </div>
            <h2 class="hero-name">{featured.name}</h2>
            <p class="hero-path mono">
              {featured.path}
              {#if featured.path}<span aria-hidden="true">·</span>{/if}
              {featured.source.toUpperCase()} → {featured.target.toUpperCase()}
            </p>

            <div class="bar-row">
              <div
                class="bar"
                role="progressbar"
                aria-label={t('home.recent.progress')}
                aria-valuenow={featured.progress}
                aria-valuemin={0}
                aria-valuemax={100}
              >
                <span class="bar-fill" style="width: {featured.progress}%"></span>
              </div>
              <span class="pct">{featured.progress}%</span>
            </div>
            {#if linesNote(featured)}
              <p class="bar-sub">{linesNote(featured)}</p>
            {/if}

            <div class="pills">
              <span class="pill pill-{heroStatus}">
                <Icon name={heroStatus === 'ready' ? 'circle-check' : 'edit'} size={12} />
                {heroStatus === 'ready' ? t('home.status.ready') : t('home.status.working')}
              </span>
              {#if featured.sourceChanged > 0}
                <span class="pill pill-warn" title={t('home.recent.sourceChangedTitle', { count: featured.sourceChanged })}>
                  <Icon name="clock" size={12} />
                  {t('home.recent.sourceChanged', { count: featured.sourceChanged })}
                </span>
              {/if}
              {#if featured.issues > 0}
                <span class="pill pill-bad" title={t('home.recent.issuesTitle', { count: featured.issues })}>
                  <Icon name="warning" size={12} />
                  {t('home.recent.issues', { count: featured.issues })}
                </span>
              {/if}
            </div>

            <div class="btn-row">
              <button
                type="button"
                class="btn btn-primary"
                data-testid={`home.recent-continue.${featured.id}`}
                onclick={() => router.navigate('workspace')}
              >
                {t('common.continue')}
                <Icon name="arrow-right" size={14} />
              </button>
              {#if featured.issues > 0}
                <button
                  type="button"
                  class="btn"
                  data-testid="home.continue.review"
                  onclick={() => router.navigate('review')}
                >
                  {t('home.continue.review')}
                  <span class="count">{featured.issues}</span>
                </button>
              {/if}
              <button
                type="button"
                class="btn"
                data-testid="home.continue.build"
                onclick={() => router.navigate('build')}
              >
                {t('home.continue.build')}
              </button>
            </div>
          </section>
        {/if}

        {#if restProjects.length > 0}
          <section class="card" aria-labelledby="recents-heading" data-testid="home.recent">
            <div class="list-head">
              <h2 id="recents-heading" class="card-title">{t('home.recent.title')}</h2>
              <span class="list-hint">
                {t('home.recent.count', { shown: restProjects.length, total: mockProjects.length })}
                · {t('home.recent.byDate')}
              </span>
            </div>
            <div class="table-scroll">
              <table class="projects">
                <thead>
                  <tr>
                    <th scope="col">{t('home.recent.colProject')}</th>
                    <th scope="col">{t('home.recent.colUpdated')}</th>
                    <th scope="col">{t('home.recent.colProgress')}</th>
                    <th scope="col">{t('home.recent.colStatus')}</th>
                    <th scope="col"><span class="visually-hidden">{t('home.recent.colActions')}</span></th>
                  </tr>
                </thead>
                <tbody>
                  {#each restProjects as p (p.id)}
                    {@const st = statusOf(p)}
                    <tr>
                      <td>
                        <span class="name">{p.name}</span>
                        {#if p.path}<span class="dir mono">{p.path}</span>{/if}
                      </td>
                      <td class="when">{t('home.recent.modifiedWhen', { when: relDate(p.modified) })}</td>
                      <td class="prog">
                        <div class="bar-row">
                          <div
                            class="bar"
                            role="progressbar"
                            aria-label={`${t('home.recent.progress')}: ${p.progress}%`}
                            aria-valuenow={p.progress}
                            aria-valuemin={0}
                            aria-valuemax={100}
                          >
                            <span class="bar-fill" style="width: {p.progress}%"></span>
                          </div>
                          <span class="pct">{p.progress}%</span>
                        </div>
                      </td>
                      <td>
                        <span class="pill pill-{st}">
                          <Icon name={statusIcon(st)} size={12} />
                          {statusLabel(st)}
                        </span>
                        {#if rowNote(p)}
                          <span class="status-note">{rowNote(p)}</span>
                        {/if}
                      </td>
                      <td class="action">
                        <button
                          type="button"
                          class="btn-ghost"
                          data-testid={`home.recent-continue.${p.id}`}
                          onclick={() => router.navigate('workspace')}
                        >
                          {t('home.recent.open')}
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </section>
        {/if}
      </div>

      <div class="col-side">
        <div class="card create">
          <button
            type="button"
            class="entry-card"
            data-testid="home.entry-new"
            onclick={() => router.navigate('wizard')}
          >
            <span class="card-icon"><Icon name="file-plus" size={28} /></span>
            <span class="card-title">{t('home.entryNew.title')}</span>
            <span class="card-desc">{t('home.entryNew.description')}</span>
          </button>
          <div class="steps">
            <p class="steps-title">{t('home.create.stepsTitle')}</p>
            <ol>
              <li><span class="n" aria-hidden="true">1</span><span>{t('home.create.step1')}</span></li>
              <li><span class="n" aria-hidden="true">2</span><span>{t('home.create.step2')}</span></li>
              <li><span class="n" aria-hidden="true">3</span><span>{t('home.create.step3')}</span></li>
            </ol>
          </div>
        </div>

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

        <section class="card flag" aria-labelledby="selfloc-heading" data-testid="home.selfloc">
          <div class="flag-head">
            <span class="flag-ic"><Icon name="languages" size={18} /></span>
            <h2 id="selfloc-heading" class="flag-title">{t('home.selfloc.title')}</h2>
            <span class="mark">{t('home.selfloc.beta')}</span>
          </div>
          <p class="flag-desc">{t('home.selfloc.desc')}</p>
          <div class="flag-actions">
            <button
              type="button"
              class="btn"
              data-testid="home.selfloc.open"
              disabled={contractBusy || selflocBusy}
              onclick={openSelfloc}
            >
              <Icon name="book" size={14} />
              {t('home.selfloc.open')}
            </button>
          </div>
          {#if selflocError}
            <p class="note-error" role="alert" data-testid="home.selfloc.error">{selflocError}</p>
          {/if}
        </section>

        <!-- W6 demo project (mandate §6/§8): bundled, synthetic, RimLoc-owned
             and clearly marked so it is never confused with a real recent
             project. State is isolated and deterministically resettable. -->
        <section class="card flag" aria-labelledby="demo-heading" data-testid="home.demo">
          <div class="flag-head">
            <span class="flag-ic"><Icon name="play" size={18} /></span>
            <h2 id="demo-heading" class="flag-title">{t('home.demo.title')}</h2>
            <span class="mark">{t('home.demo.mark')}</span>
          </div>
          <p class="flag-desc">{t('home.demo.desc')}</p>
          <div class="flag-actions">
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
      </div>
    </div>
  {/if}
</section>

<style>
  .home {
    width: min(1120px, 100%);
    margin: 0 auto;
    padding: var(--space-6) var(--space-4) var(--space-8);
    overflow-y: auto;
  }

  .page-head {
    margin: 0 0 var(--space-6);
  }

  .home-title {
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
    margin: 0 0 var(--space-1);
  }

  .lead {
    margin: 0;
    color: var(--color-muted-fg);
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .trust-ic {
    color: var(--color-success);
    display: inline-flex;
    flex: none;
  }

  .state-line {
    color: var(--color-muted-fg);
  }

  /* ---------- layout: main (continue + recents) / side (create + flags) --- */
  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 360px;
    gap: var(--space-4);
    align-items: start;
  }

  .grid-first {
    grid-template-columns: 1fr 1fr;
  }

  @media (max-width: 960px) {
    .grid,
    .grid-first {
      grid-template-columns: 1fr;
    }
  }

  .col-main,
  .col-side {
    display: grid;
    gap: var(--space-4);
    min-width: 0;
  }

  .card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
    color: var(--color-surface-fg);
  }

  .card-title {
    font-family: var(--font-heading);
    font-size: 16px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
    margin: 0;
  }

  .card-desc {
    color: var(--color-muted-fg);
    font-size: var(--text-base-size);
    margin: var(--space-1) 0 0;
  }

  .card-skeleton {
    background: var(--color-muted);
    border: none;
    min-height: 148px;
  }

  /* ---------- hero: «Продолжить работу» (first content block) ------------ */
  .kicker {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-muted-fg);
    margin-bottom: var(--space-2);
  }

  .kicker .when {
    font-weight: 500;
    letter-spacing: 0;
    text-transform: none;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .hero-name {
    font-family: var(--font-heading);
    font-size: 16px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
    margin: 0;
  }

  .hero-path {
    margin: 2px 0 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    overflow-wrap: anywhere;
  }

  .bar-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-top: var(--space-3);
  }

  .bar {
    flex: 1;
    height: 8px;
    border-radius: 999px;
    background: var(--color-muted);
    overflow: hidden;
  }

  .bar-fill {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--color-primary);
  }

  .pct {
    font-size: var(--text-dense-size);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .bar-sub {
    margin: var(--space-1) 0 0;
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
    font-variant-numeric: tabular-nums;
  }

  /* ---------- status pills: soft triples, marker + label (+ note) -------- */
  .pills {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 3px 10px;
    border-radius: 999px;
    border: 1px solid var(--color-border);
    font-size: var(--text-meta-size);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .pill :global(svg) {
    flex: none;
  }

  .pill-working {
    color: var(--color-chip-work-fg);
    background: var(--color-chip-work-bg);
    border-color: var(--color-chip-work-bd);
  }

  .pill-warn {
    color: var(--color-chip-warn-fg);
    background: var(--color-chip-warn-bg);
    border-color: var(--color-chip-warn-bd);
  }

  .pill-problems {
    color: var(--color-chip-bad-fg);
    background: var(--color-chip-bad-bg);
    border-color: var(--color-chip-bad-bd);
  }

  .pill-ready {
    color: var(--color-chip-ok-fg);
    background: var(--color-chip-ok-bg);
    border-color: var(--color-chip-ok-bd);
  }

  /* ---------- hero actions: Продолжить (solid) / Проверка / Собрать ------ */
  .btn-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-4);
  }

  .btn-row .btn {
    min-height: 38px;
    font-weight: 600;
  }

  .count {
    font-size: 11px;
    font-weight: 600;
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--color-chip-warn-bg);
    color: var(--color-chip-warn-fg);
    font-variant-numeric: tabular-nums;
  }

  /* ---------- recents: semantic table ------------------------------------ */
  .list-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-2);
    margin-bottom: var(--space-1);
  }

  .list-hint {
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .recents-empty {
    margin: var(--space-2) 0 0;
    color: var(--color-muted-fg);
  }

  .table-scroll {
    overflow-x: auto;
  }

  .projects {
    width: 100%;
    border-collapse: collapse;
    margin-top: var(--space-1);
  }

  .projects th {
    text-align: left;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--color-muted-fg);
    padding: var(--space-2);
    border-bottom: 1px solid var(--color-border);
    white-space: nowrap;
  }

  .projects td {
    padding: var(--space-3) var(--space-2);
    border-bottom: 1px solid var(--color-border);
    vertical-align: middle;
    font-size: var(--text-base-size);
  }

  .projects tbody tr {
    transition: background var(--motion-fast) var(--ease-out);
  }

  .projects tbody tr:hover {
    background: var(--color-muted);
  }

  .projects tbody tr:last-child td {
    border-bottom: 0;
  }

  .projects .name {
    font-weight: 600;
    display: block;
  }

  .projects .dir {
    display: block;
    margin-top: 2px;
    color: var(--color-muted-fg);
    font-size: 11px;
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .projects .when {
    color: var(--color-muted-fg);
    font-size: var(--text-dense-size);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .projects .prog {
    min-width: 168px;
  }

  .projects .prog .bar-row {
    margin-top: 0;
    gap: var(--space-2);
  }

  .projects .prog .pct {
    font-size: var(--text-meta-size);
  }

  .projects td.action {
    text-align: right;
  }

  .status-note {
    display: block;
    margin-top: 4px;
    font-size: 11px;
    color: var(--color-muted-fg);
    font-variant-numeric: tabular-nums;
  }

  /* Продолжить (hero, solid) vs Открыть (row, ghost) — one pattern each. */
  .btn-ghost {
    display: inline-flex;
    align-items: center;
    min-height: 29px;
    padding: 3px 12px;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-surface-fg);
    font-size: var(--text-dense-size);
    font-weight: 600;
    white-space: nowrap;
    transition: background var(--motion-fast) var(--ease-out);
  }

  .btn-ghost:hover {
    background: var(--color-muted);
  }

  .btn-ghost:disabled {
    opacity: 0.55;
    cursor: default;
  }

  /* ---------- entry cards (create / existing) + steps -------------------- */
  .entry-card {
    width: 100%;
    border: none;
    background: transparent;
    color: inherit;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    text-align: left;
    transition: color var(--motion-fast) var(--ease-out);
  }

  .create .entry-card:hover .card-title {
    color: var(--color-primary-text);
  }

  /* A bare entry button inside a plain .card keeps its own hover border. */
  button.card.entry-card {
    transition:
      border-color var(--motion-fast) var(--ease-out),
      box-shadow var(--motion-fast) var(--ease-out),
      transform var(--motion-fast) var(--ease-out);
  }

  button.card.entry-card:hover {
    border-color: var(--color-primary);
    box-shadow: var(--shadow-hover);
    transform: var(--card-hover-transform);
  }

  button.card.entry-card:active {
    transform: var(--btn-press-transform);
  }

  .card-icon {
    color: var(--card-icon-fg);
    background: var(--card-icon-bg);
    padding: var(--card-icon-pad);
    border-radius: var(--card-icon-radius);
    display: inline-flex;
    align-self: flex-start;
    margin-bottom: var(--space-2);
  }

  .entry-card .card-desc {
    font-size: var(--text-base-size);
  }

  .steps {
    margin-top: var(--space-3);
    padding-top: var(--space-3);
    border-top: 1px dashed var(--color-border);
  }

  .steps-title {
    margin: 0 0 var(--space-2);
    font-size: var(--text-meta-size);
    font-weight: 600;
    color: var(--color-muted-fg);
  }

  .steps ol {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-1);
  }

  .steps li {
    display: flex;
    gap: var(--space-2);
    align-items: baseline;
    font-size: var(--text-dense-size);
    color: var(--color-muted-fg);
    line-height: 1.5;
  }

  .steps .n {
    flex: none;
    width: 20px;
    height: 20px;
    border-radius: 999px;
    background: var(--color-chip-work-bg);
    color: var(--color-chip-work-fg);
    font-size: 11px;
    font-weight: 700;
    display: grid;
    place-items: center;
    transform: translateY(3px);
  }

  /* ---------- flagged cards (selfloc beta / demo) ------------------------- */
  .flag-head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .flag-ic {
    color: var(--color-primary-text);
    display: inline-flex;
    flex: none;
  }

  .flag-title {
    font-family: var(--font-heading);
    font-size: 15px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
    margin: 0;
  }

  .mark {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    padding: 1px 8px;
    border-radius: 999px;
    border: 1px solid var(--color-chip-warn-bd);
    background: var(--color-chip-warn-bg);
    color: var(--color-chip-warn-fg);
    white-space: nowrap;
  }

  .flag-desc {
    margin: var(--space-2) 0 0;
    font-size: var(--text-dense-size);
    line-height: 1.5;
    color: var(--color-muted-fg);
  }

  .flag-actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }

  /* ---------- create form (tauri) ----------------------------------------- */
  .field {
    margin-top: var(--space-3);
  }

  .field-label {
    display: block;
    font-size: var(--text-dense-size);
    font-weight: 600;
    margin-bottom: var(--space-1);
  }

  .path-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .path-input {
    flex: 1;
    min-width: 200px;
    min-height: var(--control-h);
    font-family: var(--font-mono);
    font-size: var(--text-dense-size);
    padding: 0 var(--space-2);
  }

  .path-input::placeholder {
    color: var(--color-muted-fg);
  }

  .helper {
    margin: var(--space-2) 0 0;
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
    line-height: 1.5;
  }

  .helper code {
    font-family: var(--font-mono);
    font-size: 11px;
    background: var(--color-muted);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 0 5px;
    overflow-wrap: anywhere;
  }

  .create-btn {
    width: 100%;
    justify-content: center;
    margin-top: var(--space-3);
    min-height: 38px;
    font-weight: 600;
  }

  /* H-1: the demo-tour wizard entry — secondary under the live create CTA. */
  .tour-btn {
    width: 100%;
    justify-content: center;
    margin-top: var(--space-2);
  }

  .tour-note {
    margin-top: var(--space-1);
  }

  /* ---------- contract recents (tauri) ------------------------------------ */
  .contract-list {
    list-style: none;
    margin: var(--space-1) 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .contract-recent {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    border-bottom: 1px solid var(--color-border);
    font-size: var(--text-dense-size);
  }

  .contract-recent:last-child {
    border-bottom: 0;
  }

  .contract-main {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .contract-main .name {
    font-weight: 600;
  }

  .contract-main .rev {
    color: var(--color-muted-fg);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }

  /* ---------- first-run extras / notes ------------------------------------ */
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

  .firstrun-flags {
    margin-top: var(--space-4);
    display: grid;
    gap: var(--space-4);
  }

  .note-error {
    margin: var(--space-2) 0 0;
    color: var(--color-error);
    font-size: var(--text-meta-size);
    border: 1px dashed var(--color-error);
    border-radius: var(--radius-sm);
    padding: var(--space-1) var(--space-2);
  }

  /* ---------- no-mods empty state ------------------------------------------ */
  .nomods {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    align-items: flex-start;
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

  /* ---------- error state --------------------------------------------------- */
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
