<script lang="ts">
  // AI Provider Manager (mandate §6, spec §12): one card per provider with
  // status, model, connection test and secure-key messaging. Mocks only —
  // no provider is ever contacted, and no secret is ever displayed: the key
  // row shows a flag ("stored in the keychain"), never a value.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { router } from '../../router.svelte';
  import { providers, type ProviderId, type ProviderStatus } from '../../stores/providers.svelte';

  let configuring = $state<ProviderId | null>(null);
  let modelEditing = $state<ProviderId | null>(null);
  let baseUrlDraft = $state('');
  /** Transient per-card feedback line (e.g. "connection OK"), keyed by provider. */
  let flash = $state<Partial<Record<ProviderId, string>>>({});
  let flashTimer: ReturnType<typeof setTimeout> | undefined;

  const STATUS_ICON: Record<ProviderStatus, string> = {
    connected: 'circle-check',
    not_configured: 'info',
    offline: 'warning',
    testing: 'clock'
  };

  function showFlash(id: ProviderId, key: string) {
    flash = { ...flash, [id]: key };
    clearTimeout(flashTimer);
    flashTimer = setTimeout(() => {
      flash = { ...flash, [id]: undefined };
    }, 2200);
  }

  function openConfigure(id: ProviderId) {
    configuring = configuring === id ? null : id;
    baseUrlDraft = providers.byId(id).baseUrl;
  }

  function saveConfigure(id: ProviderId) {
    const p = providers.byId(id);
    providers.setBaseUrl(id, baseUrlDraft.trim() || p.baseUrl);
    configuring = null;
    // Mock: a saved cloud credential counts as configured; the local mock
    // service stays down until a test succeeds (which it never does here).
    if (p.privacy === 'cloud' && !p.hasKey) providers.setHasKey(id, true);
    providers.setStatus(id, p.privacy === 'local' ? 'offline' : 'connected');
  }

  function testConnection(id: ProviderId) {
    providers.test(id);
  }

  function changeModel(id: ProviderId, model: string) {
    providers.setModel(id, model);
    showFlash(id, 'providers.test.modelSet');
  }

  function replaceKey(id: ProviderId) {
    // Mock: pretends the OS keychain prompt succeeded.
    providers.setHasKey(id, true);
    showFlash(id, 'providers.test.keySaved');
  }
</script>

