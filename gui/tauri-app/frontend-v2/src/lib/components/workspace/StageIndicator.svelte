<script lang="ts">
  // Lifecycle indicator (mandate §19): Translate → Review → Validate → Build.
  // A light answer to "where am I?" — the active stage is marked with
  // aria-current="step" and never relies on color alone (chevron + label).
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';

  export interface StageProps {
    stage: 'translate' | 'review' | 'validate' | 'build';
  }

  let { stage }: StageProps = $props();

  type Stage = StageProps['stage'];

  const STAGES: Stage[] = ['translate', 'review', 'validate', 'build'];
</script>

<ol class="stages" aria-label={t('stage.label')} data-testid="workspace.stage">
  {#each STAGES as s, i (s)}
    <li class="stage" aria-current={stage === s ? 'step' : undefined}>
      <span class="num" aria-hidden="true">{i + 1}</span>
      <span class="label">{t(`stage.${s}`)}</span>
      {#if i < STAGES.length - 1}
        <span class="sep" aria-hidden="true"><Icon name="chevron-right" size={12} /></span>
      {/if}
    </li>
  {/each}
</ol>

<style>
  .stages {
    list-style: none;
    display: flex;
    align-items: center;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
  }

  .stage {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .num {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 1px solid var(--color-border-strong);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }

  .stage[aria-current='step'] {
    color: var(--color-primary-text);
    font-weight: 600;
  }

  .stage[aria-current='step'] .num {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: var(--color-primary-fg);
  }

  .sep {
    margin-left: var(--space-1);
    display: inline-flex;
    opacity: 0.6;
  }
</style>
