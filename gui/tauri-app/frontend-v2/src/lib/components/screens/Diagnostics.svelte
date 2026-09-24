<script lang="ts">
  // Diagnostics workstation (mandate §16 "ADVANCED / DIAGNOSTICS" + §17,
  // W4.5/W5 items 5-6): the deep screen behind Help. Replays the controlled
  // known failure, shows the structured causal context (operation id, stage,
  // error chain, affected entries, provider/validator state, versions,
  // locales, timings, a small relevant trace) and builds the sanitized
  // support bundle with the Included / Redacted / Excluded preview.
  // All mock, zero backend.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { router } from '../../router.svelte';
  import { project } from '../../stores/project.svelte';
  import { diagnostics } from '../../stores/diagnostics.svelte';
  import BundlePreview from './BundlePreview.svelte';
  import { registry } from '../../languages/registry';

  let copied = $state(false);
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => () => {
    diagnostics.reset();
    if (copiedTimer) clearTimeout(copiedTimer);
  });

  const phase = $derived(diagnostics.phase);
  const causal = $derived(diagnostics.causal);
  const bundle = $derived(diagnostics.bundle);

  const sourceLanguage = $derived(registry.resolve(causal?.locales.source ?? 'en'));

  function openAffectedEntry(id: string) {
    project.select(id);
    router.navigate('workspace');
  }

  function copyForAi() {
    navigator.clipboard
      ?.writeText(diagnostics.aiPrompt)
      .then(() => {
        copied = true;
        if (copiedTimer) clearTimeout(copiedTimer);
        copiedTimer = setTimeout(() => (copied = false), 2000);
      })
      .catch(() => {
        /* clipboard unavailable (non-secure context) — keep the mock */
      });
  }
</script>

