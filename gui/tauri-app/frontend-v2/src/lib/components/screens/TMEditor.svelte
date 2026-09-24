<script lang="ts">
// Translation Memory editor (mandate §11): search, similarity, origin
// project, provenance, use/apply (mock), edit/delete, import/export and a
// duplicates view. Import/export are mocks — no file I/O. The mock corpus is
// 14 rows; a production corpus runs to tens of thousands, so the real table
// must be virtualized (see the note at the bottom — same requirement as the
// Workspace table in spec §2.4).
//
// Mutability provenance (W4.5 requirement #2): each row shows where it came
// from; reference-read-only rows have edit/delete disabled with a tooltip
// explaining why, and the mock batch-import marks its rows `imported`.
import Icon from '../Icon.svelte';
import { t } from '../../../i18n/store.svelte';
import { router } from '../../router.svelte';
import { tm, type TmEntry, type TmOrigin } from '../../stores/tm.svelte';
import { isReadOnly, mutabilityKey, type Mutability } from '../../mutability';

  let query = $state('');
  let onlyDuplicates = $state(false);

  // ---- edit form ----------------------------------------------------------
  interface EntryForm {
    editingId: string;
    source: string;
    target: string;
    similarity: number;
    originProject: string;
    provenance: TmOrigin;
  }

  let form = $state<EntryForm | null>(null);
  let confirmingRemove = $state<string | null>(null);
  let applied = $state<Record<string, boolean>>({});
  let ioFlash = $state(false);
  let ioTimer: ReturnType<typeof setTimeout> | undefined;
  let timers: ReturnType<typeof setTimeout>[] = [];

  $effect(() => {
    return () => {
      clearTimeout(ioTimer);
      timers.forEach(clearTimeout);
    };
  });

  const dupIds = $derived(tm.duplicateIds());

  const filtered = $derived.by(() => {
    let rows = tm.search(query);
    if (onlyDuplicates) rows = rows.filter((r) => dupIds.has(r.id));
    return rows;
  });

  const PROVENANCE_KEY: Record<TmOrigin, string> = {
    human: 'workspace.detail.origin.human',
    tm: 'workspace.detail.origin.TM',
    ai: 'workspace.detail.origin.LLM',
    imported: 'workspace.detail.origin.imported'
  };

  function readOnly(e: TmEntry): boolean {
    return isReadOnly(e.mutability);
  }

  function openEdit(entry: TmEntry) {
    if (readOnly(entry)) return; // store refuses anyway; button is disabled
    form = {
      editingId: entry.id,
      source: entry.source,
      target: entry.target,
      similarity: entry.similarity,
      originProject: entry.originProject,
      provenance: entry.provenance
    };
  }

  function saveForm() {
    if (!form) return;
    if (!form.source.trim() || !form.target.trim()) return;
    tm.update(form.editingId, {
      source: form.source.trim(),
      target: form.target.trim(),
      similarity: Math.min(100, Math.max(0, Math.round(form.similarity))),
      originProject: form.originProject.trim() || 'library',
      provenance: form.provenance
    });
    form = null;
  }

  function removeEntry(id: string) {
    if (confirmingRemove !== id) {
      confirmingRemove = id;
      return;
    }
    confirmingRemove = null;
    tm.remove(id); // store refuses reference-read-only rows
    applied = { ...applied, [id]: false };
  }

  /** Mock "use/apply": pretends the pair was inserted into the current
   *  project entry and flashes confirmation on the row. */
  function applyEntry(id: string) {
    applied = { ...applied, [id]: true };
    const timer = setTimeout(() => {
      applied = { ...applied, [id]: false };
    }, 2200);
    timers.push(timer);
  }

  /**
   * Mock batch import: appends a canned pair marked `imported` (W4.5 #2) so
   * the provenance is visible; no file is read.
   */
  function mockImport() {
    tm.addImported({
      source: 'Select a research project to begin.',
      target: 'Выберите исследовательский проект, чтобы начать.',
      sourceLang: 'en',
      targetLang: 'ru',
      similarity: 100,
      originProject: 'Imported pack 2026-09',
      provenance: 'imported',
      addedAt: '2026-09-25'
    });
    flashIo();
  }

  /** Mock export: builds the TMX-shaped preview and copies it when possible. */
  function mockExport() {
    const body = tm.list
      .map(
        (e) =>
          `<tu srclang="${e.sourceLang}" trglang="${e.targetLang}"><tuv xml:lang="${e.sourceLang}"><seg>${e.source}</seg></tuv><tuv xml:lang="${e.targetLang}"><seg>${e.target}</seg></tuv><prop type="origin">${e.originProject}</prop><prop type="provenance">${e.provenance}</prop></tu>`
      )
      .join('\n');
    const tmx = `<?xml version="1.0" encoding="UTF-8"?>\n<tmx version="1.4">\n${body}\n</tmx>`;
    navigator.clipboard?.writeText(tmx).catch(() => {
      /* clipboard unavailable — the status line below still shows */
    });
    flashIo();
  }

  function flashIo() {
    ioFlash = true;
    clearTimeout(ioTimer);
    ioTimer = setTimeout(() => (ioFlash = false), 2400);
  }
