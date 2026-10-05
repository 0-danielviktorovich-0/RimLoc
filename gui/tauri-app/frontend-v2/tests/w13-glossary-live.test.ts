// Wave 13 (glossary live): the workspace glossary tab over the
// `project_glossary` contract. On a CONTRACT project the tab renders the
// live term table (add / inline edit / delete-with-confirm) and sends exact
// upsert payloads; typed backend errors surface through the
// contractErrorText code-prefixed mapping. On the fixture/demo project the
// tab stays the honest stub with the demo badge — the mock transport
// refuses the glossary ops the same way (durable project state cannot be
// faked), so the component never renders a fabricated live table there.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import GlossaryStub from '../src/lib/components/workspace/GlossaryStub.svelte';
import { clientInstance } from '../src/lib/client/instance.svelte';
import { ContractClientError } from '../src/lib/client/client';
import { project } from '../src/lib/stores/project.svelte';
import { createMockTransport, createMockState } from '../src/lib/client/mock';
import { cleanupMounted, click, exists, mountCmp, q, typeInInput } from './helpers';

const TERMS = [
  { id: 'gt-1', term: 'scorched earth', translation: 'выжженная земля', note: 'GW' },
  { id: 'gt-2', term: 'raid', translation: 'рейд' }
];

const SNAPSHOT = {
  project_id: 'proj-x',
  revision: 5,
  session_epoch: 3,
  acked_revision: 5,
  project: {
    context: { active_dlc: [], active_mods: [], load_order: [], view: 'potential' },
    entries: [],
    translations: []
  }
};

interface Recorded {
  upserts: unknown[];
  deletes: unknown[];
  lists: number;
  refreshes: number;
}

function fakeClient(record: Recorded, upsertError?: { code: string; message: string }) {
  return {
    mode: 'tauri' as const,
    async glossaryList() {
      record.lists += 1;
      return structuredClone(TERMS);
    },
    async listProjects() {
      return [];
    },
    async glossaryUpsert(request: unknown) {
      record.upserts.push(request);
      if (upsertError) {
        // The real client maps backend rejections to ContractClientError
        // (client.ts toClientError) — the fake throws the same shape.
        throw new ContractClientError(
          upsertError.code as never,
          upsertError.message
        );
      }
      return {
        job_id: 'job-1',
        revision: 6,
        entry: { id: 'gt-new', term: 't', translation: 'т' }
      };
    },
    async glossaryDelete(request: unknown) {
      record.deletes.push(request);
      return { job_id: 'job-2', revision: 7, removed_id: 'gt-1' };
    },
    async refresh() {
      record.refreshes += 1;
      return structuredClone(SNAPSHOT);
    }
  };
}

function openContractProject() {
  project.reset();
  const store = project as unknown as Record<string, unknown>;
  store.source = 'contract';
  store.contractProjectId = 'proj-x';
  store.contractEpoch = 3;
  store.contractRevision = 5;
  store.contractAckedRevision = 5;
}

beforeEach(() => {
  (clientInstance as unknown as { mode: unknown; client: unknown }).mode = 'tauri';
  (clientInstance as unknown as { client: unknown }).client = null;
  (project as unknown as { rimloc: unknown }).rimloc = null;
});

afterEach(() => {
  cleanupMounted();
  (clientInstance as unknown as { mode: unknown; client: unknown }).mode = null;
  (clientInstance as unknown as { client: unknown }).client = null;
  (project as unknown as { rimloc: unknown }).rimloc = null;
  project.reset();
});

