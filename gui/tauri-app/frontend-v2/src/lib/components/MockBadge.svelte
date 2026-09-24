<script lang="ts">
  // Global data-mode badge (MOCK_LIVE_ONBOARDING_MANDATE §1): this scaffold
  // runs on synthetic demo data with zero backend calls, and the badge says
  // so on EVERY route and in EVERY build — including preview/production
  // builds. Honesty about the data mode never depends on a dev-mode env
  // flag: this component reads no build-time environment at all. The future
  // live mode arrives through the UI → RimLocClient → TauriTransport
  // capability boundary (post-freeze); until then this chip is the single
  // source of truth, replacing the local W5 diagnostics badge.
  import Icon from './Icon.svelte';
  import { t } from '../../i18n/store.svelte';
</script>

<span class="mock-badge" data-testid="mock-badge" title={t('mockbadge.title')}>
  <Icon name="info" size={12} />
  {t('mockbadge.label')}
</span>

<style>
  .mock-badge {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 2px var(--space-2);
    border: 1px dashed var(--color-warning);
    border-radius: var(--radius-sm);
    color: var(--color-warning);
    font-size: var(--text-meta-size);
    white-space: nowrap;
    cursor: help;
  }
</style>
