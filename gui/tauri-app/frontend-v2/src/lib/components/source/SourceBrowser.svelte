<script lang="ts">
  // Advanced Source Browser (SOURCE_INSPECTOR_MANDATE §6) + Compare (§10).
  // mod → version → load folder → category → files; ACTIVE/shadowed
  // candidates render fixture DATA with reason keys — the GUI computes no
  // winner. Compare shows source ↔ canonical target ↔ generated output; the
  // generated pane is labelled a build artifact, never a second truth.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import { isMissingFile, type SourceFileContent } from '../../source/types';
  import { source } from '../../source/store.svelte';

  const browser = $derived(source.browser ? source.scenario.browser : null);

  let versionSel = $state<string | null>(null);
  let folderSel = $state<string | null>(null);
  let openCategory = $state<string | null>(null);
  let viewedFile = $state<string | null>(null);
  let compareOpen = $state(false);

  $effect(() => {
    // reset local choices when the modal re-opens or the scenario changes
    if (source.browser) {
      versionSel = browser?.versions.find((v) => v.selected)?.label ?? null;
      folderSel = browser?.loadFolders.find((f) => f.selected)?.label ?? null;
      openCategory = null;
      viewedFile = null;
      compareOpen = false;
    }
  });

  const compareEntryId = $derived(source.compare);
  const triple = $derived(compareEntryId ? source.compareFor(compareEntryId) : null);
  const generated = $derived(triple?.generated ?? null);

  const generatedLines = $derived.by(() => {
    if (!generated) return [] as { no: number; text: string; highlighted: boolean }[];
    const span = triple?.generatedSpan;
    return generated.lines.map((text, i) => {
      const no = i + 1;
      const highlighted =
        span?.start !== null && span?.start !== undefined && no >= span.start && no <= (span.end ?? span.start ?? -1);
      return { no, text, highlighted };
    });
  });

  function openFile(path: string) {
    viewedFile = path;
    const f = source.fileFor(path);
    if (!isMissingFile(f)) {
      // jump straight into the full viewer for the resolved file
      source.openFileViewer(path);
    }
  }

  function nodeLineText(content: SourceFileContent, spanStart: number | null): string {
    if (spanStart === null) return '';
    return content.lines[spanStart - 1] ?? '';
  }
</script>

