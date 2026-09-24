<script lang="ts">
  // Chat-batch manager screen (GUI_DESIGN_SPEC §10): the external-AI loop
  // without an API key — copy a batch, paste it into a web chat, import the
  // reply back. Reachable as the #/chat route and from the wizard result.
  // Everything below is mock-local (chatbatch store); the transparency line
  // shows what would be copied before anything leaves the app.
  import Icon from '../Icon.svelte';
  import { t, i18n } from '../../../i18n/store.svelte';
  import { chat, CHAT_PROJECT } from '../../stores/chatbatch.svelte';
  import BatchCard from './BatchCard.svelte';
  import SizingPanel from './SizingPanel.svelte';
  import ProfilePicker from './ProfilePicker.svelte';

  const LOCALE_NAMES: Record<string, { ru: string; en: string }> = {
    ru: { ru: 'Русский', en: 'Russian' },
    en: { ru: 'Английский', en: 'English' }
  };

  function fmt(n: number): string {
    return new Intl.NumberFormat(i18n.locale === 'ru' ? 'ru-RU' : 'en-US').format(n);
  }

  /** 8360 → "8,4" (ru) / "8.4" (en); the "k" suffix lives in the i18n key. */
  function fmtK(n: number): string {
    return new Intl.NumberFormat(i18n.locale === 'ru' ? 'ru-RU' : 'en-US', {
      maximumFractionDigits: 1
    }).format(n / 1000);
  }

  const targetName = $derived(
    (LOCALE_NAMES[chat.targetLocale] ?? LOCALE_NAMES.en)[i18n.locale]
  );

  const nextToCopy = $derived(chat.nextToCopy);
  const recommended = $derived(chat.profile === chat.recommendedProfile);

  // Transparency summary describes the whole run that will pass through the
  // chat (76 entries ≈ 8.4k tokens here) — labeled as an estimate; the size
  // of the single next batch is visible on its card.
  const summaryParts = $derived.by(() => {
    const parts = [
      t('chat.summary.entries', { count: fmt(chat.totalEntries) }),
      t('chat.summary.tokens', { n: fmtK(chat.runTokensEstimate) }),
      targetName,
      t(`chat.profile.${chat.profile}`)
    ];
    if (recommended) parts.push(t('chat.summary.recommended'));
    return parts;
  });

  const progressPct = $derived(
    chat.totalEntries === 0 ? 0 : Math.round((chat.acceptedTotal / chat.totalEntries) * 100)
  );

  function copyNext() {
    if (chat.nextToCopy) chat.copy(chat.nextToCopy.id);
  }
</script>

<section class="chat" aria-labelledby="chat-heading">
  <header class="head">
    <h1 id="chat-heading" class="title">{t('chat.title')}</h1>
    <p class="subtitle">{t('chat.subtitle')}</p>
    <p class="project mono" data-testid="chat.project">
      {CHAT_PROJECT.name} · {t('chat.project.pair', {
        source: LOCALE_NAMES[CHAT_PROJECT.sourceLocale]?.[i18n.locale] ?? 'English',
        target: targetName
      })}
    </p>
    <p class="progress" role="status" aria-live="polite" data-testid="chat.progress">
      {t('chat.progress', {
        done: fmt(chat.acceptedTotal),
        total: fmt(chat.totalEntries),
        review: fmt(chat.reviewTotal)
      })}
    </p>
    <div
      class="track"
      role="progressbar"
      aria-label={t('chat.title')}
      aria-valuenow={progressPct}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      <div class="fill" style="width: {progressPct}%"></div>
    </div>
  </header>

  <div class="columns">
    <div class="main">
      <!-- Transparency summary: what is about to be copied, labeled as estimates. -->
      <section class="export" aria-labelledby="export-heading">
        <h2 id="export-heading">{t('chat.summary.label')}</h2>
        <p class="summary" data-testid="chat.summary">
          {summaryParts.join(' · ')}
        </p>
        <div class="actions">
          <button
            type="button"
            class="btn btn-primary"
            disabled={!nextToCopy}
            data-testid="chat.copy-next"
            onclick={copyNext}
          >
            <Icon name="copy" size={14} />
            {t('chat.copyNext')}
          </button>
        </div>
        {#if chat.copiedId}
          <p class="note ok" role="status" data-testid="chat.copied-note">
            {t('help.copied')} — <span class="mono">{chat.copiedId}</span>
          </p>
        {/if}
        {#if !nextToCopy}
          <p class="note" data-testid="chat.all-copied">{t('chat.allCopied')}</p>
        {/if}
      </section>

      <section class="list" aria-label={t('chat.list.label')}>
        <h2>{t('chat.list.label')}</h2>
        <ul class="batches">
          {#each chat.rootBatches as b (b.id)}
            <li>
              <BatchCard batch={b} />
            </li>
            {#each chat.childrenOf(b.id) as c (c.id)}
              <li class="child">
                <BatchCard batch={c} />
              </li>
            {/each}
          {/each}
        </ul>
      </section>
    </div>

    <aside class="side">
      <SizingPanel />
      <ProfilePicker />
    </aside>
  </div>
</section>

<style>
  .chat {
    width: min(1080px, 100%);
    margin: 0 auto;
    padding: var(--space-6) var(--space-4) var(--space-8);
    overflow-y: auto;
  }

  .head {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin-bottom: var(--space-6);
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
  }

  .subtitle {
    margin: 0;
    color: var(--color-muted-fg);
  }

  .project {
    margin: var(--space-1) 0 0;
    color: var(--color-muted-fg);
  }

  .progress {
    margin: var(--space-2) 0 0;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }

  .track {
    height: 6px;
    border-radius: var(--radius-sm);
    background: var(--color-muted);
    overflow: hidden;
    margin-top: var(--space-1);
  }

  .fill {
    height: 100%;
    background: var(--color-primary);
    border-radius: var(--radius-sm);
    transition: width var(--motion-standard) var(--ease-out);
  }

  .columns {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 320px;
    gap: var(--space-6);
    align-items: start;
  }

  @media (max-width: 960px) {
    .columns {
      grid-template-columns: 1fr;
    }
  }

  .main {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    min-width: 0;
  }

  h2 {
    margin: 0 0 var(--space-2);
    font-size: 15px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .export {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
  }

  .summary {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  button:disabled {
    opacity: 0.55;
    cursor: default;
  }

  .note {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .note.ok {
    color: var(--color-status-translated);
  }

  .list .batches {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .list .child {
    padding-left: var(--space-6);
  }

  .side {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    min-width: 0;
  }
</style>
