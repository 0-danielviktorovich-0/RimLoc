// W2 (existing-pack flow): the "update an existing translation" scenario
// closed end-to-end over the contract surface —
//   dry-run analyze (project_import_existing) → categories →
//   guarded apply (project_apply_existing) → negatives (§8 mandate).
// Mock transport rules mirror the Rust session (epoch/revision/path-form/
// locale-form guards), so the negatives are the same refusals the desktop
// app would produce.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import { createRimLocClient } from '../src/lib/client/client';
import { createMockTransport } from '../src/lib/client/mock';
import {
  capability,
  CAP_APPLY_EXISTING,
  CAP_IMPORT_EXISTING
} from '../src/lib/client/capability.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { buildState } from '../src/lib/mock/buildState.svelte';
import { existingPack } from '../src/lib/stores/existing.svelte';
import App from '../src/App.svelte';
import { cleanupMounted, click, exists, goto, mountCmp, q, qAll } from './helpers';

const client = createRimLocClient({ mode: 'mock' });

beforeEach(() => {
  capability.reset();
  project.reset();
  buildState.reset();
  existingPack.reset();
});

describe('W2 #1: handshake — the slice boundary moved honestly', () => {
  it('reports project_import_existing / project_apply_existing as supported', async () => {
    const hs = await client.handshake();
    expect(hs.capabilities.supported).toContain('project_import_existing');
    expect(hs.capabilities.supported).toContain('project_apply_existing');
    // import_pack is LIVE now — the report must not claim it is missing.
    expect(hs.capabilities.unsupported.map((u) => u.capability)).not.toContain('import_pack');
  });

  it('exposes the two capability names to the UI gate store', async () => {
    await capability.ensure();
    expect(capability.state(CAP_IMPORT_EXISTING)).toBe(true);
    expect(capability.state(CAP_APPLY_EXISTING)).toBe(true);
  });
});

describe('W2 #2: dry-run analyze classifies the pack and writes NOTHING', () => {
  it('categorizes reusable / conflicts / obsolete / invalid / ambiguous', async () => {
    const t = createMockTransport();
    const analysis = await t.call('project_import_existing', {
      request: {
        project_id: 'mock-demo-0001',
        session_epoch: 1,
        existing_dir: { path: '/mods/Demo/Languages/Russian' },
        locale: 'Russian'
      }
    });
    // Mock corpus: 2 translated (MessageLetterArrived, Gun_AssaultRifle.label)
    // + 2 untranslated (AncientComplexWarning, MeleeWeapon_LongSword.label).
    expect(analysis.reusable_count).toBe(2);
    expect(analysis.conflict_count).toBe(2);
    expect(analysis.obsolete_count).toBe(2);
    expect(analysis.invalid_count).toBe(1);
    expect(analysis.ambiguous_count).toBe(1);
    expect(analysis.ambiguous[0].candidates.length).toBe(2);
    // The reusable set carries the FULL structural identity.
    const sword = analysis.reusable.find((i) => i.key === 'MeleeWeapon_LongSword.label');
    expect(sword?.entry?.kind).toBe('DefInjected');
  });

  it('is a dry run: the project revision does not move and nothing is applied', async () => {
    const before = await client.snapshot('mock-normal-0002');
    const res = await client.importExisting({
      project_id: before.project_id,
      session_epoch: before.session_epoch,
      existing_dir: { path: '/mods/MyMod/Languages/Russian' },
      locale: 'Russian'
    });
    expect(res.reusable_count).toBeGreaterThan(0);
    const after = await client.snapshot(before.project_id);
    expect(after.revision).toBe(before.revision);
    expect(
      after.project.translations.find(
        (x) => x.source_id.key === 'MeleeWeapon_LongSword.label' && x.locale === 'Russian'
      )?.text
    ).toBeNull();
  });
});

describe('W2 #3: apply moves ONLY the reusable set (never overwrites)', () => {
  it('imports reusable lines, keeps conflicts, bumps the revision', async () => {
    const before = await client.snapshot('mock-demo-0001');
    const res = await client.applyExisting({
      project_id: before.project_id,
      expected_revision: before.revision,
      session_epoch: before.session_epoch,
      existing_dir: { path: '/mods/Demo/Languages/Russian' },
      locale: 'Russian'
    });
    expect(res.applied).toBe(2);
    expect(res.conflicts).toBe(2);
    expect(res.unmatched).toBe(2);
    expect(res.ambiguous).toBe(1);
    expect(res.revision).toBe(before.revision + 1);

    const after = await client.snapshot(before.project_id);
    const sword = after.project.translations.find(
      (x) => x.source_id.key === 'MeleeWeapon_LongSword.label' && x.locale === 'Russian'
    );
    expect(sword?.completeness).toBe('translated');
    expect(sword?.origin).toBe('imported');
    // The human translation WON — the pack line did not overwrite it.
    const rifle = after.project.translations.find(
      (x) => x.source_id.key === 'Gun_AssaultRifle.label' && x.locale === 'Russian'
    );
    expect(rifle?.origin).toBe('human');
    expect(rifle?.text).toBe('штурмовая винтовка');
  });

  it('a stale expected_revision (manual edit after the analysis) refuses the whole apply', async () => {
    const before = await client.snapshot('mock-normal-0002');
    // A manual edit lands after the caller analyzed (revision bump).
    await client.applyIntents({
      projectId: before.project_id,
      expectedRevision: before.revision,
      sessionEpoch: before.session_epoch,
      intents: [
        {
          entry: { kind: 'Keyed', key: 'AncientComplexWarning' },
          locale: 'Russian',
          action: 'set_translation',
          text: 'ручная правка'
        }
      ]
    });
    await expect(
      client.applyExisting({
        project_id: before.project_id,
        expected_revision: before.revision, // the analyzed-at base is stale
        session_epoch: before.session_epoch,
        existing_dir: { path: '/mods/MyMod/Languages/Russian' },
        locale: 'Russian'
      })
    ).rejects.toMatchObject({ code: 'stale_revision' });
  });
});

