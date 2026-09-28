// M-2 (UI audit 2026-09-29, /tmp/rimloc-ui-bugs.md): on the live Home the
// header chip claimed «Демо-данные (мок)» right above the list of REAL
// managed projects. Fix: with no project open the chip states the TRANSPORT
// truth (Live/Mock by the resolved client mode); the «Demo data» label stays
// scoped to the bundled demo dataset opened in the workspace.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { clientInstance } from '../src/lib/client/instance.svelte';
import { devMode } from '../src/lib/stores/devmode.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { onboarding } from '../src/lib/stores/onboarding.svelte';
import { demoProject } from '../src/lib/demo/demoProject.svelte';
import { i18n } from '../src/i18n/store.svelte';
import { cleanupMounted, exists, goto, mountCmp, q } from './helpers';

const HANDSHAKE_OK = {
  ui_contract_version: 1,
  capabilities: {
    contract_version: 1,
    supported: ['project_create', 'project_list'],
    unsupported: []
  }
};

function resetClientSingleton() {
  (clientInstance as unknown as { mode: unknown; client: unknown }).mode = null;
  (clientInstance as unknown as { client: unknown }).client = null;
  (project as unknown as { rimloc: unknown }).rimloc = null;
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

describe('M-2: the header badge reflects the transport while no project is open', () => {
  beforeEach(() => {
    cleanupMounted();
    resetClientSingleton();
    project.reset();
    onboarding.open = false;
    i18n.setLocale('ru');
  });
  afterEach(cleanupMounted);

  it('tauri mode: live-transport badge — the live recents are not "demo data"', () => {
    devMode.disable();
    (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
      invoke: vi.fn(async (cmd: string) => {
        if (cmd === 'contract_handshake') return HANDSHAKE_OK;
        if (cmd === 'project_list') return [];
        throw new Error(`unexpected command ${cmd}`);
      })
    };
    goto('#/home');
    mountCmp(App);
    flushSync();
    expect(exists('transport-live-badge')).toBe(true);
    expect(q('transport-live-badge').textContent).toContain('Живой транспорт');
    // The old copy is gone from the live surface…
    expect(exists('mock-badge')).toBe(false);
    expect(document.body.textContent).not.toContain('Демо-данные (мок)');
  });

  it('mock mode: mock-transport badge', () => {
    devMode.enable();
    goto('#/home');
    mountCmp(App);
    flushSync();
    expect(exists('transport-mock-badge')).toBe(true);
    expect(q('transport-mock-badge').textContent).toContain('Мок-транспорт');
    expect(exists('mock-badge')).toBe(false);
  });

  it('a live contract project keeps the live badge (P1-4, unchanged)', async () => {
    devMode.disable();
    (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
      invoke: vi.fn(async (cmd: string) => {
        if (cmd === 'contract_handshake') return HANDSHAKE_OK;
        if (cmd === 'project_list') return [];
        if (cmd === 'project_create') {
          return {
            project_id: 'proj-x',
            revision: 1,
            session_epoch: 1,
            project: {
              context: { active_dlc: [], active_mods: [], load_order: [], view: 'exact' },
              entries: [{ id: { kind: 'Keyed', key: 'a' }, text: 'A', source_locale: 'en' }],
              translations: []
            }
          };
        }
        throw new Error(`unexpected command ${cmd}`);
      })
    };
    expect(await project.createContractProject('/mods/Demo')).toBe(true);
    goto('#/home');
    mountCmp(App);
    flushSync();
    expect(exists('live-badge')).toBe(true);
    expect(exists('transport-live-badge')).toBe(false);
  });
});

describe('M-2: the «Демо-данные» badge stays scoped to the demo dataset', () => {
  beforeEach(() => {
    cleanupMounted();
    resetClientSingleton();
    devMode.enable();
    project.reset();
    onboarding.open = false;
  });
  afterEach(cleanupMounted);

  it('demo open → MockBadge; plain fixture → transport badge', () => {
    demoProject.seed();
    goto('#/home');
    mountCmp(App);
    flushSync();
    expect(exists('mock-badge')).toBe(true);
    expect(exists('transport-mock-badge')).toBe(false);

    cleanupMounted();
    project.reset(); // plain fixture, demo identity off
    goto('#/home');
    mountCmp(App);
    flushSync();
    expect(exists('mock-badge')).toBe(false);
    expect(exists('transport-mock-badge')).toBe(true);
  });
});
