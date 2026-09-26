<script lang="ts">
  // Dev-only panel: route switcher, mock state toggles per spec §3 (every
  // screen must expose loading / empty / error) + dataset reset. Not part of
  // the product UI.
  import { t, i18n } from '../../i18n/store.svelte';
  import { router, ROUTES, type RouteId } from '../router.svelte';
  import { ui, type HomeMode } from '../stores/ui.svelte';
  import { project } from '../stores/project.svelte';
  import { scenarioBrowser } from '../scenarios.svelte';

  // Self-localization wave B2: explicit data-only language pack preview.
  // The file is read fully client-side; its JSON text goes to the store's
  // typed loader, which accepts the pack whole or rejects it whole with a
  // machine reason (rendered as diagnostic data, not copy).
  // The File object itself is deliberately NOT $state: Svelte would wrap it
  // in a reactive proxy, and a proxied Blob fails the brand checks its
  // .text() relies on. Only cheap flags are reactive.
  let selectedPackFile: File | null = null;
  let hasPackFile = $state(false);
  let packRejectReason: string | null = $state(null);

  function onPackFile(e: Event) {
    selectedPackFile = (e.currentTarget as HTMLInputElement).files?.[0] ?? null;
    hasPackFile = selectedPackFile !== null;
    packRejectReason = null;
  }

  async function loadPack() {
    if (!selectedPackFile) return;
    const text = await selectedPackFile.text();
    const result = i18n.previewPack(text);
    packRejectReason = result.ok ? null : result.reason;
  }
</script>

<details class="dev" data-testid="dev.panel">
  <summary>{t('dev.title')}</summary>
  <div class="dev-body">
    <label class="dev-row">
      <span>{t('dev.route')}</span>
      <select
        data-testid="dev.route"
        value={router.route}
        onchange={(e) => router.navigate((e.currentTarget as HTMLSelectElement).value as RouteId)}
      >
        {#each ROUTES as r (r)}
          <option value={r}>{r}</option>
        {/each}
      </select>
    </label>

    <label class="dev-row">
      <span>{t('dev.homeMode')}</span>
      <select
        data-testid="dev.home-mode"
        value={ui.homeMode}
        onchange={(e) => (ui.homeMode = (e.currentTarget as HTMLSelectElement).value as HomeMode)}
      >
        <option value="returning">{t('dev.homeMode.returning')}</option>
        <option value="first-run">{t('dev.homeMode.first-run')}</option>
        <option value="no-mods">{t('dev.homeMode.no-mods')}</option>
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

    <!-- Self-localization wave B2: language pack preview. Data-only pack,
         explicit load + explicit reset; the preview never persists. -->
    <label class="dev-row">
      <span>{t('dev.pack.pick')}</span>
      <input
        type="file"
        accept=".json,application/json"
        data-testid="dev.pack.file"
        onchange={onPackFile}
      />
    </label>

    <button
      type="button"
      class="btn"
      data-testid="dev.pack.load"
      disabled={!hasPackFile}
      onclick={loadPack}
    >
      {t('dev.pack.load')}
    </button>

    {#if i18n.previewActive}
      <button
        type="button"
        class="btn"
        data-testid="dev.pack.reset"
        onclick={() => i18n.clearPreview()}
      >
        {t('dev.pack.reset')}
      </button>
      <p class="dev-note" data-testid="dev.pack.status">
        {t('dev.pack.active', { locale: i18n.preview?.locale ?? '', count: i18n.preview?.count ?? 0 })}
      </p>
    {:else if packRejectReason}
      <p class="dev-note" data-testid="dev.pack.status">
        {t('dev.pack.rejected', { reason: packRejectReason })}
      </p>
    {:else}
      <p class="dev-note" data-testid="dev.pack.status">{t('dev.pack.idle')}</p>
    {/if}

    <!-- W6: dev scenario browser (dev-only tooling, mandate §11). -->
    <button type="button" class="btn" data-testid="dev.scenarios" onclick={() => scenarioBrowser.show()}>
      {t('dev.scenarios')}
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
