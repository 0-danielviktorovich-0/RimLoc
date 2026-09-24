<script lang="ts">
  // Read-only Source Viewer (SOURCE_INSPECTOR_MANDATE §2, §5). Literal line
  // rendering with text interpolation ONLY — the fixture bodies contain XML
  // and are never handed to {@html}. Line numbers are shown only when the
  // fixture guarantees them; spans highlight the current node and drive
  // node→entry navigation; folding folds a span's body lines; search counts
  // and walks matches. Copy uses the fulfilled-write-or-fallback helper.
    import { tick } from 'svelte';
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import { isMissingFile, type SourceFileContent } from '../../source/types';
  import { source } from '../../source/store.svelte';

  // Lead review 029 #1: two viewer targets — an entry+usage anchor OR an
  // explicit fixture file (generated / shadowed / unmapped / missing). The
  // requested file renders as-is; no unrelated first-entry fallback.
  const target = $derived(source.viewer);
  const entryId = $derived(target?.kind === 'entry' ? target.entryId : null);
  const usageIndex = $derived(target?.kind === 'entry' ? target.usageIndex : 0);
  const data = $derived(entryId ? source.contextFor(entryId) : null);
  const usage = $derived(data?.usages[Math.min(usageIndex, (data?.usages.length ?? 1) - 1)] ?? null);
  const path = $derived(
    target === null ? null : target.kind === 'file' ? target.path : (usage?.location.path ?? null)
  );
  const file = $derived(path !== null ? source.fileFor(path) : null);
  const content = $derived(file && !isMissingFile(file) ? (file as SourceFileContent) : null);
  const missing = $derived(file !== null && isMissingFile(file));

  const activeSpan = $derived.by(() => {
    if (!content || !usage) return null;
    const pathSpan = content.spans.find((s) => s.start !== null && s.start === usage.location.line);
    if (pathSpan) return pathSpan;
    return content.spans.find((s) => s.entryId === entryId) ?? null;
  });

  // folding: span ids whose BODY lines are collapsed (first line stays)
  let folded = $state<Set<string>>(new Set());

  $effect(() => {
    // reset fold/search state when the viewed file changes
    void content?.path;
    folded = new Set();
    query = '';
    matchIdx = 0;
  });

  let query = $state('');
  let matchIdx = $state(0);

  const matchingLines = $derived.by(() => {
    if (!content || !query.trim()) return [] as number[];
    const q = query.toLowerCase();
    const hits: number[] = [];
    content.lines.forEach((line, i) => {
      if (line.toLowerCase().includes(q)) hits.push(i + 1);
    });
    return hits;
  });

  // Lead review 029 #4: keep the index valid across query changes —
  // a stale "3/1" was possible before the clamp.
  $effect(() => {
    if (matchIdx >= matchingLines.length) matchIdx = Math.max(0, matchingLines.length - 1);
  });

  /** Unfold folded spans covering the line, then scroll it into view. */
  async function revealLine(lineNo: number | null) {
    if (lineNo === null || !content) return;
    for (const s of content.spans) {
      if (s.start === null || s.end === null || s.start === s.end) continue;
      if (lineNo > s.start && lineNo <= s.end && folded.has(s.nodeId)) {
        const next = new Set(folded);
        next.delete(s.nodeId);
        folded = next;
      }
    }
    await tick();
    document.querySelector(`[data-line="${lineNo}"]`)?.scrollIntoView({ block: 'center' });
  }

  function nextMatch() {
    if (matchingLines.length === 0) return;
    matchIdx = (matchIdx + 1) % matchingLines.length;
    void revealLine(matchingLines[matchIdx]);
  }

  // Reveal the anchored node when the viewer opens on an entry usage.
  $effect(() => {
    if (target?.kind === 'entry' && usage && usage.location.line !== null) {
      void revealLine(usage.location.line);
    }
  });

  function inFoldedBody(lineNo: number): boolean {
    if (!content) return false;
    for (const s of content.spans) {
      if (s.start === null || s.end === null || s.start === s.end) continue;
      if (!folded.has(s.nodeId)) continue;
      if (lineNo > s.start && lineNo <= s.end) return true;
    }
    return false;
  }

  function toggleFold(nodeId: string) {
    const next = new Set(folded);
    if (next.has(nodeId)) next.delete(nodeId);
    else next.add(nodeId);
    folded = next;
  }

  function spanFor(lineNo: number) {
    return content?.spans.find((s) => s.start === lineNo) ?? null;
  }

  /** node→entry: a click on a mapped span selects the entry in the workspace. */
  function gotoEntry(nodeId: string) {
    const mapped = content ? source.entryForNode(content.path, nodeId) : null;
    if (mapped) {
      project.select(mapped);
      source.closeViewer();
    }
  }

  /** Escape closes the viewer and hands focus back to the app (029 #4). */
  function onEscape(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    e.stopPropagation();
    source.closeViewer();
    (document.querySelector('[data-testid="workspace.context"]') as HTMLElement | null)?.focus?.();
  }

  function bind(el: HTMLDivElement): void {
    if (!el.dataset.escBound) {
      el.dataset.escBound = '1';
      el.addEventListener('keydown', onEscape);
    }
  }

  let copyState = $state<'idle' | 'copied' | 'fallback'>('idle');

  async function copySelection(text: string, labelKey: string) {
    copyState = 'idle';
    const res = await source.copyText(`viewer-${labelKey}`, labelKey, text);
    copyState = res === 'copied' ? 'copied' : 'fallback';
  }
