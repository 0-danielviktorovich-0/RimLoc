<script lang="ts">
  // Glossary tab (wave 13 MOCK→LIVE): on a CONTRACT project this is the
  // live term table over `project_glossary` — add row, inline edit, delete
  // with confirmation, busy/error per the project canon. On the fixture/demo
  // project (or no project) it stays the honest stub with the demo badge:
  // the glossary is durable PROJECT state, and the mock has no backend
  // session to persist into — a fabricated table would lie (mock.ts refuses
  // the same way when called directly).
  import { t } from '../../../i18n/store.svelte';
  import { mockGlossary } from '../../mock/wizard';
  import { clientInstance } from '../../client/instance.svelte';
  import { ContractClientError } from '../../client/client';
  import { contractErrorText } from '../../client/messages';
  import type { GlossaryTermDto } from '../../client/types';
  import { project } from '../../stores/project.svelte';

  let terms = $state<GlossaryTermDto[]>([]);
  let loading = $state(false);
  let loadError = $state<string | null>(null);
  let busy = $state(false);
  let actionError = $state<string | null>(null);
  // Add-row form.
  let newTerm = $state('');
  let newTranslation = $state('');
  let newNote = $state('');
  // Inline edit state (one row at a time).
  let editId = $state<string | null>(null);
  let editTranslation = $state('');
  let editNote = $state('');
  // Delete confirmation (one row at a time).
  let confirmDeleteId = $state<string | null>(null);

  const live = $derived(project.source === 'contract' && project.contractProjectId !== null);
  const pid = $derived(project.contractProjectId ?? '');
  const epoch = $derived(project.contractEpoch);

  async function load(): Promise<void> {
    if (!live) return;
    loading = true;
    loadError = null;
    try {
      terms = await clientInstance.getClient().glossaryList(pid, epoch);
    } catch (e) {
      loadError =
        e instanceof ContractClientError
          ? contractErrorText(e.code, e.message)
          : e instanceof Error
            ? e.message
            : String(e);
      terms = [];
    } finally {
      loading = false;
    }
  }

  // Reload when the open project (or its epoch) changes.
  $effect(() => {
    void pid;
    void epoch;
    void load();
  });

  async function runAction(fn: () => Promise<void>): Promise<void> {
    if (busy) return;
    busy = true;
    actionError = null;
    try {
      await fn();
      // A mutation moved the durable revision — adopt it so the workspace's
      // apply base stays honest (persist-before-ack semantics).
      await project.refreshContract();
      await load();
    } catch (e) {
      actionError =
        e instanceof ContractClientError
          ? contractErrorText(e.code, e.message)
          : e instanceof Error
            ? e.message
            : String(e);
    } finally {
      busy = false;
    }
  }

  function addTerm(): Promise<void> {
    const term = newTerm.trim();
    const translation = newTranslation.trim();
    if (!term || !translation) return Promise.resolve();
    const note = newNote.trim();
    return runAction(async () => {
      const client = clientInstance.getClient();
      await client.glossaryUpsert({
        project_id: pid,
        session_epoch: epoch,
        term,
        translation,
        ...(note ? { note } : {})
      });
      newTerm = '';
      newTranslation = '';
      newNote = '';
    });
  }

  function startEdit(term: GlossaryTermDto): void {
    editId = term.id;
    editTranslation = term.translation;
    editNote = term.note ?? '';
    actionError = null;
  }

  function cancelEdit(): void {
    editId = null;
    editTranslation = '';
    editNote = '';
  }

  function saveEdit(original: GlossaryTermDto): Promise<void> {
    if (!editId) return Promise.resolve();
    const translation = editTranslation.trim();
    if (!translation) return Promise.resolve();
    const note = editNote.trim();
    return runAction(async () => {
      const client = clientInstance.getClient();
      await client.glossaryUpsert({
        project_id: pid,
        session_epoch: epoch,
        term: original.term,
        translation,
        ...(note ? { note } : {})
      });
      cancelEdit();
    });
  }

  function removeTerm(term: GlossaryTermDto): Promise<void> {
    return runAction(async () => {
      const client = clientInstance.getClient();
      await client.glossaryDelete({ project_id: pid, session_epoch: epoch, term: term.term });
      confirmDeleteId = null;
    });
  }
</script>