<section class="diag" aria-labelledby="diag-heading">
  <!-- W6: the local demo badge was consolidated into the global MockBadge in
       the app header — the data-mode truth is now shown on every route. -->
  <h1 id="diag-heading" class="title">
    <Icon name="cpu" size={20} />
    {t('diagnostics.title')}
  </h1>
  <p class="subtitle">{t('diagnostics.subtitle')}</p>

  <!-- 1. Controlled known failure (W4.5/W5 item 5: acceptance is a reviewer
       naming the cause, not "a bundle was generated"). -->
  <article class="card" data-testid="diagnostics.scenario">
    <h2 class="card-title"><Icon name="play" size={16} /> {t('diagnostics.scenario.title')}</h2>
    <p class="text">{t('diagnostics.scenario.desc')}</p>
    <div class="row">
      <button
        type="button"
        class="btn btn-primary"
        data-testid="diagnostics.run"
        disabled={phase === 'running'}
        onclick={() => diagnostics.runScenario()}
      >
        <Icon name="play" size={14} />
        {phase === 'running' ? t('providers.status.testing') : phase === 'idle' ? t('diagnostics.run') : t('diagnostics.runAgain')}
      </button>
      {#if phase === 'idle'}
        <span class="hint">{t('diagnostics.scenario.knownFailure', { key: 'keyed-04' })}</span>
      {/if}
    </div>
    {#if phase !== 'idle'}
      <ol class="steps" data-testid="diagnostics.steps">
        {#each diagnostics.steps as step (step.id)}
          <li class="step" data-state={step.state} data-testid={`diagnostics.step.${step.id}`}>
            <Icon name={step.state === 'done' ? 'circle-check' : step.state === 'active' ? 'clock' : 'square'} size={13} />
            {t(step.labelKey)}
          </li>
        {/each}
      </ol>
    {/if}
  </article>

  <!-- 2. Structured causal context: small, relevant, reviewer-ready. -->
  {#if causal && (phase === 'diagnosed' || phase === 'bundled')}
    <article class="card" data-testid="diagnostics.causal">
      <h2 class="card-title"><Icon name="clipboard-check" size={16} /> {t('diagnostics.causal.title')}</h2>

      <div class="chips">
        <span class="chip mono" data-testid="diagnostics.operation">{causal.operationId}</span>
        <span class="chip mono">{t('diagnostics.causal.stage')}: {causal.stage}</span>
        <span class="chip mono">{causal.errorCode}</span>
      </div>

      <dl class="chain" data-testid="diagnostics.error-chain">
        <div class="chain-row" data-variant="expected">
          <dt>{t('diagnostics.causal.expected')}</dt>
          <dd class="mono">{causal.expected}</dd>
        </div>
        <div class="chain-row" data-variant="actual">
          <dt>{t('diagnostics.causal.actual')}</dt>
          <dd class="mono">{causal.actual}</dd>
        </div>
      </dl>

      <div class="sub">
        <h3 class="sub-title">{t('diagnostics.causal.affected')}</h3>
        {#each causal.affectedEntries as entry (entry.id)}
          <button type="button" class="entry" data-testid={`diagnostics.affected.${entry.id}`} onclick={() => openAffectedEntry(entry.id)}>
            <span class="mono entry-id">{entry.id}</span>
            <span class="mono entry-meta">{entry.key} · {entry.file}:{entry.line}</span>
            <span class="entry-placeholders">
              {#each entry.placeholders as ph (ph)}
                <span class="chip mono chip-ph">{ph}</span>
              {/each}
            </span>
            <Icon name="arrow-right" size={13} />
          </button>
        {/each}
      </div>

      <dl class="meta" data-testid="diagnostics.env">
        <div class="meta-row">
          <dt>{t('diagnostics.causal.providers')}</dt>
          <dd class="mono">
            {t('diagnostics.causal.providersValue', {
              connected: causal.providerState.connected,
              notConfigured: causal.providerState.notConfigured,
              offline: causal.providerState.offline
            })}
          </dd>
        </div>
        <div class="meta-row">
          <dt>{t('diagnostics.causal.validator')}</dt>
          <dd class="mono">{causal.validator}</dd>
        </div>
        <div class="meta-row">
          <dt>{t('diagnostics.causal.version')}</dt>
          <dd class="mono">{causal.rimworldVersion}</dd>
        </div>
        <div class="meta-row">
          <dt>{t('diagnostics.causal.locales')}</dt>
          <dd class="mono">
            {sourceLanguage.nativeName} ({causal.locales.source}) → {registry.resolve(causal.locales.target).nativeName} ({causal.locales.target}) · ui {causal.locales.ui}
          </dd>
        </div>
        <div class="meta-row">
          <dt>{t('diagnostics.causal.timing')}</dt>
          <dd class="mono">{causal.durationMs} ms</dd>
        </div>
      </dl>

      <div class="sub">
        <h3 class="sub-title">{t('diagnostics.causal.trace')}</h3>
        <pre class="trace mono" data-testid="diagnostics.trace">{causal.trace
            .map((ev) => `+${String(ev.atMs).padStart(4, ' ')}ms [${ev.level}] ${ev.phase}: ${ev.message}`)
            .join('\n')}</pre>
        <p class="text trace-note">{t('diagnostics.causal.traceNote')}</p>
      </div>

      <div class="row">
        <button type="button" class="btn btn-primary" data-testid="diagnostics.prepare" onclick={() => diagnostics.prepareBundle()}>
          <Icon name="package" size={14} />
          {t('diagnostics.prepare')}
        </button>
        <span class="hint">{t('diagnostics.prepareHint')}</span>
      </div>
    </article>
  {/if}

  <!-- 3. Sanitized support bundle: three explicit sections (§17). -->
  {#if bundle && phase === 'bundled'}
    <article class="card" data-testid="diagnostics.bundle-card">
      <h2 class="card-title"><Icon name="package" size={16} /> {t('diagnostics.bundle.title')}</h2>
      <BundlePreview preview={bundle} />
      <div class="row">
        <button type="button" class="btn btn-primary" data-testid="diagnostics.copyAi" onclick={copyForAi}>
          <Icon name="copy" size={14} />
          {t('diagnostics.copyAi')}
        </button>
        {#if copied}
          <span class="copied" role="status">{t('help.copied')}</span>
        {/if}
      </div>
      <p class="text">{t('diagnostics.copyAiNote')}</p>
    </article>
  {/if}
</section>

<style>
  .diag {
    width: min(760px, 100%);
    margin: 0 auto;
    padding: var(--space-8) var(--space-4);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .subtitle {
    margin: calc(-1 * var(--space-2)) 0 0;
    color: var(--color-muted-fg);
  }

  .card {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-card);
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .card-title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: 16px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .card-title :global(svg) {
    color: var(--color-primary-text);
  }

  .text {
    margin: 0;
    color: var(--color-muted-fg);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .hint {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .copied {
    color: var(--color-success);
    font-size: var(--text-meta-size);
  }

  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .step {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-dense-size);
    color: var(--color-muted-fg);
  }

  .step[data-state='done'] {
    color: var(--color-fg);
  }

  .step[data-state='done'] :global(svg) {
    color: var(--color-success);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    padding: 2px var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-muted);
    font-size: var(--text-meta-size);
  }

  .chip-ph {
    color: var(--color-warning);
    border-color: var(--color-warning);
    background: transparent;
  }

  .chain {
    margin: 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .chain-row {
    display: grid;
    grid-template-columns: 110px 1fr;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    font-size: var(--text-dense-size);
  }

  .chain-row:last-child {
    border-bottom: none;
  }

  .chain-row[data-variant='expected'] dt {
    color: var(--color-success);
  }

  .chain-row[data-variant='actual'] dt {
    color: var(--color-destructive);
  }

  .chain-row dd {
    margin: 0;
    word-break: break-word;
  }

  .sub {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .sub-title {
    margin: 0;
    font-size: var(--text-meta-size);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-muted-fg);
  }

  .entry {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-3);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    cursor: pointer;
    text-align: left;
    font-size: var(--text-dense-size);
    transition: border-color var(--motion-fast) var(--ease-out);
  }

  .entry:hover {
    border-color: var(--color-border-strong);
  }

  .entry-id {
    color: var(--color-primary-text);
    font-weight: 600;
  }

  .entry-meta {
    color: var(--color-muted-fg);
  }

  .entry-placeholders {
    display: inline-flex;
    gap: var(--space-1);
  }

  .entry :global(svg:last-child) {
    margin-left: auto;
    color: var(--color-muted-fg);
  }

  .meta {
    margin: 0;
    display: flex;
    flex-direction: column;
  }

  .meta-row {
    display: grid;
    grid-template-columns: 170px 1fr;
    gap: var(--space-3);
    padding: var(--space-1) 0;
    border-bottom: 1px solid var(--color-border);
    font-size: var(--text-dense-size);
  }

  .meta-row:last-child {
    border-bottom: none;
  }

  .meta-row dt {
    color: var(--color-muted-fg);
  }

  .meta-row dd {
    margin: 0;
    word-break: break-word;
  }

  .trace {
    margin: 0;
    padding: var(--space-3);
    background: var(--color-muted);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow-x: auto;
    font-size: var(--text-meta-size);
    line-height: 1.5;
  }

  .trace-note {
    font-size: var(--text-meta-size);
  }
</style>
