<script lang="ts">
  // SOURCE tab of the entry detail panel (SOURCE_INSPECTOR_MANDATE §1-§4,
  // §14). Two modes, never mixed (wave 12 live bridge):
  //  - LIVE (contract transport): the snapshot projected the entry's real
  //    source as `source_ref` (backend-projected effective file relative to
  //    the project root + parser-guaranteed line + winner reason). The tab
  //    renders THAT as data — fixture usages/excerpts/browser are never
  //    shown here, unavailable blocks say so honestly.
  //  - FIXTURE (mock): typed fixture seam (source/): effective file, logical
  //    key, node location with HONEST nullable line/column, the "Why this
  //    source?" list rendered straight from provenance data (no GUI
  //    precedence logic), primary + other usages for shared TKey identities
  //    and a structured excerpt. One-click actions feed the viewer / browser
  //    / compare overlays; OS actions are demo-honest ("would open").
  import Icon from '../Icon.svelte';
  import { t, i18n } from '../../../i18n/store.svelte';
  import { project } from '../../stores/project.svelte';
  import {
    isMissingFile,
    type SourceFile,
    type SourceUsage
  } from '../../source/types';
  import { source, SOURCE_SCENARIOS } from '../../source/store.svelte';
  import { devMode } from '../../stores/devmode.svelte';
  import { loadEditorChoice } from '../../source/editor';

  // Scenario controls are a developer tool (W6 scenario browser is the
  // integrated home post-freeze); gated by the ONE explicit dev mode (W6).
  const devScenarios = devMode.enabled;

  interface Props {
    entry: {
      id: string;
      kind: string;
      key: string;
      /** Live projection (contract snapshots); null/absent in mock. */
      sourceRef?: { file: string; line: number | null; selected_by: string } | null;
    };
  }

  let { entry }: Props = $props();

  // Live = the contract transport owns the open project. The per-entry
  // projection rides the snapshot; a contract entry without one renders the
  // honest live-empty state, never fixture data.
  const live = $derived(project.source === 'contract');
  const liveRef = $derived(entry.sourceRef ?? null);

  const data = $derived(live ? null : source.contextFor(entry.id));
  const primary = $derived(data?.usages.find((u) => u.role === 'primary') ?? data?.usages[0] ?? null);
  const others = $derived(data?.usages.filter((u) => u !== primary) ?? []);

  const usageFile = (u: SourceUsage): SourceFile => source.fileFor(u.location.path);

  let copyState = $state<'idle' | 'copied' | 'fallback'>('idle');
  let editorState = $state<{ ok: boolean; text: string; errorKey?: string } | null>(null);

  /** Human formulation of a live winner reason. The winner_reason
   *  vocabulary is mirrored in i18n; an unknown FUTURE reason falls back to
   *  the raw machine token — honest, never a guessed phrase. */
  function whyLabel(selectedBy: string): string {
    const key = `source.why.${selectedBy}`;
    return i18n.has(key) ? t(key) : selectedBy;
  }

  async function copyLocation() {
    copyState = 'idle';
    const loc = live
      ? `${liveRef?.file ?? ''}:${liveRef?.line ?? '?'}`
      : `${primary?.location.displayPath ?? ''}:${primary?.location.line ?? '?'}`;
    const result = await source.copyText(`${entry.id}-location`, 'source.copy.location', loc);
    copyState = result === 'copied' ? 'copied' : 'fallback';
  }

  /** Mock launch: build + validate the structured argv, then say honestly
   *  what WOULD run. No process is started — in either mode the editor
   *  protocol only PLANS; live mode labels the preview as live. */
  function openInEditor() {
    const loc = live
      ? { displayPath: liveRef?.file ?? '', line: liveRef?.line ?? null, column: null }
      : primary?.location;
    if (!loc) return;
    const plan = source.planEditorLaunch(loadEditorChoice(), loc);
    if (!plan.ok) {
      editorState = { ok: false, text: '', errorKey: plan.reasonKey };
      return;
    }
    editorState = { ok: true, text: JSON.stringify(plan.argv) };
    source.noteMockAction(live ? 'source.editor.wouldLaunchLive' : 'source.action.wouldLaunch');
  }

  function reveal() {
    source.noteMockAction('source.action.wouldReveal', {
      path: primary?.location.displayPath ?? ''
    });
  }
