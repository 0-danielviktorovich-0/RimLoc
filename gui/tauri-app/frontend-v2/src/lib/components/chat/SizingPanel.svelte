<script lang="ts">
  // Batch sizing: Auto (default) plus Small/Medium/Large presets, with a
  // collapsed Advanced block for the four budgets (entries, token budget,
  // output tokens, characters). Presets only prefill the budgets; editing
  // them stays possible. Everything is mock-local.
  import { t } from '../../../i18n/store.svelte';
  import { chat, type SizingPreset } from '../../stores/chatbatch.svelte';

  const PRESETS: Array<{ id: SizingPreset; key: string }> = [
    { id: 'auto', key: 'chat.sizing.auto' },
    { id: 'small', key: 'chat.sizing.small' },
    { id: 'medium', key: 'chat.sizing.medium' },
    { id: 'large', key: 'chat.sizing.large' }
  ];

  const FIELDS: Array<{ key: string; hint: string; field: keyof typeof chat.advanced }> = [
    { key: 'chat.sizing.entries', hint: 'chat.sizing.entriesHint', field: 'entries' },
    { key: 'chat.sizing.tokenBudget', hint: 'chat.sizing.tokenBudgetHint', field: 'tokenBudget' },
    { key: 'chat.sizing.outputTokens', hint: 'chat.sizing.outputTokensHint', field: 'outputTokens' },
    { key: 'chat.sizing.characters', hint: 'chat.sizing.charactersHint', field: 'characters' }
  ];
</script>

<section class="panel" aria-labelledby="sizing-heading">
  <h2 id="sizing-heading">{t('chat.sizing.title')}</h2>

  <fieldset class="presets">
    <legend class="visually-hidden">{t('chat.sizing.title')}</legend>
    {#each PRESETS as p (p.id)}
      <label class="preset">
        <input
          type="radio"
          name="chat-sizing"
          value={p.id}
          checked={chat.sizing === p.id}
          data-testid={`chat.sizing.${p.id}`}
          onchange={() => chat.setPreset(p.id)}
        />
        <span class="preset-name">
          {t(p.key)}
          {#if p.id === 'auto'}<span class="rec">{t('chat.summary.recommended')}</span>{/if}
        </span>
      </label>
    {/each}
  </fieldset>

  {#if chat.sizing === 'auto'}
    <p class="hint" data-testid="chat.sizing.auto-hint">{t('chat.sizing.autoHint')}</p>
  {/if}

  <details class="advanced">
    <summary data-testid="chat.sizing.advanced-toggle">{t('chat.sizing.advanced')}</summary>
    <div class="fields">
      {#each FIELDS as f (f.field)}
        <label class="field">
          <span class="field-label">{t(f.key)}</span>
          <input
            type="number"
            min="1"
            bind:value={chat.advanced[f.field]}
            data-testid={`chat.sizing.field.${f.field}`}
          />
          <span class="hint">{t(f.hint)}</span>
        </label>
      {/each}
    </div>
  </details>
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
  }

  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .presets {
    margin: 0;
    padding: 0;
    border: none;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-2);
  }

  .preset {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: var(--control-h);
    padding: var(--space-1) var(--space-2);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    cursor: pointer;
    transition:
      border-color var(--motion-fast) var(--ease-out),
      background var(--motion-fast) var(--ease-out);
  }

  .preset:hover {
    border-color: var(--color-border-strong);
    background: var(--color-muted);
  }

  .preset:has(input:checked) {
    border-color: var(--color-primary);
    background: var(--nav-active-bg);
  }

  .preset input {
    width: 14px;
    height: 14px;
    accent-color: var(--color-primary);
    flex: none;
  }

  .preset-name {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-weight: 600;
  }

  .rec {
    padding: 0 var(--space-1);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-sm);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    font-weight: 400;
  }

  .advanced summary {
    cursor: pointer;
    color: var(--color-muted-fg);
    user-select: none;
    min-height: var(--control-h);
    display: flex;
    align-items: center;
  }

  .fields {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-3);
    padding-top: var(--space-2);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .field-label {
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
    font-weight: 600;
  }

  .hint {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  @media (max-width: 560px) {
    .presets,
    .fields {
      grid-template-columns: 1fr;
    }
  }
</style>
