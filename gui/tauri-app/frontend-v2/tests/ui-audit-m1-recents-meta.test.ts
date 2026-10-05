// M-1 (UI audit 2026-09-29, /tmp/rimloc-ui-bugs.md): «Недавние проекты» in
// tauri mode showed 7 indistinguishable «RimLoc UI (en)» rows. Fix: the row
// carries a mono secondary meta line straight from ProjectSummary — target
// version (when present), revision (rN) and the short id (last 6 chars of
// project_id). No new contract fields.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import Home from '../src/lib/components/screens/Home.svelte';
import { clientInstance } from '../src/lib/client/instance.svelte';
import { devMode } from '../src/lib/stores/devmode.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { onboarding } from '../src/lib/stores/onboarding.svelte';
import { cleanupMounted, exists, goto, mountCmp } from './helpers';

const HANDSHAKE_OK = {
  ui_contract_version: 1,
  capabilities: {
    contract_version: 1,
    supported: ['project_create', 'project_list'],
    unsupported: []
  }
};

describe('M-1: contract recents carry distinguishing summary metadata', () => {
  beforeEach(() => {
    cleanupMounted();
    (clientInstance as unknown as { mode: unknown; client: unknown }).mode = null;
    (clientInstance as unknown as { client: unknown }).client = null;
    (project as unknown as { rimloc: unknown }).rimloc = null;
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    devMode.disable();
    project.reset();
    onboarding.open = false;
  });
  afterEach(cleanupMounted);

  it('the secondary line shows target version, revision and the short id', async () => {
    (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
      invoke: vi.fn(async (cmd: string) => {
        if (cmd === 'contract_handshake') return HANDSHAKE_OK;
        if (cmd === 'project_list') {
          return [
            // The audit's look-alike name, two different summaries.
            {
              project_id: 'proj-01a0deb47e9a',
              name: 'RimLoc UI (en)',
              revision: 4,
              target_version: '1.6'
            },
            { project_id: 'proj-99ffff0000ff', name: 'RimLoc UI (en)', revision: 7 }
          ];
        }
        throw new Error(`unexpected command ${cmd}`);
      })
    };
    goto('#/home');
    mountCmp(Home);
    flushSync();
    await vi.waitFor(() => {
      expect(
        document.querySelector('[data-testid="home.contract.open.proj-01a0deb47e9a"]')
      ).not.toBeNull();
    });
    const text = document.body.textContent ?? '';
    // Target version (when the DTO carries it) · rN · #last-6-of-project_id.
    expect(text).toContain('v1.6 · r4 · #b47e9a');
    // No target_version in the summary → the meta stays honest, no invention.
    expect(text).toContain('r7 · #0000ff');
    expect(text).not.toContain('v1.6 · r7');
    // Rows are keyed by the full id even though only the suffix shows.
    expect(exists('home.contract.open.proj-99ffff0000ff')).toBe(true);
  });
});