describe('W2 #4: path / session guards are typed (§8 negatives)', () => {
  // The session guards run in order: epoch → revision (apply) → path form →
  // locale form. A caller with a fresh open() gets past epoch/revision, so
  // the path/locale refusals surface exactly like on the desktop app.
  async function freshBase() {
    const snap = await client.openProject('mock-demo-0001');
    return {
      project_id: snap.project_id,
      session_epoch: snap.session_epoch,
      existing_dir: { path: '/mods/Demo/Languages/Russian' },
      locale: 'Russian'
    };
  }

  it('relative pack dir → invalid_output_path (dry-run AND apply)', async () => {
    const base = await freshBase();
    await expect(
      client.importExisting({ ...base, existing_dir: { path: 'relative/pack' } })
    ).rejects.toMatchObject({ code: 'invalid_output_path' });
    await expect(
      client.applyExisting({
        ...base,
        expected_revision: (await client.snapshot(base.project_id)).revision,
        existing_dir: { path: 'relative/pack' }
      })
    ).rejects.toMatchObject({ code: 'invalid_output_path' });
  });

  it('malformed locale form → contract_violation', async () => {
    const base = await freshBase();
    await expect(
      client.importExisting({ ...base, locale: '../evil' })
    ).rejects.toMatchObject({ code: 'contract_violation' });
  });

  it('stale epoch refuses even the read-only dry-run', async () => {
    const base = await freshBase();
    await expect(
      client.importExisting({ ...base, session_epoch: 99 })
    ).rejects.toMatchObject({ code: 'stale_epoch' });
  });

  it('unknown project → project_not_found', async () => {
    const base = await freshBase();
    await expect(
      client.importExisting({ ...base, project_id: 'mock-nope' })
    ).rejects.toMatchObject({ code: 'project_not_found' });
  });
});

describe('W2 #5: the GUI flow (App) — analyze → table → apply', () => {
  it('runs the live flow on a contract project and refuses to apply before analyze', async () => {
    await capability.ensure();
    const ok = await project.createContractProject('/mods/Demo');
    expect(ok).toBe(true);

    goto('#/existing');
    mountCmp(App);
    // Live branch: dir picker + analyze; apply appears only after analyze.
    expect(exists('existing.live.analyze')).toBe(true);
    expect(exists('existing.live.result')).toBe(false);
    expect((q('existing.live.analyze') as HTMLButtonElement).disabled).toBe(true);

    // Simulate the OS folder picker outcome (the picker itself is a native
    // shell command — mock mode refuses it honestly, the test sets the
    // picked path the same way the real app receives it).
    existingPack.existingDir = '/mods/Demo/Languages/Russian';
    // The dir assignment flips the derived canAnalyze outside an effect —
    // flush the DOM synchronously before clicking (helpers.click flushes
    // only after the click, too late for a disabled button).
    flushSync();
    click('existing.live.analyze');
    await vi.waitFor(() => {
      expect(exists('existing.live.result')).toBe(true);
    });
    expect(q('existing.live.cat.reusable').textContent).toContain('2');
    expect(q('existing.live.cat.conflicts').textContent).toContain('2');
    expect(exists('existing.live.list.reusable')).toBe(true);
    expect(exists('existing.live.list.ambiguous')).toBe(true);

    // Apply only after analyze; then the result summary appears.
    expect((q('existing.live.apply') as HTMLButtonElement).disabled).toBe(false);
    click('existing.live.apply');
    await vi.waitFor(() => {
      expect(exists('existing.live.applied')).toBe(true);
    });
    expect(q('existing.live.applied').textContent).toContain('2');
    cleanupMounted();
  });

  it('keeps the marked demo flow when no contract project is open', () => {
    goto('#/existing');
    mountCmp(App);
    // Demo branch: the mock mod/pack selection, not the dir picker.
    expect(exists('existing.select')).toBe(true);
    expect(exists('existing.live.select')).toBe(false);
    cleanupMounted();
  });
});
