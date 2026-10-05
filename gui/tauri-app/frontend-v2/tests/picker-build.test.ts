// Folder-picker wave: the native folder dialog (main.rs pick_directory,
// window-level Tauri command) exposed through the SAME typed client surface.
//
// Covers:
//   1. mock transport: pick_directory refuses HONESTLY (unsupported_capability,
//      never a fake dialog);
//   2. client: the typed error flows back as ContractClientError; in tauri
//      mode the invoke carries the `initial` seed arg and the picked absolute
//      path (or a null cancel) flows back verbatim;
//   3. Home contract panel: [Choose folder…] fills the mod-path input from
//      the dialog; cancel is silent; a real failure surfaces in the alert;
//   4. ContractOps (build out_dir / diagnose bundle): the same picker flow
//      fills the output fields — the export run button stays gated on the
//      absolute-FORM check exactly as before.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { createRimLocClient, RimLocClient } from '../src/lib/client/client';
import { createMockState, createMockTransport } from '../src/lib/client/mock';
import { clientInstance } from '../src/lib/client/instance.svelte';
import { capability } from '../src/lib/client/capability.svelte';
import ContractOps from '../src/lib/components/screens/ContractOps.svelte';
import { contractops } from '../src/lib/stores/contractops.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { devMode } from '../src/lib/stores/devmode.svelte';
import { click, cleanupMounted, exists, mountCmp, q } from './helpers';

const HANDSHAKE_OK = {
  ui_contract_version: 1,
  capabilities: { contract_version: 1, supported: [], unsupported: [] }
};

