// M-11 (UI audit 2026-09-29, /tmp/rimloc-ui-bugs.md): a REAL configured
// provider (Z.AI Personal, keychain key, working Base URL) showed
// «Подключён (мок)» — real accounts and mock verdicts mixed on one screen.
// Fix: the «(мок)» suffix is a separate span shown ONLY on the mock
// transport; on the real transport (no network probe in this build —
// deliberately) a settled provider reads the honest «настроен (не
// проверялся)», never a faked connection verdict.
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ProviderManager from '../src/lib/components/screens/ProviderManager.svelte';
import { clientInstance } from '../src/lib/client/instance.svelte';
import { devMode } from '../src/lib/stores/devmode.svelte';
import { i18n } from '../src/i18n/store.svelte';
import { cleanupMounted, exists, mountCmp, q } from './helpers';

function setMode(mode: unknown) {
  (clientInstance as unknown as { mode: unknown; client: unknown }).mode = mode;
}

describe('M-11: the (мок) suffix follows the real transport', () => {
  beforeEach(() => {
    cleanupMounted();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    devMode.enable();
    setMode(null); // re-resolve on next resolveMode()
    i18n.setLocale('en');
  });

  it('mock transport: the status keeps its honest (mock) mark', () => {
    mountCmp(ProviderManager);
    const st = q('providers.inst.status.inst-1');
    expect(st.textContent).toContain('Connected');
    expect(st.textContent).toContain('(mock)');
    expect(q('providers.inst.status.inst-2').textContent).toContain('Offline');
  });

  it('real transport: no mock mark, a configured provider reads "Configured (not verified)"', () => {
    // Real bridge present — the manager must not claim a mock OR a
    // connection verdict it cannot know.
    (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
      invoke: vi.fn()
    };
    setMode('tauri');
    mountCmp(ProviderManager);
    expect(q('providers.inst.status.inst-1').textContent).toContain('Configured (not verified)');
    // Local Ollama was "offline" only by simulation — same honest verdict.
    expect(q('providers.inst.status.inst-2').textContent).toContain('Configured (not verified)');
    // The keyless disabled instance stays unconfigured.
    expect(q('providers.inst.status.inst-3').textContent).toContain('Not configured');
    // The mock mark is gone from the whole instances section.
    expect(q('providers.instances.title').textContent).toContain('Configured providers');
    expect(document.body.textContent).not.toContain('(mock)');
  });

  it('real transport keeps statuses honest across a simulated probe', async () => {
    vi.useFakeTimers();
    try {
      (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
        invoke: vi.fn()
      };
      setMode('tauri');
      mountCmp(ProviderManager);
      q('providers.inst.test.inst-1').click();
      await vi.runAllTimersAsync();
      // The probe is local simulation only — the verdict stays "not verified".
      expect(q('providers.inst.status.inst-1').textContent).toContain('Configured (not verified)');
      expect(q('providers.inst.status.inst-1').textContent).not.toContain('Connected');
    } finally {
      vi.useRealTimers();
    }
  });
});
