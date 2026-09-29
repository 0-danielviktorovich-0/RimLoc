<script lang="ts">
  // Live contract operations panel (final night wave): validate / export /
  // diagnose over the REAL RimLocClient on contract projects. Rendered in
  // place of the demo engines:
  //   kind="build"       → BuildStub (validate findings + explicit export)
  //   kind="diagnostics" → Diagnostics station (sanitized bundle)
  // The capability store has already opened these flows (the Rust report
  // lists all three as supported); typed contract errors surface verbatim.
  import Icon from '../Icon.svelte';
  import { t } from '../../../i18n/store.svelte';
  import { looksAbsolutePath } from '../../paths';
  import { project } from '../../stores/project.svelte';
  import { contractops, folderForm } from '../../stores/contractops.svelte';
  import { clientInstance } from '../../client/instance.svelte';

  let { kind }: { kind: 'build' | 'diagnostics' } = $props();

  // Explicit user-chosen output roots: the field starts EMPTY and nothing
  // fake is pre-filled or sent (the old `…/RimLoc-Export/…` literal with a
  // decorative ellipsis landed RELATIVE to the app's CWD). A client-side
  // absolute-form check keeps the run button honest until the path is
  // absolute; the services guard (`invalid_output_path`) is the last line.
  let exportDir = $state('');
  let modDir = $state('');
  let bundleDir = $state('');
  // M-10: the last export refusal, mirrored into the export card so the
  // failure is never silent (source of truth stays ops.error).
  let exportError = $state<string | null>(null);

  // Native folder dialog for the output roots (same flow as the Home
  // contract panel). Cancel = silent; a real failure surfaces verbatim.
  let picking = $state(false);
  let pickError = $state<string | null>(null);

  async function pickInto(target: 'export' | 'mod' | 'bundle') {
    if (picking) return;
    picking = true;
    pickError = null;
    try {
      const current =
        target === 'export' ? exportDir.trim() : target === 'mod' ? modDir.trim() : bundleDir.trim();
      const dir = await clientInstance.getClient().pickDirectory(current || undefined);
      if (target === 'export') {
        if (dir) exportDir = dir;
      } else if (target === 'mod') {
        if (dir) modDir = dir;
      } else if (dir) {
        bundleDir = dir;
      }
    } catch (e) {
      pickError = e instanceof Error ? e.message : String(e);
    } finally {
      picking = false;
    }
  }

  const ops = $derived(contractops);
  const exportDirOk = $derived(looksAbsolutePath(exportDir));
  const modDirOk = $derived(looksAbsolutePath(modDir));
  const bundleDirOk = $derived(looksAbsolutePath(bundleDir));
  const severityIcon: Record<string, string> = {
    error: 'warning',
    warning: 'warning',
    info: 'info'
  };

  function runExport() {
    // M-10 (UI audit 2026-09-29): a refusal that happens AFTER the request
    // went out must be visible where the user clicked — inside the export
    // card, not only in the section-level alert above the fold.
    exportError = null;
    void ops.runExport(exportDir).then((ok) => {
      if (!ok && ops.error) exportError = ops.error;
    });
  }
  function runBuildMod() {
    void ops.runBuildMod(modDir);
  }
  function runDiagnose() {
    void ops.runDiagnose(bundleDir);
  }
</script>