<section class="providers" aria-labelledby="providers-heading">
  <div class="head">
    <h1 id="providers-heading" class="title">
      <Icon name="cpu" size={20} />
      {t('providers.title')}
    </h1>
    <p class="subtitle">{t('providers.subtitle')}</p>
  </div>

  <button type="button" class="btn back" data-testid="providers.back" onclick={() => router.navigate('settings')}>
    <Icon name="arrow-left" size={14} />
    {t('nav.settings')}
  </button>

  <div class="cards">
    {#each providers.list as p (p.id)}
      <article class="card" data-testid={`providers.card.${p.id}`}>
        <header class="card-head">
          <span class="card-icon"><Icon name="cpu" size={18} /></span>
          <div class="card-title">
            <span class="name">{t(p.nameKey)}</span>
            <span class="desc">{t(p.descKey)}</span>
          </div>
          <span class={`status st-${p.status}`} data-testid={`providers.status.${p.id}`}>
            <Icon name={STATUS_ICON[p.status]} size={14} />
            {t(`providers.status.${p.status}`)}
          </span>
        </header>

        <dl class="meta">
          <div class="meta-row">
            <dt>{t('providers.model')}</dt>
            <dd class="model-cell">
              {#if modelEditing === p.id}
                <select
                  value={p.model}
                  data-testid={`providers.model.${p.id}`}
                  aria-label={t('providers.action.changeModel')}
                  onchange={(e) => {
                    changeModel(p.id, (e.currentTarget as HTMLSelectElement).value);
                    modelEditing = null;
                  }}
                >
                  {#each p.models as m (m)}
                    <option value={m}>{m}</option>
                  {/each}
                </select>
                <button type="button" class="btn" onclick={() => (modelEditing = null)}>
                  {t('common.cancel')}
                </button>
              {:else}
                <span class="mono">{p.model}</span>
                <button
                  type="button"
                  class="btn subtle"
                  data-testid={`providers.action.changeModel.${p.id}`}
                  onclick={() => (modelEditing = p.id)}
                >
                  {t('providers.action.changeModel')}
                </button>
              {/if}
            </dd>
          </div>
          <div class="meta-row">
            <dt>{t('providers.baseUrl')}</dt>
            <dd class="mono">{p.baseUrl}</dd>
          </div>
        </dl>

        <div class="actions">
          <button type="button" class="btn" data-testid={`providers.action.configure.${p.id}`} aria-expanded={configuring === p.id} onclick={() => openConfigure(p.id)}>
            <Icon name="sliders" size={14} />
            {t('providers.action.configure')}
          </button>
          <button
            type="button"
            class="btn"
            data-testid={`providers.action.test.${p.id}`}
            disabled={p.status === 'testing'}
            onclick={() => testConnection(p.id)}
          >
            <Icon name={p.status === 'testing' ? 'clock' : 'play'} size={14} />
            {t('providers.action.test')}
          </button>
        </div>

        {#if configuring === p.id}
          <div class="configure" data-testid={`providers.configure.${p.id}`}>
            <div class="key-row">
              <Icon name="check" size={14} />
              <div class="key-text">
                <span>{t('providers.key.hint')}</span>
                <span class="key-state" data-testid={`providers.keystate.${p.id}`}>
                  {p.hasKey ? '•••• •••• (keychain)' : t('providers.key.none')}
                </span>
              </div>
              <button type="button" class="btn" data-testid={`providers.action.replaceKey.${p.id}`} onclick={() => replaceKey(p.id)}>
                {t('providers.key.replace')}
              </button>
            </div>
            <label class="url-row">
              <span>{t('providers.baseUrl')}</span>
              <input type="text" bind:value={baseUrlDraft} data-testid={`providers.baseurl.${p.id}`} spellcheck="false" />
            </label>
            <div class="configure-actions">
              <button type="button" class="btn btn-primary" data-testid={`providers.action.save.${p.id}`} onclick={() => saveConfigure(p.id)}>
                <Icon name="check" size={14} />
                {t('providers.action.save')}
              </button>
              <button type="button" class="btn" onclick={() => (configuring = null)}>
                {t('common.cancel')}
              </button>
            </div>
          </div>
        {/if}

        {#if flash[p.id]}
          <p class="flash" role="status">{t(flash[p.id]!)}</p>
        {:else if p.status === 'offline'}
          <p class="hint warn"><Icon name="warning" size={13} /> {t('providers.offline.hint')}</p>
        {/if}

        <footer class="footnotes">
          <p class="note"><Icon name="info" size={13} /> {t(p.privacy === 'local' ? 'providers.privacy.local' : 'providers.privacy.cloud')}</p>
          {#if p.privacy === 'cloud'}
            <p class="note cost"><Icon name="warning" size={13} /> {t('providers.cost.warn')}</p>
          {/if}
        </footer>
      </article>
    {/each}
  </div>

  <p class="zero" data-testid="providers.zero"><Icon name="circle-check" size={14} /> {t('providers.zero')}</p>
</section>

<style>
  .providers {
    width: min(880px, 100%);
    margin: 0 auto;
    padding: var(--space-8) var(--space-4);
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
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .subtitle {
    margin: 0;
    color: var(--color-muted-fg);
  }

  .back {
    align-self: flex-start;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
    gap: var(--space-4);
    align-items: start;
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

  .card-head {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
  }

  .card-icon {
    color: var(--card-icon-fg);
    background: var(--card-icon-bg);
    padding: var(--card-icon-pad);
    border-radius: var(--card-icon-radius);
    display: inline-flex;
  }

  .card-title {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .name {
    font-family: var(--font-heading);
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .desc {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-meta-size);
    white-space: nowrap;
    padding: 2px var(--space-2);
    border-radius: 999px;
    border: 1px solid var(--color-border);
  }

  .st-connected {
    color: var(--color-success);
    border-color: var(--color-success);
  }

  .st-not_configured {
    color: var(--color-muted-fg);
  }

  .st-offline,
  .st-testing {
    color: var(--color-warning);
    border-color: var(--color-warning);
  }

  .meta {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .meta-row {
    display: grid;
    grid-template-columns: 88px 1fr;
    gap: var(--space-2);
    align-items: baseline;
  }

  .meta-row dt {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .meta-row dd {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .model-cell {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .btn.subtle {
    border-color: transparent;
    color: var(--color-primary-text);
    min-height: 26px;
    padding: 0 var(--space-2);
    font-size: var(--text-meta-size);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .configure {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .key-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
  }

  .key-row > :global(svg) {
    flex: none;
    margin-top: 2px;
    color: var(--color-muted-fg);
  }

  .key-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
    min-width: 0;
  }

  .key-state {
    font-family: var(--font-mono);
    color: var(--color-fg);
  }

  .url-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
  }

  .configure-actions {
    display: flex;
    gap: var(--space-2);
  }

  .flash {
    margin: 0;
    color: var(--color-success);
    font-size: var(--text-meta-size);
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .hint {
    margin: 0;
    font-size: var(--text-meta-size);
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .hint.warn {
    color: var(--color-warning);
  }

  .footnotes {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    border-top: 1px solid var(--color-border);
    padding-top: var(--space-2);
  }

  .note {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    display: flex;
    align-items: flex-start;
    gap: var(--space-1);
  }

  .note :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  .note.cost :global(svg) {
    color: var(--color-warning);
  }

  .zero {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .zero :global(svg) {
    color: var(--color-success);
  }
</style>
