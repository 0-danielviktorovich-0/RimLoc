<script lang="ts">
  // Quick Translate wizard (mandate §4, spec §14): seven clickable steps over
  // mocks only — content → select → languages → method → preflight → progress
  // → result. Step 5 numbers come from the mock store (mock/wizard.ts), not
  // from the editor corpus. Progress runs on a local interval; pause/cancel
  // are real for the simulation.
  import Icon from '../Icon.svelte';
  import { t, i18n } from '../../../i18n/store.svelte';
  import { router } from '../../router.svelte';
  import {
    mockMods,
    mockPreflight,
    mockResult,
    wizardPhases
  } from '../../mock/wizard';

  const STEPS = 7;

  type ContentKind = 'mod' | 'base' | 'dlc' | 'pack';
  type Method = 'manual' | 'tm' | 'ai' | 'external';
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
  let modId = $state<string | null>(mockMods[0].id);
  let targetLocale = $state('ru');
  let method = $state<Method>('tm');
  let quality = $state<Quality>('balanced');

  // Step 6 simulation
  let progress = $state(0);
  let paused = $state(false);
  let cancelled = $state(false);

  const selectedMod = $derived(mockMods.find((m) => m.id === modId) ?? null);
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

  function fmt(n: number): string {
    return new Intl.NumberFormat(i18n.locale === 'ru' ? 'ru-RU' : 'en-US').format(n);
  }

  function go(dir: 1 | -1) {
    const next = step + dir;
    if (next < 1 || next > STEPS) return;
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

  function open(r: 'review' | 'workspace' | 'build') {
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
    <fieldset class="panel">
      <legend class="panel-title">{t('wizard.w2.title')}</legend>
      <p class="panel-desc">{t('wizard.w2.detected')}</p>
      <ul class="mod-list" role="radiogroup" aria-label={t('wizard.w2.title')}>
        {#each mockMods as m (m.id)}
          <li>
            <button
              type="button"
              class="mod"
              role="radio"
              aria-checked={modId === m.id}
              data-testid={`wizard.mod.${m.id}`}
              onclick={() => {
                modId = m.id;
                content = 'mod';
              }}
            >
              <span class="mod-name">{m.name}</span>
              <span class="mod-meta">{t('wizard.w2.author')} {m.author} · v{m.version} · {fmt(m.defs)} {t('wizard.w2.defs')}</span>
            </button>
          </li>
        {/each}
      </ul>
      <div class="dropzone">
        <button type="button" class="btn" data-testid="wizard.choose-folder">
          <Icon name="folder-open" size={14} />
          {t('wizard.w2.folder')}
        </button>
        <span class="drop-hint">{t('wizard.w2.drop')}</span>
      </div>
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
      <div class="option-grid two">
        {#each [['manual', 'edit'], ['tm', 'database'], ['ai', 'cpu'], ['external', 'external']] as const as [m, icon] (m)}
          <button
            type="button"
            class="option"
            aria-pressed={method === m}
            data-testid={`wizard.method.${m}`}
            onclick={() => (method = m)}
          >
            <Icon name={icon} size={18} />
            <span class="option-title">{t(`wizard.w4.${m}`)}</span>
            <span class="option-desc">{t(`wizard.w4.${m}Desc`)}</span>
          </button>
        {/each}
      </div>
      <label class="field">
        <span class="field-label">{t('wizard.w4.quality')}</span>
        <select bind:value={quality} data-testid="wizard.quality">
          <option value="fast">{t('wizard.w4.quality.fast')}</option>
          <option value="balanced">{t('wizard.w4.quality.balanced')}</option>
          <option value="max">{t('wizard.w4.quality.max')}</option>
          <option value="suggest">{t('wizard.w4.quality.suggest')}</option>
        </select>
      </label>
      {#if method === 'ai' || method === 'external'}
        <p class="note cost" data-testid="wizard.cost-note">{t('wizard.w4.cost')}</p>
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
          <dd class="mono">{fmt(mockPreflight.reusable)}</dd>
        </div>
        <div class="pf-row">
          <dt>{t('wizard.w5.need')}</dt>
          <dd class="mono">{fmt(mockPreflight.needTranslation)}</dd>
        </div>
        <div class="pf-row">
          <dt>{t('wizard.w5.attention')}</dt>
          <dd class="mono warn">{fmt(mockPreflight.attention)}</dd>
        </div>
      </dl>
      {#if selectedMod}
        <p class="note">
          <span class="mono">{selectedMod.name}</span> · v{selectedMod.version}
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
          disabled={step === 2 && modId === null}
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