function resetClientSingleton() {
  (clientInstance as unknown as { mode: unknown; client: unknown }).mode = null;
  (clientInstance as unknown as { client: unknown }).client = null;
  // The project store caches ITS OWN client handle (rimloc field) — the
  // singleton reset alone leaves a stale tauri client answering project ops.
  (project as unknown as { rimloc: unknown }).rimloc = null;
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

function pathInput(testid: string): HTMLInputElement {
  return q(testid) as HTMLInputElement;
}

describe('mock transport: pick_directory refuses honestly', () => {
  it('typed unsupported_capability, never a fake dialog', async () => {
    const transport = createMockTransport(createMockState());
    await expect(transport.call('pick_directory', {})).rejects.toMatchObject({
      code: 'unsupported_capability'
    });
    await expect(transport.call('pick_directory', {})).rejects.toThrow(/mock/);
  });

  it('client surfaces the refusal as a typed ContractClientError', async () => {
    const client = new RimLocClient({ mode: 'mock' });
    (client as unknown as { transport: unknown }).transport = createMockTransport(createMockState());
    await expect(client.pickDirectory()).rejects.toMatchObject({
      code: 'unsupported_capability',
      name: 'Error'
    });
  });
});

describe('tauri client: pick_directory invoke shape', () => {
  function tauriClient(invoke: ReturnType<typeof vi.fn>): RimLocClient {
    return new RimLocClient({ mode: 'tauri', invoke });
  }

  function bridge(picked: string | null | Error) {
    return vi.fn(async (cmd: string) => {
      if (cmd === 'contract_handshake') return HANDSHAKE_OK;
      if (cmd === 'pick_directory') {
        if (picked instanceof Error) throw picked;
        return picked;
      }
      throw new Error(`unexpected command ${cmd}`);
    });
  }

  it('sends the initial seed arg and returns the picked path', async () => {
    const invoke = bridge('/mods/MyMod');
    const client = tauriClient(invoke);
    expect(await client.pickDirectory('/mods')).toBe('/mods/MyMod');
    expect(invoke).toHaveBeenCalledWith('pick_directory', { initial: '/mods' });
  });

  it('sends no arg when no initial seed is given', async () => {
    const invoke = bridge(null);
    const client = tauriClient(invoke);
    expect(await client.pickDirectory()).toBe(null); // cancel = null, not an error
    expect(invoke).toHaveBeenCalledWith('pick_directory', {});
  });

  it('dialog failure is a typed error, not a silent null', async () => {
    const client = tauriClient(bridge(new Error('dialog backend blew up')));
    await expect(client.pickDirectory()).rejects.toMatchObject({ code: 'internal' });
  });
});

describe('Home contract panel: [Choose folder…] fills the mod path', () => {
  beforeEach(() => {
    resetClientSingleton();
    devMode.disable();
    project.reset();
    capability.reset();
  });

  afterEach(() => {
    cleanupMounted();
  });

  function bridgeInvoke(picked: string | null | Error) {
    return vi.fn(async (cmd: string) => {
      if (cmd === 'contract_handshake') return HANDSHAKE_OK;
      if (cmd === 'project_list') return [];
      if (cmd === 'pick_directory') {
        if (picked instanceof Error) throw picked;
        return picked;
      }
      throw new Error(`unexpected command ${cmd}`);
    });
  }

  function mountTauriHome(picked: string | null | Error) {
    (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
      invoke: bridgeInvoke(picked)
    };
    window.location.hash = '#/home';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    mountCmp(App);
    flushSync();
  }

  it('picked folder lands in the path input', async () => {
    mountTauriHome('/Users/you/Mods/MyMod');
    expect(exists('home.contract.pick')).toBe(true);
    click('home.contract.pick');
    await vi.waitFor(() => {
      expect(pathInput('home.contract.path').value).toBe('/Users/you/Mods/MyMod');
    });
    expect(exists('home.contract.error')).toBe(false);
  });

  it('cancel (null) is silent: input unchanged, no error', async () => {
    mountTauriHome(null);
    click('home.contract.pick');
    await vi.waitFor(() => {
      expect(
        (window as unknown as { __TAURI_INTERNALS__?: { invoke: { mock: { calls: unknown[][] } } } })
          .__TAURI_INTERNALS__!.invoke.mock.calls.some(([cmd]) => cmd === 'pick_directory')
      ).toBe(true);
    });
    expect(pathInput('home.contract.path').value).toBe('');
    expect(exists('home.contract.error')).toBe(false);
  });

  it('real dialog failure surfaces verbatim in the shared alert', async () => {
    mountTauriHome(new Error('dialog backend blew up'));
    click('home.contract.pick');
    await vi.waitFor(() => {
      expect(exists('home.contract.error')).toBe(true);
    });
    expect(q('home.contract.error').textContent).toContain('dialog backend blew up');
  });
});

describe('ContractOps: picker fills the output roots', () => {
  beforeEach(() => {
    resetClientSingleton();
    devMode.enable();
    project.reset();
    contractops.reset();
  });

  afterEach(() => {
    cleanupMounted();
  });

  async function withLiveProject() {
    // The project opens over the REAL mock client (fixture corpus), then the
    // client singleton is swapped to a tauri-mode client that only serves the
    // dialog (documented test seam, same as contract-ops.test).
    const snap = await project.createContractProject('/mods/Demo');
    expect(snap).toBe(true);
    const invoke = vi.fn(async (cmd: string) => {
      if (cmd === 'contract_handshake') return HANDSHAKE_OK;
      if (cmd === 'pick_directory') return '/tmp/rimloc-picked';
      throw new Error(`unexpected command ${cmd}`);
    });
    (clientInstance as unknown as { client: unknown }).client = new RimLocClient({
      mode: 'tauri',
      invoke
    });
    return invoke;
  }

  it('build: picked out dir lands in the export field', async () => {
    await withLiveProject();
    mountCmp(ContractOps, { kind: 'build' });
    click('contractops.export.pick');
    await vi.waitFor(() => {
      expect(pathInput('contractops.export.outdir').value).toBe('/tmp/rimloc-picked');
    });
    // The absolute-FORM gate accepts the picked path — run stays honest.
    expect((q('contractops.export.run') as HTMLButtonElement).disabled).toBe(false);
  });

  it('diagnose: picked bundle dir lands in the bundle field', async () => {
    await withLiveProject();
    mountCmp(ContractOps, { kind: 'diagnostics' });
    click('contractops.diagnose.pick');
    await vi.waitFor(() => {
      expect(pathInput('contractops.diagnose.outdir').value).toBe('/tmp/rimloc-picked');
    });
  });

  it('mock mode shows the honest refusal instead of pretending', async () => {
    await project.createContractProject('/mods/Demo');
    // Default mock client (devMode on): pick_directory is an honest refusal.
    mountCmp(ContractOps, { kind: 'build' });
    click('contractops.export.pick');
    await vi.waitFor(() => {
      expect(exists('contractops.pick.error')).toBe(true);
    });
    // The component surfaces the message verbatim; the mock refusal text is
    // the honest "unavailable in mock" note (never a fake dialog).
    expect(q('contractops.pick.error').textContent).toContain('unavailable in mock');
    expect(pathInput('contractops.export.outdir').value).toBe('');
  });
});