{#if browser}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions, a11y_no_noninteractive_element_interactions -->
  <div
    class="overlay"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label={t('source.browser.title')}
    data-testid="source.browser"
    onclick={(e) => {
      if (e.target === e.currentTarget) source.closeBrowser();
    }}
  >
    <div class="sheet">
      <header>
        <h3>
          <Icon name="layers" size={15} />
          {t('source.browser.title')}
        </h3>
        <button
          type="button"
          class="btn subtle"
          aria-label={t('common.close')}
          data-testid="source.browser.close"
          onclick={() => source.closeBrowser()}
        >
          <Icon name="close" size={14} />
        </button>
      </header>

      <p class="mod-line">
        <strong>{browser.modName}</strong>
        <span class="mono pkg">{browser.packageId}</span>
      </p>

      <div class="selectors">
        <div class="group">
          <span class="label">{t('source.browser.versions')}</span>
          <div class="chips">
            {#each browser.versions as v (v.label)}
              <button
                type="button"
                class="chip"
                class:sel={versionSel === v.label || (!versionSel && v.selected)}
                data-testid={`source.browser.version.${v.label}`}
                onclick={() => (versionSel = v.label)}
              >
                {v.label}{v.selected ? ` · ${t('source.browser.selectedTag')}` : ''}
              </button>
            {/each}
          </div>
        </div>
        <div class="group">
          <span class="label">{t('source.browser.loadFolders')}</span>
          <div class="chips">
            {#each browser.loadFolders as f (f.label)}
              <button
                type="button"
                class="chip"
                class:sel={folderSel === f.label || (!folderSel && f.selected)}
                onclick={() => (folderSel = f.label)}
              >
                {f.label}{f.selected ? ` · ${t('source.browser.selectedTag')}` : ''}
              </button>
            {/each}
          </div>
        </div>
      </div>

      <div class="tree" data-testid="source.browser.tree">
        {#each browser.categories as cat (cat.name)}
          <details
            open={openCategory === cat.name}
            ontoggle={(e) => {
              if ((e.currentTarget as HTMLDetailsElement).open) openCategory = cat.name;
            }}
          >
            <summary class="mono">{cat.name}</summary>
            <ul>
              {#each cat.files as f (f.path)}
                <li>
                  <button
                    type="button"
                    class="file-btn mono"
                    data-testid={`source.browser.file`}
                    onclick={() => openFile(f.path)}
                  >
                    <Icon name="file-code" size={12} />
                    {f.displayPath}
                  </button>
                </li>
              {/each}
            </ul>
          </details>
        {/each}
      </div>

      <section>
        <h4>{t('source.browser.candidates')}</h4>
        <table class="cands" data-testid="source.browser.candidates">
          <tbody>
            {#each browser.candidates as c (c.path)}
              <tr class={c.status}>
                <td class="mono path">{c.displayPath}</td>
                <td>
                  <span class="badge {c.status}">{t(`source.badge.${c.status === 'active' ? 'effective' : 'shadowed'}`)}</span>
                </td>
                <td class="reason">{t(c.reasonKey)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>

      {#if viewedFile}
        {@const f = source.fileFor(viewedFile)}
        {#if !isMissingFile(f)}
          <section>
            <h4>{t('source.browser.preview')}</h4>
            <pre class="preview mono">{nodeLineText(f, f.spans[0]?.start ?? null) || f.lines.slice(0, 6).join('\n')}</pre>
          </section>
        {/if}
      {/if}

      {#if compareEntryId && triple}
        <section class="compare" data-testid="source.compare">
          <h4>
            <Icon name="git-compare" size={13} />
            {t('source.compare.title')}
          </h4>
          <div class="panes">
            <div class="pane">
              <span class="pane-title">{t('source.compare.source')}</span>
              <p class="mono text">{triple.sourceText}</p>
            </div>
            <div class="pane">
              <span class="pane-title">{t('source.compare.target')}</span>
              <p class="mono text">{triple.targetText ?? '—'}</p>
            </div>
            <div class="pane">
              <span class="pane-title">{t('source.compare.generated')}</span>
              <div class="gen mono">
                {#each generatedLines as l (l.no)}
                  <div class="gen-row" class:hl={l.highlighted}>
                    <span class="no">{l.no}</span>
                    <span>{l.text}</span>
                  </div>
                {/each}
              </div>
            </div>
          </div>
          <p class="note">{t('source.compare.notTruth')}</p>
          <div class="actions">
            <button
              type="button"
              class="btn"
              onclick={() => {
                compareOpen = !compareOpen;
              }}
            >
              {compareOpen ? t('source.compare.hideFile') : t('source.compare.showFile')}
            </button>
            {#if compareOpen && generated}
              <button type="button" class="btn" onclick={() => source.openFileViewer(generated.path)}>
                {t('source.compare.openViewer')}
              </button>
            {/if}
          </div>
        </section>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: color-mix(in srgb, var(--bg, #111) 55%, transparent);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 70;
  }
  .sheet {
    width: min(860px, 92vw);
    max-height: 86vh;
    overflow: auto;
    background: var(--surface, #1a1a1a);
    border: 1px solid var(--border, #ccc3);
    border-radius: var(--radius-lg, 12px);
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: var(--font-md, 14px);
  }
  h4 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 6px;
    font-size: var(--font-xs, 11px);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-2, inherit);
  }
  .mod-line {
    margin: 0;
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .pkg {
    font-size: var(--font-xs, 11px);
    color: var(--text-2, inherit);
  }
  .selectors {
    display: flex;
    gap: 16px;
    flex-wrap: wrap;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .label {
    font-size: var(--font-xs, 11px);
    color: var(--text-2, inherit);
  }
  .chips {
    display: flex;
    gap: 6px;
  }
  .chip {
    border: 1px solid var(--border, #ccc3);
    border-radius: 999px;
    background: transparent;
    color: inherit;
    font-size: var(--font-xs, 11px);
    padding: 2px 10px;
    cursor: pointer;
  }
  .chip.sel {
    background: color-mix(in srgb, var(--accent, #4a8) 22%, transparent);
    border-color: var(--accent, #4a8);
  }
  .tree details {
    margin-bottom: 4px;
  }
  .tree summary {
    cursor: pointer;
    font-size: var(--font-xs, 11px);
  }
  .tree ul {
    list-style: none;
    margin: 4px 0 0;
    padding: 0 0 0 14px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .file-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    border: none;
    color: inherit;
    cursor: pointer;
    font-size: var(--font-xs, 11px);
    padding: 2px 4px;
    border-radius: 4px;
  }
  .file-btn:hover {
    background: color-mix(in srgb, var(--accent, #4a8) 12%, transparent);
  }
  .cands {
    width: 100%;
    border-collapse: collapse;
  }
  .cands td {
    padding: 4px 6px;
    font-size: var(--font-xs, 11px);
    border-top: 1px solid var(--border, #ccc2);
  }
  .cands tr.shadowed .path {
    opacity: 0.7;
  }
  .path {
    word-break: break-all;
  }
  .badge {
    display: inline-flex;
    padding: 0 6px;
    border-radius: 999px;
    font-size: var(--font-xs, 10px);
    line-height: 16px;
  }
  .badge.active {
    background: color-mix(in srgb, var(--accent, #4a8) 18%, transparent);
  }
  .badge.shadowed {
    background: color-mix(in srgb, var(--warning, #c90) 18%, transparent);
  }
  .reason {
    color: var(--text-2, inherit);
  }
  .preview {
    margin: 0;
    padding: 8px;
    border: 1px solid var(--border, #ccc2);
    border-radius: 8px;
    font-size: var(--font-xs, 11px);
    white-space: pre-wrap;
    word-break: break-all;
    max-height: 160px;
    overflow: auto;
  }
  .panes {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 8px;
  }
  .pane {
    border: 1px solid var(--border, #ccc2);
    border-radius: 8px;
    padding: 6px;
    min-width: 0;
  }
  .pane-title {
    font-size: var(--font-xs, 10px);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-2, inherit);
  }
  .text {
    margin: 4px 0 0;
    font-size: var(--font-xs, 11px);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .gen {
    margin-top: 4px;
    max-height: 180px;
    overflow: auto;
    font-size: var(--font-xs, 10px);
  }
  .gen-row {
    display: flex;
    gap: 6px;
  }
  .gen-row.hl {
    background: color-mix(in srgb, var(--accent, #4a8) 18%, transparent);
  }
  .gen-row .no {
    color: var(--text-3, inherit);
    width: 18px;
    text-align: right;
    flex: none;
    user-select: none;
  }
  .note {
    margin: 0;
    font-size: var(--font-xs, 11px);
    color: var(--text-2, inherit);
  }
  .actions {
    display: flex;
    gap: 6px;
  }
</style>