{#if live}
  <section class="glossary" aria-labelledby="glossary-heading" data-testid="workspace.glossary-live">
    <div class="head">
      <h2 id="glossary-heading" class="title">{t('glossary.title')}</h2>
      <span class="live-badge" data-testid="workspace.glossary-live-badge">{t('glossary.live.badge')}</span>
    </div>

    {#if loading}
      <p class="state" role="status">{t('glossary.live.loading')}</p>
    {:else if loadError}
      <p class="state error" role="alert" data-testid="workspace.glossary-load-error">{loadError}</p>
    {:else}
      {#if actionError}
        <p class="state error" role="alert" data-testid="workspace.glossary-action-error">{actionError}</p>
      {/if}

      <div class="scroll">
        <table class="terms">
          <thead>
            <tr>
              <th scope="col">{t('glossary.term')}</th>
              <th scope="col">{t('glossary.translation')}</th>
              <th scope="col">{t('glossary.live.note')}</th>
              <th scope="col"><span class="visually-hidden">{t('glossary.live.actions')}</span></th>
            </tr>
          </thead>
          <tbody>
            {#each terms as term (term.id)}
              <tr data-testid={`workspace.glossary.row-${term.id}`}>
                <td class="mono">{term.term}</td>
                <td>
                  {#if editId === term.id}
                    <input
                      class="cell-input"
                      data-testid={`workspace.glossary.edit-translation-${term.id}`}
                      bind:value={editTranslation}
                      aria-label={t('glossary.translation')}
                      disabled={busy}
                    />
                  {:else}
                    {term.translation}
                  {/if}
                </td>
                <td>
                  {#if editId === term.id}
                    <input
                      class="cell-input"
                      data-testid={`workspace.glossary.edit-note-${term.id}`}
                      bind:value={editNote}
                      aria-label={t('glossary.live.note')}
                      disabled={busy}
                    />
                  {:else}
                    {term.note ?? '—'}
                  {/if}
                </td>
                <td class="actions">
                  {#if editId === term.id}
                    <button
                      class="btn"
                      data-testid={`workspace.glossary.save-${term.id}`}
                      disabled={busy}
                      onclick={() => void saveEdit(term)}>{t('glossary.live.save')}</button
                    >
                    <button class="btn" disabled={busy} onclick={cancelEdit}>
                      {t('glossary.live.cancel')}
                    </button>
                  {:else if confirmDeleteId === term.id}
                    <span class="confirm" role="alertdialog" aria-label={t('glossary.live.confirmDelete')}>
                      {t('glossary.live.confirmDelete')}
                      <button
                        class="btn danger"
                        data-testid={`workspace.glossary.confirm-delete-${term.id}`}
                        disabled={busy}
                        onclick={() => void removeTerm(term)}>{t('glossary.live.delete')}</button
                      >
                      <button
                        class="btn"
                        disabled={busy}
                        onclick={() => (confirmDeleteId = null)}>{t('glossary.live.cancel')}</button
                      >
                    </span>
                  {:else}
                    <button
                      class="btn"
                      data-testid={`workspace.glossary.edit-${term.id}`}
                      disabled={busy}
                      onclick={() => startEdit(term)}>{t('glossary.live.edit')}</button
                    >
                    <button
                      class="btn danger"
                      data-testid={`workspace.glossary.delete-${term.id}`}
                      disabled={busy}
                      onclick={() => (confirmDeleteId = term.id)}>{t('glossary.live.delete')}</button
                    >
                  {/if}
                </td>
              </tr>
            {:else}
              <tr>
                <td colspan="4" class="empty">{t('glossary.live.empty')}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <form
        class="add-row"
        data-testid="workspace.glossary.add-form"
        onsubmit={(e) => {
          e.preventDefault();
          void addTerm();
        }}
      >
        <input
          class="cell-input"
          data-testid="workspace.glossary.add-term"
          placeholder={t('glossary.live.placeholderTerm')}
          bind:value={newTerm}
          disabled={busy}
          required
        />
        <input
          class="cell-input"
          data-testid="workspace.glossary.add-translation"
          placeholder={t('glossary.live.placeholderTranslation')}
          bind:value={newTranslation}
          disabled={busy}
          required
        />
        <input
          class="cell-input"
          data-testid="workspace.glossary.add-note"
          placeholder={t('glossary.live.placeholderNote')}
          bind:value={newNote}
          disabled={busy}
        />
        <button class="btn primary" data-testid="workspace.glossary.add-submit" disabled={busy}>
          {t('glossary.live.add')}
        </button>
      </form>
    {/if}
  </section>
{:else}
  <section class="glossary" aria-labelledby="glossary-heading" data-testid="workspace.glossary-stub">
    <div class="head">
      <h2 id="glossary-heading" class="title">{t('glossary.title')}</h2>
      <span class="demo-badge" data-testid="workspace.glossary.demo-badge">{t('glossary.live.demoBadge')}</span>
    </div>
    <!-- Wave 9 (owner blank-tail class): the term table is the stretching scroll
         container, the stub note anchors the bottom — no dead tail below. -->
    <div class="scroll">
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
    </div>
    <p class="note">{t('glossary.note')}</p>
  </section>
{/if}

<style>
  .glossary {
    padding: var(--space-4) var(--space-6);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    max-width: 640px;
    flex: 1;
    min-height: 0;
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .live-badge,
  .demo-badge {
    font-size: var(--font-size-xs);
    padding: 2px var(--space-2);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }

  .live-badge {
    border-color: var(--accent, var(--border));
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .terms {
    width: 100%;
    border-collapse: collapse;
  }

  .terms th,
  .terms td {
    text-align: left;
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--border);
    vertical-align: top;
  }

  .mono {
    font-family: var(--font-mono);
  }

  .empty {
    color: var(--text-secondary);
    font-style: italic;
  }

  .actions {
    white-space: nowrap;
  }

  .btn {
    font-size: var(--font-size-xs);
    padding: 2px var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-primary);
    cursor: pointer;
    margin-right: var(--space-1);
  }

  .btn.primary {
    border-color: var(--accent, var(--border));
  }

  .btn.danger {
    color: var(--danger, var(--text-primary));
    border-color: var(--danger, var(--border));
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .cell-input {
    width: 100%;
    font: inherit;
    padding: 2px var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface, transparent);
    color: var(--text-primary);
  }

  .add-row {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }

  .add-row .cell-input {
    flex: 1;
    min-width: 0;
  }

  .confirm {
    display: inline-flex;
    gap: var(--space-1);
    align-items: center;
    color: var(--text-secondary);
  }

  .state {
    color: var(--text-secondary);
  }

  .state.error {
    color: var(--danger, var(--text-primary));
  }

  .note {
    color: var(--text-secondary);
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
