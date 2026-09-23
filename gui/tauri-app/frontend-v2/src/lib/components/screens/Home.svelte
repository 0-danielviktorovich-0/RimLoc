<script lang="ts">
  // Home screen per spec §2.2: two equal-weight entry cards (no "primary + secondary"
  // hierarchy), mock loading/error states driven by the dev panel.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { ui, type SimpleState } from '../../stores/ui.svelte';
  import { MOCK_ERROR_CODE, MOCK_ERROR_RAW } from '../../../lib/mock/data';

  let { state = 'ready' }: { state?: SimpleState } = $props();

  function enterWorkspace() {
    ui.openWorkspace();
  }
</script>

<section class="home" aria-labelledby="home-heading">
  <h1 id="home-heading" class="home-title">{t('home.title')}</h1>

  {#if state === 'loading'}
    <p class="state-line" role="status">{t('home.state.loading')}</p>
    <div class="cards">
      <div class="card card-skeleton" aria-hidden="true"></div>
      <div class="card card-skeleton" aria-hidden="true"></div>
    </div>
  {:else if state === 'error'}
    <div class="error-card" role="alert">
      <p class="error-title">{t('common.errorTitle')}</p>
      <p>{t('home.state.error')}</p>
      <p class="mono error-code">{MOCK_ERROR_CODE}</p>
      <button type="button" class="btn" onclick={() => (ui.homeState = 'ready')}>
        {t('common.retry')}
      </button>
      <details class="raw">
        <summary>{t('common.details')}</summary>
        <pre class="mono">{MOCK_ERROR_RAW}</pre>
      </details>
    </div>
  {:else}
    <div class="cards">
      <button
        type="button"
        class="card entry-card"
        data-testid="home.entry-new"
        onclick={enterWorkspace}
      >
        <span class="card-icon"><Icon name="file-plus" size={28} /></span>
        <span class="card-title">{t('home.entryNew.title')}</span>
        <span class="card-desc">{t('home.entryNew.description')}</span>
      </button>

      <button
        type="button"
        class="card entry-card"
        data-testid="home.entry-existing"
        onclick={enterWorkspace}
      >
        <span class="card-icon"><Icon name="folder-open" size={28} /></span>
        <span class="card-title">{t('home.entryExisting.title')}</span>
        <span class="card-desc">{t('home.entryExisting.description')}</span>
      </button>
    </div>
    <!-- Recent projects: intentionally not rendered while the list is empty (spec §2.2). -->
  {/if}
</section>

<style>
  .home {
    width: min(860px, 100%);
    margin: 0 auto;
    padding: var(--space-8) var(--space-4);
  }

  .home-title {
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
    margin: 0 0 var(--space-6);
  }

  .state-line {
    color: var(--color-muted-fg);
  }

  .cards {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-4);
  }

  @media (max-width: 640px) {
    .cards {
      grid-template-columns: 1fr;
    }
  }

  .card {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    color: var(--color-surface-fg);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    text-align: left;
    min-height: 148px;
    box-shadow: var(--shadow-card);
    transition:
      border-color var(--motion-fast) var(--ease-out),
      box-shadow var(--motion-fast) var(--ease-out),
      transform var(--motion-fast) var(--ease-out);
  }

  .entry-card:hover {
    border-color: var(--color-primary);
    box-shadow: var(--shadow-hover);
    transform: var(--card-hover-transform);
  }

  .entry-card:active {
    transform: var(--btn-press-transform);
  }

  .card-icon {
    color: var(--card-icon-fg);
    background: var(--card-icon-bg);
    padding: var(--card-icon-pad);
    border-radius: var(--card-icon-radius);
    display: inline-flex;
    /* Keep the icon chip hug-content even when it carries a filled background. */
    align-self: flex-start;
    margin-bottom: var(--space-2);
  }

  .card-title {
    font-family: var(--font-heading);
    font-size: 16px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .card-desc {
    color: var(--color-muted-fg);
    font-size: var(--text-base-size);
  }

  .card-skeleton {
    background: var(--color-muted);
    border: none;
    min-height: 148px;
  }

  .error-card {
    border: 1px solid var(--color-border);
    border-left: 3px solid var(--color-destructive);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    align-items: flex-start;
  }

  .error-title {
    font-weight: 600;
    margin: 0;
  }

  .error-code {
    color: var(--color-muted-fg);
    margin: 0;
  }

  .raw {
    width: 100%;
  }

  .raw pre {
    background: var(--color-muted);
    border-radius: var(--radius-sm);
    padding: var(--space-2);
    overflow: auto;
  }
</style>
