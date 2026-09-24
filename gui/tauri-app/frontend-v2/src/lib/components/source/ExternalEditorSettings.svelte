<script lang="ts">
  // Settings → External editor card (SOURCE_INSPECTOR_MANDATE §8). Honest
  // mock: select/detect/test — detect lists preset "detections" from fixture
  // data, test previews the STRUCTURED argv (executable + argument array)
  // that the future backend would execute. Nothing launches. Unknown
  // placeholders, empty executables and unusable targets are rejected before
  // anything is saved.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import {
    EDITOR_PRESETS,
    loadEditorChoice,
    saveEditorChoice,
    splitArgsText,
    templateFor,
    type EditorId,
    type StoredEditorChoice
  } from '../../source/editor';
  import { source } from '../../source/store.svelte';

  let choice: StoredEditorChoice = $state(loadEditorChoice());
  let preview = $state<{ ok: boolean; argv?: string[]; reasonKey?: string } | null>(null);
  let savedFlash = $state(false);

  const argsItems = $derived.by(() => {
    if (choice.preset !== 'custom') {
      const tpl = templateFor(choice);
      return tpl.ok ? tpl.plan.args : [];
    }
    return splitArgsText(choice.custom.argsText); // live typing preview: lenient
  });

  function selectPreset(id: EditorId) {
    choice = { ...choice, preset: id };
    preview = null;
  }

  function persist() {
    saveEditorChoice(choice);
    savedFlash = true;
    setTimeout(() => (savedFlash = false), 1600);
  }

  /** Mock test: validates the structured plan against a demo target and
   *  shows the exact argv array. No process starts; "would launch" only. */
  function testLaunch() {
    const template = templateFor(choice);
    if (!template.ok) {
      preview = { ok: false, reasonKey: template.reasonKey };
      return;
    }
    const result = source.planEditorLaunch(choice, {
      displayPath: 'Languages/English/Keyed/Misc_Gameplay.xml',
      line: 12,
      column: null
    });
    if (!result.ok) {
      preview = { ok: false, reasonKey: result.reasonKey };
      return;
    }
    // template output keeps placeholders; planEditorLaunch already
    // substituted — show both shapes so the user sees the mapping.
    preview = {
      ok: true,
      argv: [
        t('source.editor.preview.template'),
        JSON.stringify([template.plan.executable, ...template.plan.args]),
        t('source.editor.preview.substituted'),
        JSON.stringify(result.argv)
      ]
    };
    source.noteMockAction('source.action.wouldLaunch');
  }
</script>

<div class="card" data-testid="source.editor.settings">
  <h4 class="title">
    <Icon name="edit" size={13} />
    {t('source.editor.title')}
  </h4>

  <div class="presets" role="radiogroup" aria-label={t('source.editor.title')}>
    {#each EDITOR_PRESETS as p (p.id)}
      <button
        type="button"
        role="radio"
        aria-checked={choice.preset === p.id}
        class="preset"
        class:sel={choice.preset === p.id}
        data-testid={`source.editor.preset.${p.id}`}
        onclick={() => selectPreset(p.id)}
      >
        <span class="name">{t(p.labelKey)}</span>
        {#if p.detectedInDemo}
          <span class="det">{t('source.editor.detected')}</span>
        {/if}
      </button>
    {/each}
  </div>

  {#if choice.preset === 'custom'}
    <label class="field">
      <span>{t('source.editor.executable')}</span>
      <input
        type="text"
        class="mono"
        data-testid="source.editor.executable"
        bind:value={choice.custom.executable}
        placeholder="/usr/local/bin/editor-or-酒場 path"
      />
    </label>
    <label class="field">
      <span>{t('source.editor.args')}</span>
      <input
        type="text"
        class="mono"
        data-testid="source.editor.args"
        bind:value={choice.custom.argsText}
        placeholder={'{path} {line}   — "quoted path" stays one item'}
      />
    </label>
    <p class="hint">
      {t('source.editor.argsHint')}
    </p>
    {#if argsItems.length > 0}
      <p class="mono parsed" data-testid="source.editor.parsed">{JSON.stringify(argsItems)}</p>
    {/if}
  {/if}

  <div class="actions">
    <button type="button" class="btn" data-testid="source.editor.test" onclick={testLaunch}>
      {t('source.editor.test')}
    </button>
    <button
      type="button"
      class="btn btn-primary"
      data-testid="source.editor.save"
      onclick={persist}
      disabled={choice.preset === 'custom' && choice.custom.executable.trim() === ''}
    >
      {t('source.editor.save')}
    </button>
    {#if savedFlash}
      <span class="saved" role="status">{t('source.editor.saved')}</span>
    {/if}
  </div>

  {#if preview}
    {#if preview.ok}
      <div class="preview" data-testid="source.editor.preview">
        {#each preview.argv as line (line)}
          <p class="mono">{line}</p>
        {/each}
      </div>
      <p class="note">{t('source.editor.wouldLaunch')}</p>
    {:else}
      <p class="note warn" role="alert" data-testid="source.editor.error">{t(preview.reasonKey ?? '')}</p>
    {/if}
  {/if}
</div>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    border: 1px solid var(--border, #ccc2);
    border-radius: 10px;
    padding: 10px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: var(--font-xs, 11px);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-2, inherit);
  }
  .presets {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .preset {
    display: flex;
    align-items: center;
    gap: 6px;
    border: 1px solid var(--border, #ccc3);
    background: transparent;
    color: inherit;
    border-radius: 8px;
    padding: 4px 10px;
    font-size: var(--font-sm, 12px);
    cursor: pointer;
  }
  .preset.sel {
    border-color: var(--accent, #4a8);
    background: color-mix(in srgb, var(--accent, #4a8) 16%, transparent);
  }
  .det {
    font-size: var(--font-xs, 10px);
    color: var(--ok, #3a7);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: var(--font-xs, 11px);
  }
  .field input {
    background: transparent;
    border: 1px solid var(--border, #ccc3);
    border-radius: 8px;
    color: inherit;
    padding: 4px 8px;
    font-size: var(--font-xs, 11px);
  }
  .mono {
    font-family: var(--font-mono, monospace);
  }
  .hint {
    margin: 0;
    font-size: var(--font-xs, 10px);
    color: var(--text-3, inherit);
  }
  .parsed {
    margin: 0;
    font-size: var(--font-xs, 11px);
    word-break: break-all;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .saved {
    font-size: var(--font-xs, 11px);
    color: var(--ok, #3a7);
  }
  .preview {
    border: 1px dashed var(--border, #ccc3);
    border-radius: 8px;
    padding: 6px 8px;
  }
  .preview p {
    margin: 0;
    font-size: var(--font-xs, 11px);
    word-break: break-all;
  }
  .note {
    margin: 0;
    font-size: var(--font-xs, 11px);
  }
  .note.warn {
    color: var(--warning, #c90);
  }
</style>
