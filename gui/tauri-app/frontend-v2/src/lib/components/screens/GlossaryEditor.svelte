<script lang="ts">
  // Glossary editor (mandate §10): search / add / edit / delete over bound
  // source→target terms with scope (project vs user), accepted variants and
  // case-sensitivity. Import/export are mocks — buttons flash a status, no
  // file I/O. Conflicts (two terms claiming the same normalized source) are
  // surfaced in a dedicated view instead of failing silently.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { router } from '../../router.svelte';
  import { glossary, type GlossaryScope, type GlossaryTerm } from '../../stores/glossary.svelte';

  const LANGS = ['en', 'ru', 'uk', 'de', 'es', 'fr', 'ja', 'zh-hans'];

  let query = $state('');
  let scopeFilter = $state<'all' | GlossaryScope>('all');

  // ---- add/edit form ------------------------------------------------------
  interface TermForm {
    editingId: string | null;
    source: string;
    target: string;
    note: string;
    sourceLang: string;
    targetLang: string;
    scope: GlossaryScope;
    variantsText: string;
    caseSensitive: boolean;
  }

  let form = $state<TermForm | null>(null);
  let confirmingRemove = $state<string | null>(null);
  let imported = $state(false);
  let importedTimer: ReturnType<typeof setTimeout> | undefined;
  let timers: ReturnType<typeof setTimeout>[] = [];

  $effect(() => {
    return () => {
      clearTimeout(importedTimer);
      timers.forEach(clearTimeout);
    };
  });

  const filtered = $derived.by(() => {
    let rows = glossary.search(query);
    if (scopeFilter !== 'all') rows = rows.filter((r) => r.scope === scopeFilter);
    return rows;
  });

  const conflicts = $derived(glossary.conflicts());

  function openAdd() {
    form = {
      editingId: null,
      source: '',
      target: '',
      note: '',
      sourceLang: 'en',
      targetLang: 'ru',
      scope: 'project',
      variantsText: '',
      caseSensitive: false
    };
  }

  function openEdit(term: GlossaryTerm) {
    form = {
      editingId: term.id,
      source: term.source,
      target: term.target,
      note: term.note,
      sourceLang: term.sourceLang,
      targetLang: term.targetLang,
      scope: term.scope,
      variantsText: term.variants.join(', '),
      caseSensitive: term.caseSensitive
    };
  }

  function saveForm() {
    if (!form) return;
    const patch = {
      source: form.source.trim(),
      target: form.target.trim(),
      note: form.note.trim(),
      sourceLang: form.sourceLang,
      targetLang: form.targetLang,
      scope: form.scope,
      variants: form.variantsText
        .split(',')
        .map((v) => v.trim())
        .filter(Boolean),
      caseSensitive: form.caseSensitive
    };
    if (!patch.source || !patch.target) return;
    if (form.editingId) glossary.update(form.editingId, patch);
    else glossary.add(patch);
    form = null;
  }

  function removeTerm(id: string) {
    if (confirmingRemove !== id) {
      confirmingRemove = id;
      return;
    }
    confirmingRemove = null;
    glossary.remove(id);
  }

  /** Mock import: nothing is read; the button confirms with a status line. */
  function mockImport() {
    imported = true;
    clearTimeout(importedTimer);
    importedTimer = setTimeout(() => (imported = false), 2400);
  }

  /** Mock export: builds the CSV preview string (same format the real export
   *  would write) and copies it to the clipboard when available. */
  function mockExport() {
    const csv = [
      'source,target,source_lang,target_lang,scope,variants,case_sensitive',
      ...glossary.list.map(
        (g) =>
          `${JSON.stringify(g.source)},${JSON.stringify(g.target)},${g.sourceLang},${g.targetLang},${g.scope},"${g.variants.join('; ')}",${g.caseSensitive}`
      )
    ].join('\n');
    navigator.clipboard?.writeText(csv).catch(() => {
      /* clipboard unavailable — the mock status below still shows */
    });
    imported = true;
    clearTimeout(importedTimer);
    importedTimer = setTimeout(() => (imported = false), 2400);
  }
