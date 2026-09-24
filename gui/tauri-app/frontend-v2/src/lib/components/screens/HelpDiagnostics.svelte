<script lang="ts">
  // Help & diagnostics (mandate §16): quick start, onboarding replay,
  // shortcuts, docs, troubleshooting and a structured diagnose workflow whose
  // output is copyable "for a human or an AI" — a visible stand-in for the
  // future observability pipeline. All mock, zero backend.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { router } from '../../router.svelte';
  import { i18n } from '../../../i18n/store.svelte';
  import { theme } from '../../stores/theme.svelte';
  import { stylelab } from '../../stores/stylelab.svelte';
  import { settings } from '../../stores/settings.svelte';
  import { providers } from '../../stores/providers.svelte';
  import { project } from '../../stores/project.svelte';
  import { onboarding } from '../../stores/onboarding.svelte';
  import ShortcutsEditor from '../ShortcutsEditor.svelte';
  import About from './About.svelte';

  type CheckState = 'pending' | 'ok' | 'warn' | 'fail';

  interface DiagCheck {
    id: 'game' | 'mods' | 'config' | 'provider';
    result: Exclude<CheckState, 'pending'>;
  }

  // The provider check intentionally warns: this mock build has providers
  // not configured / offline, which is exactly what the report should surface.
  const CHECKS: DiagCheck[] = [
    { id: 'game', result: 'ok' },
    { id: 'mods', result: 'ok' },
    { id: 'config', result: 'ok' },
    { id: 'provider', result: 'warn' }
  ];

  let diagState = $state<'idle' | 'running' | 'done'>('idle');
  let checks = $state<(DiagCheck & { state: CheckState })[]>([]);
  let copied = $state<'' | 'diag' | 'bug'>('');
  let reportShown = $state(false);
  let timers: ReturnType<typeof setTimeout>[] = [];

  $effect(() => {
    // Clear pending mock timers when the screen unmounts.
    return () => timers.forEach(clearTimeout);
  });

  const counts = $derived(providers.counts());

  function runDiagnostics() {
    timers.forEach(clearTimeout);
    timers = [];
    diagState = 'running';
    checks = CHECKS.map((ch) => ({ ...ch, state: 'pending' as CheckState }));
    CHECKS.forEach((ch, i) => {
      timers.push(
        setTimeout(() => {
          checks[i].state = ch.result;
          if (i === CHECKS.length - 1) diagState = 'done';
        }, 450 * (i + 1))
      );
    });
  }

  function copyText(kind: 'diag' | 'bug', text: string) {
    navigator.clipboard
      ?.writeText(text)
      .then(() => {
        copied = kind;
        timers.push(setTimeout(() => (copied = ''), 2000));
      })
      .catch(() => {
        /* clipboard unavailable (non-secure context) — silently keep the mock */
      });
  }

  // QA mandate §13: "Replay tips" must actually re-show the onboarding
  // overlay in the Workspace — clear the persisted seen-flag, open the
  // overlay, then navigate. If the Workspace is already mounted (impossible
  // from Help, but safe) navigation is a no-op and the overlay still opens.
  function replayOnboarding() {
    onboarding.replay();
    router.navigate('workspace');
  }

  const diagText = $derived.by(() => {
    if (diagState !== 'done') return '';
    const lines = [
      'RimLoc diagnostics (mock)',
      `checks: ${checks.map((ch) => `${ch.id}=${ch.state}`).join(', ')}`,
      `providers: ${counts.connected} connected, ${counts.notConfigured} not configured, ${counts.offline} offline`,
      `locale: ${i18n.locale} · theme: ${theme.mode} · style: ${stylelab.style}/${stylelab.palette}/${stylelab.density}`
    ];
    return lines.join('\n');
  });

  const bugReport = $derived.by(() => [
    'RimLoc bug report (mock)',
    `app: 0.1.0 · ui: ${i18n.locale} · theme: ${theme.mode}`,
    `style: ${stylelab.style}/${stylelab.palette}/${stylelab.density} · motion: ${settings.data.motion}`,
    `project: ${project.projectName} · en → ${project.targetLocale}`,
    `diagnostics: ${diagState === 'done' ? `${checks.filter((ch) => ch.state === 'warn').length} warning(s)` : 'not run'}`,
    'last error: ERR_MOCK_EXAMPLE (mandate §16 demonstrates the L/observability workflow)'
  ].join('\n'));
