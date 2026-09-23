<script lang="ts">
  // Dev-only panel: mock state toggles per spec §3 (every screen must expose
  // loading / empty / error) + dataset reset. Not part of the product UI.
  import { t } from '../../i18n/store.svelte';
  import { ui } from '../stores/ui.svelte';
  import { project } from '../stores/project.svelte';
</script>

<details class="dev" data-testid="dev.panel">
  <summary>{t('dev.title')}</summary>
  <div class="dev-body">
    <label class="dev-row">
      <span>{t('dev.screen')}</span>
      <select
        data-testid="dev.screen"
        value={ui.screen}
        onchange={(e) => (ui.screen = (e.currentTarget as HTMLSelectElement).value as 'home' | 'workspace')}
      >
        <option value="home">{t('dev.screen.home')}</option>
        <option value="workspace">{t('dev.screen.workspace')}</option>
      </select>
    </label>

    <label class="dev-row">
      <span>{t('dev.homeState')}</span>
      <select
        data-testid="dev.home-state"
        value={ui.homeState}
        onchange={(e) =>
          (ui.homeState = (e.currentTarget as HTMLSelectElement).value as 'ready' | 'loading' | 'error')}
      >
        <option value="ready">{t('dev.homeState.ready')}</option>
        <option value="loading">{t('dev.homeState.loading')}</option>
        <option value="error">{t('dev.homeState.error')}</option>
      </select>
    </label>

    <label class="dev-row">
      <span>{t('dev.wsState')}</span>
      <select
        data-testid="dev.ws-state"
        value={ui.wsState}
        onchange={(e) =>
          (ui.wsState = (e.currentTarget as HTMLSelectElement).value as
            | 'ready'
            | 'loading'
            | 'empty'
            | 'error')}
      >
        <option value="ready">{t('dev.wsState.ready')}</option>
        <option value="loading">{t('dev.wsState.loading')}</option>
        <option value="empty">{t('dev.wsState.empty')}</option>
        <option value="error">{t('dev.wsState.error')}</option>
      </select>
    </label>

    <button type="button" class="btn" data-testid="dev.reset" onclick={() => project.reset()}>
      {t('dev.reset')}
    </button>

    <p class="dev-note">{t('dev.summary')}</p>
  </div>
</details>

<style>
  .dev {
    position: fixed;
    right: var(--space-3);
    bottom: var(--space-3);
    z-index: 10;
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: 0 4px 16px rgb(0 0 0 / 18%);
    font-size: var(--text-meta-size);
    max-width: 260px;
  }

  summary {
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
    color: var(--color-muted-fg);
    user-select: none;
  }

  .dev-body {
    padding: 0 var(--space-3) var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .dev-row {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: var(--space-2);
  }

  .dev-row span {
    color: var(--color-muted-fg);
  }

  .dev-note {
    margin: 0;
    color: var(--color-muted-fg);
  }
</style>
