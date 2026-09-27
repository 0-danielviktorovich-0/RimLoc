<script lang="ts">
  // Language Manager (W2): the project-language surface — source card, target
  // list (progress / issues / modified / provenance), row actions (open, pin,
  // compare stub, import, export stub, detach with volume confirmation), the
  // add-target flow (Empty / From pack / TM / Import) and the create-custom-
  // language dialog (advanced fields collapsed).
  //
  // Safety invariants (mandate): the source language is never removable; a
  // detach is confirmed and shows the volume of work being removed; deleting
  // a custom language requires an explicit second confirmation and is
  // permanent. Custom definitions live in the registry and persist.
  import { t } from '../../i18n/store.svelte';
  import { languages, type TargetSummary } from '../languages/store.svelte';
  import { registry, type Script, type Direction } from '../languages/registry';
  import type { AddFlow } from '../languages/corpus';
  import Icon from './Icon.svelte';

  type Dialog =
    | { kind: 'main' }
    | { kind: 'add' }
    | { kind: 'create' }
    | { kind: 'detach'; locale: string }
    | { kind: 'delete-custom'; locale: string };

  let dialog: Dialog = $state({ kind: 'main' });
  let notice = $state('');

  // Add-flow state
  let addLocale = $state('');
  let addFlow: AddFlow = $state('empty');

  // Create-custom state
  let newName = $state('');
  let newNative = $state('');
  let newId = $state('');
  let newFolder = $state('');
  let newScript: Script = $state('latin');
  let newDirection: Direction = $state('ltr');
  let createError = $state('');

  const summaries = $derived(languages.summaries());
  const addable = $derived(
    registry.addable().filter((l) => !languages.targets[l.localeId])
  );
  const detachVolume = $derived.by(() => {
    if (dialog.kind === 'detach') return languages.detachVolume(dialog.locale);
    return null;
  });

  // Pre-focus the add flow when opened from the palette / [+] button.
  $effect(() => {
    if (languages.managerOpen) {
      notice = '';
      dialog = { kind: languages.managerIntent === 'add' ? 'add' : 'main' };
    }
  });

  function close() {
    languages.closeManager();
  }

  function showStub(text: string) {
    notice = text;
  }

  function goAdd() {
    addLocale = '';
    addFlow = 'empty';
    dialog = { kind: 'add' };
  }

  function confirmAdd() {
    if (!addLocale) return;
    if (languages.addTarget(addLocale, addFlow)) {
      // §7 audit: the notice must describe what ACTUALLY happened. The demo
      // corpus seeds sample texts only for some flows/locales — an "empty"
      // (or corpus-less) add must never claim an import that never ran.
      const summary = languages.summaries().find((s) => s.locale === addLocale);
      notice =
        summary && summary.progress > 0
          ? t('languages.manager.importDone')
          : t('languages.manager.addedEmpty');
      dialog = { kind: 'main' };
    }
  }

  function openCreate() {
    newName = '';
    newNative = '';
    newId = '';
    newFolder = '';
    newScript = 'latin';
    newDirection = 'ltr';
    createError = '';
    dialog = { kind: 'create' };
  }

  function submitCreate() {
    const result = registry.createCustom({
      displayName: newName,
      nativeName: newNative,
      localeId: newId,
      rimworldFolder: newFolder || undefined,
      script: newScript,
      direction: newDirection
    });
    if (!result.ok) {
      createError = t(`languages.create.error.${result.error}`);
      return;
    }
    languages.refreshCustom();
    addLocale = result.language.localeId;
    addFlow = 'empty';
    dialog = { kind: 'add' };
  }

  function askDetach(locale: string) {
    dialog = { kind: 'detach', locale };
  }

  function confirmDetach() {
    if (dialog.kind !== 'detach') return;
    const result = languages.detach(dialog.locale);
    if (!result.ok && result.reason === 'last') showStub(t('languages.manager.detachLast'));
    dialog = { kind: 'main' };
  }

  function askDeleteCustom(locale: string) {
    dialog = { kind: 'delete-custom', locale };
  }

  function confirmDeleteCustom() {
    if (dialog.kind !== 'delete-custom') return;
    languages.removeCustomLanguage(dialog.locale);
    dialog = { kind: 'main' };
  }

  function back() {
    dialog = { kind: 'main' };
  }

  function viaLabel(via: TargetSummary['addedVia']): string {
    return t(`languages.via.${via}`);
  }

  function modifiedLabel(iso: string): string {
    try {
      return new Date(iso).toLocaleDateString();
    } catch {
      return iso;
    }
  }

  function onWindowKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && languages.managerOpen) close();
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

