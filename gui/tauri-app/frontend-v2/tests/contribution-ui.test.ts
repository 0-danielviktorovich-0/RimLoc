// Contribution UI (wave 7, beta): the offline contribution bundle builder on
// the ProjectOverview screen — ONLY on the RimLoc UI catalog project.
//
// Covers:
//   1. mock transport: selfloc_build_contribution refuses HONESTLY
//      (unsupported_capability — no fabricated READY, no fake bundle file);
//   2. client: the invoke carries the open session coordinates + the picked
//      out_dir + the active target locale, and the typed result flows back;
//   3. ProjectOverview: READY / PARTIAL-BUT-VALID / NEEDS-FIXES all render —
//      status line with counts, the bundle path, the collapsible rejection
//      list (secret-free reasons verbatim);
//   4. the block is hidden everywhere else: the fixture demo project and a
//      NON-selfloc contract project never see it;
//   5. cancel in the native dialog is silent; a typed failure surfaces
//      verbatim and never pretends a bundle exists.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import ProjectOverview from '../src/lib/components/screens/ProjectOverview.svelte';
import { RimLocClient } from '../src/lib/client/client';
import { createMockState, createMockTransport } from '../src/lib/client/mock';
import { clientInstance } from '../src/lib/client/instance.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { click, cleanupMounted, exists, mountCmp, q } from './helpers';

const HANDSHAKE_OK = {
  ui_contract_version: 1,
  capabilities: { contract_version: 1, supported: [], unsupported: [] }
};

const READY_RESULT = {
  status: 'READY',
  bundle_path: '/tmp/contrib-out/rimloc-ui-contribution-ru.json',
  accepted_count: 2,
  rejected: []
};

const PARTIAL_RESULT = {
  status: 'PARTIAL-BUT-VALID',
  bundle_path: '/tmp/contrib-out/rimloc-ui-contribution-ru.json',
  accepted_count: 1,
  rejected: [
    {
      id: 'home.greeting',
      reason: 'Placeholder mismatch vs source: missing {name} — Hint: keep exactly the same {name} placeholders as the source text.'
    }
  ]
};

const NEEDS_FIXES_RESULT = {
  status: 'NEEDS-FIXES',
  bundle_path: null,
  accepted_count: 0,
  rejected: [
    { id: '<root>', reason: 'locale `en` is not contributable (must match a locale tag and must not be the source locale "en")' }
  ]
};

function resetClientSingleton() {
  (clientInstance as unknown as { mode: unknown; client: unknown }).mode = null;
  (clientInstance as unknown as { client: unknown }).client = null;
  // The project store caches ITS OWN client handle (same seam as
  // picker-build.test / selfloc-ui-entry.test).
  (project as unknown as { rimloc: unknown }).rimloc = null;
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

/** Put the project store into "the selfloc project is OPEN" state — the
 * identity the overview block keys on (contract source + resolved display
 * name stamped by the selfloc entry). */
function openSelflocProject() {
  project.reset();
  const store = project as unknown as Record<string, unknown>;
  store.source = 'contract';
  store.projectName = 'proj-selfloc';
  store.projectDisplayName = 'RimLoc UI (en)';
  store.contractProjectId = 'proj-selfloc';
  store.contractEpoch = 3;
}

function mountTauriOverview(result: unknown) {
  const invoke = vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
    if (cmd === 'contract_handshake') return HANDSHAKE_OK;
    if (cmd === 'pick_directory') return '/tmp/contrib-out';
    if (cmd === 'selfloc_build_contribution') {
      if (result instanceof Error) throw result;
      return result;
    }
    throw new Error(`unexpected command ${cmd}: ${JSON.stringify(args ?? {})}`);
  });
  (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
    invoke
  };
  mountCmp(ProjectOverview);
  flushSync();
  return invoke;
}

describe('mock transport: selfloc_build_contribution refuses honestly', () => {
  it('typed unsupported_capability, never a fabricated READY result', async () => {
    const transport = createMockTransport(createMockState());
    await expect(
      transport.call('selfloc_build_contribution', {
        project_id: 'proj-x',
        session_epoch: 1,
        out_dir: '/tmp/out',
        locale: 'ru'
      })
    ).rejects.toMatchObject({ code: 'unsupported_capability' });
    await expect(
      transport.call('selfloc_build_contribution', {
        project_id: 'proj-x',
        session_epoch: 1,
        out_dir: '/tmp/out',
        locale: 'ru'
      })
    ).rejects.toThrow(/mock/);
  });
});

describe('tauri client: selfloc_build_contribution invoke shape', () => {
  it('sends the session coordinates, the out dir and the locale tag', async () => {
    const invoke = vi.fn(async (cmd: string) => {
      if (cmd === 'contract_handshake') return HANDSHAKE_OK;
      if (cmd === 'selfloc_build_contribution') return READY_RESULT;
      throw new Error(`unexpected command ${cmd}`);
    });
    const client = new RimLocClient({ mode: 'tauri', invoke });
    const res = await client.selflocBuildContribution('proj-selfloc', 3, '/tmp/out', 'ru');
    expect(res).toEqual(READY_RESULT);
    expect(invoke).toHaveBeenCalledWith('selfloc_build_contribution', {
      project_id: 'proj-selfloc',
      session_epoch: 3,
      out_dir: '/tmp/out',
      locale: 'ru'
    });
  });
});

