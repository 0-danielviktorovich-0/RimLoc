// Final night wave: the three live contract flows — validate / export /
// diagnose — over the mock transport (same DTOs, same typed errors the
// Tauri bridge carries). Scenario teeth mirror the Rust semantics:
//   - validate: findings DERIVED from stored state; the stored-issues entry
//     is the error tooth; untranslated rows are warnings; never mutates;
//   - export: strict language-folder form (P1-2), RELATIVE out-dir form
//     (invalid_output_path) and source-tree guard are typed refusals;
//     reparse parity; unknown def-type skip is surfaced;
//   - diagnose: sanitized bundle over the project.
import { beforeEach, describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import { createRimLocClient, RimLocClient } from '../src/lib/client/client';
import { createMockState, createMockTransport } from '../src/lib/client/mock';
import { contractops } from '../src/lib/stores/contractops.svelte';
import { project } from '../src/lib/stores/project.svelte';
import ContractOps from '../src/lib/components/screens/ContractOps.svelte';
import { looksAbsolutePath } from '../src/lib/paths';
import { mountCmp, q } from './helpers';

function mockClient() {
  const transport = createMockTransport(createMockState());
  const client = new RimLocClient({ mode: 'mock' });
  // Replace the transport wholesale (test seam, same as wave2-client).
  (client as unknown as { transport: unknown }).transport = transport;
  return { client, transport };
}

describe('mock contract flows (validate / export / diagnose)', () => {
  let client: RimLocClient;
  let transport: ReturnType<typeof createMockTransport>;

  beforeEach(() => {
    ({ client, transport } = mockClient());
  });

  async function freshProject() {
    const snap = await client.createProject('/mods/Demo', '1.6');
    return { snap, epoch: snap.session_epoch, id: snap.project_id };
  }

  it('validate derives findings from stored state and never mutates', async () => {
    const { id, epoch } = await freshProject();
    const revBefore = transport.call('project_snapshot', { project_id: id }).then((s) => s.revision);
    const res = await client.validateProject(id, epoch, 'Russian');
    // The stored-issues entry is the ERROR tooth; the two untranslated rows
    // are warnings.
    expect(res.status).toBe('failed');
    expect(res.error_count).toBe(1);
    expect(res.warning_count).toBe(2);
    const error = res.findings.find((f) => f.severity === 'error')!;
    expect(error.id).toEqual({ kind: 'DefInjected', key: 'Gun_AssaultRifle.label', def_type: 'Weapons' });
    expect(error.message).toContain('placeholder');
    expect(res.findings.find((f) => f.severity === 'warning')!.id!.key).toBeDefined();
    expect(await revBefore).toBe(
      await transport.call('project_snapshot', { project_id: id }).then((s) => s.revision)
    );
  });

  it('validate goes green once the flagged translation is fixed', async () => {
    const { id, epoch } = await freshProject();
    await client.applyIntents({
      projectId: id,
      expectedRevision: 1,
      sessionEpoch: epoch,
      intents: [
        {
          entry: { kind: 'DefInjected', key: 'Gun_AssaultRifle.label', def_type: 'Weapons' },
          locale: 'Russian',
          action: 'set_translation',
          text: 'штурмовая винтовка (исправлено)'
        }
      ]
    });
    const res = await client.validateProject(id, epoch, 'Russian');
    expect(res.error_count).toBe(0);
    expect(res.status).toBe('succeeded');
  });

  it('validate refuses a stale epoch', async () => {
    const { id, epoch } = await freshProject();
    await expect(client.validateProject(id, epoch + 5, 'Russian')).rejects.toMatchObject({
      code: 'stale_epoch'
    });
  });

  it('export refuses an out dir inside the source tree (guard tooth)', async () => {
    const { id, epoch } = await freshProject();
    await expect(
      client.exportProject(id, epoch, '/mods/Demo/Languages/Russian', 'Russian')
    ).rejects.toMatchObject({ code: 'guard_output_denied' });
  });

  it('export refuses a RELATIVE out dir (invalid_output_path form tooth)', async () => {
    const { id, epoch } = await freshProject();
    // The legacy decorative-ellipsis default and a plain relative name are
    // BOTH refused before anything is written.
    await expect(
      client.exportProject(id, epoch, '…/RimLoc-Export/proj-x-Russian', 'Russian')
    ).rejects.toMatchObject({ code: 'invalid_output_path' });
    await expect(
      client.exportProject(id, epoch, 'RimLoc-Export', 'Russian')
    ).rejects.toMatchObject({ code: 'invalid_output_path' });
  });

  it('diagnose refuses a relative bundle dir (invalid_output_path)', async () => {
    const { id } = await freshProject();
    await expect(client.diagnoseProject(id, 'RimLoc-Bundles')).rejects.toMatchObject({
      code: 'invalid_output_path'
    });
  });

  it('export refuses a locale that is not the strict folder form (P1-2 tooth)', async () => {
    const { id, epoch } = await freshProject();
    await expect(client.exportProject(id, epoch, '/tmp/rimloc-export', 'ru')).rejects.toMatchObject(
      { code: 'contract_violation' }
    );
  });

  it('export writes, reparses with parity and surfaces unknown def types', async () => {
    const { id, epoch } = await freshProject();
    const res = await client.exportProject(id, epoch, '/tmp/rimloc-export', 'Russian');
    expect(res.out_dir.path).toBe('/tmp/rimloc-export');
    // Two translated rows written; the scanner re-parses exactly those.
    expect(res.files_written).toBe(2);
    expect(res.reparsed_keys).toBe(2);
    // Writer tooth: DefInjected entries with an unknown def type are skipped
    // and surfaced for review (delta risk #5).
    expect(res.skipped_unknown_type).toContain('MeleeWeapon_LongSword.label');
  });

  it('diagnose returns the sanitized bundle shape', async () => {
    const { id } = await freshProject();
    const res = await client.diagnoseProject(id, '/tmp/rimloc-bundles');
    expect(res.operation_id).toMatch(/^mock-op-\d+$/);
    expect(res.bundle_dir.path).toContain('/tmp/rimloc-bundles');
    expect(res.files.length).toBeGreaterThan(0);
    expect(res.redacted_count).toBeGreaterThan(0);
  });
});

describe('contractops store wiring on a contract project', () => {
  beforeEach(() => {
    project.reset();
    contractops.reset();
  });

  it('runValidate maps error findings onto workspace validation badges', async () => {
    const ok = await project.createContractProject('/mods/Demo');
    expect(ok).toBe(true);
    const entry = project.byId('DefInjected:Gun_AssaultRifle.label:Weapons');
    expect(entry?.validation).toBe('issues'); // from the snapshot

    // Fix it locally (this mirrors the persisted state through apply_intents).
    project.setDraft('DefInjected:Gun_AssaultRifle.label:Weapons', 'штурмовая винтовка (ок)');
    expect(await project.flushDraft('DefInjected:Gun_AssaultRifle.label:Weapons')).toBe(true);

    expect(await contractops.runValidate()).toBe(true);
    expect(contractops.validateResult?.status).toBe('succeeded');
    expect(contractops.validateResult?.error_count).toBe(0);
    // Warnings reported but NOT stamped as failures on the entries.
    expect(contractops.validateResult?.warning_count).toBe(2);
    expect(project.byId('DefInjected:Gun_AssaultRifle.label:Weapons')?.validation).toBe('ok');
  });

  it('runExport surfaces typed guard refusals and succeeds outside the tree', async () => {
    await project.createContractProject('/mods/Demo');
    // Inside the source tree → typed refusal, no result.
    expect(await contractops.runExport('/mods/Demo/Languages')).toBe(false);
    expect(contractops.error).toContain('guard_output_denied');
    expect(contractops.exportResult).toBe(null);

    expect(await contractops.runExport('/tmp/rimloc-export')).toBe(true);
    expect(contractops.exportResult?.files_written).toBe(2);
    expect(contractops.exportResult?.skipped_unknown_type).toContain('MeleeWeapon_LongSword.label');
  });

  it('runExport surfaces the invalid_output_path refusal for relative dirs', async () => {
    await project.createContractProject('/mods/Demo');
    expect(await contractops.runExport('…/RimLoc-Export/proj-x-Russian')).toBe(false);
    expect(contractops.error).toContain('invalid_output_path');
    expect(contractops.exportResult).toBe(null);
  });

  it('runDiagnose collects the bundle over the open project', async () => {
    await project.createContractProject('/mods/Demo');
    expect(await contractops.runDiagnose('/tmp/rimloc-bundles')).toBe(true);
    expect(contractops.diagnoseResult?.operation_id).toMatch(/^mock-op-\d+$/);
    expect(contractops.diagnoseResult?.files).toContain('operation.json');
  });

  it('refuses to run without a live contract project', async () => {
    project.reset(); // fixture
    expect(await contractops.runValidate()).toBe(false);
    expect(contractops.error).toContain('no live contract project');
  });
});

describe('looksAbsolutePath (client mirror of the invalid_output_path form guard)', () => {
  it('accepts POSIX, drive-letter and UNC forms', () => {
    expect(looksAbsolutePath('/Users/you/RimLoc-Export')).toBe(true);
    expect(looksAbsolutePath('C:/Users/you/RimLoc-Export')).toBe(true);
    expect(looksAbsolutePath('C:\\Users\\you')).toBe(true);
    expect(looksAbsolutePath('\\\\server\\share')).toBe(true);
  });

  it('refuses relative shapes, including the legacy decorative ellipsis', () => {
    expect(looksAbsolutePath('RimLoc-Export')).toBe(false);
    expect(looksAbsolutePath('…/RimLoc-Export/proj-x-Russian')).toBe(false);
    expect(looksAbsolutePath('out/../more')).toBe(false);
    expect(looksAbsolutePath('.')).toBe(false);
    expect(looksAbsolutePath('')).toBe(false);
    expect(looksAbsolutePath('~/RimLoc-Export')).toBe(false); // unexpanded tilde
  });
});

describe('ContractOps panel: honest out-dir field (no fake default)', () => {
  beforeEach(() => {
    project.reset();
    contractops.reset();
  });

  it('the export field starts empty and the run button is gated on an absolute path', async () => {
    await project.createContractProject('/mods/Demo');
    mountCmp(ContractOps, { kind: 'build' });
    const input = q('contractops.export.outdir') as HTMLInputElement;
    const run = q('contractops.export.run') as HTMLButtonElement;
    // Honest empty start: no `…/RimLoc-Export/…` literal is ever sent.
    expect(input.value).toBe('');
    expect(run.disabled).toBe(true); // empty is not absolute
    input.value = 'RimLoc-Export';
    input.dispatchEvent(new Event('input'));
    flushSync();
    expect(run.disabled).toBe(true); // relative form refused up front
    input.value = '/tmp/rimloc-export';
    input.dispatchEvent(new Event('input'));
    flushSync();
    expect(run.disabled).toBe(false); // absolute → runnable
  });

  it('the diagnose field follows the same absolute-path gate', async () => {
    await project.createContractProject('/mods/Demo');
    mountCmp(ContractOps, { kind: 'diagnostics' });
    const input = q('contractops.diagnose.outdir') as HTMLInputElement;
    const run = q('contractops.diagnose.run') as HTMLButtonElement;
    expect(input.value).toBe('');
    expect(run.disabled).toBe(true);
    input.value = '\\\\server\\share\\rimloc-bundles';
    input.dispatchEvent(new Event('input'));
    flushSync();
    expect(run.disabled).toBe(false); // UNC counts as absolute
  });
});