</script>

<section class="tm" aria-labelledby="tm-heading">
  <div class="head">
    <h1 id="tm-heading" class="title">
      <Icon name="database" size={20} />
      {t('tm.title')}
    </h1>
    <p class="subtitle">{t('tm.ed.subtitle')}</p>
  </div>

  <button type="button" class="btn back" data-testid="tm.back" onclick={() => router.navigate('home')}>
    <Icon name="arrow-left" size={14} />
    {t('common.back')}
  </button>

  <div class="toolbar">
    <div class="search">
      <Icon name="search" size={15} />
      <input
        type="search"
        bind:value={query}
        placeholder={t('tm.ed.search')}
        aria-label={t('tm.ed.search')}
        data-testid="tm.search"
      />
    </div>
    <button
      type="button"
      class="btn"
      aria-pressed={onlyDuplicates}
      data-testid="tm.duplicates"
      onclick={() => (onlyDuplicates = !onlyDuplicates)}
    >
      <Icon name="layers" size={14} />
      {t('tm.ed.duplicates')}
      {#if dupIds.size > 0}
        <span class="dup-count">{dupIds.size}</span>
      {/if}
    </button>
    <div class="toolbar-actions">
      <button type="button" class="btn" data-testid="tm.import" onclick={mockImport}>
        <Icon name="upload" size={14} />
        {t('glossary.ed.import')}
      </button>
      <button type="button" class="btn" data-testid="tm.export" onclick={mockExport}>
        <Icon name="download" size={14} />
        {t('glossary.ed.export')}
      </button>
    </div>
  </div>

  {#if ioFlash}
    <p class="flash" role="status" data-testid="tm.ioDone">{t('tm.ed.ioDone')}</p>
  {/if}

  <div class="table-wrap">
    <table class="table">
      <thead>
        <tr>
          <th scope="col">{t('tm.source')}</th>
          <th scope="col">{t('tm.target')}</th>
          <th scope="col">{t('tm.match')}</th>
          <th scope="col">{t('tm.ed.origin')}</th>
          <th scope="col">{t('tm.from')}</th>
          <th scope="col">{t('mutability.label')}</th>
          <th scope="col"><span class="visually-hidden">{t('shortcuts.col.actions')}</span></th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as entry (entry.id)}
          {@const locked = readOnly(entry)}
          <tr class:duplicate={dupIds.has(entry.id)} data-testid={`tm.row.${entry.id}`}>
            <td class="source-cell">
              <span class="mono">{entry.source}</span>
              {#if dupIds.has(entry.id)}
                <span class="chip dup-chip" data-testid={`tm.dup.${entry.id}`}>{t('tm.ed.duplicateTag')}</span>
              {/if}
            </td>
            <td class="target-cell">{entry.target}</td>
            <td class="sim-cell">
              <span class="sim-bar" aria-hidden="true"><span class="sim-fill" style={`width: ${entry.similarity}%`}></span></span>
              <span class="mono sim-num">{entry.similarity}%</span>
            </td>
            <td>
              <span class="chip prov">{t(PROVENANCE_KEY[entry.provenance])}</span>
              <span class="mono pair"> {entry.sourceLang} → {entry.targetLang}</span>
            </td>
            <td class="origin-cell">{entry.originProject}</td>
            <td class="mut-cell">
              <span
                class="chip mut"
                class:locked
                data-testid={`tm.mut.${entry.id}`}
                title={locked ? t('mutability.readOnlyReason') : undefined}
              >
                {#if locked}<Icon name="lock-closed" size={10} />{/if}
                {t(mutabilityKey(entry.mutability as Mutability))}
              </span>
            </td>
            <td class="row-actions">
              <button type="button" class="btn subtle" data-testid={`tm.apply.${entry.id}`} onclick={() => applyEntry(entry.id)}>
                {applied[entry.id] ? t('tm.ed.applied') : t('tm.ed.apply')}
              </button>
              <button
                type="button"
                class="btn subtle"
                data-testid={`tm.edit.${entry.id}`}
                disabled={locked}
                title={locked ? t('mutability.readOnlyReason') : undefined}
                onclick={() => openEdit(entry)}
              >
                <Icon name="edit" size={13} />
                {t('providers.inst.edit')}
              </button>
              <button
                type="button"
                class="btn subtle danger-text"
                data-testid={`tm.delete.${entry.id}`}
                disabled={locked}
                title={locked ? t('mutability.readOnlyReason') : undefined}
                onclick={() => removeEntry(entry.id)}
              >
                {confirmingRemove === entry.id ? t('glossary.ed.deleteConfirm') : t('common.delete')}
              </button>
            </td>
          </tr>
        {:else}
          <tr>
            <td colspan="7" class="empty" data-testid="tm.empty">
              {t('tm.ed.empty')}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <p class="count" data-testid="tm.count">{t('tm.ed.count', { count: filtered.length })}</p>

  <p class="virtual-note" data-testid="tm.virtualNote">
    <Icon name="info" size={13} />
    {t('tm.ed.virtualNote')}
  </p>

  {#if form}
    <div class="form" data-testid="tm.form">
      <h3 class="form-title">{t('tm.ed.editTitle')}</h3>
      <div class="form-grid">
        <label class="form-row wide">
          <span class="form-label">{t('tm.source')}</span>
          <input type="text" class="mono" bind:value={form.source} data-testid="tm.form.source" spellcheck="false" />
        </label>
        <label class="form-row wide">
          <span class="form-label">{t('tm.target')}</span>
          <input type="text" bind:value={form.target} data-testid="tm.form.target" />
        </label>
        <label class="form-row">
          <span class="form-label">{t('tm.match')} %</span>
          <input
            type="number"
            min="0"
            max="100"
            bind:value={form.similarity}
            data-testid="tm.form.similarity"
          />
        </label>
        <label class="form-row">
          <span class="form-label">{t('tm.from')}</span>
          <input type="text" bind:value={form.originProject} data-testid="tm.form.origin" spellcheck="false" />
        </label>
        <div class="form-row">
          <span class="form-label" id="tm-form-prov">{t('workspace.detail.origin')}</span>
          <select bind:value={form.provenance} aria-labelledby="tm-form-prov" data-testid="tm.form.provenance">
            {#each (Object.keys(PROVENANCE_KEY) as TmOrigin[]) as o (o)}
              <option value={o}>{t(PROVENANCE_KEY[o])}</option>
            {/each}
          </select>
        </div>
      </div>
      <div class="form-actions">
        <button type="button" class="btn btn-primary" data-testid="tm.form.save" onclick={saveForm}>
          <Icon name="check" size={14} />
          {t('providers.action.save')}
        </button>
        <button type="button" class="btn" onclick={() => (form = null)}>{t('common.cancel')}</button>
      </div>
    </div>
  {/if}
</section>

<style>
  .tm {
    width: min(1040px, 100%);
    margin: 0 auto;
    padding: var(--space-8) var(--space-4);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .head {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .subtitle {
    margin: 0;
    color: var(--color-muted-fg);
  }

  .back {
    align-self: flex-start;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }

  .search {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex: 1;
    min-width: 220px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    padding: 0 var(--space-2);
    color: var(--color-muted-fg);
  }

  .search input {
    flex: 1;
    min-height: var(--control-h);
    border: none;
    background: transparent;
    color: var(--color-fg);
    font-size: var(--text-base-size);
  }

  .search input:focus-visible {
    outline: none;
  }

  .toolbar-actions {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-left: auto;
  }

  .dup-count {
    font-family: var(--font-mono);
    font-size: var(--text-meta-size);
    border: 1px solid var(--color-border);
    border-radius: 999px;
    padding: 0 var(--space-1);
  }

  .flash {
    margin: 0;
    color: var(--color-success);
    font-size: var(--text-meta-size);
  }

  .table-wrap {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-card);
    overflow-x: auto;
  }

  .table {
    border-collapse: collapse;
    width: 100%;
    min-width: 860px;
  }

  th {
    text-align: left;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    font-weight: 600;
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    white-space: nowrap;
  }

  td {
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    vertical-align: top;
  }

  tr:last-child td {
    border-bottom: none;
  }

  tr.duplicate td {
    background: var(--color-muted);
  }

  .source-cell {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 180px;
  }

  .source-cell .mono {
    font-weight: 600;
  }

  .target-cell {
    min-width: 180px;
  }

  .sim-cell {
    white-space: nowrap;
  }

  .sim-bar {
    display: inline-block;
    width: 60px;
    height: 4px;
    border-radius: 999px;
    background: var(--color-muted);
    overflow: hidden;
    vertical-align: middle;
  }

  .sim-fill {
    display: block;
    height: 100%;
    background: var(--color-primary);
    border-radius: 999px;
  }

  .sim-num {
    margin-left: var(--space-1);
    color: var(--color-muted-fg);
  }

  .mono {
    font-family: var(--font-mono);
    font-size: var(--text-dense-size);
  }

  .pair {
    color: var(--color-muted-fg);
    white-space: nowrap;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
    border: 1px solid var(--color-border);
    border-radius: 999px;
    padding: 0 var(--space-1);
    white-space: nowrap;
  }

  .chip.prov {
    margin-right: var(--space-1);
  }

  .dup-chip {
    color: var(--color-warning);
    border-color: var(--color-warning);
    align-self: flex-start;
  }

  .origin-cell {
    color: var(--color-muted-fg);
    font-size: var(--text-dense-size);
    max-width: 160px;
    overflow-wrap: anywhere;
  }

  .mut-cell {
    white-space: nowrap;
  }

  .chip.mut.locked {
    color: var(--color-warning);
    border-color: var(--color-warning);
    gap: 3px;
  }

  .row-actions {
    white-space: nowrap;
    text-align: right;
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

  .danger-text {
    color: var(--color-error);
  }

  .empty {
    text-align: center;
    color: var(--color-muted-fg);
    padding: var(--space-6);
  }

  .count {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .virtual-note {
    margin: 0;
    display: flex;
    align-items: flex-start;
    gap: var(--space-1);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .virtual-note :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  .form {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--shadow-card);
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .form-title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: 16px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-3);
  }

  .form-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .form-row.wide {
    grid-column: 1 / -1;
  }

  .form-row input[type='text'],
  .form-row input[type='number'],
  .form-row select {
    flex: 1;
    min-width: 0;
  }

  .form-row input[type='number'] {
    max-width: 90px;
  }

  .form-label {
    width: 130px;
    flex: none;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .form-actions {
    display: flex;
    gap: var(--space-2);
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  @media (max-width: 640px) {
    .form-grid {
      grid-template-columns: 1fr;
    }

    .form-label {
      width: 100%;
    }
  }
</style>
