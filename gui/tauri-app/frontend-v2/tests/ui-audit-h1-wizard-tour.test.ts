// H-1 (UI audit 2026-09-29, /tmp/rimloc-ui-bugs.md): the seven-step wizard
// was unreachable on the live Home — the entry cards render only outside
// tauri mode. Fix: a secondary "demo tour" entry in the live create panel
// that navigates to the wizard, honestly labeled as a DEMO tour so it is
// never passed off as the live create flow.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import Home from '../src/lib/components/screens/Home.svelte';
import { clientInstance } from '../src/lib/client/instance.svelte';
import { devMode } from '../src/lib/stores/devmode.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { onboarding } from '../src/lib/stores/onboarding.svelte';
import { i18n } from '../src/i18n/store.svelte';
import { click, cleanupMounted, exists, goto, mountCmp, q } from './helpers';

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

describe('H-1: the wizard demo-tour entry on the live Home', () => {
  beforeEach(() => {
    cleanupMounted();
    resetClientSingleton();
    devMode.disable();
    project.reset();
    onboarding.open = false;
    i18n.setLocale('ru');
  });
  afterEach(cleanupMounted);

  function mountTauriHome() {
    (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
      invoke: vi.fn(async (cmd: string) => {
        if (cmd === 'contract_handshake') return HANDSHAKE_OK;
        if (cmd === 'project_list') return [];
        throw new Error(`unexpected command ${cmd}`);
      })
    };
    goto('#/home');
    mountCmp(Home);
    flushSync();
  }

  it('the live create panel exposes the tour button and navigates to the wizard', () => {
    mountTauriHome();
    expect(exists('home.contract.wizardTour')).toBe(true);
    click('home.contract.wizardTour');
    expect(window.location.hash).toContain('wizard');
  });

  it('the entry is honestly labeled a demo tour in both locales', () => {
    mountTauriHome();
    expect(q('home.contract.wizardTour').textContent).toContain('демо-тур');
    cleanupMounted();
    i18n.setLocale('en');
    mountTauriHome();
    expect(q('home.contract.wizardTour').textContent).toContain('demo tour');
    // The honest note is part of the panel, not a hidden tooltip.
    expect(document.body.textContent).toContain('demo project');
  });
});
