<script lang="ts">
  // Anchored product tour (W6, MOCK_LIVE_ONBOARDING_MANDATE §4-§5/§7): a dim
  // layer with a real highlighted control per step and a card that can go
  // Back on every step, run a guided REAL action, Skip or advance with
  // progress. Two scripts share this renderer: the W1 4-step workspace coach
  // (first open, replay from Help — behavior and testids preserved) and the
  // guided demo flow. Not a text carousel: every step points at the actual
  // screen the user is about to act on.
  // Missing anchors degrade honestly to a centered card (narrow layouts,
  // jsdom) — the tour never blocks the UI.
  import Icon from './Icon.svelte';
  import { t } from '../../i18n/store.svelte';
  import { onboarding } from '../stores/onboarding.svelte';
  import { router } from '../router.svelte';
  import { stepsFor, type TourStep } from '../onboarding/steps';

  const steps = $derived(stepsFor(onboarding.script));
  const step: TourStep = $derived(steps[Math.min(onboarding.step, steps.length - 1)]);
  const last = $derived(onboarding.step >= onboarding.total - 1);
  // Guided honesty: when this step requires performed actions and some are
  // missing, show the honest fallback copy instead of a success claim.
  const missing = $derived(step.requires ? onboarding.missingGuided(step.requires) : []);
  const honest = $derived(missing.length > 0 && !!step.honestTitleKey);
  const titleKey = $derived(honest ? (step.honestTitleKey as string) : step.titleKey);
  const textKey = $derived(honest ? (step.honestTextKey as string) : step.textKey);

  interface Box {
    top: number;
    left: number;
    width: number;
    height: number;
  }
  let box = $state<Box | null>(null);
  let cardPos = $state<{ top: number; left: number } | null>(null);

  function measure() {
    if (!onboarding.open) return;
    const selector = step.anchor;
    const el = selector ? document.querySelector<HTMLElement>(selector) : null;
    if (!el) {
      box = null;
      cardPos = null;
      return;
    }
    const r = el.getBoundingClientRect();
    if (r.width === 0 && r.height === 0) {
      // No layout information (jsdom, hidden pane) — centered fallback.
      box = null;
      cardPos = null;
      return;
    }
    const pad = 6;
    box = {
      top: Math.max(0, r.top - pad),
      left: Math.max(0, r.left - pad),
      width: r.width + pad * 2,
      height: r.height + pad * 2
    };
    // Place the card below the anchor, or above when there is no room.
    const below = r.bottom + 12;
    const cardH = 190;
    const top = below + cardH > window.innerHeight ? Math.max(12, r.top - cardH - 12) : below;
    cardPos = {
      top,
      left: Math.min(Math.max(12, r.left), Math.max(12, window.innerWidth - 372))
    };
  }

  // Re-measure whenever the step changes; navigate to the step's route only
  // when the step ENTERS (never yank the user back if they wander off
  // mid-step), then keep the anchor glued on resize/scroll.
  let enteredKey = '';
  $effect(() => {
    const key = `${onboarding.script}:${onboarding.step}:${onboarding.open}`;
    if (onboarding.open && key !== enteredKey) {
      enteredKey = key;
      if (step.route && router.route !== step.route) {
        router.navigate(step.route);
      }
    }
    measure();
    window.addEventListener('resize', measure);
    window.addEventListener('scroll', measure, true);
    return () => {
      window.removeEventListener('resize', measure);
      window.removeEventListener('scroll', measure, true);
    };
  });

  // W6 (lead 027): a guided action is recorded ONLY after it verifiably
  // completed. While it runs, the button is busy; a result landing after a
  // Skip/replay/step-change belongs to a dead pass and is discarded without
  // marking or advancing.
  // Pending is per-step: the user may move on while an earlier action is
  // still in flight — that in-flight result cannot earn its own mark, and a
  // later action is never blocked by it.
  let pendingStep = $state<string | null>(null);
  let actionFailed = $state(false);

  async function runAction() {
    const action = step.action;
    if (!action || pendingStep === step.id) return;
    pendingStep = step.id;
    actionFailed = false;
    const passId = onboarding.passId;
    const stepAt = onboarding.step;
    let ok = false;
    try {
      ok = (await action.run()) !== false;
    } catch {
      ok = false;
    }
    if (pendingStep === step.id) pendingStep = null;
    if (passId !== onboarding.passId || !onboarding.open || onboarding.step !== stepAt) {
      return; // stale completion — belongs to a pass that no longer exists
    }
    if (ok) {
      onboarding.markGuided(step.id);
      onboarding.next();
    } else {
      actionFailed = true;
    }
  }
