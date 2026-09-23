<script lang="ts">
  // Route dispatcher + the FULL Settings screen (mandate §15, spec §17).
  // App.svelte routes the phase-2 app-level routes here; providers and help
  // delegate to their screens, #/settings renders seven task-grouped sections
  // with a left rail. CLI flags are deliberately NOT exposed 1:1 (spec §17):
  // experimental/internal options live only under Advanced.
  //
  // Appearance controls re-surface the existing theme/style-lab stores
  // (theme.svelte.ts, stylelab.svelte.ts) — one source of truth per axis.
  // The bare palette-boot import below keeps the global Cmd/Ctrl+K palette
  // alive on every route (App.svelte statically imports this file).
  import Icon from '../Icon.svelte';
  import { t, i18n, type Locale } from '../../../i18n/store.svelte';
  import { router, type RouteId } from '../../router.svelte';
  import { theme, type ThemeMode } from '../../stores/theme.svelte';
  import {
    stylelab,
    type StyleId,
    type PaletteId,
    type DensityId
  } from '../../stores/stylelab.svelte';
  import {
    settings,
    type StartupMode,
    type RimworldVersion,
    type TargetLocaleId,
    type QualityMode,
    type MotionPref,
    type FontSizeId,
    type GlossaryMode
  } from '../../stores/settings.svelte';
  import { providers } from '../../stores/providers.svelte';
  import ProviderManager from './ProviderManager.svelte';
  import HelpDiagnostics from './HelpDiagnostics.svelte';
  import '../../palette-boot.svelte';

  let { route }: { route: Extract<RouteId, 'settings' | 'providers' | 'help'> } = $props();

  type SectionId =
    | 'general'
    | 'rimworld'
    | 'translation'
    | 'ai'
    | 'editor'
    | 'appearance'
    | 'advanced';

  let section = $state<SectionId>('general');

  const NAV: { id: SectionId; icon: string; labelKey: string }[] = [
    { id: 'general', icon: 'sliders', labelKey: 'settings.section.general' },
    { id: 'rimworld', icon: 'package', labelKey: 'settings.section.rimworld' },
    { id: 'translation', icon: 'languages', labelKey: 'settings.section.translation' },
    { id: 'ai', icon: 'cpu', labelKey: 'settings.section.ai' },
    { id: 'editor', icon: 'edit', labelKey: 'settings.section.editor' },
    { id: 'appearance', icon: 'palette', labelKey: 'settings.section.appearance' },
    { id: 'advanced', icon: 'database', labelKey: 'settings.section.advanced' }
  ];

  // ---- mock data (no backend) --------------------------------------------
  const INSTALLS = [
    { id: 'steam', label: 'Steam · RimWorld', version: '1.6.4521', primary: true },
    { id: 'gog', label: 'GOG · RimWorld', version: '1.5.4409', primary: false }
  ];
  const WORKSHOP_PATH = '<Steam>/steamapps/workshop/content/294100';

  const VERSION_OPTS = [
    { v: '1.4', label: '1.4' },
    { v: '1.5', label: '1.5' },
    { v: '1.6', label: '1.6' }
  ];

  const TARGETS: { v: TargetLocaleId; label: string }[] = [
    { v: 'ru', label: 'Русский' },
    { v: 'uk', label: 'Українська' },
    { v: 'de', label: 'Deutsch' },
    { v: 'es', label: 'Español' },
    { v: 'fr', label: 'Français' },
    { v: 'pt-br', label: 'Português (Brasil)' },
    { v: 'ja', label: '日本語' },
    { v: 'zh-hans', label: '简体中文' }
  ];

  const QUALITY: { id: QualityMode; labelKey: string; descKey: string }[] = [
    { id: 'fast', labelKey: 'wizard.w4.quality.fast', descKey: 'settings.ai.quality.desc.fast' },
    { id: 'balanced', labelKey: 'wizard.w4.quality.balanced', descKey: 'settings.ai.quality.desc.balanced' },
    { id: 'max', labelKey: 'wizard.w4.quality.max', descKey: 'settings.ai.quality.desc.max' },
    { id: 'suggest', labelKey: 'wizard.w4.quality.suggest', descKey: 'settings.ai.quality.desc.suggest' }
  ];

  const FONT_OPTS: { v: FontSizeId; label: string }[] = [
    { v: 'small', label: 'settings.editor.size.small' },
    { v: 'medium', label: 'settings.editor.size.medium' },
    { v: 'large', label: 'settings.editor.size.large' }
  ];

  const counts = $derived(providers.counts());

  // ---- mock one-shot actions ---------------------------------------------
  let timers: ReturnType<typeof setTimeout>[] = [];
  $effect(() => {
    return () => timers.forEach(clearTimeout);
  });

  let updateState = $state<'idle' | 'checking' | 'ok'>('idle');
  let rescanState = $state<'idle' | 'running' | 'done'>('idle');
  let advancedNote = $state('');

  function checkUpdates() {
    updateState = 'checking';
    timers.push(setTimeout(() => (updateState = 'ok'), 900));
  }

  function rescanInstalls() {
    rescanState = 'running';
    timers.push(setTimeout(() => (rescanState = 'done'), 900));
  }

  function showAdvancedNote(key: string) {
    advancedNote = key;
    timers.push(setTimeout(() => (advancedNote = ''), 2400));
  }
