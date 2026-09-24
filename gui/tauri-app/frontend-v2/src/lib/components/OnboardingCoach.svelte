<script lang="ts">
  // Contextual onboarding coach (QA mandate §20): 4-step skippable overlay on
  // the first Workspace open, replayable from Help. Visibility and persistence
  // live in stores/onboarding.svelte.ts; this component only renders steps.
  import Icon from './Icon.svelte';
  import { t } from '../../i18n/store.svelte';
  import { onboarding } from '../stores/onboarding.svelte';

  const STEPS = [
    { title: 'onboarding.s1.title', text: 'onboarding.s1.text', icon: 'search' },
    { title: 'onboarding.s2.title', text: 'onboarding.s2.text', icon: 'edit' },
    { title: 'onboarding.s3.title', text: 'onboarding.s3.text', icon: 'filter' },
    { title: 'onboarding.s4.title', text: 'onboarding.s4.text', icon: 'package' }
  ] as const;

  const last = $derived(onboarding.step === STEPS.length - 1);
</script>

{#if onboarding.open}
  <div
    class="overlay"
    data-testid="onboarding.overlay"
    role="dialog"
    aria-modal="true"
    aria-labelledby="onboarding-title"
  >
    <div class="card">
      <p class="step-of" data-testid="onboarding.step-of">
        {t('onboarding.step', { step: onboarding.step + 1, total: STEPS.length })}
      </p>
      <h2 id="onboarding-title" class="title">
        <Icon name={STEPS[onboarding.step].icon} size={18} />
        {t(STEPS[onboarding.step].title)}
      </h2>
      <p class="text">{t(STEPS[onboarding.step].text)}</p>

      <div class="dots" aria-hidden="true">
        {#each STEPS as _, i (i)}
          <span class="dot" class:active={i === onboarding.step}></span>
        {/each}
      </div>

      <div class="actions">
        <button
          type="button"
          class="btn"
          data-testid="onboarding.skip"
          onclick={() => onboarding.finish()}
        >
          {t('onboarding.skip')}
        </button>
        <button
          type="button"
          class="btn btn-primary"
          data-testid="onboarding.next"
          onclick={() => onboarding.next(STEPS.length)}
        >
          {last ? t('onboarding.finish') : t('onboarding.next')}
          <Icon name="arrow-right" size={14} />
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 60;
    background: var(--color-overlay-bg, rgb(0 0 0 / 45%));
    display: grid;
    place-items: center;
    padding: var(--space-4);
  }

  .card {
    width: min(440px, 100%);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-overlay);
    padding: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .step-of {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
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

  .title :global(svg) {
    color: var(--color-primary-text);
  }

  .text {
    margin: 0;
    color: var(--color-muted-fg);
  }

  .dots {
    display: flex;
    gap: var(--space-1);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: var(--color-muted);
  }

  .dot.active {
    background: var(--color-primary);
  }

  .actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
</style>