describe('wave 13: glossary tab MOCK→LIVE', () => {
  it('a contract project renders the live table from glossary_list', async () => {
    const record: Recorded = { upserts: [], deletes: [], lists: 0, refreshes: 0 };
    (clientInstance as unknown as { client: unknown }).client = fakeClient(record);
    openContractProject();
    mountCmp(GlossaryStub);
    await vi.waitFor(() => {
      expect(exists('workspace.glossary-live')).toBe(true);
      expect(exists('workspace.glossary-live-badge')).toBe(true);
    });
    await vi.waitFor(() => {
      expect(exists('workspace.glossary.row-gt-1')).toBe(true);
    });
    expect(exists('workspace.glossary.demo-badge')).toBe(false);
    expect(q('workspace.glossary.row-gt-1').textContent).toContain('scorched earth');
    expect(q('workspace.glossary.row-gt-2').textContent).toContain('рейд');
  });

  it('the add row sends an exact upsert payload and adopts the new revision', async () => {
    const record: Recorded = { upserts: [], deletes: [], lists: 0, refreshes: 0 };
    (clientInstance as unknown as { client: unknown }).client = fakeClient(record);
    (project as unknown as { rimloc: unknown }).rimloc = fakeClient(record);
    openContractProject();
    mountCmp(GlossaryStub);
    await vi.waitFor(() => expect(exists('workspace.glossary.add-term')).toBe(true));

    typeInInput('workspace.glossary.add-term', 'power');
    typeInInput('workspace.glossary.add-translation', 'энергия');
    typeInInput('workspace.glossary.add-note', 'не «мощность»');
    click('workspace.glossary.add-submit');

    await vi.waitFor(() => expect(record.upserts.length).toBe(1));
    expect(record.upserts[0]).toMatchObject({
      project_id: 'proj-x',
      session_epoch: 3,
      term: 'power',
      translation: 'энергия',
      note: 'не «мощность»'
    });
    await vi.waitFor(() => expect(record.refreshes).toBeGreaterThan(0));
  });

  it('a typed backend error surfaces code-prefixed through contractErrorText', async () => {
    const record: Recorded = { upserts: [], deletes: [], lists: 0, refreshes: 0 };
    (clientInstance as unknown as { client: unknown }).client = fakeClient(record, {
      code: 'contract_violation',
      message: 'glossary term exceeds 200 chars'
    });
    openContractProject();
    mountCmp(GlossaryStub);
    await vi.waitFor(() => expect(exists('workspace.glossary.add-term')).toBe(true));

    typeInInput('workspace.glossary.add-term', 'x'.repeat(201));
    typeInInput('workspace.glossary.add-translation', 'y');
    click('workspace.glossary.add-submit');

    await vi.waitFor(() => expect(exists('workspace.glossary-action-error')).toBe(true));
    const text = q('workspace.glossary-action-error').textContent ?? '';
    expect(text).toContain('contract_violation');
    expect(text).toContain('glossary term exceeds 200 chars');
  });

  it('delete asks for confirmation before calling the backend', async () => {
    const record: Recorded = { upserts: [], deletes: [], lists: 0, refreshes: 0 };
    (clientInstance as unknown as { client: unknown }).client = fakeClient(record);
    (project as unknown as { rimloc: unknown }).rimloc = fakeClient(record);
    openContractProject();
    mountCmp(GlossaryStub);
    await vi.waitFor(() => expect(exists('workspace.glossary.delete-gt-1')).toBe(true));

    click('workspace.glossary.delete-gt-1');
    await vi.waitFor(() => expect(exists('workspace.glossary.confirm-delete-gt-1')).toBe(true));
    expect(record.deletes.length).toBe(0);

    click('workspace.glossary.confirm-delete-gt-1');
    await vi.waitFor(() => expect(record.deletes.length).toBe(1));
    expect(record.deletes[0]).toMatchObject({
      project_id: 'proj-x',
      session_epoch: 3,
      term: 'scorched earth'
    });
  });

  it('the fixture project keeps the honest stub with the demo badge', async () => {
    project.reset(); // fixture source, no contract project
    mountCmp(GlossaryStub);
    flushSync();
    expect(exists('workspace.glossary-stub')).toBe(true);
    expect(exists('workspace.glossary.demo-badge')).toBe(true);
    expect(exists('workspace.glossary-live')).toBe(false);
    expect(exists('workspace.glossary.add-form')).toBe(false);
  });

  it('mock transport refuses the glossary ops honestly (same rule as the UI)', async () => {
    const transport = createMockTransport(createMockState());
    await expect(
      transport.call('project_glossary', { project_id: 'p', session_epoch: 1 })
    ).rejects.toMatchObject({ code: 'unsupported_capability' });
    await expect(
      transport.call('project_glossary_upsert', {
        request: { project_id: 'p', session_epoch: 1, term: 'a', translation: 'б' }
      })
    ).rejects.toMatchObject({ code: 'unsupported_capability' });
    await expect(
      transport.call('project_glossary_delete', {
        request: { project_id: 'p', session_epoch: 1, term: 'a' }
      })
    ).rejects.toMatchObject({ code: 'unsupported_capability' });
  });
});