</script>

<section class="help" aria-labelledby="help-heading">
  <h1 id="help-heading" class="title">
    <Icon name="help" size={20} />
    {t('help.title')}
  </h1>
  <p class="subtitle">{t('help.subtitle')}</p>

  <!-- Quick start -->
  <article class="card" data-testid="help.quickstart">
    <h2 class="card-title"><Icon name="play" size={16} /> {t('help.quickstart.title')}</h2>
    <ol class="steps">
      <li>{t('help.quickstart.s1')}</li>
      <li>{t('help.quickstart.s2')}</li>
      <li>{t('help.quickstart.s3')}</li>
      <li>{t('help.quickstart.s4')}</li>
    </ol>
  </article>

  <!-- Replay onboarding -->
  <article class="card" data-testid="help.replay">
    <h2 class="card-title"><Icon name="lightbulb" size={16} /> {t('help.replay.title')}</h2>
    <p class="text">{t('help.replay.desc')}</p>
    <button type="button" class="btn" data-testid="help.replay.action" onclick={replayOnboarding}>
      <Icon name="arrow-right" size={14} />
      {t('help.replay.action')}
    </button>
  </article>

  <!-- Shortcuts (mandate §14): full remappable table with conflict detection;
       the static key list moved into the editor component. -->
  <article class="card" data-testid="help.shortcuts">
    <h2 class="card-title"><Icon name="sliders" size={16} /> {t('help.shortcuts.title')}</h2>
    <ShortcutsEditor />
  </article>

  <!-- Documentation -->
  <article class="card" data-testid="help.docs">
    <h2 class="card-title"><Icon name="book" size={16} /> {t('help.docs.title')}</h2>
    <p class="text">{t('help.docs.desc')}</p>
    <div class="row">
      <button type="button" class="btn" data-testid="help.docs.guide">{t('help.docs.guide')} <Icon name="external" size={13} /></button>
      <button type="button" class="btn" data-testid="help.docs.cli">{t('help.docs.cli')} <Icon name="external" size={13} /></button>
      <button type="button" class="btn" data-testid="help.docs.repo">{t('help.docs.repo')} <Icon name="external" size={13} /></button>
    </div>
  </article>

  <!-- Troubleshooting -->
  <article class="card" data-testid="help.trouble">
    <h2 class="card-title"><Icon name="warning" size={16} /> {t('help.trouble.title')}</h2>
    <details class="qa">
      <summary>{t('help.trouble.q1')}</summary>
      <p class="text">{t('help.trouble.a1')}</p>
    </details>
    <details class="qa">
      <summary>{t('help.trouble.q2')}</summary>
      <p class="text">{t('help.trouble.a2')}</p>
    </details>
    <details class="qa">
      <summary>{t('help.trouble.q3')}</summary>
      <p class="text">{t('help.trouble.a3')}</p>
    </details>
  </article>

  <!-- Diagnose problem -->
  <article class="card" data-testid="help.diagnose">
    <h2 class="card-title"><Icon name="clipboard-check" size={16} /> {t('help.diagnose.title')}</h2>
    <p class="text">{t('help.diagnose.desc')}</p>
    <div class="row">
      <button type="button" class="btn btn-primary" data-testid="help.diagnose.run" disabled={diagState === 'running'} onclick={runDiagnostics}>
        <Icon name="play" size={14} />
        {diagState === 'running' ? t('providers.status.testing') : diagState === 'done' ? t('help.diagnose.again') : t('help.diagnose.run')}
      </button>
      {#if diagState === 'done'}
        <button type="button" class="btn" data-testid="help.diagnose.copy" onclick={() => copyText('diag', diagText)}>
          <Icon name="copy" size={14} />
          {t('help.diagnose.copy')}
        </button>
      {/if}
      {#if copied === 'diag'}
        <span class="copied" role="status">{t('help.copied')}</span>
      {/if}
    </div>

    {#if checks.length > 0}
      <ul class="checklist" data-testid="help.diagnose.results">
        {#each checks as ch (ch.id)}
          <li class="check" data-testid={`help.diagnose.check.${ch.id}`} data-state={ch.state}>
            <Icon name={ch.state === 'pending' ? 'clock' : ch.state === 'ok' ? 'circle-check' : 'warning'} size={14} />
            <span class="check-label">{t(`help.diagnose.check.${ch.id}`)}</span>
            <span class="check-state">{t(ch.state === 'pending' ? 'providers.status.testing' : ch.state === 'ok' ? 'help.diagnose.ok' : ch.state === 'warn' ? 'help.diagnose.warn' : 'help.diagnose.fail')}</span>
          </li>
        {/each}
      </ul>
      {#if diagState === 'done'}
        <p class="text done-line" role="status">
          {t('help.diagnose.done', { warn: checks.filter((ch) => ch.state === 'warn').length })}
        </p>
      {/if}
    {/if}
  </article>

  <!-- Bug report -->
  <article class="card" data-testid="help.bugreport">
    <h2 class="card-title"><Icon name="package" size={16} /> {t('help.bugreport.title')}</h2>
    <p class="text">{t('help.bugreport.desc')}</p>
    <div class="row">
      <button type="button" class="btn btn-primary" data-testid="help.bugreport.prepare" onclick={() => (reportShown = true)}>
        <Icon name="clipboard-check" size={14} />
        {t('help.bugreport.prepare')}
      </button>
      <button type="button" class="btn" data-testid="help.bugreport.copyAI" onclick={() => copyText('bug', bugReport)}>
        <Icon name="copy" size={14} />
        {t('help.bugreport.copyAI')}
      </button>
      {#if copied === 'bug'}
        <span class="copied" role="status">{t('help.copied')}</span>
      {/if}
    </div>
    {#if reportShown}
      <pre class="report mono" data-testid="help.bugreport.preview">{bugReport}</pre>
    {/if}
  </article>

  <!-- About (mandate §24): version, credits, license and font notices. -->
  <About />
</section>

<style>
  .help {
    width: min(720px, 100%);
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

  .steps {
    margin: 0;
    padding-left: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .text {
    margin: 0;
    color: var(--color-muted-fg);
  }

  .done-line {
    color: var(--color-fg);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .qa {
    border-top: 1px solid var(--color-border);
    padding: var(--space-2) 0;
  }

  .qa:first-of-type {
    border-top: none;
    padding-top: 0;
  }

  .qa summary {
    cursor: pointer;
    font-weight: 600;
  }

  .qa .text {
    margin-top: var(--space-2);
  }

  .checklist {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    font-size: var(--text-dense-size);
    animation: check-in var(--motion-standard) var(--ease-out);
  }

  .check:last-child {
    border-bottom: none;
  }

  .check[data-state='ok'] :global(svg) {
    color: var(--color-success);
  }

  .check[data-state='warn'] :global(svg) {
    color: var(--color-warning);
  }

  .check[data-state='pending'] :global(svg),
  .check[data-state='pending'] .check-state {
    color: var(--color-muted-fg);
  }

  .check-label {
    flex: 1;
  }

  .check-state {
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
  }

  .copied {
    color: var(--color-success);
    font-size: var(--text-meta-size);
  }

  .report {
    margin: 0;
    background: var(--color-muted);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-3);
    overflow-x: auto;
    white-space: pre-wrap;
  }

  @keyframes check-in {
    from {
      opacity: 0;
      transform: translateX(-4px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }
</style>