<section class="contract-ops" data-testid={`contractops.${kind}`}>
  {#if ops.error}
    <p class="ops-error" role="alert" data-testid="contractops.error">
      <Icon name="warning" size={14} />
      {ops.error}
    </p>
  {/if}

  {#if pickError}
    <p class="ops-error" role="alert" data-testid="contractops.pick.error">
      <Icon name="warning" size={14} />
      {pickError}
    </p>
  {/if}

  {#if kind === 'build'}
    <!-- 1. VALIDATE: read-only, findings from the trusted session state. -->
    <div class="card" data-testid="contractops.validate">
      <h2 class="card-title"><Icon name="clipboard-check" size={16} /> {t('contractops.validate.title')}</h2>
      <p class="hint">{t('contractops.validate.desc')}</p>
      <div class="row">
        <button
          type="button"
          class="btn btn-primary"
          data-testid="contractops.validate.run"
          disabled={ops.validating}
          onclick={() => void ops.runValidate()}
        >
          <Icon name="clipboard-check" size={14} />
          {ops.validating ? t('contractops.running') : ops.validateResult ? t('contractops.validate.rerun') : t('contractops.validate.run')}
        </button>
        {#if ops.validateResult}
          <span
            class="status-chip {ops.validateResult.status}"
            data-testid="contractops.validate.status"
            role="status"
          >
            {t(`contractops.validate.status.${ops.validateResult.status}`)}
          </span>
        {/if}
      </div>

      {#if ops.validateResult}
        <dl class="counts" data-testid="contractops.validate.counts">
          <div><dt>{t('contractops.counts.errors')}</dt><dd class="mono">{ops.validateResult.error_count}</dd></div>
          <div><dt>{t('contractops.counts.warnings')}</dt><dd class="mono">{ops.validateResult.warning_count}</dd></div>
          <div><dt>{t('contractops.counts.info')}</dt><dd class="mono">{ops.validateResult.info_count}</dd></div>
        </dl>
        {#if ops.validateResult.findings.length > 0}
          <ul class="findings" data-testid="contractops.findings">
            {#each ops.validateResult.findings as f, i (i)}
              <li class="finding finding-{f.severity}" data-testid={`contractops.finding.${i}`}>
                <span class="sev"><Icon name={severityIcon[f.severity] ?? 'info'} size={13} /> {f.severity}</span>
                <span class="mono key">{f.key}{f.line !== undefined ? `:${f.line}` : ''}</span>
                <span class="msg">{f.message}</span>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="hint ok"><Icon name="circle-check" size={14} /> {t('contractops.findings.empty')}</p>
        {/if}
      {/if}
    </div>

    <!-- 2. EXPORT: isolated output into an explicit user-chosen directory. -->
    <div class="card" data-testid="contractops.export">
      <h2 class="card-title"><Icon name="package" size={16} /> {t('contractops.export.title')}</h2>
      <p class="hint">{t('contractops.export.desc', { locale: folderForm(project.targetLocale) })}</p>
      <label class="field">
        <span>{t('contractops.export.outdir')}</span>
        <input
          type="text"
          class="mono"
          bind:value={exportDir}
          placeholder={t('contractops.abs_path_example')}
          data-testid="contractops.export.outdir"
          spellcheck="false"
        />
        <button
          type="button"
          class="btn pick"
          data-testid="contractops.export.pick"
          disabled={picking}
          onclick={() => void pickInto('export')}
        >
          <Icon name="folder-open" size={14} />
          {t('contractops.pick')}
        </button>
        <span class="hint">{t('contractops.abs_path_hint')}</span>
      </label>
      <div class="row">
        <button
          type="button"
          class="btn btn-primary"
          data-testid="contractops.export.run"
          disabled={ops.exporting || !exportDirOk}
          onclick={runExport}
        >
          <Icon name="package" size={14} />
          {ops.exporting ? t('contractops.running') : t('contractops.export.run')}
        </button>
      </div>

      <!-- M-10 (UI audit): the refusal surfaces IN the export card, next to
           the button that sent the request — never as silence. -->
      {#if exportError}
        <p class="ops-error" role="alert" data-testid="contractops.export.error">
          <Icon name="warning" size={14} />
          {exportError}
        </p>
      {/if}

      {#if ops.exportResult}
        <div class="result" data-testid="contractops.export.result">
          <p class="hint ok"><Icon name="circle-check" size={14} /> {t('contractops.export.done')}</p>
          <dl class="counts">
            <div><dt>{t('contractops.export.files')}</dt><dd class="mono">{ops.exportResult.files_written}</dd></div>
            <div><dt>{t('contractops.export.reparsed')}</dt><dd class="mono">{ops.exportResult.reparsed_keys}</dd></div>
          </dl>
          <p class="mono dest">{ops.exportResult.out_dir.path}</p>
          {#if ops.exportResult.skipped_unknown_type.length > 0}
            <p class="hint" data-testid="contractops.export.skipped">
              {t('contractops.export.skipped', { count: ops.exportResult.skipped_unknown_type.length })}:
              {#each ops.exportResult.skipped_unknown_type as key (key)}
                <span class="mono key">{key}</span>
              {/each}
            </p>
          {/if}
        </div>
      {/if}
    </div>

    <!-- 3. BUILD MOD: the FULL drop-in package (About `<ModMetaData>` +
         Languages) into an explicit user-chosen directory — no terminal. -->
    <div class="card" data-testid="contractops.buildmod">
      <h2 class="card-title"><Icon name="package" size={16} /> {t('contractops.buildmod.title')}</h2>
      <p class="hint">{t('contractops.buildmod.desc', { locale: folderForm(project.targetLocale) })}</p>
      <label class="field">
        <span>{t('contractops.buildmod.outdir')}</span>
        <input
          type="text"
          class="mono"
          bind:value={modDir}
          placeholder={t('contractops.abs_path_example_mod')}
          data-testid="contractops.buildmod.outdir"
          spellcheck="false"
        />
        <button
          type="button"
          class="btn pick"
          data-testid="contractops.buildmod.pick"
          disabled={picking}
          onclick={() => void pickInto('mod')}
        >
          <Icon name="folder-open" size={14} />
          {t('contractops.pick')}
        </button>
        <span class="hint">{t('contractops.abs_path_hint')}</span>
      </label>
      <div class="row">
        <button
          type="button"
          class="btn btn-primary"
          data-testid="contractops.buildmod.run"
          disabled={ops.buildingMod || !modDirOk}
          onclick={runBuildMod}
        >
          <Icon name="package" size={14} />
          {ops.buildingMod ? t('contractops.running') : t('contractops.buildmod.run')}
        </button>
      </div>

      {#if ops.buildModResult}
        <div class="result" data-testid="contractops.buildmod.result">
          <p class="hint ok"><Icon name="circle-check" size={14} /> {t('contractops.buildmod.done')}</p>
          <dl class="counts">
            <div><dt>{t('contractops.buildmod.files')}</dt><dd class="mono">{ops.buildModResult.files_written}</dd></div>
            <div><dt>{t('contractops.export.reparsed')}</dt><dd class="mono">{ops.buildModResult.reparsed_keys}</dd></div>
          </dl>
          <p class="mono dest">{ops.buildModResult.out_dir.path}</p>
          {#if ops.buildModResult.skipped_unknown_type.length > 0}
            <p class="hint" data-testid="contractops.buildmod.skipped">
              {t('contractops.export.skipped', { count: ops.buildModResult.skipped_unknown_type.length })}:
              {#each ops.buildModResult.skipped_unknown_type as key (key)}
                <span class="mono key">{key}</span>
              {/each}
            </p>
          {/if}
        </div>
      {/if}
    </div>
  {:else}
    <!-- DIAGNOSTICS: sanitized bundle over the last failed operation. -->
    <div class="card" data-testid="contractops.diagnose">
      <h2 class="card-title"><Icon name="package" size={16} /> {t('contractops.diagnose.title')}</h2>
      <p class="hint">{t('contractops.diagnose.desc')}</p>
      <label class="field">
        <span>{t('contractops.diagnose.outdir')}</span>
        <input
          type="text"
          class="mono"
          bind:value={bundleDir}
          placeholder={t('contractops.abs_path_example_bundle')}
          data-testid="contractops.diagnose.outdir"
          spellcheck="false"
        />
        <button
          type="button"
          class="btn pick"
          data-testid="contractops.diagnose.pick"
          disabled={picking}
          onclick={() => void pickInto('bundle')}
        >
          <Icon name="folder-open" size={14} />
          {t('contractops.pick')}
        </button>
        <span class="hint">{t('contractops.abs_path_hint')}</span>
      </label>
      <div class="row">
        <button
          type="button"
          class="btn btn-primary"
          data-testid="contractops.diagnose.run"
          disabled={ops.diagnosing || !bundleDirOk}
          onclick={runDiagnose}
        >
          <Icon name="package" size={14} />
          {ops.diagnosing ? t('contractops.running') : t('contractops.diagnose.run')}
        </button>
      </div>

      {#if ops.diagnoseResult}
        <div class="result" data-testid="contractops.diagnose.result">
          <p class="hint ok"><Icon name="circle-check" size={14} /> {t('contractops.diagnose.done')}</p>
          <dl class="counts">
            <div><dt>{t('contractops.diagnose.operation')}</dt><dd class="mono">{ops.diagnoseResult.operation_id}</dd></div>
            <div><dt>{t('contractops.diagnose.redacted')}</dt><dd class="mono">{ops.diagnoseResult.redacted_count}</dd></div>
            <div><dt>{t('contractops.diagnose.excluded')}</dt><dd class="mono">{ops.diagnoseResult.excluded_count}</dd></div>
          </dl>
          <p class="mono dest">{ops.diagnoseResult.bundle_dir.path}</p>
          <ul class="files">
            {#each ops.diagnoseResult.files as file (file)}
              <li class="mono">{file}</li>
            {/each}
          </ul>
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .contract-ops {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-width: 760px;
  }

  .card {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .card-title {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    font-size: var(--text-heading-size);
  }

  .hint {
    margin: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .hint.ok {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--color-positive, var(--color-fg));
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-meta-size);
  }

  .field input {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-fg);
    padding: var(--space-1) var(--space-2);
  }

  /* Native-folder-dialog button under the path fields. */
  .btn.pick {
    align-self: flex-start;
  }

  .ops-error {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-destructive);
    border-radius: var(--radius-md);
    color: var(--color-destructive);
    font-size: var(--text-meta-size);
  }

  .status-chip {
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    padding: 2px var(--space-2);
    font-size: var(--text-meta-size);
  }

  .status-chip.failed {
    border-color: var(--color-destructive);
    color: var(--color-destructive);
  }

  /* M-9 (UI audit 2026-09-29): the counters row used to reference an
     UNDEFINED spacing token (--space-5 is not in tokens.css), so the gap
     collapsed to 0 and the labels fused into «ОшибкиПредупрежденияИнфо».
     Real tokens now, plus a divider between the number+label pairs so each
     number visibly owns its label. */
  .counts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-4);
    margin: 0;
  }

  .counts div {
    display: flex;
    flex-direction: column;
  }

  .counts div + div {
    border-left: 1px solid var(--color-border);
    padding-left: var(--space-4);
  }

  .counts dt {
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }

  .counts dd {
    margin: 0;
    font-size: var(--text-heading-size);
  }

  .findings {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .finding {
    display: flex;
    align-items: baseline;
    gap: var(--space-3);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    font-size: var(--text-meta-size);
  }

  .finding-error {
    background: color-mix(in srgb, var(--color-destructive) 12%, transparent);
  }

  .finding-warning {
    background: color-mix(in srgb, var(--color-warning) 12%, transparent);
  }

  .sev {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 70px;
  }

  .key {
    color: var(--color-muted-fg);
  }

  .dest {
    margin: 0;
    font-size: var(--text-meta-size);
    word-break: break-all;
  }

  .files {
    list-style: none;
    margin: 0;
    padding: 0;
    color: var(--color-muted-fg);
    font-size: var(--text-meta-size);
  }
</style>
