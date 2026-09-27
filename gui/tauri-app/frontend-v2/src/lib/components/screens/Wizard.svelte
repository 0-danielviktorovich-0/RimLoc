<script lang="ts">
  // Quick Translate wizard (mandate §4, QA mandate §2): seven clickable steps
  // over mocks only, with step 2 as a finite state machine — the selection
  // panel branches by the content type chosen on step 1: MOD → discovered
  // mods + folder/drop, BASE GAME → detected installations (Core/version),
  // DLC → installation + checkboxes of installed DLC, LANGUAGE PACK → pack
  // selection + language mapping. Selections live per branch, so going back
  // to the content step (or switching the type) never resets a made choice.
  // Step 5 numbers come from the mock store (mock/wizard.ts), not from the
  // editor corpus. Progress runs on a local interval; pause/cancel are real
  // for the simulation.
  import Icon from '../Icon.svelte';
  import { t, i18n } from '../../../i18n/store.svelte';
  import { router } from '../../router.svelte';
  import { devMode } from '../../stores/devmode.svelte';
  import {
    mockDlc,
    mockInstallations,
    mockLanguagePacks,
    mockMods,
    mockPreflight,
    mockResult,
    wizardPhases
  } from '../../mock/wizard';

  const STEPS = 7;

  type ContentKind = 'mod' | 'base' | 'dlc' | 'pack';
  // Strategy step (wave W3): the old mutually-exclusive "method" cards became
  // two blocks — what existing knowledge to use (checkboxes) and how to
  // translate the remainder (one choice). Manual editing is always available
  // afterwards and is no longer modeled as a project-wide exclusive mode.
  type Remaining = 'manual' | 'ai' | 'chat' | 'skip';
  type Quality = 'fast' | 'balanced' | 'max' | 'suggest';

  const LOCALES = [
    { id: 'ru', label: 'Русский' },
    { id: 'de', label: 'Deutsch' },
    { id: 'ja', label: '日本語' },
    { id: 'uk', label: 'Українська' },
    { id: 'pl', label: 'Polski' }
  ];

  let step = $state(1);
  let content = $state<ContentKind>('mod');

  // W6 scenario deep-links: `#/wizard?scenario=wizard/<branch>` preselects the
  // content branch exactly as a user click would. Dev-gated together with the
  // scenario runner — with the mode off, unknown/malicious deep-links are
  // ignored and the wizard starts from its default branch.
  $effect(() => {
    if (!devMode.enabled) return;
    const query = window.location.hash.split('?')[1];
    const scenario = query ? new URLSearchParams(query).get('scenario') : null;
    if (scenario === 'wizard/base-game') content = 'base';
    else if (scenario === 'wizard/dlc') content = 'dlc';
    else if (scenario === 'wizard/language-pack') content = 'pack';
  });

  // Selection state per content branch — preserved across branch switches and
  // back-navigation (QA mandate §2: "back does not reset the choice").
  let modId = $state<string | null>(mockMods[0].id);
  let installId = $state<string | null>(mockInstallations[0]?.id ?? null);
  let dlcIds = $state<string[]>([]);
  let packId = $state<string | null>(mockLanguagePacks[0]?.id ?? null);
  /** Per-pack language mapping (language the pack delivers), keyed by pack id. */
  let packTargets = $state<Record<string, string>>(
    Object.fromEntries(mockLanguagePacks.map((p) => [p.id, p.to]))
  );

  let targetLocale = $state('ru');

  // §7 audit: honest-stub note for the folder/drop actions — no OS dialog and
  // no drop handling exist in this build, so the click explains instead of
  // pretending.
  let folderStub = $state(false);

  // Strategy step state: existing-knowledge checkboxes default on; the
  // remainder choice defaults to manual — zero cost, nothing to confirm.
  let useTranslation = $state(true);
  let useTm = $state(true);
  let useGlossary = $state(true);
  let remaining = $state<Remaining>('manual');
  let quality = $state<Quality>('balanced');

  // Step 6 simulation
  let progress = $state(0);
  let paused = $state(false);
  let cancelled = $state(false);

  const selectedMod = $derived(mockMods.find((m) => m.id === modId) ?? null);
  const selectedInstall = $derived(
    mockInstallations.find((i) => i.id === installId) ?? null
  );
  const selectedDlc = $derived(
    mockDlc.filter((d) => d.installed && dlcIds.includes(d.id))
  );
  const selectedPack = $derived(
    mockLanguagePacks.find((p) => p.id === packId) ?? null
  );

  /** Whether the current branch has a valid selection for Next. */
  const branchReady = $derived.by(() => {
    switch (content) {
      case 'mod':
        return modId !== null;
      case 'base':
        return installId !== null;
      case 'dlc':
        return dlcIds.length > 0;
      case 'pack':
        return packId !== null;
    }
  });

  /** Human summary of the branch selection, shown on preflight (step 5). */
  const selectionSummary = $derived.by(() => {
    switch (content) {
      case 'mod':
        return selectedMod ? `${selectedMod.name} · v${selectedMod.version}` : null;
      case 'base':
        return selectedInstall
          ? `${selectedInstall.label} · Core v${selectedInstall.version}`
          : null;
      case 'dlc':
        return selectedDlc.length > 0 ? selectedDlc.map((d) => d.name).join(', ') : null;
      case 'pack':
        return selectedPack
          ? `${selectedPack.name} · ${selectedPack.from.toUpperCase()} → ${(
              packTargets[selectedPack.id] ?? selectedPack.to
            ).toUpperCase()}`
          : null;
    }
  });

  const currentPhase = $derived.by(() => {
    let acc = 0;
    for (const p of wizardPhases) {
      acc += p.weight;
      if (progress < acc) return p.id;
    }
    return 'validate';
  });
  const processed = $derived(Math.round((mockPreflight.entries * progress) / 100));
  const finished = $derived(progress >= 100);

  // Preflight counters recompute from the strategy: TM matches and the
  // existing-pack coverage only count when their checkbox is on; whatever is
  // left either gets translated by the chosen remainder path or stays
  // untranslated when the user opted out.
  const PACK_MATCHES = 243;
  const reusableNow = $derived((useTm ? mockPreflight.reusable : 0) + (useTranslation ? PACK_MATCHES : 0));
  const leftover = $derived(Math.max(0, mockPreflight.entries - reusableNow));
  const needNow = $derived(remaining === 'skip' ? 0 : leftover);
  const skippedNow = $derived(remaining === 'skip' ? leftover : 0);

  function fmt(n: number): string {
    return new Intl.NumberFormat(i18n.locale === 'ru' ? 'ru-RU' : 'en-US').format(n);
  }

  function toggleDlc(id: string) {
    const d = mockDlc.find((x) => x.id === id);
    if (!d || !d.installed) return;
    dlcIds = dlcIds.includes(id) ? dlcIds.filter((x) => x !== id) : [...dlcIds, id];
  }

  function go(dir: 1 | -1) {
    const next = step + dir;
    if (next < 1 || next > STEPS) return;
    // Guard: never advance out of the selection step without a branch choice.
    if (dir === 1 && step === 2 && !branchReady) return;
    step = next;
    if (step === 6 && !finished) {
      // Re-entering the run step restarts a cancelled simulation.
      if (cancelled) {
        progress = 0;
        cancelled = false;
      }
      paused = false;
    }
  }

  function cancelRun() {
    cancelled = true;
    paused = false;
    step = 5;
  }

  // Drive the mock pipeline while the user watches step 6.
  $effect(() => {
    if (step !== 6 || paused || finished) return;
    const id = setInterval(() => {
      progress = Math.min(100, progress + 2);
    }, 90);
    return () => clearInterval(id);
  });

  // Completed run hands over to the result step automatically.
  $effect(() => {
    if (step === 6 && finished) {
      const id = setTimeout(() => {
        step = 7;
      }, 700);
      return () => clearTimeout(id);
    }
  });

  function open(r: 'review' | 'workspace' | 'build' | 'chat') {
    router.navigate(r);
  }
