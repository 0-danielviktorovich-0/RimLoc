// M-5 (UI audit 2026-09-29, /tmp/rimloc-ui-bugs.md): the header showed the
// SERVICE id «Проект: proj-01a0deb47e9a-000-16836 · English → Ру…» — the
// human name was nowhere and the language pair truncated. Fix: headers show
// the display name resolved from the project_list summaries (the wire shape
// that carries it), the id stays only as fallback/title; the pair renders as
// compact codes with the full names in the title hint.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import AppHeader from '../src/lib/components/AppHeader.svelte';
import { clientInstance } from '../src/lib/client/instance.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { onboarding } from '../src/lib/stores/onboarding.svelte';
import { goto, cleanupMounted, mountCmp, q } from './helpers';

function resetClientSingleton() {
  (clientInstance as unknown as { mode: unknown; client: unknown }).mode = null;
  (clientInstance as unknown as { client: unknown }).client = null;
  (project as unknown as { rimloc: unknown }).rimloc = null;
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
}

const SNAPSHOT = {
  project_id: 'proj-x-0001',
  revision: 1,
  session_epoch: 1,
  project: {
    context: { active_dlc: [], active_mods: [], load_order: [], view: 'exact' },
    entries: [{ id: { kind: 'Keyed', key: 'a' }, text: 'A', source_locale: 'en' }],
    translations: []
  }
};

function tauriMock() {
  (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
    invoke: vi.fn(async (cmd: string) => {
      if (cmd === 'contract_handshake') {
        return { ui_contract_version: 1, capabilities: { contract_version: 1, supported: [], unsupported: [] } };
      }
      if (cmd === 'project_create') return SNAPSHOT;
      if (cmd === 'project_list') {
        return [{ project_id: 'proj-x-0001', name: '1814383360', revision: 1 }];
      }
      throw new Error(`unexpected command ${cmd}`);
    })
  };
}

describe('M-5: the header shows the project display name, not the service id', () => {
  beforeEach(() => {
    cleanupMounted();
    resetClientSingleton();
    project.reset();
    onboarding.open = false;
  });
  afterEach(cleanupMounted);

  it('display name resolves from project_list; the raw id never renders', async () => {
    tauriMock();
    expect(await project.createContractProject('/mods/1814383360')).toBe(true);
    await vi.waitFor(() => {
      expect(project.projectDisplayName).toBe('1814383360');
    });
    goto('#/workspace');
    mountCmp(AppHeader);
    const head = q('header.project');
    expect(head.textContent).toContain('Проект:');
    expect(head.textContent).toContain('1814383360');
    expect(head.textContent).not.toContain('proj-x-0001');
  });

  it('the language pair is compact codes with the full names in the title', async () => {
    tauriMock();
    expect(await project.createContractProject('/mods/Demo')).toBe(true);
    await vi.waitFor(() => {
      expect(project.projectDisplayName).not.toBeNull();
    });
    goto('#/workspace');
    mountCmp(AppHeader);
    const pair = q('header.langpair');
    expect(pair.textContent?.replace(/\s+/g, ' ').trim()).toBe('EN → RU');
    const hint = pair.getAttribute('title') ?? '';
    expect(hint).toContain('English');
    expect(hint).toContain('Русский');
  });

  it('without a resolvable name the header falls back to the id (never a fake)', async () => {
    tauriMock();
    // A snapshot whose id is missing from the list → no name resolution.
    (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
      invoke: vi.fn(async (cmd: string) => {
        if (cmd === 'contract_handshake') {
          return { ui_contract_version: 1, capabilities: { contract_version: 1, supported: [], unsupported: [] } };
        }
        if (cmd === 'project_create') return SNAPSHOT;
        if (cmd === 'project_list') return [];
        throw new Error(`unexpected command ${cmd}`);
      })
    };
    expect(await project.createContractProject('/mods/Demo')).toBe(true);
    await new Promise((r) => setTimeout(r, 0));
    expect(project.projectDisplayName).toBeNull();
    expect(project.displayName).toBe('proj-x-0001');
  });
});
