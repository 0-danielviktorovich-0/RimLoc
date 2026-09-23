<script lang="ts">
  // Global header per spec §2.1: app name, project + target locale (when a project
  // is open), theme triad and interface language switch.
  import Icon from './Icon.svelte';
  import { t } from '../../i18n/store.svelte';
  import type { Locale } from '../../i18n/store.svelte';
  import type { ThemeMode } from '../stores/theme.svelte';
  import { theme } from '../stores/theme.svelte';
  import { i18n } from '../../i18n/store.svelte';
  import { ui } from '../stores/ui.svelte';
  import { project } from '../stores/project.svelte';

  const THEME_MODES: ThemeMode[] = ['light', 'dark', 'system'];
  const LOCALES: Locale[] = ['ru', 'en'];

  const themeIcon: Record<ThemeMode, string> = {
    light: 'sun',
    dark: 'moon',
    system: 'monitor'
  };
</script>

<header class="app-header">
  <div class="brand">
    <span class="brand-name">{t('common.appName')}</span>
    {#if ui.screen === 'workspace'}
      <span class="brand-meta" aria-hidden="true">·</span>
      <span class="brand-project">
        {t('header.project')}: <span class="mono">{project.projectName}</span> ·
        {t('header.locale')}: <span class="mono">{project.targetLocale.toUpperCase()}</span>
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
