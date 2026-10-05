<script lang="ts">
  // Global header per spec §2.1: app name, nav to app-level routes, project
  // meta (when a project route is open), theme triad and interface language.
  // Deliberately not overloaded (mandate §8): only Home / Settings / Help icons.
  import Icon from './Icon.svelte';
  import { t } from '../../i18n/store.svelte';
  import type { Locale } from '../../i18n/store.svelte';
  import type { ThemeMode } from '../stores/theme.svelte';
  import { theme } from '../stores/theme.svelte';
  import { i18n } from '../../i18n/store.svelte';
  import { router } from '../router.svelte';
  import { project } from '../stores/project.svelte';
  import { languages } from '../languages/store.svelte';
  import { registry } from '../languages/registry';
  import type { LanguageDefinition } from '../languages/registry';
  import { clientInstance } from '../client/instance.svelte';
  import MockBadge from './MockBadge.svelte';

  const THEME_MODES: ThemeMode[] = ['light', 'dark', 'system'];
  const LOCALES: Locale[] = ['ru', 'en'];

  /** M-5: header-safe language code — the full locale id uppercased
   * ('en' → 'EN', 'zh-Hans' → 'ZH-HANS'); the complete names stay in the
   * title tooltip of the pair. */
  function langCode(def: LanguageDefinition): string {
    return def.localeId.toUpperCase();
  }

  // M-2 (UI audit 2026-09-29): with no project open the chip tells the
  // TRANSPORT truth (Live/Mock by the resolved client mode). Mode 'none'
  // never reaches this header — the boot gate replaces the whole shell.
  const transportMode = clientInstance.resolveMode();

  const themeIcon: Record<ThemeMode, string> = {
    light: 'sun',
    dark: 'moon',
    system: 'monitor'
  };

  const NAV_ITEMS = [
    { route: 'home', icon: 'home', key: 'nav.home' },
    { route: 'settings', icon: 'settings', key: 'nav.settings' },
    { route: 'help', icon: 'help', key: 'nav.help' }
  ] as const;
</script>

<header class="app-header">
  <div class="brand">
    <span class="brand-name">{t('common.appName')}</span>

    <!-- W6: global honesty chip — visible on every route and every build.
         Audit P1-4: on a real contract project it is the live badge.
         Audit M-2 (2026-09-29): with no project open it states the
         TRANSPORT (Live/Mock by client mode) — the live recents on Home are
         no longer called demo data. The «Demo data (mock)» label stays
         scoped to the bundled demo dataset opened in the workspace. -->
    {#if project.source === 'contract'}
      <span class="live-badge" data-testid="live-badge" title={t('livebadge.title')}>
        <Icon name="circle-check" size={12} />
        {t('livebadge.label')}
      </span>
    {:else if project.isDemo}
      <MockBadge />
    {:else if transportMode === 'tauri'}
      <span
        class="live-badge"
        data-testid="transport-live-badge"
        title={t('transportbadge.live.title')}
      >
        <Icon name="circle-check" size={12} />
        {t('transportbadge.live.label')}
      </span>
    {:else}
      <span
        class="mock-transport"
        data-testid="transport-mock-badge"
        title={t('transportbadge.mock.title')}
      >
        <Icon name="info" size={12} />
        {t('transportbadge.mock.label')}
      </span>
    {/if}

    <nav class="nav" aria-label={t('nav.home')}>
      {#each NAV_ITEMS as item (item.route)}
        <button
          type="button"
          class="nav-btn"
          aria-label={t(item.key)}
          title={t(item.key)}
          aria-current={router.route === item.route ? 'page' : undefined}
          data-testid={`nav.${item.route}`}
          onclick={() => router.navigate(item.route)}
        >
          <Icon name={item.icon} size={14} />
        </button>
      {/each}
    </nav>

    {#if router.isProjectRoute()}
      <span class="brand-meta" aria-hidden="true">·</span>
      <span class="brand-project" data-testid="header.project">
        {t('header.project')}: <span class="mono">{project.displayName}</span> ·
        <!-- Multi-target (W2): the pair follows the ACTIVE target locale, not a
             fixed one; it stays independent of the interface language below. -->
        <!-- M-5 (UI audit 2026-09-29): compact CODES so the header stops
             truncating («English → Ру…»), full names ride the title hint. -->
        <span
          class="mono"
          data-testid="header.langpair"
          title="{registry.resolve(languages.sourceLocale).nativeName} → {languages.activeDefinition.nativeName}"
        >
          {langCode(registry.resolve(languages.sourceLocale))} → {langCode(languages.activeDefinition)}
        </span> ·
        {t('workspace.meta.version')}
      </span>
    {/if}
  </div>

  <div class="controls">
    <div class="seg" role="group" aria-label={t('theme.label')}>
      {#each THEME_MODES as mode (mode)}
        <button
          type="button"
          class="seg-btn"
          aria-pressed={theme.mode === mode}
          aria-label={t(`theme.${mode}`)}
          title={t(`theme.${mode}`)}
          data-testid={`theme.${mode}`}
          onclick={() => theme.set(mode)}
        >
          <Icon name={themeIcon[mode]} />
          <span class="seg-text">{t(`theme.${mode}`)}</span>
        </button>
      {/each}
    </div>

    <div class="seg" role="group" aria-label={t('lang.label')}>
      {#each LOCALES as locale (locale)}
        <button
          type="button"
          class="seg-btn"
          aria-pressed={i18n.locale === locale}
          aria-label={locale === 'ru' ? 'Русский' : 'English'}
          data-testid={`lang.${locale}`}
          onclick={() => i18n.setLocale(locale)}
        >
          {locale === 'ru' ? 'RU' : 'EN'}
        </button>
      {/each}
    </div>
  </div>
</header>

<style>
  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface);
    flex: none;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .brand-name {
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
  }

  .nav {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    margin-left: var(--space-2);
  }

  .nav-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: var(--control-h);
    height: var(--control-h);
    border-radius: var(--radius-sm);
    color: var(--color-muted-fg);
    transition:
      background var(--motion-fast) var(--ease-out),
      color var(--motion-fast) var(--ease-out);
  }

  .nav-btn:hover {
    background: var(--color-muted);
    color: var(--color-fg);
  }

  .nav-btn[aria-current='page'] {
    background: var(--nav-active-bg);
    color: var(--color-primary-text);
  }

  .brand-project {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .brand-meta {
    color: var(--color-muted-fg);
  }

  /* Audit P1-4: the honest live counterpart of the mock badge — same chip
     shape, calm positive color instead of the warning tone. */
  .live-badge {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 2px var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    white-space: nowrap;
    cursor: help;
  }

  /* Audit M-2: the mock-TRANSPORT counterpart — MockBadge's dashed warning
     shape, since the client is on the explicit demo transport. */
  .mock-transport {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 2px var(--space-2);
    border: 1px dashed var(--color-warning);
    border-radius: var(--radius-sm);
    color: var(--color-warning);
    font-size: var(--text-meta-size);
    white-space: nowrap;
    cursor: help;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex: none;
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
    gap: var(--space-1);
    min-height: 32px;
    padding: 0 var(--space-2);
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
</style>