</script>

{#if onboarding.open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="tour" data-testid="onboarding.overlay" role="dialog" aria-modal="true" aria-labelledby="onboarding-title">
    {#if box}
      <!-- Spotlight: one huge box-shadow dims everything around the anchor. -->
      <div class="spotlight" style={`top:${box.top}px;left:${box.left}px;width:${box.width}px;height:${box.height}px`} data-testid="onboarding.anchor"></div>
    {:else}
      <div class="dim"></div>
    {/if}

    <div
      class="card"
      class:anchored={cardPos}
      style={cardPos ? `top:${cardPos.top}px;left:${cardPos.left}px` : ''}
    >
      <p class="step-of" data-testid="onboarding.step-of">
        {t('onboarding.step', { step: onboarding.step + 1, total: onboarding.total })}
      </p>
      <h2 id="onboarding-title" class="title">
        <Icon name="lightbulb" size={18} />
        {t(titleKey)}
      </h2>
      <p class="text">
        {t(textKey, honest ? { list: missing.map((id) => t(`tour.missing.${id}`)).join(', ') } : undefined)}
      </p>

      {#if honest}
        <p class="honest-note" data-testid="onboarding.honest-note">{t('tour.honestNote')}</p>
      {/if}
      {#if actionFailed}
        <p class="honest-note" data-testid="onboarding.action-failed">{t('tour.actionFailed')}</p>
      {/if}

      <div class="dots" aria-hidden="true">
        {#each Array(onboarding.total) as _, i (i)}
          <span class="dot" class:active={i === onboarding.step}></span>
        {/each}
      </div>

      <div class="actions">
        <button
          type="button"
          class="btn"
          data-testid="onboarding.back"
          disabled={onboarding.step === 0}
          onclick={() => onboarding.back()}
        >
          {t('common.back')}
        </button>
        <div class="right">
          <button type="button" class="btn" data-testid="onboarding.skip" onclick={() => onboarding.finish()}>
            {t('onboarding.skip')}
          </button>
          {#if step.action}
            <!-- Guided action performs the real work and is recorded only
                 after it verifiably completes; plain Next stays available for
                 skip-ahead — the final step's honest variant then reports the
                 outstanding work instead of fake success. -->
            <button
              type="button"
              class="btn btn-primary"
              data-testid={step.action.testid}
              disabled={pendingStep === step.id}
              onclick={runAction}
            >
              {t(step.action.labelKey)}
              <Icon name="arrow-right" size={14} />
            </button>
            {#if !last}
              <button type="button" class="btn" data-testid="onboarding.next" onclick={() => onboarding.next()}>
                {t('onboarding.next')}
              </button>
            {/if}
          {:else}
            <button type="button" class="btn btn-primary" data-testid="onboarding.next" onclick={() => onboarding.next()}>
              {last ? (onboarding.script === 'demo' ? t('common.done') : t('onboarding.finish')) : t('onboarding.next')}
              <Icon name="arrow-right" size={14} />
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .tour {
    position: fixed;
    inset: 0;
    z-index: 60;
  }

  .dim {
    position: absolute;
    inset: 0;
    background: var(--color-overlay-bg, rgb(0 0 0 / 45%));
  }

  .spotlight {
    position: absolute;
    border-radius: var(--radius-md);
    border: 2px solid var(--color-primary);
    box-shadow: 0 0 0 9999px var(--color-overlay-bg, rgb(0 0 0 / 45%));
    pointer-events: none;
    transition: all var(--motion-standard) var(--ease-out);
  }

  .card {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: min(400px, calc(100vw - var(--space-8)));
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-overlay);
    padding: var(--space-5);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .card.anchored {
    position: fixed;
    transform: none;
    width: min(360px, calc(100vw - var(--space-6)));
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

  .honest-note {
    margin: 0;
    color: var(--color-warning);
    font-size: var(--text-meta-size);
    border: 1px dashed var(--color-warning);
    border-radius: var(--radius-sm);
    padding: var(--space-1) var(--space-2);
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
    margin-top: var(--space-1);
  }

  .right {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-left: auto;
  }
</style>
