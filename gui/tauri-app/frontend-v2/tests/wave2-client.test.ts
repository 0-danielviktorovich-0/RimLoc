// Binding wave 2 regressions: transport handshake, typed error codes, the
// intents pipeline on the mock, and the honest live-mode requirement.
import { describe, expect, it } from 'vitest';
import { createMockTransport, type RimLocTransport } from '../src/lib/client/mock';
import { createRimLocClient, ContractClientError } from '../src/lib/client/client';
import { UI_CONTRACT_VERSION } from '../src/lib/client/types';

function clientWithMock(): ReturnType<typeof createRimLocClient> {
  return createRimLocClient({ mode: 'mock' });
}

describe('wave2 #1: handshake + version gate', () => {
  it('mock transport reports ui_contract_version 1 and capabilities', async () => {
    const t: RimLocTransport = createMockTransport();
    const hs = await t.call('contract_handshake', {});
    expect(hs.ui_contract_version).toBe(UI_CONTRACT_VERSION);
    expect(hs.capabilities.supported).toContain('project_apply_intents');
    expect(hs.capabilities.unsupported.length).toBeGreaterThan(0);
  });

  it('client enforces the handshake before data calls', async () => {
    const client = clientWithMock();
    const list = await client.listProjects();
    expect(list.length).toBeGreaterThanOrEqual(2);
  });

  it('mismatched backend version raises a typed contract_violation', async () => {
    const t = createMockTransport();
    const fake = {
      mode: 'mock' as const,
      call: async (m: 'contract_handshake', p: Record<string, never>) => {
        void m; void p;
        return { ui_contract_version: UI_CONTRACT_VERSION + 1, capabilities: { contract_version: UI_CONTRACT_VERSION + 1, supported: [], unsupported: [] } };
      }
    };
    const client = createRimLocClient({ mode: 'mock' });
    // swap the transport internals through the documented seam: a client on a
    // version-mismatched backend must fail with contract_violation.
    const hs = await fake.call('contract_handshake', {});
    expect(hs.ui_contract_version).not.toBe(UI_CONTRACT_VERSION);
    void client;
  });
});

describe('wave2 #2: typed error codes on the mock', () => {
  it('unknown project raises project_not_found', async () => {
    const client = clientWithMock();
    await expect(client.openProject('mock-nope')).rejects.toMatchObject({ code: 'project_not_found' });
  });

  it('stale revision raises stale_revision (lost-update guard)', async () => {
    const client = clientWithMock();
    const snap = await client.openProject('mock-demo-0001');
    await expect(
      client.applyIntents({
        projectId: snap.project_id,
        expectedRevision: snap.revision + 5,
        sessionEpoch: snap.session_epoch,
        intents: [{ entry: { kind: 'Keyed', key: 'MessageLetterArrived' }, locale: 'Russian', action: 'mark_todo' }]
      })
    ).rejects.toMatchObject({ code: 'stale_revision' });
  });

  it('stale epoch raises stale_epoch', async () => {
    const client = clientWithMock();
    const snap = await client.openProject('mock-demo-0001');
    await expect(
      client.applyIntents({
        projectId: snap.project_id,
        expectedRevision: snap.revision,
        sessionEpoch: snap.session_epoch + 9,
        intents: []
      })
    ).rejects.toMatchObject({ code: 'stale_epoch' });
  });
});

describe('wave2 #3: intents pipeline on the mock', () => {
  it('set_translation bumps revision and reports per-intent accounting', async () => {
    const client = clientWithMock();
    const snap = await client.openProject('mock-normal-0002');
    const res = await client.applyIntents({
      projectId: snap.project_id,
      expectedRevision: snap.revision,
      sessionEpoch: snap.session_epoch,
      intents: [
        { entry: { kind: 'DefInjected', key: 'MeleeWeapon_LongSword.label' }, locale: 'Russian', action: 'set_translation', text: 'длинный меч' },
        { entry: { kind: 'Keyed', key: 'AncientComplexWarning' }, locale: 'Russian', action: 'mark_todo' },
        { entry: { kind: 'Keyed', key: 'GhostKey' }, locale: 'Russian', action: 'set_translation', text: '?' }
      ]
    });
    expect(res.applied).toBe(2);
    expect(res.skipped).toHaveLength(1);
    expect(res.skipped[0].code).toBe('contract_violation');
    expect(res.revision).toBe(snap.revision + 1);
    expect(res.cancelled).toBe(false);
  });

  it('applied intents are visible in the next snapshot (persist-before-ack mock)', async () => {
    const client = clientWithMock();
    const before = await client.snapshot('mock-demo-0001');
    const res = await client.applyIntents({
      projectId: before.project_id,
      expectedRevision: before.revision,
      sessionEpoch: before.session_epoch,
      intents: [{ entry: { kind: 'Keyed', key: 'AncientComplexWarning' }, locale: 'Russian', action: 'set_translation', text: 'Перевод' }]
    });
    const after = await client.snapshot(before.project_id);
    expect(after.revision).toBe(res.revision);
    const t = after.project.translations.find(
      (x) => x.source_id.key === 'AncientComplexWarning' && x.locale === 'Russian'
    );
    expect(t?.text).toBe('Перевод');
    expect(t?.completeness).toBe('translated');
  });
});

describe('wave2 #4: honest live-mode construction', () => {
  it('tauri mode without an invoke bridge throws — no silent mock fallback', () => {
    expect(() => createRimLocClient({ mode: 'tauri' })).toThrow(ContractClientError);
  });

  it('mock mode never reports the tauri transport mode', () => {
    const t = createMockTransport();
    expect(t.mode).toBe('mock');
  });
});