</script>

<section class="wizard" aria-labelledby="wizard-heading">
  <header class="head">
    <h1 id="wizard-heading" class="title">{t('wizard.title')}</h1>
    <p class="step-of" data-testid="wizard.step-of" aria-live="polite">
      {t('wizard.stepOf', { step, total: STEPS })}
    </p>
    <div class="steps-track" aria-hidden="true">
      <div class="steps-fill" style="width: {((step - 1) / (STEPS - 1)) * 100}%"></div>
    </div>
  </header>

  {#if step === 1}
    <fieldset class="panel">
      <legend class="panel-title">{t('wizard.w1.title')}</legend>
      <p class="panel-desc">{t('wizard.w1.desc')}</p>
      <div class="option-grid">
        <button
          type="button"
          class="option"
          aria-pressed={content === 'mod'}
          data-testid="wizard.content.mod"
          onclick={() => (content = 'mod')}
        >
          <Icon name="package" size={20} />
          <span class="option-title">{t('wizard.w1.mod')}</span>
          <span class="option-desc">{t('wizard.w1.modDesc')}</span>
        </button>
        <button type="button" class="option" aria-pressed={content === 'base'} data-testid="wizard.content.base" onclick={() => (content = 'base')}>
          <Icon name="database" size={20} />
          <span class="option-title">{t('wizard.w1.base')}</span>
        </button>
        <button type="button" class="option" aria-pressed={content === 'dlc'} data-testid="wizard.content.dlc" onclick={() => (content = 'dlc')}>
          <Icon name="sparkles" size={20} />
          <span class="option-title">{t('wizard.w1.dlc')}</span>
        </button>
        <button type="button" class="option" aria-pressed={content === 'pack'} data-testid="wizard.content.pack" onclick={() => (content = 'pack')}>
          <Icon name="languages" size={20} />
          <span class="option-title">{t('wizard.w1.pack')}</span>
        </button>
      </div>
    </fieldset>
  {:else if step === 2}
    <fieldset class="panel" data-testid={`wizard.step2.${content}`}>
      <legend class="panel-title">{t('wizard.w2.title')}</legend>
      <p class="panel-desc">{t(`wizard.w2.subtitle.${content}`)}</p>

      {#if content === 'mod'}
        <!-- MOD branch: discovered mods + folder/drop (mandate §14). -->
        <ul class="mod-list" role="radiogroup" aria-label={t('wizard.w2.subtitle.mod')}>
          {#each mockMods as m (m.id)}
            <li>
              <button
                type="button"
                class="mod"
                role="radio"
                aria-checked={modId === m.id}
                data-testid={`wizard.mod.${m.id}`}
                onclick={() => (modId = m.id)}
              >
                <span class="mod-name">{m.name}</span>
                <span class="mod-meta">{t('wizard.w2.author')} {m.author} · v{m.version} · {fmt(m.defs)} {t('wizard.w2.defs')}</span>
              </button>
            </li>
          {/each}
        </ul>
        <div class="dropzone">
          <!-- §7 audit: this build has no OS folder dialog and no drop
               handling — the button used to be dead and the hint promised a
               drag that never worked. Honest stub: click explains instead of
               pretending (mirrors the Home no-mods folder note). -->
          <button
            type="button"
            class="btn"
            data-testid="wizard.choose-folder"
            aria-expanded={folderStub}
            onclick={() => (folderStub = true)}
          >
            <Icon name="folder-open" size={14} />
            {t('wizard.w2.folder')}
          </button>
          <span class="drop-hint">{t('wizard.w2.drop')}</span>
        </div>
        {#if folderStub}
          <p class="note" role="note" data-testid="wizard.folder-stub">{t('wizard.w2.folderStub')}</p>
        {/if}
      {:else if content === 'base'}
        <!-- BASE GAME branch: detected installations, Core + version. -->
        <ul class="mod-list" role="radiogroup" aria-label={t('wizard.w2.subtitle.base')}>
          {#each mockInstallations as inst (inst.id)}
            <li>
              <button
                type="button"
                class="mod"
                role="radio"
                aria-checked={installId === inst.id}
                data-testid={`wizard.install.${inst.id}`}
                onclick={() => (installId = inst.id)}
              >
                <span class="mod-name"><Icon name="database" size={14} /> {inst.label}</span>
                <span class="mod-meta">Core v{inst.version} · <span class="mono">{inst.path}</span></span>
              </button>
            </li>
          {/each}
        </ul>
      {:else if content === 'dlc'}
        <!-- DLC branch: installation first, then installed-DLC checkboxes. -->
        <div class="sub-field">
          <span class="field-label">{t('wizard.w2.installation')}</span>
          <ul class="mod-list" role="radiogroup" aria-label={t('wizard.w2.installation')}>
            {#each mockInstallations as inst (inst.id)}
              <li>
                <button
                  type="button"
                  class="mod"
                  role="radio"
                  aria-checked={installId === inst.id}
                  data-testid={`wizard.install.${inst.id}`}
                  onclick={() => (installId = inst.id)}
                >
                  <span class="mod-name"><Icon name="database" size={14} /> {inst.label}</span>
                  <span class="mod-meta">Core v{inst.version}</span>
                </button>
              </li>
            {/each}
          </ul>
        </div>
        <ul class="mod-list" role="group" aria-label={t('wizard.w1.dlc')}>
          {#each mockDlc as d (d.id)}
            <li>
              <label class="mod dlc-row" class:disabled={!d.installed}>
                <input
                  type="checkbox"
                  checked={dlcIds.includes(d.id)}
                  disabled={!d.installed}
                  data-testid={`wizard.dlc.${d.id}`}
                  onchange={() => toggleDlc(d.id)}
                />
                <span class="mod-name"><Icon name="layers" size={14} /> {d.name}</span>
                <span class="mod-meta">
                  v{d.version}{#if !d.installed} · {t('wizard.w2.dlc.notInstalled')}{/if}
                </span>
                {#if dlcIds.includes(d.id)}
                  <span class="dlc-check"><Icon name="check" size={14} /></span>
                {/if}
              </label>
            </li>
          {/each}
        </ul>
        <p class="note" data-testid="wizard.dlc-count">
          {#if dlcIds.length > 0}
            {t('wizard.w2.dlc.selected', { count: dlcIds.length })}
          {:else}
            {t('wizard.w2.dlc.needOne')}
          {/if}
        </p>
      {:else}
        <!-- LANGUAGE PACK branch: pack selection + language/source mapping. -->
        <ul class="mod-list" role="radiogroup" aria-label={t('wizard.w2.subtitle.pack')}>
          {#each mockLanguagePacks as p (p.id)}
            <li>
              <button
                type="button"
                class="mod"
                role="radio"
                aria-checked={packId === p.id}
                data-testid={`wizard.pack.${p.id}`}
                onclick={() => (packId = p.id)}
              >
                <span class="mod-name">
                  <Icon name="languages" size={14} /> {p.name}
                  <span class="origin" data-testid={`wizard.pack-origin.${p.id}`}>{t(`wizard.w2.pack.origin.${p.origin}`)}</span>
                </span>
                <span class="mod-meta">{fmt(p.entries)} {t('wizard.w2.defs')} · v{p.version}</span>
              </button>
            </li>
          {/each}
        </ul>
        {#if selectedPack}
          <div class="mapping" data-testid="wizard.pack-mapping">
            <span class="field-label">{t('wizard.w2.pack.mapping')}</span>
            <div class="lang-grid">
              <label class="field">
                <span class="field-label">
                  {t('wizard.w2.pack.from')}
                  <span class="auto">{t('wizard.w3.autoDetected')}</span>
                </span>
                <select value={selectedPack.from} disabled data-testid="wizard.pack-from">
                  <option value={selectedPack.from}>English</option>
                </select>
              </label>
              <label class="field">
                <span class="field-label">{t('wizard.w2.pack.to')}</span>
                <select
                  value={packTargets[selectedPack.id] ?? selectedPack.to}
                  onchange={(e) => {
                    if (selectedPack) packTargets[selectedPack.id] = (e.currentTarget as HTMLSelectElement).value;
                  }}
                  data-testid="wizard.pack-to"
                >
                  {#each LOCALES as l (l.id)}
                    <option value={l.id}>{l.label}</option>
                  {/each}
                </select>
              </label>
            </div>
            <p class="note">
              {t('wizard.w2.pack.entries', { count: fmt(selectedPack.entries) })}
            </p>
          </div>
        {/if}
      {/if}
    </fieldset>
  {:else if step === 3}
    <fieldset class="panel">
      <legend class="panel-title">{t('wizard.w3.title')}</legend>
      <div class="lang-grid">
        <label class="field">
          <span class="field-label">
            {t('wizard.w3.source')}
            <span class="auto">{t('wizard.w3.autoDetected')}</span>
          </span>
          <select value="en" disabled data-testid="wizard.source-lang">
            <option value="en">English</option>
          </select>
        </label>
        <label class="field">
          <span class="field-label">{t('wizard.w3.target')}</span>
          <select bind:value={targetLocale} data-testid="wizard.target-lang">
            {#each LOCALES as l (l.id)}
              <option value={l.id}>{l.label}</option>
            {/each}
          </select>
        </label>
      </div>
      <p class="note">{t('wizard.w3.note')}</p>
    </fieldset>
  {:else if step === 4}
    <fieldset class="panel">
      <legend class="panel-title">{t('wizard.w4.title')}</legend>
      <p class="panel-desc">{t('wizard.w4.desc')}</p>

      <!-- Block A: existing knowledge — checkboxes, freely combinable. -->
      <div class="strategy-block" role="group" aria-labelledby="wizard-existing-title">
        <span class="block-title" id="wizard-existing-title">{t('wizard.w4.existing.title')}</span>
        <ul class="check-list">
          <li>
            <label class="check-row">
              <input
                type="checkbox"
                bind:checked={useTranslation}
                data-testid="wizard.existing.translation"
              />
              <span class="check-name">{t('wizard.w4.existing.translation')}</span>
              <span class="check-desc">{t('wizard.w4.existing.translationDesc')}</span>
            </label>
          </li>
          <li>
            <label class="check-row">
              <input type="checkbox" bind:checked={useTm} data-testid="wizard.existing.tm" />
              <span class="check-name">{t('wizard.w4.existing.tm')}</span>
              <span class="check-desc">{t('wizard.w4.existing.tmDesc')}</span>
            </label>
          </li>
          <li>
            <label class="check-row">
              <input type="checkbox" bind:checked={useGlossary} data-testid="wizard.existing.glossary" />
              <span class="check-name">{t('wizard.w4.existing.glossary')}</span>
              <span class="check-desc">{t('wizard.w4.existing.glossaryDesc')}</span>
            </label>
          </li>
        </ul>
      </div>

      <!-- Block B: how to translate the remainder — one choice. -->
      <div class="strategy-block" role="group" aria-labelledby="wizard-remaining-title">
        <span class="block-title" id="wizard-remaining-title">{t('wizard.w4.remaining.title')}</span>
        <div class="option-grid two" role="radiogroup" aria-label={t('wizard.w4.remaining.title')}>
          {#each [['manual', 'edit'], ['ai', 'cpu'], ['chat', 'external'], ['skip', 'clock']] as const as [m, icon] (m)}
            <button
              type="button"
              class="option"
              role="radio"
              aria-checked={remaining === m}
              data-testid={`wizard.remaining.${m}`}
              onclick={() => (remaining = m)}
            >
              <Icon name={icon} size={18} />
              <span class="option-title">{t(`wizard.w4.remaining.${m}`)}</span>
              <span class="option-desc">{t(`wizard.w4.remaining.${m}Desc`)}</span>
            </button>
          {/each}
        </div>
      </div>

      <p class="note strategy-note" data-testid="wizard.strategy-note">
        {t('wizard.w4.editNote')} {t('wizard.w4.changeable')}
      </p>

      <label class="field">
        <span class="field-label">{t('wizard.w4.quality')}</span>
        <select bind:value={quality} data-testid="wizard.quality">
          <option value="fast">{t('wizard.w4.quality.fast')}</option>
          <option value="balanced">{t('wizard.w4.quality.balanced')}</option>
          <option value="max">{t('wizard.w4.quality.max')}</option>
          <option value="suggest">{t('wizard.w4.quality.suggest')}</option>
        </select>
      </label>
      {#if remaining === 'ai'}
        <p class="note cost" data-testid="wizard.cost-note">{t('wizard.w4.cost')}</p>
      {:else if remaining === 'chat'}
        <p class="note" data-testid="wizard.chat-note">{t('wizard.w4.chatNote')}</p>
      {/if}
    </fieldset>
  {:else if step === 5}
    <fieldset class="panel">
      <legend class="panel-title">{t('wizard.w5.title')}</legend>
      <dl class="preflight" data-testid="wizard.preflight">
        <div class="pf-row">
          <dt>{t('wizard.w5.entries')}</dt>
          <dd class="mono">{fmt(mockPreflight.entries)}</dd>
        </div>
        <div class="pf-row">
          <dt>{t('wizard.w5.reusable')}</dt>
          <dd class="mono">{fmt(reusableNow)}</dd>
        </div>
        <div class="pf-row">
          <dt>{t('wizard.w5.need')}</dt>
          <dd class="mono">{fmt(needNow)}</dd>
        </div>
        {#if skippedNow > 0}
          <div class="pf-row">
            <dt>{t('wizard.w5.skipped')}</dt>
            <dd class="mono warn">{fmt(skippedNow)}</dd>
          </div>
        {/if}
        <div class="pf-row">
          <dt>{t('wizard.w5.attention')}</dt>
          <dd class="mono warn">{fmt(mockPreflight.attention)}</dd>
        </div>
      </dl>
      {#if selectionSummary}
        <p class="note" data-testid="wizard.selection-summary">
          {t('wizard.w2.selected.summary', { name: selectionSummary })}
        </p>
      {/if}
      <p class="note">{t('wizard.w5.note')}</p>
    </fieldset>
  {:else if step === 6}
    <fieldset class="panel" data-testid="wizard.progress">
      <legend class="panel-title">{t('wizard.w6.title')}</legend>
      <p class="phase" role="status" aria-live="polite">
        {#if paused}
          <Icon name="pause" size={14} />
          {t('wizard.w6.paused')} ·
        {:else}
          <Icon name="play" size={14} />
        {/if}
        {t(`wizard.w6.phase.${currentPhase}`)}
      </p>
      <div
        class="bar"
        role="progressbar"
        aria-label={t('wizard.w6.title')}
        aria-valuenow={progress}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div class="bar-fill" style="width: {progress}%"></div>
      </div>
      <p class="processed">
        {t('wizard.w6.processed', { done: fmt(processed), total: fmt(mockPreflight.entries) })}
      </p>
      <div class="run-actions">
        <button type="button" class="btn" data-testid="wizard.pause" onclick={() => (paused = !paused)}>
          {#if paused}
            <Icon name="play" size={14} />
            {t('common.resume')}
          {:else}
            <Icon name="pause" size={14} />
            {t('common.pause')}
          {/if}
        </button>
        <button type="button" class="btn" data-testid="wizard.cancel" onclick={cancelRun}>
          <Icon name="square" size={14} />
          {t('common.cancel')}
        </button>
      </div>
    </fieldset>
  {:else}
    <fieldset class="panel" data-testid="wizard.result">
      <legend class="panel-title">{t('wizard.w7.title')}</legend>
      <dl class="preflight result">
        <div class="pf-row">
          <dt>{t('wizard.w7.validated')}</dt>
          <dd class="mono ok">{fmt(mockResult.validated)}</dd>
        </div>
        <div class="pf-row">
          <dt>{t('wizard.w7.review')}</dt>
          <dd class="mono warn">{fmt(mockResult.review)}</dd>
        </div>
        <div class="pf-row">
          <dt>{t('wizard.w7.errors')}</dt>
          <dd class="mono err">{fmt(mockResult.errors)}</dd>
        </div>
      </dl>
      <p class="note">{t('wizard.w7.note')}</p>
      <div class="result-actions">
        <button type="button" class="btn" data-testid="wizard.result-review" onclick={() => open('review')}>
          <Icon name="clipboard-check" size={14} />
          {t('wizard.w7.reviewProblems')}
        </button>
        <button type="button" class="btn btn-primary" data-testid="wizard.result-editor" onclick={() => open('workspace')}>
          <Icon name="edit" size={14} />
          {t('wizard.w7.openEditor')}
        </button>
        {#if remaining === 'chat'}
          <button type="button" class="btn" data-testid="wizard.result-chat" onclick={() => open('chat')}>
            <Icon name="external" size={14} />
            {t('wizard.w7.openChat')}
          </button>
        {/if}
        <button type="button" class="btn" data-testid="wizard.result-build" onclick={() => open('build')}>
          <Icon name="package" size={14} />
          {t('wizard.w7.build')}
        </button>
      </div>
    </fieldset>
  {/if}

  {#if step < 6}
    <footer class="nav-row">
      <button
        type="button"
        class="btn"
        data-testid="wizard.back"
        disabled={step === 1}
        onclick={() => go(-1)}
      >
        <Icon name="arrow-left" size={14} />
        {t('common.back')}
      </button>
      {#if step === 5}
        <button type="button" class="btn btn-primary" data-testid="wizard.start" onclick={() => go(1)}>
          <Icon name="play" size={14} />
          {t('common.startTranslation')}
        </button>
      {:else}
        <button
          type="button"
          class="btn btn-primary"
          data-testid="wizard.next"
          disabled={step === 2 && !branchReady}
          onclick={() => go(1)}
        >
          {t('common.next')}
          <Icon name="arrow-right" size={14} />
        </button>
      {/if}
    </footer>
  {/if}
</section>

<style>
  .wizard {
    width: min(720px, 100%);
    margin: 0 auto;
    padding: var(--space-6) var(--space-4) var(--space-8);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .head {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
  }

  .step-of {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .steps-track {
    height: 4px;
    border-radius: var(--radius-sm);
    background: var(--color-muted);
    overflow: hidden;
  }

  .steps-fill {
    height: 100%;
    background: var(--color-primary);
    border-radius: var(--radius-sm);
    transition: width var(--motion-standard) var(--ease-out);
  }

  .panel {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    padding: var(--space-4) var(--space-6) var(--space-6);
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .panel-title {
    font-family: var(--font-heading);
    font-size: 16px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
    padding: 0 var(--space-2);
  }

  .panel-desc {
    margin: 0;
    color: var(--color-muted-fg);
  }

  .option-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--space-2);
  }

  .option-grid.two {
    grid-template-columns: 1fr 1fr;
  }

  @media (max-width: 720px) {
    .option-grid,
    .option-grid.two {
      grid-template-columns: 1fr;
    }
  }

  .option {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-1);
    padding: var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    text-align: left;
    transition:
      border-color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out);
  }

  .option:hover {
    border-color: var(--color-border-strong);
    background: var(--color-muted);
  }

  .option[aria-pressed='true'] {
    border-color: var(--color-primary);
    background: var(--nav-active-bg);
  }

  .option-title {
    font-weight: 600;
  }

  .option-desc {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .mod-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .mod {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    text-align: left;
    transition:
      border-color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out);
  }

  .mod:hover {
    background: var(--color-muted);
  }

  .mod[aria-checked='true'] {
    border-color: var(--color-primary);
    background: var(--nav-active-bg);
  }

  .mod-name {
    font-weight: 600;
  }

  .mod-meta {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .dropzone {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-3);
  }

  .drop-hint {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  /* Content-branch extras: DLC checkbox rows and pack mapping. */
  .dlc-row {
    flex-direction: row;
    align-items: center;
    gap: var(--space-2);
  }

  .dlc-row input[type='checkbox'] {
    width: 15px;
    height: 15px;
    accent-color: var(--color-primary);
    flex: none;
  }

  .dlc-row.disabled {
    opacity: 0.55;
  }

  .dlc-check {
    margin-left: auto;
    color: var(--color-primary);
    display: inline-flex;
  }

  .mod-name {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .origin {
    margin-left: var(--space-2);
    padding: 1px var(--space-1);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    font-weight: 400;
  }

  .sub-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .mapping {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    border-top: 1px solid var(--color-border);
    padding-top: var(--space-3);
  }

  .lang-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .field-label {
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .auto {
    color: var(--color-status-translated);
  }

  select:disabled {
    color: var(--color-muted-fg);
  }

  .note {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .note.cost {
    color: var(--color-warning);
  }

  /* Strategy step: two composed blocks instead of exclusive method cards. */
  .strategy-block {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
  }

  .block-title {
    font-weight: 600;
  }

  .check-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .check-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--control-h);
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    cursor: pointer;
    transition: border-color var(--motion-fast) var(--ease-out);
  }

  .check-row:hover {
    border-color: var(--color-border-strong);
  }

  .check-row input[type='checkbox'] {
    width: 15px;
    height: 15px;
    accent-color: var(--color-primary);
    flex: none;
  }

  .check-name {
    font-weight: 600;
    white-space: nowrap;
  }

  .check-desc {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    min-width: 0;
  }

  .strategy-note {
    border-top: 1px solid var(--color-border);
    padding-top: var(--space-2);
  }

  .preflight {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .pf-row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: var(--space-3);
    padding: var(--space-1) 0;
    border-bottom: 1px solid var(--color-border);
    font-size: var(--text-base-size);
  }

  .pf-row:last-child {
    border-bottom: none;
  }

  .pf-row dt {
    color: var(--color-muted-fg);
  }

  .pf-row dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }

  .mono.warn {
    color: var(--color-warning);
  }

  .mono.ok {
    color: var(--color-success);
  }

  .mono.err {
    color: var(--color-destructive);
  }

  .phase {
    margin: 0;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-weight: 600;
  }

  .bar {
    height: 8px;
    border-radius: var(--radius-sm);
    background: var(--color-muted);
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    background: var(--color-primary);
    border-radius: var(--radius-sm);
    transition: width var(--motion-standard) var(--ease-out);
  }

  .processed {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    font-variant-numeric: tabular-nums;
  }

  .run-actions,
  .result-actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .nav-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }
</style>
