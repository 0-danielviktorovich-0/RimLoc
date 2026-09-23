<script lang="ts">
  // SOURCE | TARGET table (spec §2.4). Simplification for the mock phase:
  // every filtered row is rendered instead of a virtual window, but the ARIA
  // contract for a virtualized grid is kept — the container declares
  // aria-rowcount (full filtered count) and each row carries aria-rowindex,
  // so swapping in a virtual scroller later does not change semantics.
  import { t } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import Icon from '../Icon.svelte';
  import type { Entry, SaveState } from '../../mock/types';

  let { rows, rowcount }: { rows: Entry[]; rowcount: number } = $props();

  let editingId = $state<string | null>(null);
  let draft = $state('');

  // The context panel region is a stable ARIA anchor for editors (spec §5).
  const CONTEXT_REGION_ID = 'workspace-context';

  // Local focus action: the editor must be ready for typing right away.
  function autofocus(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  function display(e: Entry): string {
    return project.drafts[e.id] ?? e.target;
  }

  function isDraft(e: Entry): boolean {
    const pending = project.drafts[e.id];
    return pending !== undefined && pending !== e.target;
  }

  function badge(e: Entry): SaveState {
    return project.saveStates[e.id] ?? 'idle';
  }

  function select(e: Entry) {
    project.select(e.id);
  }

  function beginEdit(e: Entry) {
    project.select(e.id);
    editingId = e.id;
    draft = display(e);
  }

  function closeEditor() {
    editingId = null;
    draft = '';
  }

  function onEditorInput(e: Event, entry: Entry) {
    draft = (e.currentTarget as HTMLInputElement).value;
    project.setDraft(entry.id, draft);
  }

  function onEditorBlur(entry: Entry) {
    if (editingId === entry.id) {
      project.flushDraft(entry.id);
      closeEditor();
    }
  }

  function onEditorKeydown(event: KeyboardEvent, entry: Entry) {
    if (event.key === 'Enter') {
      // Enter / Ctrl+Enter — commit and leave edit mode.
      event.preventDefault();
      project.flushDraft(entry.id);
      closeEditor();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      project.discardDraft(entry.id);
      closeEditor();
    } else if (event.key === 'Tab') {
      // The translator's main loop: jump to the next untranslated row.
      event.preventDefault();
      jumpToNextUntranslated(entry, event.shiftKey ? -1 : 1);
    }
  }

  function jumpToNextUntranslated(current: Entry, dir: 1 | -1) {
    project.flushDraft(current.id);
    const from = rows.findIndex((r) => r.id === current.id);
    for (let step = 1; step <= rows.length; step += 1) {
      const idx = from + dir * step;
      if (idx < 0 || idx >= rows.length) break;
      if (rows[idx].status === 'untranslated') {
        closeEditor();
        beginEdit(rows[idx]);
        return;
      }
    }
    closeEditor();
  }

  function onRowKeydown(event: KeyboardEvent, entry: Entry) {
    if (event.key === 'Enter' && editingId !== entry.id) {
      event.preventDefault();
      beginEdit(entry);
    }
  }
</script>

<div class="table" role="table" aria-rowcount={rowcount} data-testid="workspace.table">
  <div class="thead" role="rowgroup">
    <div class="row head" role="row" aria-rowindex={1}>
      <div class="cell status" role="columnheader" aria-label={t('workspace.col.status')}></div>
      <div class="cell source" role="columnheader">{t('workspace.col.source')}</div>
      <div class="cell target" role="columnheader">{t('workspace.col.target')}</div>
      <div class="cell changed" role="columnheader" aria-label={t('workspace.col.changed')}></div>
    </div>
  </div>
  <div class="tbody" role="rowgroup">
    {#each rows as entry, index (entry.id)}
      <div
        class="row"
        class:selected={project.selectedId === entry.id}
        role="row"
        aria-rowindex={index + 2}
        aria-label={entry.key}
        tabindex="0"
        data-testid={`workspace.row.${entry.id}`}
        onclick={() => select(entry)}
        ondblclick={() => {
          if (editingId !== entry.id) beginEdit(entry);
        }}
        onkeydown={(e) => onRowKeydown(e, entry)}
      >
        <div class="cell status" role="cell">
          <span class="dot dot-{entry.status}" aria-hidden="true"></span>
          <span class="visually-hidden">{t(`status.${entry.status}`)}</span>
        </div>
        <div class="cell source" role="cell">
          <span class="source-text" class:mono-key={entry.kind === 'TKey'}>{entry.source}</span>
        </div>
        <!-- Keyboard users edit via the row: it is focusable and Enter opens the
             editor (see onRowKeydown), so the cell click is a mouse-only shortcut. -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_interactive_supports_focus -->
        <div
          class="cell target"
          role="cell"
          title={t('workspace.editor.columnName')}
          onclick={(e) => {
            if (editingId !== entry.id) {
              e.stopPropagation();
              beginEdit(entry);
            }
          }}
        >
          {#if editingId === entry.id}
            <input
              class="editor"
              type="text"
              value={draft}
              use:autofocus
              aria-label={t('workspace.editor.columnName')}
              aria-describedby={CONTEXT_REGION_ID}
              data-testid={`workspace.editor.${entry.id}`}
              oninput={(e) => onEditorInput(e, entry)}
              onblur={() => onEditorBlur(entry)}
              onkeydown={(e) => onEditorKeydown(e, entry)}
            />
          {:else}
            <span class="target-text" class:draft={isDraft(entry)}>{display(entry)}</span>
            {#if badge(entry) === 'saving'}
              <span class="badge saving">{t('workspace.editor.saving')}</span>
            {:else if badge(entry) === 'saved'}
              <span class="badge saved">{t('workspace.editor.saved')}</span>
            {:else if badge(entry) === 'dirty'}
              <span class="badge dirty">{t('workspace.editor.draft')}</span>
            {/if}
          {/if}
        </div>
        <div class="cell changed" role="cell">
          {#if entry.status === 'sourceChanged'}
            <span class="changed-icon" title={t('workspace.changed.yes')}>
              <Icon name="alert" size={14} />
              <span class="visually-hidden">{t('workspace.changed.yes')}</span>
            </span>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .table {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    font-size: var(--text-dense-size);
    line-height: var(--text-dense-lh);
  }

  .row {
    display: grid;
    grid-template-columns: 28px minmax(220px, 1fr) minmax(260px, 1.2fr) 28px;
    align-items: stretch;
    min-height: 40px;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface);
    transition: background var(--motion-fast);
  }

  .row:hover {
    background: var(--color-muted);
  }

  .row.selected {
    background: var(--color-muted);
    box-shadow: inset 2px 0 0 var(--color-primary);
  }

  .row.head {
    position: sticky;
    top: 0;
    z-index: 1;
    background: var(--color-bg);
    font-weight: 600;
    color: var(--color-muted-fg);
    min-height: 36px;
  }

  .cell {
    padding: var(--space-1) var(--space-2);
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    overflow: hidden;
  }

  .cell.status {
    justify-content: center;
    padding: 0;
  }

  .cell.changed {
    justify-content: center;
    padding: 0;
  }

  .source-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-fg);
  }

  /* TKey family sources are paths/identifiers: mono per spec §1.3. */
  .mono-key {
    font-family: var(--font-mono);
  }

  .target-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-fg);
  }

  .target-text.draft {
    font-style: italic;
    color: var(--color-muted-fg);
  }

  .target:has(.editor) {
    padding: var(--space-1);
  }

  .editor {
    width: 100%;
    min-height: 30px;
    padding: var(--space-1) var(--space-2);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }

  .dot-untranslated {
    background: var(--color-status-untranslated);
  }
  .dot-translated {
    background: var(--color-status-translated);
  }
  .dot-todo {
    background: var(--color-status-todo);
  }
  .dot-sourceChanged {
    background: var(--color-status-source-changed);
  }
  .dot-pending_review {
    background: var(--color-status-pending-review);
  }
  .dot-orphan {
    background: var(--color-status-orphan);
  }

  .changed-icon {
    color: var(--color-status-source-changed);
    display: inline-flex;
  }

  .badge {
    flex: none;
    font-size: var(--text-meta-size);
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
    white-space: nowrap;
  }

  .badge.saving {
    color: var(--color-muted-fg);
    font-style: italic;
  }

  .badge.saved {
    color: var(--color-status-translated);
  }

  .badge.dirty {
    color: var(--color-muted-fg);
  }
</style>
