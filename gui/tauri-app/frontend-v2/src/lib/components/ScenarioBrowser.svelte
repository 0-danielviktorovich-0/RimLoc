<script lang="ts">
  // Dev scenario browser UI (W6, MOCK_LIVE_ONBOARDING_MANDATE §11): the
  // dev-panel opens this dialog; every entry shows its stable id and the
  // deep-link that reproduces it. Deep-links are applied by the module-level
  // runner in scenarios.svelte.ts (registered before component effects), so
  // this component is only the PICKER — it can be closed at any time.
  import Icon from './Icon.svelte';
  import { t } from '../../i18n/store.svelte';
  import {
    SCENARIOS,
    scenarioBrowser,
    scenarioHref,
    runScenario,
    findScenario
  } from '../scenarios.svelte';

  const GROUPS = [
    'home',
    'wizard',
    'workspace',
    'review',
    'build',
    'settings',
    'onboarding',
    'diagnostics'
  ] as const;

  function pick(id: string) {
    const scenario = findScenario(id);
    if (!scenario) return;
    runScenario(scenario);
    scenarioBrowser.hide();
  }
</script>

{#if scenarioBrowser.open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="overlay"
    data-testid="scenarios.overlay"
    onpointerdown={(e) => {
      if (e.target === e.currentTarget) scenarioBrowser.hide();
    }}
  >
    <div class="dialog" role="dialog" aria-modal="true" aria-label={t('scenarios.title')} data-testid="scenarios.dialog">
      <div class="head">
        <h2 class="title">{t('scenarios.title')}</h2>
        <button type="button" class="btn" data-testid="scenarios.close" onclick={() => scenarioBrowser.hide()}>
          <Icon name="close" size={14} />
          {t('common.close')}
        </button>
      </div>
      <p class="hint">{t('scenarios.hint')}</p>

      <div class="body">
        {#each GROUPS as group (group)}
          {@const items = SCENARIOS.filter((s) => s.group === group)}
          {#if items.length > 0}
            <h3 class="group">{t(`scenarios.group.${group}`)}</h3>
            <ul class="list">
              {#each items as scenario (scenario.id)}
                <li class="item">
                  <button type="button" class="run" data-testid={`scenarios.item.${scenario.id}`} onclick={() => pick(scenario.id)}>
                    <span class="label">{t(scenario.labelKey)}</span>
                    <span class="mono link-path">{scenarioHref(scenario)}</span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 900;
    background: var(--color-overlay-bg, rgb(0 0 0 / 45%));
    display: grid;
    place-items: center;
    padding: var(--space-4);
  }

  .dialog {
    width: min(560px, 100%);
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    background: var(--color-surface);
    color: var(--color-surface-fg);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-overlay);
    overflow: hidden;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
  }

  .hint {
    margin: 0;
    padding: var(--space-2) var(--space-4);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    border-bottom: 1px solid var(--color-border);
  }

  .body {
    overflow-y: auto;
    padding: var(--space-2) var(--space-4) var(--space-4);
  }

  .group {
    margin: var(--space-3) 0 var(--space-1);
    font-size: var(--text-meta-size);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-muted-fg);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .item {
    display: flex;
  }

  .run {
    width: 100%;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    cursor: pointer;
    text-align: left;
    transition: border-color var(--motion-fast) var(--ease-out);
  }

  .run:hover {
    border-color: var(--color-primary);
  }

  .label {
    font-size: var(--text-base-size);
    color: var(--color-surface-fg);
  }

  .link-path {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    word-break: break-all;
  }
</style>
