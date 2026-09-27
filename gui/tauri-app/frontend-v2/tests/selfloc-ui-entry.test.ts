// Selfloc UI entry (mandate D, first wave): "Translate RimLoc" — the
// app-bundled UI catalog opened as an ORDINARY project through the EXISTING
// contract create flow (selfloc_catalog_dir → project_create → workspace).
//
// Covers:
//   1. mock transport: selfloc_catalog_dir refuses HONESTLY
//      (unsupported_capability — no fabricated catalog dir, no fake success);
//   2. client: the typed refusal flows back as ContractClientError; in tauri
//      mode the invoke carries no args and the resolved dir flows back
//      verbatim;
//   3. Home (tauri): the beta-labeled card resolves the dir and feeds it
//      into project_create's mod_root — the ordinary create path, no
//      special-cased client flow; a resolver failure is VISIBLE and
//      project_create is never called;
//   4. Home (mock): the card is present and the click surfaces the honest
//      refusal in the section alert.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { createRimLocClient, RimLocClient } from '../src/lib/client/client';
import { createMockState, createMockTransport } from '../src/lib/client/mock';
import { clientInstance } from '../src/lib/client/instance.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { devMode } from '../src/lib/stores/devmode.svelte';
import { onboarding } from '../src/lib/stores/onboarding.svelte';
import { click, cleanupMounted, exists, mountCmp, q } from './helpers';

const HANDSHAKE_OK = {
  ui_contract_version: 1,
  capabilities: {
    contract_version: 1,
    supported: ['project_create', 'selfloc_catalog'],
    unsupported: []
  }
};

const CATALOG_DIR = '/Users/you/Library/Application Support/com.rimloc.gui/RimLoc/selfloc-catalog/RimLoc UI (en)';

