<script lang="ts">
  // About (mandate §24): version, one-line product description, project
  // ownership, repository link and licensing — including the third-party
  // font notices (IBM Plex / JetBrains Mono ship under the SIL OFL 1.1).
  // Rendered as a card inside Help; version matches the bug-report block.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';

  const REPO_URL = 'https://github.com/0-danielviktorovich-0/RimLoc';
</script>

<article class="about" data-testid="help.about">
  <h2 class="card-title"><Icon name="info" size={16} /> {t('about.title')}</h2>

  <p class="tagline">{t('about.tagline')}</p>

  <dl class="meta">
    <div class="meta-row">
      <dt>{t('about.version')}</dt>
      <dd class="mono" data-testid="help.about.version">RimLoc 0.1.0</dd>
    </div>
    <div class="meta-row">
      <dt>{t('about.maintainer')}</dt>
      <dd>Daniel Kamyshan <span class="muted">· {t('about.maintainerRole')}</span></dd>
    </div>
    <div class="meta-row">
      <dt>{t('about.repo')}</dt>
      <dd>
        <a class="link mono" href={REPO_URL} target="_blank" rel="noreferrer" data-testid="help.about.repo">
          0-danielviktorovich-0/RimLoc <Icon name="external" size={12} />
        </a>
      </dd>
    </div>
    <div class="meta-row">
      <dt>{t('about.license')}</dt>
      <dd><span class="mono">GPL-3.0-only</span></dd>
    </div>
  </dl>

  <details class="notices">
    <summary data-testid="help.about.notices">{t('about.notices')}</summary>
    <ul class="notice-list">
      <li class="mono">IBM Plex Sans — SIL Open Font License 1.1</li>
      <li class="mono">JetBrains Mono — SIL Open Font License 1.1</li>
    </ul>
    <p class="notice-note">{t('about.noticesNote')}</p>
  </details>
</article>

<style>
  .about {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-card);
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .card-title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: 16px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .card-title :global(svg) {
    color: var(--color-primary-text);
  }

  .tagline {
    margin: 0;
    color: var(--color-muted-fg);
  }

  .meta {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .meta-row {
    display: grid;
    grid-template-columns: 140px 1fr;
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

  .mono {
    font-family: var(--font-mono);
    font-size: var(--text-meta-size);
  }

  .muted {
    color: var(--color-muted-fg);
  }

  .link {
    color: var(--color-primary-text);
    text-decoration: none;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .link:hover {
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .notices summary {
    cursor: pointer;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .notices summary:hover {
    color: var(--color-fg);
  }

  .notice-list {
    list-style: none;
    margin: var(--space-2) 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .notice-note {
    margin: var(--space-2) 0 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }
</style>