</script>

<section class="glossary" aria-labelledby="glossary-heading">
  <div class="head">
    <h1 id="glossary-heading" class="title">
      <Icon name="book" size={20} />
      {t('glossary.title')}
    </h1>
    <p class="subtitle">{t('glossary.ed.subtitle')}</p>
  </div>

  <button type="button" class="btn back" data-testid="glossary.back" onclick={() => router.navigate('home')}>
    <Icon name="arrow-left" size={14} />
    {t('common.back')}
  </button>

  <!-- Conflict view: 2+ terms share a normalized source -->
  {#if conflicts.length > 0}
    <article class="conflicts" data-testid="glossary.conflicts">
      <h2 class="conflicts-title">
        <Icon name="warning" size={15} />
        {t('glossary.ed.conflicts', { count: conflicts.length })}
      </h2>
      {#each conflicts as group, gi (gi)}
        <p class="conflict-row">
          {t('glossary.ed.conflictGroup', { term: group[0].source.toLowerCase() })}:
          {#each group as term, ti (term.id)}
            {#if ti > 0}·{/if}
            <button type="button" class="link" data-testid={`glossary.conflict.${gi}.${ti}`} onclick={() => openEdit(term)}>
              «{term.target}» ({term.scope === 'project' ? t('glossary.ed.scope.project') : t('glossary.ed.scope.user')})
            </button>
          {/each}
        </p>
      {/each}
      <p class="conflict-note">{t('glossary.ed.conflictNote')}</p>
    </article>
  {/if}

  <!-- Toolbar: search + scope + import/export + add -->
  <div class="toolbar">
    <div class="search">
      <Icon name="search" size={15} />
      <input
        type="search"
        bind:value={query}
        placeholder={t('glossary.ed.search')}
        aria-label={t('glossary.ed.search')}
        data-testid="glossary.search"
      />
    </div>
    <div class="seg" role="group" aria-label={t('glossary.ed.scope')}>
      <button type="button" class="seg-btn" aria-pressed={scopeFilter === 'all'} data-testid="glossary.scope.all" onclick={() => (scopeFilter = 'all')}>
        {t('glossary.ed.scope.all')}
      </button>
      <button type="button" class="seg-btn" aria-pressed={scopeFilter === 'project'} data-testid="glossary.scope.project" onclick={() => (scopeFilter = 'project')}>
        {t('glossary.ed.scope.project')}
      </button>
      <button type="button" class="seg-btn" aria-pressed={scopeFilter === 'user'} data-testid="glossary.scope.user" onclick={() => (scopeFilter = 'user')}>
        {t('glossary.ed.scope.user')}
      </button>
    </div>
    <div class="toolbar-actions">
      <button type="button" class="btn" data-testid="glossary.import" onclick={mockImport}>
        <Icon name="upload" size={14} />
        {t('glossary.ed.import')}
      </button>
      <button type="button" class="btn" data-testid="glossary.export" onclick={mockExport}>
        <Icon name="download" size={14} />
        {t('glossary.ed.export')}
      </button>
      <button type="button" class="btn btn-primary" data-testid="glossary.add" onclick={openAdd}>
        <Icon name="file-plus" size={14} />
        {t('glossary.ed.add')}
      </button>
    </div>
  </div>

  {#if imported}
    <p class="flash" role="status" data-testid="glossary.importExportDone">{t('glossary.ed.importExportDone')}</p>
  {/if}

  <!-- Terms table -->
  <div class="table-wrap">
    <table class="table">
      <thead>
        <tr>
          <th scope="col">{t('glossary.term')}</th>
          <th scope="col">{t('glossary.translation')}</th>
          <th scope="col">{t('glossary.ed.pair')}</th>
          <th scope="col">{t('glossary.ed.scope')}</th>
          <th scope="col">{t('glossary.ed.variants')}</th>
          <th scope="col"><span class="visually-hidden">{t('shortcuts.col.actions')}</span></th>
        </tr>
      </thead>
      <tbody>
        {#each filtered as term (term.id)}
          <tr data-testid={`glossary.row.${term.id}`}>
            <td class="term-cell">
              <span class="mono">{term.source}</span>
              {#if term.caseSensitive}
                <span class="chip" title={t('glossary.ed.caseSensitive')}>Aa</span>
              {/if}
              {#if term.note}
                <span class="note-line">{term.note}</span>
              {/if}
            </td>
            <td class="target-cell">{term.target}</td>
            <td class="mono pair-cell">{term.sourceLang} → {term.targetLang}</td>
            <td>
              <span class="chip scope">{term.scope === 'project' ? t('glossary.ed.scope.project') : t('glossary.ed.scope.user')}</span>
            </td>
            <td class="variants-cell">
              {#if term.variants.length > 0}
                {term.variants.join(', ')}
              {:else}
                <span class="muted">—</span>
              {/if}
            </td>
            <td class="row-actions">
              <button type="button" class="btn subtle" data-testid={`glossary.edit.${term.id}`} onclick={() => openEdit(term)}>
                <Icon name="edit" size={13} />
                {t('providers.inst.edit')}
              </button>
              <button
                type="button"
                class="btn subtle danger-text"
                data-testid={`glossary.delete.${term.id}`}
                onclick={() => removeTerm(term.id)}
              >
                {confirmingRemove === term.id ? t('glossary.ed.deleteConfirm') : t('common.delete')}
              </button>
            </td>
          </tr>
        {:else}
          <tr>
            <td colspan="6" class="empty" data-testid="glossary.empty">
              {t('glossary.ed.empty')}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <p class="count" data-testid="glossary.count">{t('glossary.ed.count', { count: filtered.length })}</p>

  <!-- Add/edit form -->
  {#if form}
    <div class="form" data-testid="glossary.form">
      <h3 class="form-title">{form.editingId ? t('glossary.ed.editTitle') : t('glossary.ed.addTitle')}</h3>

      <div class="form-grid">
        <label class="form-row">
          <span class="form-label">{t('glossary.term')}</span>
          <input type="text" bind:value={form.source} data-testid="glossary.form.source" spellcheck="false" />
        </label>
        <label class="form-row">
          <span class="form-label">{t('glossary.translation')}</span>
          <input type="text" bind:value={form.target} data-testid="glossary.form.target" />
        </label>
        <label class="form-row wide">
          <span class="form-label">{t('glossary.ed.notes')}</span>
          <input type="text" bind:value={form.note} placeholder={t('glossary.ed.notesPlaceholder')} data-testid="glossary.form.note" />
        </label>

        <div class="form-row">
          <span class="form-label" id="glossary-form-pair">{t('glossary.ed.pair')}</span>
          <div class="pair-selects">
            <select bind:value={form.sourceLang} aria-label={t('settings.translation.source')} data-testid="glossary.form.sourceLang">
              {#each LANGS as l (l)}<option value={l}>{l}</option>{/each}
            </select>
            <Icon name="arrow-right" size={13} />
            <select bind:value={form.targetLang} aria-label={t('settings.translation.target')} data-testid="glossary.form.targetLang">
              {#each LANGS as l (l)}<option value={l}>{l}</option>{/each}
            </select>
          </div>
        </div>

        <div class="form-row">
          <span class="form-label" id="glossary-form-scope">{t('glossary.ed.scope')}</span>
          <div class="seg" role="group" aria-labelledby="glossary-form-scope">
            <button type="button" class="seg-btn" aria-pressed={form.scope === 'project'} data-testid="glossary.form.scope.project" onclick={() => form!.scope = 'project'}>
              {t('glossary.ed.scope.project')}
            </button>
            <button type="button" class="seg-btn" aria-pressed={form.scope === 'user'} data-testid="glossary.form.scope.user" onclick={() => form!.scope = 'user'}>
              {t('glossary.ed.scope.user')}
            </button>
          </div>
        </div>

        <label class="form-row wide">
          <span class="form-label">{t('glossary.ed.variants')}</span>
          <input
            type="text"
            bind:value={form.variantsText}
            placeholder={t('glossary.ed.variantsPlaceholder')}
            data-testid="glossary.form.variants"
            spellcheck="false"
          />
        </label>

        <label class="form-row toggle-row">
          <span class="form-label">{t('glossary.ed.caseSensitive')}</span>
          <input type="checkbox" checked={form.caseSensitive} data-testid="glossary.form.caseSensitive" onchange={(e) => (form!.caseSensitive = e.currentTarget.checked)} />
        </label>
      </div>

      <div class="form-actions">
        <button type="button" class="btn btn-primary" data-testid="glossary.form.save" onclick={saveForm}>
          <Icon name="check" size={14} />
          {t('providers.action.save')}
        </button>
        <button type="button" class="btn" onclick={() => (form = null)}>{t('common.cancel')}</button>
      </div>
    </div>
  {/if}
</section>

<style>
  .glossary {
    width: min(960px, 100%);
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

  .conflicts {
    border: 1px solid var(--color-warning);
    border-radius: var(--radius-lg);
    padding: var(--space-3) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .conflicts-title {
    margin: 0;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-warning);
    font-family: var(--font-heading);
    font-size: 15px;
    font-weight: 600;
    letter-spacing: var(--heading-tracking);
  }

  .conflict-row {
    margin: 0;
    font-size: var(--text-dense-size);
  }

  .conflict-note {
    margin: var(--space-1) 0 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .link {
    color: var(--color-primary-text);
    text-decoration: underline;
    text-underline-offset: 2px;
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
    min-width: 720px;
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

  .term-cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 140px;
  }

  .term-cell .mono {
    font-weight: 600;
  }

  .note-line {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .term-cell .chip {
    align-self: flex-start;
  }

  .target-cell {
    min-width: 140px;
  }

  .pair-cell {
    color: var(--color-muted-fg);
    white-space: nowrap;
  }

  .variants-cell {
    color: var(--color-muted-fg);
    font-size: var(--text-dense-size);
    max-width: 200px;
  }

  .mono {
    font-family: var(--font-mono);
    font-size: var(--text-dense-size);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    font-size: var(--text-meta-size);
    font-family: var(--font-mono);
    color: var(--color-muted-fg);
    border: 1px solid var(--color-border);
    border-radius: 999px;
    padding: 0 var(--space-1);
    margin-left: var(--space-1);
  }

  .chip.scope {
    margin-left: 0;
    font-family: inherit;
  }

  .muted {
    color: var(--color-muted-fg);
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
  .form-row select {
    flex: 1;
    min-width: 0;
  }

  .form-label {
    width: 130px;
    flex: none;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .toggle-row input {
    accent-color: var(--color-primary);
  }

  .pair-selects {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-muted-fg);
  }

  .seg {
    display: inline-flex;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    overflow: hidden;
    background: var(--color-bg);
  }

  .seg-btn {
    display: inline-flex;
    align-items: center;
    min-height: var(--control-h);
    padding: 0 var(--space-3);
    color: var(--color-muted-fg);
    transition: background var(--motion-fast), color var(--motion-fast);
  }

  .seg-btn + .seg-btn {
    border-left: 1px solid var(--color-border);
  }

  .seg-btn:hover {
    background: var(--color-muted);
    color: var(--color-fg);
  }

  .seg-btn[aria-pressed='true'] {
    background: var(--color-primary);
    color: var(--color-primary-fg);
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