</script>

{#if route === 'providers'}
  <ProviderManager />
{:else if route === 'help'}
  <HelpDiagnostics />
{:else}
  <section class="settings" aria-labelledby="settings-heading">
    <h1 id="settings-heading" class="visually-hidden">{t('settings.title')}</h1>

    <aside class="nav" aria-label={t('settings.nav.label')}>
      {#each NAV as n (n.id)}
        <button
          type="button"
          class="nav-btn"
          class:active={section === n.id}
          aria-current={section === n.id ? 'true' : undefined}
          data-testid={`settings.nav.${n.id}`}
          onclick={() => (section = n.id)}
        >
          <Icon name={n.icon} size={15} />
          {t(n.labelKey)}
        </button>
      {/each}
    </aside>

    <div class="content">
      <header class="content-head">
        <h2 class="content-title">{t(`settings.section.${section}`)}</h2>
        <p class="content-desc">{t(`settings.${section}.desc`)}</p>
      </header>

      {#snippet seg(label: string, options: { v: string; label: string }[], current: string, onpick: (v: string) => void, testid: string)}
        <div class="field">
          <span class="field-label" id={`${testid}.label`}>{label}</span>
          <div class="seg" role="group" aria-labelledby={`${testid}.label`}>
            {#each options as o (o.v)}
              <button
                type="button"
                class="seg-btn"
                aria-pressed={current === o.v}
                data-testid={`${testid}.${o.v}`}
                onclick={() => onpick(o.v)}
              >
                {o.label}
              </button>
            {/each}
          </div>
        </div>
      {/snippet}

      {#snippet toggle(label: string, value: boolean, onchange: (v: boolean) => void, testid: string)}
        <label class="field toggle">
          <span class="field-label">{label}</span>
          <input
            type="checkbox"
            checked={value}
            data-testid={testid}
            onchange={(e) => onchange(e.currentTarget.checked)}
          />
        </label>
      {/snippet}

      {#if section === 'general'}
        <div class="panel" data-testid="settings.panel.general">
          {@render seg(
            t('settings.general.language'),
            [
              { v: 'ru', label: 'Русский' },
              { v: 'en', label: 'English' }
            ],
            i18n.locale,
            (v) => i18n.setLocale(v as Locale),
            'settings.language'
          )}
          {@render seg(
            t('settings.general.startup'),
            [
              { v: 'last-project', label: t('settings.general.startup.last') },
              { v: 'home', label: t('settings.general.startup.home') }
            ],
            settings.data.startup,
            (v) => settings.set('startup', v as StartupMode),
            'settings.startup'
          )}
          <div class="field">
            <span class="field-label">{t('settings.general.updates')}</span>
            <div class="stack">
              <label class="inline-toggle">
                <input
                  type="checkbox"
                  checked={settings.data.checkUpdates}
                  data-testid="settings.toggle.checkUpdates"
                  onchange={(e) => settings.set('checkUpdates', e.currentTarget.checked)}
                />
                {t('settings.general.updates.auto')}
              </label>
              <button
                type="button"
                class="btn"
                data-testid="settings.btn.checkUpdates"
                disabled={updateState === 'checking'}
                onclick={checkUpdates}
              >
                <Icon name={updateState === 'checking' ? 'clock' : 'download'} size={14} />
                {t('settings.general.updates.check')}
              </button>
            </div>
          </div>
          {#if updateState === 'ok'}
            <p class="ok-line" role="status">{t('settings.general.updates.ok')}</p>
          {/if}
        </div>
      {:else if section === 'rimworld'}
        <div class="panel" data-testid="settings.panel.rimworld">
          <div class="field">
            <span class="field-label">{t('settings.rimworld.installs')}</span>
            <ul class="installs">
              {#each INSTALLS as inst (inst.id)}
                <li class="install">
                  <Icon name="package" size={14} />
                  <span class="install-name">{inst.label}</span>
                  {#if inst.primary}
                    <span class="chip">{t('settings.rimworld.primary')}</span>
                  {/if}
                  <span class="mono install-ver">{inst.version}</span>
                </li>
              {/each}
            </ul>
            <div class="stack">
              <button
                type="button"
                class="btn"
                data-testid="settings.btn.rescan"
                disabled={rescanState === 'running'}
                onclick={rescanInstalls}
              >
                <Icon name={rescanState === 'running' ? 'clock' : 'search'} size={14} />
                {t('settings.rimworld.rescan')}
              </button>
              {#if rescanState === 'done'}
                <span class="ok-line inline" role="status">{t('settings.rimworld.found')}</span>
              {/if}
            </div>
          </div>

          <div class="field">
            <label class="field-label" for="settings-workshop">{t('settings.rimworld.workshop')}</label>
            <input
              id="settings-workshop"
              class="path mono"
              type="text"
              readonly
              value={WORKSHOP_PATH}
              data-testid="settings.input.workshop"
              spellcheck="false"
            />
          </div>

          {@render seg(
            t('settings.rimworld.version'),
            VERSION_OPTS,
            settings.data.rimworldVersion,
            (v) => settings.set('rimworldVersion', v as RimworldVersion),
            'settings.version'
          )}
          {@render toggle(
            t('settings.rimworld.autodetect'),
            settings.data.autoDetect,
            (v) => settings.set('autoDetect', v),
            'settings.toggle.autoDetect'
          )}

          <div class="planned">
            <span class="field-label">{t('settings.rimworld.rimsort')}</span>
            <p class="note">{t('settings.rimworld.rimsortNote')}</p>
          </div>
        </div>
      {:else if section === 'translation'}
        <div class="panel" data-testid="settings.panel.translation">
          <div class="field">
            <span class="field-label">{t('settings.translation.pair')}</span>
            <div class="pair">
              <span class="pair-item">
                <span class="pair-name">{t('settings.translation.source')}</span>
                <span class="mono">English</span>
              </span>
              <Icon name="arrow-right" size={14} />
              <label class="pair-item">
                <span class="pair-name">{t('settings.translation.target')}</span>
                <select
                  value={settings.data.defaultTarget}
                  data-testid="settings.select.target"
                  onchange={(e) =>
                    settings.set(
                      'defaultTarget',
                      (e.currentTarget as HTMLSelectElement).value as TargetLocaleId
                    )}
                >
                  {#each TARGETS as tg (tg.v)}
                    <option value={tg.v}>{tg.label}</option>
                  {/each}
                </select>
              </label>
            </div>
            <p class="note">{t('settings.translation.pairNote')}</p>
          </div>

          {@render toggle(
            t('settings.translation.autosave'),
            settings.data.autosave,
            (v) => settings.set('autosave', v),
            'settings.toggle.autosave'
          )}
          {@render toggle(
            t('settings.translation.tmFirst'),
            settings.data.tmFirst,
            (v) => settings.set('tmFirst', v),
            'settings.toggle.tmFirst'
          )}
          {@render toggle(
            t('settings.translation.markAI'),
            settings.data.markAI,
            (v) => settings.set('markAI', v),
            'settings.toggle.markAI'
          )}
          {@render seg(
            t('settings.translation.glossary'),
            [
              { v: 'suggest', label: t('settings.translation.glossary.suggest') },
              { v: 'enforce', label: t('settings.translation.glossary.enforce') }
            ],
            settings.data.glossary,
            (v) => settings.set('glossary', v as GlossaryMode),
            'settings.glossary'
          )}
        </div>
      {:else if section === 'ai'}
        <div class="panel" data-testid="settings.panel.ai">
          <div class="field">
            <span class="field-label">{t('settings.section.ai')}</span>
            <p class="summary" data-testid="settings.ai.summary">
              {t('settings.ai.summary', {
                connected: counts.connected,
                unconfigured: counts.notConfigured,
                offline: counts.offline
              })}
            </p>
            <button type="button" class="btn" data-testid="settings.btn.providers" onclick={() => router.navigate('providers')}>
              <Icon name="cpu" size={14} />
              {t('settings.ai.manage')}
              <Icon name="arrow-right" size={13} />
            </button>
          </div>

          <div class="field">
            <span class="field-label">{t('settings.ai.quality')}</span>
            <div class="quality" role="radiogroup" aria-label={t('settings.ai.quality')}>
              {#each QUALITY as q (q.id)}
                <button
                  type="button"
                  class="quality-opt"
                  role="radio"
                  aria-checked={settings.data.quality === q.id}
                  data-testid={`settings.quality.${q.id}`}
                  onclick={() => settings.set('quality', q.id)}
                >
                  <span class="quality-name">{t(q.labelKey)}</span>
                  <span class="quality-desc">{t(q.descKey)}</span>
                </button>
              {/each}
            </div>
          </div>

          <p class="note"><Icon name="info" size={13} /> {t('settings.ai.privacy')}</p>
          <p class="note"><Icon name="warning" size={13} /> {t('settings.ai.cost')}</p>
        </div>
      {:else if section === 'editor'}
        <div class="panel" data-testid="settings.panel.editor">
          {@render seg(
            t('settings.editor.fontSize'),
            FONT_OPTS.map((o) => ({ v: o.v, label: t(o.label) })),
            settings.data.fontSize,
            (v) => settings.set('fontSize', v as FontSizeId),
            'settings.fontSize'
          )}
          {@render toggle(
            t('settings.editor.wrap'),
            settings.data.softWrap,
            (v) => settings.set('softWrap', v),
            'settings.toggle.softWrap'
          )}
          {@render seg(
            t('settings.editor.density'),
            [
              { v: 'comfortable', label: t('stylelab.density.comfortable') },
              { v: 'compact', label: t('stylelab.density.compact') }
            ],
            stylelab.density,
            (v) => stylelab.setDensity(v as DensityId),
            'settings.density'
          )}
          <p class="note">{t('settings.editor.densityNote')}</p>

          <div class="field">
            <span class="field-label">{t('settings.editor.shortcuts')}</span>
            <table class="keys">
              <tbody>
                <tr>
                  <td><kbd class="kbd">⌘K</kbd> <span class="kbd-sep">/</span> <kbd class="kbd">Ctrl+K</kbd></td>
                  <td>{t('help.shortcuts.palette')}</td>
                </tr>
                <tr>
                  <td><kbd class="kbd">Enter</kbd></td>
                  <td>{t('help.shortcuts.save')}</td>
                </tr>
                <tr>
                  <td><kbd class="kbd">Esc</kbd></td>
                  <td>{t('help.shortcuts.cancel')}</td>
                </tr>
                <tr>
                  <td><kbd class="kbd">Tab</kbd></td>
                  <td>{t('help.shortcuts.next')}</td>
                </tr>
              </tbody>
            </table>
            <button type="button" class="link" data-testid="settings.btn.allShortcuts" onclick={() => router.navigate('help')}>
              {t('settings.editor.shortcutsMore')}
            </button>
          </div>
        </div>
      {:else if section === 'appearance'}
        <div class="panel" data-testid="settings.panel.appearance">
          {@render seg(
            t('settings.appearance.theme'),
            [
              { v: 'light', label: t('theme.light') },
              { v: 'dark', label: t('theme.dark') },
              { v: 'system', label: t('theme.system') }
            ],
            theme.mode,
            (v) => theme.set(v as ThemeMode),
            'settings.theme'
          )}
          {@render seg(
            t('settings.appearance.style'),
            [
              { v: 'precision', label: t('stylelab.style.precision') },
              { v: 'aurora', label: t('stylelab.style.aurora') },
              { v: 'workshop', label: t('stylelab.style.workshop') },
              { v: 'editorial', label: t('stylelab.style.editorial') }
            ],
            stylelab.style,
            (v) => stylelab.setStyle(v as StyleId),
            'settings.style'
          )}
          {@render seg(
            t('settings.appearance.palette'),
            [
              { v: 'indigo', label: t('stylelab.palette.indigo') },
              { v: 'blue', label: t('stylelab.palette.blue') },
              { v: 'emerald', label: t('stylelab.palette.emerald') },
              { v: 'amber', label: t('stylelab.palette.amber') }
            ],
            stylelab.palette,
            (v) => stylelab.setPalette(v as PaletteId),
            'settings.palette'
          )}
          {@render seg(
            t('settings.appearance.density'),
            [
              { v: 'comfortable', label: t('stylelab.density.comfortable') },
              { v: 'compact', label: t('stylelab.density.compact') }
            ],
            stylelab.density,
            (v) => stylelab.setDensity(v as DensityId),
            'settings.appearance.density'
          )}
          {@render seg(
            t('settings.appearance.motion'),
            [
              { v: 'system', label: t('settings.appearance.motion.system') },
              { v: 'full', label: t('settings.appearance.motion.full') },
              { v: 'reduced', label: t('settings.appearance.motion.reduced') }
            ],
            settings.data.motion,
            (v) => settings.set('motion', v as MotionPref),
            'settings.motion'
          )}
          {@render toggle(
            t('settings.appearance.sounds'),
            settings.data.sounds,
            (v) => settings.set('sounds', v),
            'settings.toggle.sounds'
          )}
          <p class="note">{t('settings.appearance.soundsNote')}</p>
          {@render toggle(
            t('settings.appearance.sysnotify'),
            settings.data.systemNotifications,
            (v) => settings.set('systemNotifications', v),
            'settings.toggle.sysnotify'
          )}
        </div>
      {:else}
        <div class="panel" data-testid="settings.panel.advanced">
          <div class="field">
            <span class="field-label">{t('settings.advanced.logs')}</span>
            <button
              type="button"
              class="btn"
              data-testid="settings.btn.openLogs"
              onclick={() => showAdvancedNote('settings.advanced.logsDone')}
            >
              <Icon name="database" size={14} />
              {t('settings.advanced.openLogs')}
            </button>
          </div>

          <div class="field">
            <span class="field-label">{t('settings.advanced.diagnostics')}</span>
            <button type="button" class="btn" data-testid="settings.btn.runDiag" onclick={() => router.navigate('help')}>
              <Icon name="clipboard-check" size={14} />
              {t('settings.advanced.runDiag')}
            </button>
          </div>

          <div class="field">
            <span class="field-label">{t('settings.advanced.rules')}</span>
            <details class="qa">
              <summary data-testid="settings.details.rules">{t('settings.advanced.viewRules')}</summary>
              <ul class="rules">
                <li class="mono">placeholders kept verbatim</li>
                <li class="mono">glossary terms preferred</li>
                <li class="mono"> Tone register: neutral game UI</li>
              </ul>
            </details>
          </div>

          <div class="field">
            <span class="field-label">{t('settings.advanced.config')}</span>
            <div class="stack">
              <button type="button" class="btn" data-testid="settings.btn.exportCfg" onclick={() => showAdvancedNote('settings.advanced.exported')}>
                <Icon name="download" size={14} />
                {t('settings.advanced.export')}
              </button>
              <button type="button" class="btn" data-testid="settings.btn.importCfg" onclick={() => showAdvancedNote('settings.advanced.imported')}>
                <Icon name="upload" size={14} />
                {t('settings.advanced.import')}
              </button>
            </div>
          </div>

          {#if advancedNote}
            <p class="ok-line" role="status">{t(advancedNote)}</p>
          {/if}

          <details class="qa dev" data-testid="settings.details.dev">
            <summary>{t('settings.advanced.dev')}</summary>
            <p class="note">{t('settings.advanced.mocks')}</p>
          </details>
        </div>
      {/if}
    </div>
  </section>
{/if}

<style>
  .settings {
    width: min(960px, 100%);
    margin: 0 auto;
    padding: var(--space-6) var(--space-4) var(--space-8);
    overflow-y: auto;
    display: grid;
    grid-template-columns: 210px minmax(0, 1fr);
    gap: var(--space-6);
    align-items: start;
  }

  .nav {
    position: sticky;
    top: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .nav-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    color: var(--color-muted-fg);
    text-align: left;
    transition:
      background var(--motion-fast) var(--ease-out),
      color var(--motion-fast) var(--ease-out);
  }

  .nav-btn:hover {
    background: var(--color-muted);
    color: var(--color-fg);
  }

  .nav-btn.active {
    background: var(--nav-active-bg);
    color: var(--color-primary-text);
  }

  .content {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-width: 0;
  }

  .content-head {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .content-title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
  }

  .content-desc {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-base-size);
  }

  .panel {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-card);
    padding: var(--space-2) var(--space-4);
    display: flex;
    flex-direction: column;
  }

  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-3) 0;
    border-bottom: 1px solid var(--color-border);
    flex-wrap: wrap;
  }

  .field:last-child {
    border-bottom: none;
  }

  .field-label {
    color: var(--color-fg);
  }

  .field.toggle {
    cursor: pointer;
  }

  .field input[type='checkbox'] {
    width: 16px;
    height: 16px;
    accent-color: var(--color-primary);
  }

  .stack {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .inline-toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-muted-fg);
    cursor: pointer;
  }

  .inline-toggle input {
    accent-color: var(--color-primary);
  }

  .seg {
    display: inline-flex;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--color-bg);
  }

  .seg-btn {
    display: inline-flex;
    align-items: center;
    min-height: var(--control-h);
    padding: 0 var(--space-3);
    color: var(--color-muted-fg);
    transition: background var(--motion-fast), color var(--motion-fast);
  }

  .seg-btn + .seg-btn {
    border-left: 1px solid var(--color-border);
  }

  .seg-btn:hover {
    background: var(--color-muted);
    color: var(--color-fg);
  }

  .seg-btn[aria-pressed='true'] {
    background: var(--color-primary);
    color: var(--color-primary-fg);
  }

  .installs {
    list-style: none;
    margin: 0 0 var(--space-2);
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    width: 100%;
  }

  .install {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    font-size: var(--text-dense-size);
  }

  .install :global(svg) {
    color: var(--color-muted-fg);
  }

  .install-name {
    flex: 1;
  }

  .install-ver {
    color: var(--color-muted-fg);
  }

  .chip {
    font-size: var(--text-meta-size);
    color: var(--color-primary-text);
    border: 1px solid var(--color-border);
    border-radius: 999px;
    padding: 0 var(--space-2);
  }

  .path {
    width: 100%;
    max-width: 460px;
    color: var(--color-muted-fg);
  }

  .pair {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .pair-item {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .pair-name {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .quality {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: var(--space-2);
    width: 100%;
  }

  .quality-opt {
    display: flex;
    flex-direction: column;
    gap: 2px;
    text-align: left;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-2) var(--space-3);
    transition: border-color var(--motion-fast) var(--ease-out);
  }

  .quality-opt:hover {
    border-color: var(--color-border-strong);
  }

  .quality-opt[aria-checked='true'] {
    border-color: var(--color-primary);
  }

  .quality-name {
    font-weight: 600;
  }

  .quality-desc {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .summary {
    margin: 0 0 var(--space-2);
    color: var(--color-muted-fg);
  }

  .note {
    margin: 0;
    padding: var(--space-2) 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    display: flex;
    align-items: flex-start;
    gap: var(--space-1);
    width: 100%;
  }

  .note :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  .ok-line {
    margin: 0;
    padding: var(--space-2) 0;
    color: var(--color-success);
    font-size: var(--text-meta-size);
  }

  .ok-line.inline {
    padding: 0;
  }

  .planned {
    padding: var(--space-3) 0;
    border-bottom: none;
  }

  .keys {
    border-collapse: collapse;
    width: 100%;
    margin-bottom: var(--space-2);
  }

  .keys td {
    padding: var(--space-1) 0;
    vertical-align: baseline;
  }

  .keys td:first-child {
    width: 150px;
    white-space: nowrap;
  }

  .kbd {
    font-family: var(--font-mono);
    font-size: var(--text-meta-size);
    border: 1px solid var(--color-border-strong);
    border-bottom-width: 2px;
    border-radius: var(--radius-sm);
    padding: 1px var(--space-1);
    background: var(--color-muted);
  }

  .kbd-sep {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .link {
    color: var(--color-primary-text);
    text-decoration: underline;
    text-underline-offset: 2px;
    align-self: flex-start;
    font-size: var(--text-meta-size);
  }

  .qa {
    width: 100%;
  }

  .qa summary {
    cursor: pointer;
    color: var(--color-muted-fg);
  }

  .qa summary:hover {
    color: var(--color-fg);
  }

  .rules {
    list-style: none;
    margin: var(--space-2) 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .dev {
    padding-top: var(--space-3);
    border-top: 1px dashed var(--color-border);
  }

  @media (max-width: 720px) {
    .settings {
      grid-template-columns: 1fr;
      gap: var(--space-4);
    }

    .nav {
      position: static;
      flex-direction: row;
      flex-wrap: wrap;
    }
  }
</style>
