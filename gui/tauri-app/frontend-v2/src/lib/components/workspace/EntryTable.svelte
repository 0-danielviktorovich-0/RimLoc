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
            {#if display(entry)}
              <span class="target-text" class:draft={isDraft(entry)}>{display(entry)}</span>
            {:else}
              <!-- L-3 (UI audit 2026-09-29): an empty TARGET cell is a state,
                   not a glitch — a quiet explicit marker instead of blank. -->
              <span class="target-empty">{t('workspace.col.noTranslation')}</span>
            {/if}
            {#if badge(entry) === 'saving'}
              <span class="badge saving">{t('workspace.editor.saving')}</span>
            {:else if badge(entry) === 'saved'}
              <span class="badge saved">
                <Icon name="circle-check" size={12} />
                {t('workspace.editor.saved')}
              </span>
            {:else if badge(entry) === 'dirty'}
              <span class="badge dirty">
                <Icon name="edit" size={12} />
                {t('workspace.editor.draft')}
              </span>
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
    font-size: var(--row-font-size);
    line-height: var(--row-lh);
    /* L-5 (UI audit 2026-09-29): the overlay scrollbar rides on the right
       edge of this scroll container and covered the sticky TARGET header.
       Reserving the strip as padding keeps rows/header clear of it (the
       header stays sticky above them, but nothing sits under the bar). */
    padding-right: 14px;
  }

  /* Rows change state instantly — lists and tables are never animated
     (spec §1.4); only the editor input and badges carry motion. */
  .row {
    display: grid;
    grid-template-columns: 28px minmax(220px, 1fr) minmax(260px, 1.2fr) 28px;
    align-items: stretch;
    min-height: var(--row-height);
    border-bottom: 1px solid var(--color-border);
    background: var(--color-surface);
  }

  .row:hover {
    background: var(--color-muted);
  }

  .row.selected {
    background: var(--color-muted);
    box-shadow: inset 2px 0 0 var(--row-accent);
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
    /* Direction-owned: editorial dims SOURCE to build the SOURCE|TARGET axis */
    color: var(--color-source-text);
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

  /* Empty-translation marker: same muted-italic convention as the review
     context's empty text block. */
  .target-empty {
    color: var(--color-muted-fg);
    font-style: italic;
  }

  .target:has(.editor) {
    padding: var(--space-1);
  }

  /* Inline-edit transition: the input itself eases its border and halo —
     the row layout is untouched, so nothing jumps. */
  .editor {
    width: 100%;
    min-height: var(--control-h);
    padding: var(--space-1) var(--space-2);
    transition:
      border-color var(--motion-fast) var(--ease-out),
      box-shadow var(--motion-fast) var(--ease-out);
  }

  .editor:focus-visible {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 18%, transparent);
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
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: var(--text-meta-size);
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
    white-space: nowrap;
  }

  .badge :global(svg) {
    flex: none;
  }

  .badge.saving {
    color: var(--color-muted-fg);
    font-style: italic;
  }

  /* Validation feedback: pass (check) and draft (edit) carry icon + color. */
  .badge.saved {
    color: var(--color-success);
  }

  .badge.dirty {
    color: var(--color-warning);
  }
</style>