function resetClientSingleton() {
  (clientInstance as unknown as { mode: unknown; client: unknown }).mode = null;
  (clientInstance as unknown as { client: unknown }).client = null;
  // The project store caches ITS OWN client handle — reset it too (same
  // seam as picker-build.test).
  (project as unknown as { rimloc: unknown }).rimloc = null;
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

describe('mock transport: selfloc_catalog_dir refuses honestly', () => {
  it('typed unsupported_capability, never a fabricated catalog dir', async () => {
    const transport = createMockTransport(createMockState());
    await expect(transport.call('selfloc_catalog_dir', {})).rejects.toMatchObject({
      code: 'unsupported_capability'
    });
    await expect(transport.call('selfloc_catalog_dir', {})).rejects.toThrow(/mock/);
  });

  it('client surfaces the refusal as a typed ContractClientError', async () => {
    const client = new RimLocClient({ mode: 'mock' });
    (client as unknown as { transport: unknown }).transport = createMockTransport(createMockState());
    await expect(client.selflocCatalogDir()).rejects.toMatchObject({
      code: 'unsupported_capability',
      name: 'Error'
    });
  });
});

describe('tauri client: selfloc_catalog_dir invoke shape', () => {
  it('sends no args and returns the resolved project dir', async () => {
    const invoke = vi.fn(async (cmd: string) => {
      if (cmd === 'contract_handshake') return HANDSHAKE_OK;
      if (cmd === 'selfloc_catalog_dir') return CATALOG_DIR;
      throw new Error(`unexpected command ${cmd}`);
    });
    const client = new RimLocClient({ mode: 'tauri', invoke });
    expect(await client.selflocCatalogDir()).toBe(CATALOG_DIR);
    expect(invoke).toHaveBeenCalledWith('selfloc_catalog_dir', {});
  });

  it('resolver failure is a typed error, not a silent path', async () => {
    const invoke = vi.fn(async (cmd: string) => {
      if (cmd === 'contract_handshake') return HANDSHAKE_OK;
      if (cmd === 'selfloc_catalog_dir') throw { message: 'the RimLoc UI catalog is not available in this build' };
      throw new Error(`unexpected command ${cmd}`);
    });
    const client = new RimLocClient({ mode: 'tauri', invoke });
    await expect(client.selflocCatalogDir()).rejects.toMatchObject({ code: 'internal' });
  });
});

describe('Home (tauri): the Translate RimLoc card drives the ordinary create flow', () => {
  beforeEach(() => {
    resetClientSingleton();
    devMode.disable();
    project.reset();
    // The onboarding singleton survives unmount: a coach opened by an earlier
    // test's workspace visit would re-navigate the router at the next App
    // mount (OnboardingCoach effect) and yank Home away mid-assertion.
    onboarding.open = false;
  });

  afterEach(() => {
    cleanupMounted();
  });

  function snapshot() {
    return {
      project_id: 'proj-selfloc',
      revision: 1,
      session_epoch: 1,
      project: {
        context: { active_dlc: [], active_mods: [], load_order: [], view: 'exact' },
        entries: [
          { id: { kind: 'Keyed', key: 'common.appName' }, text: 'RimLoc', source_locale: 'en' }
        ],
        translations: []
      }
    };
  }

  function mountTauriHome(selflocResult: string | Error) {
    const invoke = vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'contract_handshake') return HANDSHAKE_OK;
      if (cmd === 'project_list') return [];
      if (cmd === 'selfloc_catalog_dir') {
        if (selflocResult instanceof Error) throw selflocResult;
        return selflocResult;
      }
      if (cmd === 'project_create') return snapshot();
      throw new Error(`unexpected command ${cmd}: ${JSON.stringify(args ?? {})}`);
    });
    (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
      invoke
    };
    window.location.hash = '#/home';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    mountCmp(App);
    flushSync();
    return invoke;
  }

  it('beta card resolves the catalog dir and creates the project from it', async () => {
    const invoke = mountTauriHome(CATALOG_DIR);
    expect(exists('home.selfloc.open')).toBe(true);
    click('home.selfloc.open');
    await vi.waitFor(() => {
      expect(invoke.mock.calls.some(([cmd]) => cmd === 'project_create')).toBe(true);
    });
    // The EXISTING contract create flow receives the resolved dir as
    // mod_root — no special-cased client path, honest ordinary project.
    const create = invoke.mock.calls.find(([cmd]) => cmd === 'project_create');
    expect((create?.[1] as { request: { mod_root: { path: string } } }).request.mod_root.path).toBe(CATALOG_DIR);
    // Success lands in the workspace, not stuck on Home.
    await vi.waitFor(() => {
      expect(window.location.hash).toContain('workspace');
    });
    expect(exists('home.selfloc.error')).toBe(false);
  });

  it('resolver failure is visible in the section alert; create is never called', async () => {
    const invoke = mountTauriHome(new Error('the RimLoc UI catalog is not available in this build'));
    click('home.selfloc.open');
    await vi.waitFor(() => {
      expect(exists('home.selfloc.error')).toBe(true);
    });
    expect(q('home.selfloc.error').textContent).toContain('not available in this build');
    expect(invoke.mock.calls.some(([cmd]) => cmd === 'project_create')).toBe(false);
    expect(window.location.hash).not.toContain('workspace');
  });

  it('create failure after a resolved dir surfaces verbatim, stays on Home', async () => {
    const invoke = vi.fn(async (cmd: string) => {
      if (cmd === 'contract_handshake') return HANDSHAKE_OK;
      if (cmd === 'project_list') return [];
      if (cmd === 'selfloc_catalog_dir') return CATALOG_DIR;
      if (cmd === 'project_create') throw { code: 'internal', message: 'source fingerprint failed' };
      throw new Error(`unexpected command ${cmd}`);
    });
    (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
      invoke
    };
    window.location.hash = '#/home';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    mountCmp(App);
    flushSync();
    click('home.selfloc.open');
    await vi.waitFor(() => {
      expect(exists('home.selfloc.error')).toBe(true);
    });
    expect(q('home.selfloc.error').textContent).toContain('source fingerprint failed');
    expect(window.location.hash).not.toContain('workspace');
  });
});

describe('Home (mock): the card present, the refusal honest', () => {
  beforeEach(() => {
    resetClientSingleton();
    devMode.enable();
    project.reset();
    onboarding.open = false;
  });

  afterEach(() => {
    cleanupMounted();
  });

  it('click surfaces the typed refusal; no fake success, no navigation', async () => {
    window.location.hash = '#/home';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    mountCmp(App);
    flushSync();
    expect(exists('home.selfloc.open')).toBe(true);
    expect(exists('home.selfloc.error')).toBe(false);
    click('home.selfloc.open');
    await vi.waitFor(() => {
      expect(exists('home.selfloc.error')).toBe(true);
    });
    expect(q('home.selfloc.error').textContent).toContain('unavailable in mock');
    expect(window.location.hash).not.toContain('workspace');
  });
});