{#if languages.managerOpen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="overlay"
    data-testid="languages.manager.overlay"
    onpointerdown={(e) => {
      if (e.target === e.currentTarget) close();
    }}
  >
    <div
      class="manager"
      role="dialog"
      aria-modal="true"
      aria-label={t('languages.manager.title')}
      data-testid="languages.manager.dialog"
    >
      <header class="head">
        <h2 class="title">
          {dialog.kind === 'add'
            ? t('languages.manager.addTitle')
            : dialog.kind === 'create'
              ? t('languages.manager.createTitle')
              : t('languages.manager.title')}
        </h2>
        {#if dialog.kind !== 'main'}
          <button type="button" class="btn" onclick={back} data-testid="languages.manager.back">
            <Icon name="arrow-left" size={14} />
            {t('common.back')}
          </button>
        {/if}
        <button
          type="button"
          class="btn close-btn"
          aria-label={t('common.close')}
          data-testid="languages.manager.close"
          onclick={close}
        >
          <Icon name="close" size={14} />
        </button>
      </header>

      {#if notice}
        <p class="notice" role="status">{notice}</p>
      {/if}

      <!-- ================================================== MAIN: source + targets -->
      {#if dialog.kind === 'main'}
        <section class="section" aria-label={t('languages.switcher.source')}>
          <div class="source-card" data-testid="languages.manager.source">
            <div class="lang-id">
              <span class="lang-name">{languages.sourceLanguage.nativeName}</span>
              <span class="lang-sub mono">
                {languages.sourceLanguage.displayName} · {languages.sourceLanguage.localeId}
              </span>
            </div>
            <span class="badge badge-source">{t('languages.switcher.source')}</span>
          </div>
          <p class="hint">{t('languages.manager.sourceNote')}</p>
        </section>

        <section class="section" aria-label={t('languages.manager.targets')}>
          <div class="section-head">
            <h3 class="section-title">{t('languages.manager.targets')}</h3>
            <button type="button" class="btn btn-primary" onclick={goAdd} data-testid="languages.manager.add">
              <Icon name="file-plus" size={14} />
              {t('languages.switcher.add')}
            </button>
          </div>

          <ul class="targets" data-testid="languages.manager.target-list">
            {#each summaries as s (s.locale)}
              <li class="target-row" data-testid={`languages.manager.target.${s.locale}`}>
                <div class="lang-id">
                  <span class="lang-name" dir={s.definition.direction}>{s.definition.nativeName}</span>
                  <span class="lang-sub mono">
                    {s.definition.displayName} · {s.definition.localeId}
                    {#if s.definition.origin === 'user'} · {t('languages.manager.createTitle').toLowerCase()}{/if}
                  </span>
                </div>

                <div class="metrics">
                  <span class="metric" title={t('languages.manager.progress')}>
                    <span class="metric-label">{t('languages.manager.progress')}</span>
                    <span class="bar" aria-hidden="true"><span class="fill" style="width: {s.progress}%"></span></span>
                    <span class="mono">{s.progress}%</span>
                  </span>
                  <span class="metric mono" title={t('languages.manager.issues')}>
                    <Icon name="warning" size={12} />
                    {s.issueCount}
                  </span>
                  <span class="metric muted">
                    {t('languages.manager.modified')}: {modifiedLabel(s.lastModified)}
                  </span>
                  <span class="metric muted">{t('languages.manager.addedVia')}: {viaLabel(s.addedVia)}</span>
                </div>

                <div class="row-actions">
                  <button
                    type="button"
                    class="btn"
                    data-testid={`languages.manager.target.${s.locale}.open`}
                    onclick={() => {
                      languages.setActive(s.locale);
                      close();
                    }}
                  >
                    {t('languages.manager.open')}
                  </button>
                  <button
                    type="button"
                    class="btn"
                    aria-pressed={languages.isPinned(s.locale)}
                    data-testid={`languages.manager.target.${s.locale}.pin`}
                    onclick={() => languages.togglePin(s.locale)}
                  >
                    {languages.isPinned(s.locale) ? t('languages.manager.unpin') : t('languages.manager.pin')}
                  </button>
                  <button
                    type="button"
                    class="btn"
                    data-testid={`languages.manager.target.${s.locale}.compare`}
                    onclick={() => showStub(t('languages.manager.compareStub'))}
                  >
                    {t('languages.manager.compare')}
                  </button>
                  <button
                    type="button"
                    class="btn"
                    data-testid={`languages.manager.target.${s.locale}.import`}
                    onclick={() => showStub(t('languages.manager.importStub'))}
                  >
                    {t('languages.manager.import')}
                  </button>
                  <button
                    type="button"
                    class="btn"
                    data-testid={`languages.manager.target.${s.locale}.export`}
                    onclick={() => showStub(t('languages.manager.exportStub'))}
                  >
                    {t('languages.manager.export')}
                  </button>
                  <button
                    type="button"
                    class="btn btn-danger"
                    data-testid={`languages.manager.target.${s.locale}.detach`}
                    onclick={() => askDetach(s.locale)}
                  >
                    {t('languages.manager.detach')}
                  </button>
                  {#if registry.isCustom(s.locale)}
                    <button
                      type="button"
                      class="btn btn-danger"
                      data-testid={`languages.manager.target.${s.locale}.delete`}
                      onclick={() => askDeleteCustom(s.locale)}
                    >
                      {t('common.delete')}
                    </button>
                  {/if}
                </div>
              </li>
            {/each}
          </ul>
        </section>
      {:else if dialog.kind === 'add'}
        <!-- ================================================== ADD TARGET FLOW -->
        <section class="section">
          <h3 class="section-title">{t('languages.manager.addLanguage')}</h3>
          <div class="picker" role="listbox" aria-label={t('languages.manager.addLanguage')}>
            {#each addable as l (l.localeId)}
              <button
                type="button"
                role="option"
                aria-selected={addLocale === l.localeId}
                class="pick"
                class:active={addLocale === l.localeId}
                data-testid={`languages.manager.add.language.${l.localeId}`}
                onclick={() => (addLocale = l.localeId)}
              >
                <span class="pick-name" dir={l.direction}>{l.nativeName}</span>
                <span class="pick-sub mono">{l.localeId}</span>
              </button>
            {:else}
              <p class="hint">{t('languages.manager.alreadyTarget')}</p>
            {/each}
          </div>
          <button type="button" class="btn create-open" onclick={openCreate} data-testid="languages.manager.create.open">
            <Icon name="file-plus" size={14} />
            {t('languages.manager.createOpen')}
          </button>
        </section>

        <section class="section">
          <h3 class="section-title">{t('languages.manager.addFlow')}</h3>
          <div class="flows">
            {#each ['empty', 'pack', 'tm', 'import'] as flow (flow)}
              <button
                type="button"
                class="flow"
                class:active={addFlow === flow}
                aria-pressed={addFlow === flow}
                data-testid={`languages.manager.add.flow.${flow}`}
                onclick={() => (addFlow = flow as AddFlow)}
              >
                <span class="flow-name">{t(`languages.manager.flow.${flow}`)}</span>
                <span class="flow-desc">{t(`languages.manager.flow.${flow}Desc`)}</span>
              </button>
            {/each}
          </div>
          <div class="foot">
            <button
              type="button"
              class="btn btn-primary"
              disabled={!addLocale}
              data-testid="languages.manager.add.confirm"
              onclick={confirmAdd}
            >
              {t('languages.manager.addConfirm')}
            </button>
          </div>
        </section>
      {:else if dialog.kind === 'create'}
        <!-- ================================================== CREATE CUSTOM -->
        <form
          class="section create"
          onsubmit={(e) => {
            e.preventDefault();
            submitCreate();
          }}
        >
          <label class="field">
            <span class="field-label">{t('languages.manager.createName')}</span>
            <input type="text" bind:value={newName} required data-testid="languages.manager.create.name" />
          </label>
          <label class="field">
            <span class="field-label">{t('languages.manager.createNative')}</span>
            <input type="text" bind:value={newNative} data-testid="languages.manager.create.native" />
          </label>
          <label class="field">
            <span class="field-label">{t('languages.manager.createId')}</span>
            <input
              type="text"
              bind:value={newId}
              required
              placeholder="tok"
              data-testid="languages.manager.create.id"
            />
            <span class="field-hint">{t('languages.manager.createIdHint')}</span>
          </label>

          <details class="advanced">
            <summary data-testid="languages.manager.create.advanced">{t('languages.manager.createAdvanced')}</summary>
            <label class="field">
              <span class="field-label">{t('languages.manager.createFolder')}</span>
              <input
                type="text"
                bind:value={newFolder}
                placeholder="Tok Pisin (Tok Pisin)"
                data-testid="languages.manager.create.folder"
              />
            </label>
            <label class="field">
              <span class="field-label">{t('languages.manager.createScript')}</span>
              <select bind:value={newScript} data-testid="languages.manager.create.script">
                {#each ['latin', 'cyrillic', 'cjk', 'unknown'] as s (s)}
                  <option value={s}>{t(`languages.script.${s}`)}</option>
                {/each}
              </select>
            </label>
            <label class="field">
              <span class="field-label">{t('languages.manager.createDirection')}</span>
              <select bind:value={newDirection} data-testid="languages.manager.create.direction">
                <option value="ltr">{t('languages.direction.ltr')}</option>
                <option value="rtl">{t('languages.direction.rtl')}</option>
              </select>
            </label>
          </details>

          {#if createError}
            <p class="error" role="alert" data-testid="languages.manager.create.error">{createError}</p>
          {/if}
          <div class="foot">
            <button type="submit" class="btn btn-primary" data-testid="languages.manager.create.submit">
              {t('languages.manager.createSubmit')}
            </button>
          </div>
        </form>
      {:else if dialog.kind === 'detach'}
        <!-- ================================================== DETACH CONFIRM -->
        <section class="section confirm">
          <p class="confirm-title">
            {t('languages.manager.detachTitle', { name: registry.resolve(dialog.locale).nativeName })}
          </p>
          <p class="confirm-desc">
            {t('languages.manager.detachDesc', { count: detachVolume ?? 0 })}
          </p>
          <div class="foot">
            <button type="button" class="btn" data-testid="languages.manager.detach.cancel" onclick={back}>
              {t('languages.manager.detachKeep')}
            </button>
            <button
              type="button"
              class="btn btn-danger"
              data-testid="languages.manager.detach.confirm"
              onclick={confirmDetach}
            >
              {t('languages.manager.detachConfirm')}
            </button>
          </div>
        </section>
      {:else if dialog.kind === 'delete-custom'}
        <!-- ================================================== PERMANENT DELETE -->
        <section class="section confirm">
          <p class="confirm-title danger">
            {t('languages.manager.deleteCustomTitle', { name: registry.resolve(dialog.locale).nativeName })}
          </p>
          <p class="confirm-desc">{t('languages.manager.deleteCustomDesc')}</p>
          <div class="foot">
            <button type="button" class="btn" data-testid="languages.manager.delete.cancel" onclick={back}>
              {t('common.cancel')}
            </button>
            <button
              type="button"
              class="btn btn-danger"
              data-testid="languages.manager.delete.confirm"
              onclick={confirmDeleteCustom}
            >
              {t('languages.manager.deleteCustomConfirm')}
            </button>
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
    z-index: 200;
    background: rgb(0 0 0 / 45%);
    animation: overlay-in var(--motion-fast) var(--ease-out);
  }

  .manager {
    width: min(720px, calc(100vw - var(--space-8)));
    max-height: min(80vh, 720px);
    margin: 8vh auto 0;
    display: flex;
    flex-direction: column;
    background: var(--color-surface);
    color: var(--color-surface-fg);
    border: 1px solid var(--color-border-strong);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-overlay);
    overflow: hidden;
    animation: manager-in var(--motion-emphasis) var(--ease-emphasis);
  }

  @keyframes overlay-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes manager-in {
    from {
      opacity: 0;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    flex: none;
  }

  .title {
    margin: 0;
    font-family: var(--font-heading);
    font-size: var(--text-heading-size);
    font-weight: var(--text-heading-weight);
    letter-spacing: var(--heading-tracking);
    flex: 1;
  }

  .close-btn {
    padding: 0 var(--space-2);
  }

  .section {
    padding: var(--space-3) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    overflow-y: auto;
  }

  .section-title {
    margin: 0;
    font-size: var(--text-meta-size);
    font-weight: 600;
    color: var(--color-muted-fg);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .source-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-muted);
  }

  .lang-id {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .lang-name {
    font-weight: 600;
  }

  .lang-sub {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .badge {
    flex: none;
    padding: 2px var(--space-2);
    border-radius: 999px;
    font-size: var(--text-meta-size);
    background: var(--color-primary);
    color: var(--color-primary-fg);
  }

  .badge-source {
    background: var(--color-muted);
    color: var(--color-muted-fg);
    border: 1px solid var(--color-border);
  }

  .hint {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .targets {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .target-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    padding: var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
  }

  .target-row > .lang-id {
    flex: 1 1 200px;
  }

  .metrics {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1) var(--space-3);
    font-size: var(--text-meta-size);
    flex: 1 1 220px;
  }

  .metric {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }

  .metric-label {
    color: var(--color-muted-fg);
  }

  .metric.muted {
    color: var(--color-muted-fg);
  }

  .bar {
    width: 80px;
    height: 6px;
    border-radius: 3px;
    background: var(--color-muted);
    overflow: hidden;
    display: inline-block;
  }

  .fill {
    display: block;
    height: 100%;
    background: var(--color-primary);
    border-radius: 3px;
  }

  .row-actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
    flex: 1 1 100%;
  }

  .row-actions .btn {
    min-height: 28px;
    padding: 0 var(--space-2);
    font-size: var(--text-meta-size);
  }

  .btn-danger {
    color: var(--color-destructive);
    border-color: var(--color-destructive);
  }

  .btn-danger:hover {
    background: var(--color-destructive);
    color: #ffffff;
  }

  .picker {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }

  .pick {
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-2);
    min-height: var(--control-h);
    padding: 0 var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg);
    color: inherit;
  }

  .pick.active {
    border-color: var(--color-primary);
    background: var(--nav-active-bg);
    color: var(--color-primary-text);
  }

  .pick-sub {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .create-open {
    align-self: flex-start;
  }

  .flows {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--space-2);
  }

  .flow {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    align-items: flex-start;
    text-align: left;
    padding: var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-bg);
    color: inherit;
  }

  .flow.active {
    border-color: var(--color-primary);
    background: var(--nav-active-bg);
  }

  .flow-name {
    font-weight: 600;
  }

  .flow-desc {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .foot {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    padding-top: var(--space-2);
  }

  .create {
    gap: var(--space-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .field-label {
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
  }

  .field-hint {
    font-size: var(--text-meta-size);
    color: var(--color-muted-fg);
  }

  .advanced summary {
    cursor: pointer;
    color: var(--color-primary-text);
    font-size: var(--text-meta-size);
    padding: var(--space-1) 0;
  }

  .advanced[open] summary {
    margin-bottom: var(--space-1);
  }

  .advanced {
    display: flex;
    flex-direction: column;
    border: 1px dashed var(--color-border-strong);
    border-radius: var(--radius-md);
    padding: var(--space-1) var(--space-3) var(--space-3);
  }

  .error {
    margin: 0;
    color: var(--color-destructive);
    font-size: var(--text-meta-size);
  }

  .notice {
    margin: 0;
    padding: var(--space-2) var(--space-4);
    border-bottom: 1px solid var(--color-border);
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
    flex: none;
  }

  .confirm {
    gap: var(--space-2);
  }

  .confirm-title {
    margin: 0;
    font-weight: 600;
    font-size: var(--text-base-size);
  }

  .confirm-title.danger {
    color: var(--color-destructive);
  }

  .confirm-desc {
    margin: 0;
    color: var(--color-muted-fg);
  }

  input[type='text'],
  select {
    min-height: var(--control-h);
  }
</style>
