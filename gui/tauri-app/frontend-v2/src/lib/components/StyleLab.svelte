<script lang="ts">
  // Style Lab panel — DEVELOPMENT-ONLY. Visibility is gated by
  // stylelab.enabled (?stylelab=1 or a localStorage opt-in); it must never
  // render in the product by default. Switches the visual axes live on the
  // same markup/data: one app, four directions.
  import Icon from './Icon.svelte';
  import { t } from '../../i18n/store.svelte';
  import { theme, type ThemeMode } from '../stores/theme.svelte';
  import {
    stylelab,
    STYLE_IDS,
    PALETTE_IDS,
    DENSITY_IDS,
    type StyleId,
    type PaletteId,
    type DensityId
  } from '../stores/stylelab.svelte';

  const PALETTE_LABEL: Record<PaletteId, string> = {
    indigo: 'stylelab.palette.indigo',
    blue: 'stylelab.palette.blue',
    emerald: 'stylelab.palette.emerald',
    amber: 'stylelab.palette.amber'
  };
</script>

<aside class="lab" data-testid="stylelab.panel" aria-label={t('stylelab.title')}>
  <details class="lab-box" open>
    <summary>
      <span class="lab-summary-icon"><Icon name="palette" size={14} /></span>
      {t('stylelab.title')}
    </summary>

    <div class="lab-body">
      <div class="lab-group" role="group" aria-label={t('stylelab.style')}>
        <span class="lab-label">{t('stylelab.style')}</span>
        <div class="lab-seg">
          {#each STYLE_IDS as id (id)}
            <button
              type="button"
              class="lab-btn"
              class:active={stylelab.style === id}
              aria-pressed={stylelab.style === id}
              data-testid={`stylelab.style.${id}`}
              onclick={() => stylelab.setStyle(id as StyleId)}
            >
              {t(`stylelab.style.${id}`)}
            </button>
          {/each}
        </div>
      </div>

      <div class="lab-group" role="group" aria-label={t('stylelab.palette')}>
        <span class="lab-label">{t('stylelab.palette')}</span>
        <div class="lab-seg">
          {#each PALETTE_IDS as id (id)}
            <button
              type="button"
              class="lab-btn"
              class:active={stylelab.palette === id}
              aria-pressed={stylelab.palette === id}
              data-testid={`stylelab.palette.${id}`}
              onclick={() => stylelab.setPalette(id as PaletteId)}
            >
              {t(PALETTE_LABEL[id])}
            </button>
          {/each}
        </div>
      </div>

      <div class="lab-group" role="group" aria-label={t('stylelab.density')}>
        <span class="lab-label">{t('stylelab.density')}</span>
        <div class="lab-seg">
          {#each DENSITY_IDS as id (id)}
            <button
              type="button"
              class="lab-btn"
              class:active={stylelab.density === id}
              aria-pressed={stylelab.density === id}
              data-testid={`stylelab.density.${id}`}
              onclick={() => stylelab.setDensity(id as DensityId)}
            >
              {t(`stylelab.density.${id}`)}
            </button>
          {/each}
        </div>
      </div>

      <div class="lab-group" role="group" aria-label={t('stylelab.theme')}>
        <span class="lab-label">{t('stylelab.theme')}</span>
        <div class="lab-seg">
          {#each ['light', 'dark'] as mode (mode)}
            <button
              type="button"
              class="lab-btn"
              class:active={theme.mode === mode}
              aria-pressed={theme.mode === mode}
              data-testid={`stylelab.theme.${mode}`}
              onclick={() => theme.set(mode as ThemeMode)}
            >
              {t(`theme.${mode}`)}
            </button>
          {/each}
        </div>
      </div>

      <label class="lab-remember">
        <input
          type="checkbox"
          data-testid="stylelab.remember"
          checked={stylelab.remember}
          onchange={(e) => stylelab.setRemember((e.currentTarget as HTMLInputElement).checked)}
        />
        {t('stylelab.remember')}
      </label>

      <p class="lab-hint">{t('stylelab.hint')}</p>
    </div>
  </details>
</aside>

<style>
  .lab {
    position: fixed;
    left: var(--space-3);
    bottom: var(--space-3);
    z-index: 10;
    max-width: 300px;
  }

  .lab-box {
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-panel);
    font-size: var(--text-meta-size);
  }

  summary {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
    color: var(--color-muted-fg);
    user-select: none;
    font-weight: 600;
    list-style: none;
  }

  summary::-webkit-details-marker {
    display: none;
  }

  .lab-summary-icon {
    display: inline-flex;
    color: var(--color-primary-text);
  }

  .lab-body {
    padding: 0 var(--space-3) var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .lab-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .lab-label {
    color: var(--color-muted-fg);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .lab-seg {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }

  .lab-btn {
    min-height: var(--control-h);
    padding: 0 var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
    color: var(--color-fg);
    transition:
      border-color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out),
      transform var(--motion-fast) var(--ease-out);
  }

  .lab-btn:hover {
    border-color: var(--color-border-strong);
    background: var(--color-muted);
  }

  .lab-btn:active {
    transform: var(--btn-press-transform);
  }

  .lab-btn.active {
    border-color: var(--color-primary);
    background: var(--nav-active-bg);
    color: var(--color-primary-text);
    font-weight: 600;
  }

  .lab-remember {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--color-fg);
  }

  .lab-remember input {
    width: 14px;
    height: 14px;
    accent-color: var(--color-primary);
  }

  .lab-hint {
    margin: 0;
    color: var(--color-muted-fg);
  }
</style>
