<script lang="ts">
  // Translation-memory tab — phase-2 stub with two mock TM rows.
  import { t } from '../../../i18n/store.svelte';
  import { mockTm } from '../../mock/wizard';
</script>

<section class="tm" aria-labelledby="tm-heading" data-testid="workspace.tm-stub">
  <h2 id="tm-heading" class="title">{t('tm.title')}</h2>
  <!-- Wave 9 (owner blank-tail class): the TM rows table is the stretching
       scroll container, the stub note anchors the bottom — no dead tail. -->
  <div class="scroll">
    <table class="rows">
      <thead>
        <tr>
          <th scope="col">{t('tm.source')}</th>
          <th scope="col">{t('tm.target')}</th>
          <th scope="col">{t('tm.from')}</th>
          <th scope="col">{t('tm.match')}</th>
        </tr>
      </thead>
      <tbody>
        {#each mockTm as row (row.source)}
          <tr>
            <td>{row.source}</td>
            <td>{row.target}</td>
            <td class="mono">{row.from}</td>
            <td class="mono">{row.match}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  <p class="note">{t('tm.note')}</p>
</section>

<style>
  /* Wave 9 (owner blank-tail class): the section fills the tab column instead
     of clamping to content height; the rows area is the scroll container. */
  .tm {
    padding: var(--space-4) var(--space-6);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    max-width: 760px;
    flex: 1;
    min-height: 0;
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: 18px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .rows {
    border-collapse: collapse;
    font-size: var(--text-base-size);
  }

  .rows th {
    text-align: left;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: var(--space-1) var(--space-3) var(--space-1) 0;
    border-bottom: 1px solid var(--color-border-strong);
  }

  .rows td {
    padding: var(--space-1) var(--space-3) var(--space-1) 0;
    border-bottom: 1px solid var(--color-border);
    vertical-align: top;
  }

  .note {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }
</style>