</script>

{#if live}
  {#if liveRef}
    <div class="source-tab" data-testid="source.tab.live">
      <!-- effective location, backend-projected -->
      <section>
        <h4 class="sec-title">
          <Icon name="file-code" size={13} />
          {t('source.tab.location')}
          <span class="badge active">{t('source.badge.effective')}</span>
        </h4>
        <p class="loc-path mono" data-testid="source.tab.live.path">{liveRef.file}</p>
        <p class="hint">{t('source.tab.live.relativePath')}</p>
        <p class="loc-line" data-testid="source.tab.live.line">
          {t('source.tab.line')}:
          {#if liveRef.line !== null}
            <span class="mono">{liveRef.line}</span>
          {:else}
            <span class="unknown" title={t('source.tab.unknownHint')}>—</span>
            <span class="unknown-note">{t('source.tab.lineUnknown')}</span>
          {/if}
        </p>
      </section>

      <!-- why this source: the winner reason, human-formulated -->
      <section>
        <h4 class="sec-title">{t('source.tab.why')}</h4>
        <ul class="why" data-testid="source.tab.live.why">
          <li>
            <span class="why-kind">{whyLabel(liveRef.selected_by)}</span>
          </li>
        </ul>
      </section>

      <!-- usages/compare are fixture-only capabilities: honest unavailability,
           never fixture data standing in for a live project -->
      <section>
        <p class="unavailable" data-testid="source.tab.live.usagesUnavailable">
          <Icon name="info" size={13} />
          {t('source.tab.live.usagesUnavailable')}
        </p>
      </section>

      <!-- actions: only what a live project can honestly do -->
      <div class="actions" data-testid="source.tab.live.actions">
        <button type="button" class="btn" data-testid="source.tab.live.copy" onclick={copyLocation}>
          <Icon name="copy" size={13} />
          {t('source.action.copy')}
        </button>
        <button
          type="button"
          class="btn"
          data-testid="source.tab.live.editor"
          onclick={openInEditor}
        >
          <Icon name="edit" size={13} />
          {t('source.action.editor')}
        </button>
      </div>
      {#if copyState === 'copied'}
        <p class="note ok" role="status">{t('source.copy.copied')}</p>
      {:else if copyState === 'fallback'}
        <p class="note warn" role="status">{t('source.copy.fallback')}</p>
      {/if}
      {#if editorState}
        {#if editorState.ok}
          <p class="note mono" role="status" data-testid="source.tab.live.editorPreview">
            {editorState.text}
          </p>
          <p class="note">{t('source.editor.wouldLaunchLive')}</p>
        {:else}
          <p class="note warn" role="alert">{t(editorState.errorKey ?? 'source.editor.error.noExecutable')}</p>
        {/if}
      {/if}
    </div>
  {:else}
    <!-- honest live empty state: the backend could not project this entry -->
    <div class="source-tab empty" data-testid="source.tab.live.empty">
      <p>
        <Icon name="info" size={14} />
        {t('source.empty.live')}
      </p>
    </div>
  {/if}
{:else if data && primary}
  <div class="source-tab" data-testid="source.tab">
    <!-- effective location -->
    <section>
      <h4 class="sec-title">
        <Icon name="file-code" size={13} />
        {t('source.tab.location')}
        {#if !primary.effective}
          <span class="badge shadowed">{t('source.badge.shadowed')}</span>
        {:else}
          <span class="badge active">{t('source.badge.effective')}</span>
        {/if}
      </h4>
      <p class="loc-path mono" data-testid="source.tab.path">{primary.location.displayPath}</p>
      <p class="loc-node mono">{primary.location.nodePath}</p>
      <p class="loc-line" data-testid="source.tab.lineCol">
        {t('source.tab.line')}:
        {#if primary.location.line !== null}
          <span class="mono">{primary.location.line}</span>
        {:else}
          <span class="unknown" title={t('source.tab.unknownHint')}>—</span>
        {/if}
        · {t('source.tab.column')}:
        {#if primary.location.column !== null}
          <span class="mono">{primary.location.column}</span>
        {:else}
          <span class="unknown" title={t('source.tab.unknownHint')}>—</span>
        {/if}
      </p>
    </section>

    <!-- why this source: provenance facts AS DATA -->
    <section>
      <h4 class="sec-title">{t('source.tab.why')}</h4>
      <ul class="why" data-testid="source.tab.why">
        {#each primary.provenance as fact (fact.kind + fact.detail)}
          <li>
            <span class="why-kind">{t(`source.why.${fact.kind}`)}</span>
            <span class="why-detail">{fact.detail}</span>
          </li>
        {/each}
      </ul>
    </section>

    <!-- other usages (TKey multi-context) -->
    {#if others.length > 0}
      <section>
        <h4 class="sec-title">{t('source.tab.otherUsages', { n: others.length })}</h4>
        <ul class="usages">
          {#each others as u, i (u.location.path + u.location.line)}
            <li class="usage">
              <span class="badge {u.effective ? 'active' : 'shadowed'}">
                {u.effective ? t('source.badge.effective') : t('source.badge.shadowed')}
              </span>
              <span class="mono usage-loc">{u.location.displayPath}:{u.location.line ?? '—'}</span>
              <button
                type="button"
                class="btn subtle"
                data-testid={`source.tab.other.${i}`}
                onclick={() => {
                  const idx = data.usages.indexOf(u);
                  source.openViewer(entry.id, idx);
                }}
              >
                {t('source.action.view')}
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <!-- structured excerpt -->
    <section>
      <h4 class="sec-title">{t('source.tab.excerpt')}</h4>
      <dl class="excerpt" data-testid="source.tab.excerpt">
        <div class="row">
          <dt>{t('source.excerpt.defName')}</dt>
          <dd class="mono">{data.excerpt.defName}</dd>
        </div>
        <div class="row">
          <dt>{t('source.excerpt.field')}</dt>
          <dd class="mono">{data.excerpt.field}</dd>
        </div>
        {#each data.excerpt.related as rel (rel.labelKey + rel.value)}
          <div class="row">
            <dt>{t(rel.labelKey)}</dt>
            <dd class="mono">{rel.value}</dd>
          </div>
        {/each}
      </dl>
    </section>

    <!-- one-click actions -->
    <div class="actions" data-testid="source.tab.actions">
      <button
        type="button"
        class="btn btn-primary"
        data-testid="source.action.open"
        onclick={() => source.openViewer(entry.id, 0)}
      >
        <Icon name="file-code" size={13} />
        {t('source.action.open')}
      </button>
      <button type="button" class="btn" onclick={reveal}>
        <Icon name="external" size={13} />
        {t('source.action.reveal')}
      </button>
      <button type="button" class="btn" data-testid="source.action.editor" onclick={openInEditor}>
        <Icon name="edit" size={13} />
        {t('source.action.editor')}
      </button>
      <button type="button" class="btn" data-testid="source.action.copy" onclick={copyLocation}>
        <Icon name="copy" size={13} />
        {t('source.action.copy')}
      </button>
      <button type="button" class="btn" onclick={() => source.openBrowser()}>
        <Icon name="layers" size={13} />
        {t('source.action.candidates')}
      </button>
      {#if source.compareFor(entry.id)}
        <button
          type="button"
          class="btn"
          data-testid="source.action.compare"
          onclick={() => source.openCompare(entry.id)}
        >
          <Icon name="git-compare" size={13} />
          {t('source.action.compare')}
        </button>
      {/if}
    </div>
    {#if copyState === 'copied'}
      <p class="note ok" role="status">{t('source.copy.copied')}</p>
    {:else if copyState === 'fallback'}
      <p class="note warn" role="status">{t('source.copy.fallback')}</p>
    {/if}
    {#if editorState}
      {#if editorState.ok}
        <p class="note mono" role="status" data-testid="source.editor.preview">{editorState.text}</p>
        <p class="note">{t('source.editor.wouldLaunch')}</p>
      {:else}
        <p class="note warn" role="alert">{t(editorState.errorKey ?? 'source.editor.error.noExecutable')}</p>
      {/if}
    {/if}

    <!-- advanced: raw file references + demo scenario picker -->
    <details class="advanced">
      <summary>{t('source.tab.advanced')}</summary>
      <dl class="meta">
        {#each data.usages as u (u.location.path + u.location.line)}
          {@const f = usageFile(u)}
          <div class="row">
            <dt>{t('source.tab.file')} · {u.role}</dt>
            <dd class="mono">
              {#if isMissingFile(f)}
                {f.displayPath} — {t(f.reasonKey)}
              {:else}
                {f.displayPath} · {t(`source.file.kind.${f.kind}`)}
                {#if !f.lineNumbersGuaranteed}· {t('source.file.linesNotGuaranteed')}{/if}
              {/if}
            </dd>
          </div>
        {/each}
      </dl>
      {#if devScenarios}
        <label class="scenario-row">
          <span>{t('source.scenario.label')}</span>
          <select
            data-testid="source.scenario.select"
            value={source.scenarioId}
            onchange={(e) => source.setScenario((e.currentTarget as HTMLSelectElement).value)}
          >
            {#each SOURCE_SCENARIOS as s (s.id)}
              <option value={s.id}>{s.id}</option>
            {/each}
          </select>
        </label>
        <p class="hint">{t('source.scenario.hint')}</p>
      {/if}
    </details>
  </div>
{:else}
  <!-- honest empty state: this demo scenario does not cover the entry -->
  <div class="source-tab empty" data-testid="source.tab.empty">
    <p>
      <Icon name="info" size={14} />
      {t('source.empty.noData', { id: entry.id })}
    </p>
    <p class="hint">{t('source.empty.covered', { ids: source.coveredEntryIds().join(', ') })}</p>
    {#if devScenarios}
      <label class="scenario-row">
        <span>{t('source.scenario.label')}</span>
        <select
          data-testid="source.scenario.select.empty"
          value={source.scenarioId}
          onchange={(e) => source.setScenario((e.currentTarget as HTMLSelectElement).value)}
        >
          {#each SOURCE_SCENARIOS as s (s.id)}
            <option value={s.id}>{s.id}</option>
          {/each}
        </select>
      </label>
    {/if}
  </div>
{/if}

<style>
  .source-tab {
    display: flex;
    flex-direction: column;
    gap: var(--space-3, 10px);
  }
  .sec-title {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 4px;
    font-size: var(--font-xs, 11px);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-2, inherit);
  }
  .loc-path {
    margin: 0;
    font-size: var(--font-xs, 11px);
    word-break: break-all;
  }
  .loc-node {
    margin: 2px 0;
    color: var(--text-2, inherit);
    word-break: break-all;
  }
  .loc-line {
    margin: 2px 0 0;
    font-size: var(--font-xs, 11px);
  }
  .unknown {
    opacity: 0.7;
  }
  .unknown-note {
    margin-left: 6px;
    font-size: var(--font-xs, 10px);
    opacity: 0.7;
  }
  .unavailable {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: 4px 6px;
    border: 1px dashed var(--border, #ccc2);
    border-radius: 6px;
    font-size: var(--font-xs, 11px);
    color: var(--text-2, inherit);
  }
  .badge {
    display: inline-flex;
    align-items: center;
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
  .why {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .why li {
    display: flex;
    flex-direction: column;
    padding: 4px 6px;
    border: 1px solid var(--border, #ccc2);
    border-radius: 6px;
  }
  .why-kind {
    font-size: var(--font-xs, 11px);
    font-weight: 600;
  }
  .why-detail {
    font-size: var(--font-xs, 11px);
    color: var(--text-2, inherit);
  }
  .usages {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .usage {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .usage-loc {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .excerpt,
  .meta {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .excerpt .row,
  .meta .row {
    display: grid;
    grid-template-columns: 88px 1fr;
    gap: 6px;
  }
  .excerpt dt,
  .meta dt {
    color: var(--text-2, inherit);
    font-size: var(--font-xs, 11px);
  }
  .excerpt dd,
  .meta dd {
    margin: 0;
    font-size: var(--font-xs, 11px);
    word-break: break-all;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
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
  .advanced summary {
    cursor: pointer;
    font-size: var(--font-xs, 11px);
    color: var(--text-2, inherit);
  }
  .scenario-row {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    font-size: var(--font-xs, 11px);
  }
  .hint {
    margin: 4px 0 0;
    font-size: var(--font-xs, 10px);
    color: var(--text-3, inherit);
  }
  .empty p {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 6px;
  }
</style>
