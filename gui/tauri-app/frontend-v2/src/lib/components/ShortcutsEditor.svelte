<script lang="ts">
  // Remappable shortcut table (mandate §14). Click a row's "Change" button to
  // capture the next key press. Safety classes (W4.5 requirement #3):
  //   - a combo already used by another RimLoc command is an ERROR and the
  //     customization is NOT saved — the row shows a blocking note;
  //   - system-reserved combos (Cmd+Q / Cmd+W / Cmd+Tab class) save with a
  //     warning badge — customization is never hard-blocked by warnings;
  //   - convention-shadowing combos (Cmd+S / Cmd+Z class on a different
  //     command) save with an info badge.
  // Modifier follows platform convention: Cmd on macOS, Ctrl elsewhere — the
  // store detects it once via navigator.platform.
  import Icon from './Icon.svelte';
  import { t } from '../../i18n/store.svelte';
  import {
    shortcuts,
    SHORTCUT_DEFS,
    comboLabel,
    classifyBinding,
    IS_MAC,
    type ShortcutId,
    type ShortcutIssueClass
  } from '../stores/shortcuts.svelte';

  /** Shortcut currently waiting for a key press (only one at a time). */
  let capturing = $state<ShortcutId | null>(null);
  /** Blocked (error-class) attempt: id → shown until the next capture. */
  let blockedId = $state<ShortcutId | null>(null);

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
      // RimLoc-command conflicts are the only hard block (W4.5 #3a):
      // system-reserved and convention combos save with warnings instead.
      const candidate = classifyBinding(target, binding, shortcuts.bindings);
      if (candidate?.kind === 'rimloc-conflict') {
        blockedId = target;
        capturing = null;
        return;
      }
      shortcuts.assign(target, binding);
      blockedId = null;
      capturing = null;
    };
    window.addEventListener('keydown', onKey, { capture: true });
    return () => window.removeEventListener('keydown', onKey, { capture: true });
  });

  function startCapture(id: ShortcutId) {
    blockedId = null;
    capturing = capturing === id ? null : id;
  }

  const issues = $derived(shortcuts.issues());
  const warningCount = $derived(issues.filter((i) => i.cls === 'warning').length);
  const infoCount = $derived(issues.filter((i) => i.cls === 'info').length);

  function rowClass(cls: ShortcutIssueClass | null): string {
    return cls === 'error' ? 'row-error' : cls === 'warning' ? 'row-warning' : cls === 'info' ? 'row-info' : '';
  }
</script>

<div class="sc" data-testid="shortcuts.editor">
  {#if warningCount > 0}
    <p class="note warn" role="status" data-testid="shortcuts.warnReserved">
      <Icon name="warning" size={14} />
      {t('shortcuts.warnReserved', { count: warningCount })}
    </p>
  {/if}
  {#if infoCount > 0}
    <p class="note info" role="status" data-testid="shortcuts.infoConvention">
      <Icon name="info" size={14} />
      {t('shortcuts.infoConvention', { count: infoCount })}
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
        {@const issue = shortcuts.issueFor(def.id)}
        <tr class={rowClass(issue?.cls ?? null)} data-testid={`shortcuts.row.${def.id}`}>
          <td class="cmd">
            {t(def.labelKey)}
            {#if issue?.kind === 'rimloc-conflict'}
              <span class="flag err" data-testid={`shortcuts.conflict.${def.id}`}>
                <Icon name="warning" size={12} />
                {t('shortcuts.issue.rimlocConflict')}
              </span>
            {:else if issue?.kind === 'system-reserved'}
              <span class="flag warn" data-testid={`shortcuts.reserved.${def.id}`}>
                <Icon name="warning" size={12} />
                {t('shortcuts.issue.systemReserved')}
              </span>
            {:else if issue?.kind === 'convention'}
              <span class="flag info" data-testid={`shortcuts.convention.${def.id}`}>
                <Icon name="info" size={12} />
                {t('shortcuts.issue.convention')}
              </span>
            {/if}
            {#if blockedId === def.id}
              <span class="flag err blocked" role="alert" data-testid={`shortcuts.blocked.${def.id}`}>
                <Icon name="warning" size={12} />
                {t('shortcuts.blocked')}
              </span>
            {/if}
          </td>
          <td class="combo">
            {#if capturing === def.id}
              <span class="listening" role="status" data-testid={`shortcuts.listening.${def.id}`}>
                {t('shortcuts.listening')}
              </span>
            {:else if binding.key === ''}
              <kbd class="kbd unbound" data-testid={`shortcuts.combo.${def.id}`}>
                {t('shortcuts.unbound')}
              </kbd>
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

  .note {
    margin: 0;
    display: flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-meta-size);
  }

  .note.warn {
    color: var(--color-warning);
  }

  .note.info {
    color: var(--color-muted-fg);
  }

  .flag {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: var(--text-meta-size);
  }

  .flag.err {
    color: var(--color-error);
  }

  .flag.warn {
    color: var(--color-warning);
  }

  .flag.info {
    color: var(--color-muted-fg);
  }

  .flag.blocked {
    border: 1px solid var(--color-error);
    border-radius: var(--radius-sm);
    padding: 0 var(--space-1);
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

  .combo {
    white-space: nowrap;
  }

  .kbd.unbound {
    opacity: 0.6;
    border-style: dashed;
    font-weight: 400;
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

  tr.row-warning .kbd {
    border-color: var(--color-warning);
    color: var(--color-warning);
  }

  tr.row-info .kbd {
    color: var(--color-muted-fg);
  }

  tr.row-error .kbd {
    border-color: var(--color-error);
    color: var(--color-error);
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
