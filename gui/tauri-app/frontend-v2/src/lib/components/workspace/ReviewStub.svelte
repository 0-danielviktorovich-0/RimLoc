<script lang="ts">
  // Review tab (mandate §12) — phase-2 stub with the real overview numbers and
  // issue categories so the lifecycle CTA has somewhere honest to land.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { mockReviewCategories, mockReviewOverview } from '../../mock/wizard';

  const OVERVIEW = [
    { key: 'needsReview', value: mockReviewOverview.needsReview, icon: 'clipboard-check' },
    { key: 'errors', value: mockReviewOverview.errors, icon: 'warning' },
    { key: 'sourceChanged', value: mockReviewOverview.sourceChanged, icon: 'alert' },
    { key: 'glossaryConflicts', value: mockReviewOverview.glossaryConflicts, icon: 'book' }
  ] as const;
</script>

<section class="review" aria-labelledby="review-heading" data-testid="workspace.review-stub">
  <h2 id="review-heading" class="title">{t('review.title')}</h2>

  <dl class="overview">
    {#each OVERVIEW as o (o.key)}
      <div class="card">
        <dt>
          <Icon name={o.icon} size={14} />
          {t(`review.${o.key}`)}
        </dt>
        <dd class="mono">{new Intl.NumberFormat('en-US').format(o.value)}</dd>
      </div>
    {/each}
  </dl>

  <h3 class="section">{t('review.categories')}</h3>
  <ul class="categories">
    {#each mockReviewCategories as c (c.id)}
      <li class="cat">
        <span>{t(`issue.${c.id}`)}</span>
        <span class="mono count">{c.count}</span>
      </li>
    {/each}
  </ul>

  <p class="note">{t('review.note')}</p>
</section>

<style>
  .review {
    padding: var(--space-4) var(--space-6);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    max-width: 640px;
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: 18px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .overview {
    margin: 0;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--space-2);
  }

  @media (max-width: 900px) {
    .overview {
      grid-template-columns: repeat(2, 1fr);
    }
  }

  .card {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    padding: var(--space-3);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .card dt {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .card dd {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .section {
    margin: var(--space-2) 0 0;
    font-size: var(--text-meta-size);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-muted-fg);
  }

  .categories {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .cat {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-1) 0;
    border-bottom: 1px solid var(--color-border);
    font-size: var(--text-base-size);
  }

  .cat:last-child {
    border-bottom: none;
  }

  .count {
    color: var(--color-muted-fg);
    font-variant-numeric: tabular-nums;
  }

  .note {
    margin: var(--space-2) 0 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }
</style>