</script>

{#if target && (target.kind === 'file' || entryId)}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions, a11y_no_noninteractive_element_interactions -->
  <div
    class="overlay"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label={t('source.viewer.title')}
    data-testid="source.viewer"
    use:bind
    onclick={(e) => {
      if (e.target === e.currentTarget) source.closeViewer();
    }}
  >
    <div class="sheet">
      <header>
        <div class="crumbs">
          {#if content}
            {#each content.breadcrumb as crumb, i (i)}
              <span class="crumb" class:file={i === content.breadcrumb.length - 1}>{crumb}</span>
              {#if i < content.breadcrumb.length - 1}<span class="sep">›</span>{/if}
            {/each}
          {:else if file && isMissingFile(file)}
            <span class="crumb file">{file.displayPath}</span>
          {/if}
        </div>
        <button
          type="button"
          class="btn subtle"
          aria-label={t('common.close')}
          data-testid="source.viewer.close"
          onclick={() => source.closeViewer()}
        >
          <Icon name="close" size={14} />
        </button>
      </header>

      {#if content}
        <div class="tools">
          <div class="search">
            <Icon name="search" size={13} />
            <input
              type="search"
              placeholder={t('source.viewer.searchPlaceholder')}
              data-testid="source.viewer.search"
              bind:value={query}
              onkeydown={(e) => {
                if (e.key === 'Enter') nextMatch();
              }}
            />
            {#if query.trim()}
              <span class="match-count" data-testid="source.viewer.matchCount">
                {matchingLines.length === 0 ? t('source.viewer.noMatches') : `${matchIdx + 1}/${matchingLines.length}`}
              </span>
              <button type="button" class="btn subtle" onclick={nextMatch} disabled={matchingLines.length === 0}>
                <Icon name="arrow-right" size={12} />
              </button>
            {/if}
          </div>
          <button
            type="button"
            class="btn"
            data-testid="source.viewer.copyLine"
            onclick={() => {
              if (!usage || usage.location.line === null) return;
              void copySelection(content.lines[usage.location.line - 1] ?? '', 'source.copy.line');
            }}
            disabled={usage?.location.line === null || usage === null}
          >
            <Icon name="copy" size={12} />
            {t('source.viewer.copyLine')}
          </button>
          <button
            type="button"
            class="btn"
            data-testid="source.viewer.copyPath"
            onclick={() => void copySelection(content.displayPath, 'source.copy.location')}
          >
            <Icon name="copy" size={12} />
            {t('source.viewer.copyPath')}
          </button>
        </div>

        {#if usage?.location.line === null}
          <p class="note warn">{t('source.viewer.lineUnknown')}</p>
        {/if}

        {#if copyState !== 'idle'}
          <p class="note {copyState === 'copied' ? 'ok' : 'warn'}" role="status">
            {copyState === 'copied' ? t('source.copy.copied') : t('source.copy.fallback')}
          </p>
        {/if}
        {#if source.clipboardFallback && source.clipboardFallback.id.startsWith('viewer-')}
          <pre class="fallback mono" data-testid="source.viewer.fallback">{source.clipboardFallback.text}</pre>
        {/if}

        <div class="code" data-testid="source.viewer.code">
          {#each content.lines as line, i (i)}
            {@const lineNo = i + 1}
            {@const span = spanFor(lineNo)}
            {@const foldedAway = inFoldedBody(lineNo)}
            {#if !foldedAway}
              <div
                class="row"
                class:hit={query.trim() !== '' && matchingLines.includes(lineNo)}
                class:activeMatch={query.trim() !== '' && matchingLines[matchIdx] === lineNo}
                class:current={activeSpan !== null && lineNo >= (activeSpan.start ?? -1) && lineNo <= (activeSpan.end ?? -1)}
              >
                <span class="gutter mono" data-line={lineNo}>
                  {content.lineNumbersGuaranteed ? lineNo : '·'}
                </span>
                {#if span}
                  <button
                    type="button"
                    class="fold"
                    aria-label={t('source.viewer.fold')}
                    onclick={() => toggleFold(span.nodeId)}
                  >
                    <Icon name={folded.has(span.nodeId) ? 'chevron-right' : 'chevron-down'} size={11} />
                  </button>
                {:else}
                  <span class="fold-spacer"></span>
                {/if}
                {#if span && span.entryId}
                  <button
                    type="button"
                    class="node mono"
                    data-testid={`source.viewer.node.${span.nodeId}`}
                    title={t('source.viewer.gotoEntry')}
                    onclick={() => gotoEntry(span.nodeId)}
                  >
                    {line}
                  </button>
                {:else}
                  <span class="line mono">{line}</span>
                {/if}
              </div>
            {/if}
          {/each}
        </div>
        <p class="note">{t('source.viewer.readOnly')}</p>
      {:else if file && isMissingFile(file)}
        <div class="missing" data-testid="source.viewer.missing">
          <p>
            <Icon name="warning" size={14} />
            {t('source.missing.title')}
          </p>
          <p class="mono">{file.displayPath}</p>
          <p class="hint">{t(file.reasonKey)}</p>
        </div>
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
    display: flex;
    flex-direction: column;
    background: var(--surface, #1a1a1a);
    border: 1px solid var(--border, #ccc3);
    border-radius: var(--radius-lg, 12px);
    padding: 12px;
    gap: 8px;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-wrap: wrap;
    min-width: 0;
  }
  .crumb {
    font-size: var(--font-xs, 11px);
    color: var(--text-2, inherit);
  }
  .crumb.file {
    color: var(--text-1, inherit);
    font-weight: 600;
  }
  .sep {
    color: var(--text-3, inherit);
  }
  .tools {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    border: 1px solid var(--border, #ccc3);
    border-radius: 8px;
    padding: 2px 8px;
    flex: 1;
    min-width: 200px;
  }
  .search input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: none;
    outline: none;
    color: inherit;
    font-size: var(--font-sm, 12px);
  }
  .match-count {
    font-size: var(--font-xs, 11px);
    white-space: nowrap;
  }
  .note {
    margin: 0;
    font-size: var(--font-xs, 11px);
  }
  .note.ok {
    color: var(--ok, #3a7);
  }
  .note.warn {
    color: var(--warning, #c90);
  }
  .fallback {
    max-height: 120px;
    overflow: auto;
    margin: 0;
    padding: 8px;
    border: 1px dashed var(--border, #ccc3);
    border-radius: 8px;
    font-size: var(--font-xs, 11px);
    white-space: pre-wrap;
    user-select: text;
  }
  .code {
    overflow: auto;
    border: 1px solid var(--border, #ccc2);
    border-radius: 8px;
    padding: 6px 0;
    min-height: 120px;
  }
  .row {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 0 8px;
  }
  .row.hit {
    background: color-mix(in srgb, var(--accent, #4a8) 14%, transparent);
  }
  .row.activeMatch {
    background: color-mix(in srgb, var(--accent, #4a8) 30%, transparent);
  }
  .row.current {
    box-shadow: inset 2px 0 0 var(--accent, #4a8);
  }
  .gutter {
    width: 34px;
    flex: none;
    text-align: right;
    color: var(--text-3, inherit);
    font-size: var(--font-xs, 11px);
    user-select: none;
  }
  .fold,
  .fold-spacer {
    width: 18px;
    flex: none;
  }
  .fold {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--text-2, inherit);
    cursor: pointer;
    padding: 0;
    height: 18px;
  }
  .line,
  .node {
    font-family: var(--font-mono, monospace);
    font-size: var(--font-xs, 11px);
    white-space: pre-wrap;
    word-break: break-all;
  }
  .node {
    text-align: left;
    background: color-mix(in srgb, var(--accent, #4a8) 16%, transparent);
    border: none;
    border-radius: 4px;
    color: inherit;
    cursor: pointer;
    flex: 1;
    padding: 0 4px;
  }
  .missing {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 24px 8px;
  }
  .missing p {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
  }
  .hint {
    color: var(--text-2, inherit);
    font-size: var(--font-xs, 11px);
  }
</style>