describe('ProjectOverview (selfloc open): the three statuses render', () => {
  beforeEach(() => {
    resetClientSingleton();
    openSelflocProject();
  });

  afterEach(() => {
    cleanupMounted();
  });

  it('READY: status line with the accepted count and the bundle path', async () => {
    const invoke = mountTauriOverview(READY_RESULT);
    expect(exists('workspace.project.contribution')).toBe(true);
    click('workspace.project.contribution.build');
    await vi.waitFor(() => {
      expect(
        invoke.mock.calls.some(([cmd]) => cmd === 'selfloc_build_contribution')
      ).toBe(true);
    });
    const call = invoke.mock.calls.find(([cmd]) => cmd === 'selfloc_build_contribution');
    expect(call?.[1]).toEqual({
      project_id: 'proj-selfloc',
      session_epoch: 3,
      out_dir: '/tmp/contrib-out',
      locale: 'ru'
    });
    const status = q('workspace.project.contribution.status');
    expect(status.textContent).toContain('READY');
    expect(status.textContent).toContain('2');
    expect(q('workspace.project.contribution.path').textContent).toContain(
      'rimloc-ui-contribution-ru.json'
    );
    // Zero refusals — no rejection disclosure exists to show.
    expect(exists('workspace.project.contribution.rejected')).toBe(false);
  });

  it('PARTIAL-BUT-VALID: the rejection list is collapsed until opened', async () => {
    const invoke = mountTauriOverview(PARTIAL_RESULT);
    click('workspace.project.contribution.build');
    await vi.waitFor(() => {
      expect(exists('workspace.project.contribution.result')).toBe(true);
    });
    const status = q('workspace.project.contribution.status');
    expect(status.textContent).toContain('PARTIAL-BUT-VALID');
    expect(status.textContent).toContain('1');
    expect(exists('workspace.project.contribution.rejections')).toBe(false);
    click('workspace.project.contribution.rejected');
    expect(exists('workspace.project.contribution.rejections')).toBe(true);
    const list = q('workspace.project.contribution.rejections').textContent ?? '';
    expect(list).toContain('home.greeting');
    expect(list).toContain('Placeholder mismatch vs source');
    // The command ran exactly once — toggling the disclosure never re-runs it.
    expect(
      invoke.mock.calls.filter(([cmd]) => cmd === 'selfloc_build_contribution').length
    ).toBe(1);
  });

  it('NEEDS-FIXES: no bundle path, refusals enumerated', async () => {
    mountTauriOverview(NEEDS_FIXES_RESULT);
    click('workspace.project.contribution.build');
    await vi.waitFor(() => {
      expect(exists('workspace.project.contribution.result')).toBe(true);
    });
    const status = q('workspace.project.contribution.status');
    expect(status.textContent).toContain('NEEDS-FIXES');
    expect(exists('workspace.project.contribution.path')).toBe(false);
    click('workspace.project.contribution.rejected');
    expect(q('workspace.project.contribution.rejections').textContent).toContain(
      'not contributable'
    );
  });

  it('typed failure surfaces verbatim; no result section appears', async () => {
    mountTauriOverview(new Error('output directory `/x` is inside the read-only source tree `/y`'));
    click('workspace.project.contribution.build');
    await vi.waitFor(() => {
      expect(exists('workspace.project.contribution.error')).toBe(true);
    });
    expect(q('workspace.project.contribution.error').textContent).toContain(
      'read-only source tree'
    );
    expect(exists('workspace.project.contribution.result')).toBe(false);
  });

  it('dialog cancel is silent: no command, no error, no result', async () => {
    const invoke = vi.fn(async (cmd: string) => {
      if (cmd === 'contract_handshake') return HANDSHAKE_OK;
      if (cmd === 'pick_directory') return null; // the user cancelled
      throw new Error(`unexpected command ${cmd}`);
    });
    (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
      invoke
    };
    mountCmp(ProjectOverview);
    flushSync();
    click('workspace.project.contribution.build');
    await vi.waitFor(() => {
      expect(
        invoke.mock.calls.some(([cmd]) => cmd === 'pick_directory')
      ).toBe(true);
    });
    expect(
      invoke.mock.calls.some(([cmd]) => cmd === 'selfloc_build_contribution')
    ).toBe(false);
    expect(exists('workspace.project.contribution.error')).toBe(false);
    expect(exists('workspace.project.contribution.result')).toBe(false);
  });
});

describe('ProjectOverview: the block exists ONLY on the selfloc project', () => {
  beforeEach(() => {
    resetClientSingleton();
  });

  afterEach(() => {
    cleanupMounted();
  });

  function mountWithStore(mutate: () => void) {
    project.reset();
    mutate();
    const invoke = vi.fn(async (cmd: string) => {
      if (cmd === 'contract_handshake') return HANDSHAKE_OK;
      throw new Error(`unexpected command ${cmd}`);
    });
    (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
      invoke
    };
    mountCmp(ProjectOverview);
    flushSync();
  }

  it('hidden on the fixture demo project', () => {
    mountWithStore(() => {});
    expect(exists('workspace.project.contribution')).toBe(false);
  });

  it('hidden on a NON-selfloc contract project (resolved display name differs)', () => {
    mountWithStore(() => {
      const store = project as unknown as Record<string, unknown>;
      store.source = 'contract';
      store.projectName = 'proj-abc';
      store.projectDisplayName = 'TestMod';
      store.contractProjectId = 'proj-abc';
      store.contractEpoch = 1;
    });
    expect(exists('workspace.project.contribution')).toBe(false);
  });

  it('hidden while the display name has not resolved yet (id fallback)', () => {
    mountWithStore(() => {
      const store = project as unknown as Record<string, unknown>;
      store.source = 'contract';
      store.projectName = 'proj-selfloc';
      store.projectDisplayName = null;
      store.contractProjectId = 'proj-selfloc';
      store.contractEpoch = 1;
    });
    expect(exists('workspace.project.contribution')).toBe(false);
  });
});
