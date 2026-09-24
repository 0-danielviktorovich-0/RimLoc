<script lang="ts">
  // Remappable shortcut table (mandate §14). Click a row's "Change" button to
  // capture the next key press; duplicates between commands are highlighted
  // (conflict detection is derived in the store). Modifier follows platform
  // convention: Cmd on macOS, Ctrl elsewhere — the store detects it once via
  // navigator.platform, the note below explains the rule to the user.
  import Icon from './Icon.svelte';
  import { t } from '../../i18n/store.svelte';
  import {
    shortcuts,
    SHORTCUT_DEFS,
    comboLabel,
    IS_MAC,
    type ShortcutId
  } from '../stores/shortcuts.svelte';

  /** Shortcut currently waiting for a key press (only one at a time). */
  let capturing = $state<ShortcutId | null>(null);

  $effect(() => {
    if (!capturing) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const target = capturing;
      if (!target) return;
      const binding = shortcuts.fromEvent(e);
      if (!binding) return; // bare modifier — keep listening
      // Escape while capturing always cancels the capture itself; to bind
      // Esc to a command, remap something else onto it is intentionally not
      // supported — cancel-and-retry is the safer default.
      if (e.key === 'Escape') {
        capturing = null;
        return;
      }
      shortcuts.assign(target, binding);
      capturing = null;
    };
    window.addEventListener('keydown', onKey, { capture: true });
    return () => window.removeEventListener('keydown', onKey, { capture: true });
  });

  const conflicts = $derived(shortcuts.conflictMap());
  const hasConflicts = $derived(conflicts.size > 0);

  function startCapture(id: ShortcutId) {
    capturing = capturing === id ? null : id;
  }
</script>

<div class="sc" data-testid="shortcuts.editor">
  {#if hasConflicts}
    <p class="conflict-note" role="alert" data-testid="shortcuts.conflicts">
      <Icon name="warning" size={14} />
      {t('shortcuts.conflict', { count: conflicts.size })}
    </p>
  {/if}

  <table class="table">
    <thead>
      <tr>
        <th scope="col">{t('shortcuts.col.command')}</th>
        <th scope="col">{t('shortcuts.col.combo')}</th>
        <th scope="col"><span class="visually-hidden">{t('shortcuts.col.actions')}</span></th>
      </tr>
    </thead>
    <tbody>
      {#each SHORTCUT_DEFS as def (def.id)}
        {@const binding = shortcuts.binding(def.id)}
        {@const isConflict = shortcuts.isConflicting(def.id)}
        <tr class:conflict={isConflict} data-testid={`shortcuts.row.${def.id}`}>
          <td class="cmd">
            {t(def.labelKey)}
            {#if isConflict}
              <span class="dup" data-testid={`shortcuts.dup.${def.id}`}>
                <Icon name="warning" size={12} />
                {t('shortcuts.duplicate')}
              </span>
            {/if}
          </td>
          <td class="combo">
            {#if capturing === def.id}
              <span class="listening" role="status" data-testid={`shortcuts.listening.${def.id}`}>
                {t('shortcuts.listening')}
              </span>
            {:else}
              <kbd class="kbd" data-testid={`shortcuts.combo.${def.id}`}>{comboLabel(binding)}</kbd>
            {/if}
          </td>
          <td class="row-actions">
            <button
              type="button"
              class="btn subtle"
              data-testid={`shortcuts.remap.${def.id}`}
              aria-pressed={capturing === def.id}
              onclick={() => startCapture(def.id)}
            >
              {capturing === def.id ? t('common.cancel') : t('shortcuts.change')}
            </button>
            <button
              type="button"
              class="btn subtle"
              data-testid={`shortcuts.reset.${def.id}`}
              onclick={() => shortcuts.reset(def.id)}
              disabled={capturing === def.id}
            >
              {t('common.reset')}
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  <div class="foot">
    <p class="note">
      <Icon name="info" size={13} />
      {IS_MAC ? t('shortcuts.platform.mac') : t('shortcuts.platform.other')}
      {t('shortcuts.platform.hint')}
    </p>
    <button type="button" class="btn" data-testid="shortcuts.resetAll" onclick={() => shortcuts.resetAll()}>
      {t('shortcuts.resetAll')}
    </button>
  </div>
</div>

<style>
  .sc {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    width: 100%;
  }

  .conflict-note {
    margin: 0;
    display: flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-warning);
    font-size: var(--text-meta-size);
  }

  .table {
    border-collapse: collapse;
    width: 100%;
  }

  th {
    text-align: left;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    font-weight: 600;
    padding: var(--space-1) var(--space-2) var(--space-1) 0;
    border-bottom: 1px solid var(--color-border);
  }

  td {
    padding: var(--space-2) var(--space-2) var(--space-2) 0;
    border-bottom: 1px solid var(--color-border);
    vertical-align: middle;
  }

  tr:last-child td {
    border-bottom: none;
  }

  .cmd {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  .dup {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--color-warning);
    font-size: var(--text-meta-size);
  }

  .combo {
    white-space: nowrap;
  }

  .kbd {
    font-family: var(--font-mono);
    font-size: var(--text-meta-size);
    border: 1px solid var(--color-border-strong);
    border-bottom-width: 2px;
    border-radius: var(--radius-sm);
    padding: 1px var(--space-1);
    background: var(--color-muted);
    white-space: nowrap;
  }

  .listening {
    font-size: var(--text-meta-size);
    color: var(--color-primary-text);
  }

  tr.conflict .kbd {
    border-color: var(--color-warning);
    color: var(--color-warning);
  }

  .row-actions {
    white-space: nowrap;
  }

  .btn.subtle {
    border-color: transparent;
    color: var(--color-primary-text);
    min-height: 26px;
    padding: 0 var(--space-2);
    font-size: var(--text-meta-size);
  }

  .btn.subtle + .btn.subtle {
    margin-left: var(--space-1);
  }

  .btn.subtle:disabled {
    color: var(--color-muted-fg);
    cursor: default;
  }

  .foot {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .note {
    margin: 0;
    display: flex;
    align-items: flex-start;
    gap: var(--space-1);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .note :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
</style>
