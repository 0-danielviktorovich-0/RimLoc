<script lang="ts">
  // Glossary tab — phase-2 stub with a small glimpse of mock terms.
  import { t } from '../../../i18n/store.svelte';
  import { mockGlossary } from '../../mock/wizard';
</script>

<section class="glossary" aria-labelledby="glossary-heading" data-testid="workspace.glossary-stub">
  <h2 id="glossary-heading" class="title">{t('glossary.title')}</h2>
  <table class="terms">
    <thead>
      <tr>
        <th scope="col">{t('glossary.term')}</th>
        <th scope="col">{t('glossary.translation')}</th>
      </tr>
    </thead>
    <tbody>
      {#each mockGlossary as term (term.en)}
        <tr>
          <td class="mono">{term.en}</td>
          <td>{term.ru}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  <p class="note">{t('glossary.note')}</p>
</section>

<style>
  .glossary {
    padding: var(--space-4) var(--space-6);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    max-width: 560px;
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: 18px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .terms {
    border-collapse: collapse;
    font-size: var(--text-base-size);
  }

  .terms th {
    text-align: left;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: var(--space-1) var(--space-3) var(--space-1) 0;
    border-bottom: 1px solid var(--color-border-strong);
  }

  .terms td {
    padding: var(--space-1) var(--space-3) var(--space-1) 0;
    border-bottom: 1px solid var(--color-border);
  }

  .note {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }
</style>
