// W-built regression: the client instance gate — tauri when the bridge is
// present, mock only with an explicit dev/demo opt-in, and an honest
// configuration error otherwise (never a silent mock default).
import { describe, expect, it, beforeEach, afterEach, vi } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { clientInstance, ClientConfigError } from '../src/lib/client/instance.svelte';
import { click, cleanupMounted, exists, goto, mountCmp } from './helpers';
import { project } from '../src/lib/stores/project.svelte';
import { devMode } from '../src/lib/stores/devmode.svelte';

describe('client instance mode gate', () => {
  beforeEach(() => {
    // Reset the cached singleton between cases.
    (clientInstance as unknown as { mode: unknown; client: unknown }).mode = null;
    (clientInstance as unknown as { client: unknown }).client = null;
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  it('mock mode requires the explicit dev opt-in', () => {
    devMode.enable();
    const client = clientInstance.getClient();
    expect(client.mode).toBe('mock');
  });

  it('no bridge + no opt-in = configuration error, never a silent mock', () => {
    devMode.disable();
    expect(() => clientInstance.getClient()).toThrow(ClientConfigError);
    expect(clientInstance.resolveMode()).toBe('none');
    expect(clientInstance.configError).toContain('dev');
  });

  it('tauri bridge present = the real transport, even without devMode', () => {
    devMode.disable();
    (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
      invoke: vi.fn()
    };
    const client = clientInstance.getClient();
    expect(client.mode).toBe('tauri');
  });
});

describe('tauri mode Home surface (P2-a: no fixture leak)', () => {
  it('fixture cards are hidden and the live panel is shown', async () => {
    cleanupMounted();
    devMode.disable();
    (window as unknown as { __TAURI_INTERNALS__?: { invoke: unknown } }).__TAURI_INTERNALS__ = {
      invoke: vi.fn()
    };
    window.location.hash = '#/home';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    mountCmp(App);
    flushSync();
    expect(exists('home.entry-new')).toBe(false);
    expect(exists('home.entry-existing')).toBe(false);
    expect(exists('home.demo')).toBe(false);
    expect(exists('home.contract.create')).toBe(true);
    // Audit M-2: with no project open the chip states the transport (live
    // bridge) — the honest-mode chip stays, it no longer claims demo data.
    expect(exists('transport-live-badge')).toBe(true);
    expect(exists('mock-badge')).toBe(false);
    cleanupMounted();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  it('fixture workspace shows the not-persisted notice (P2-a)', async () => {
    cleanupMounted();
    devMode.disable();
    project.reset(); // plain fixture dataset
    window.location.hash = '#/workspace';
    window.dispatchEvent(new HashChangeEvent('hashchange'));
    mountCmp(App);
    flushSync();
    // The overview lives on the Project tab.
    click('tabs.project');
    expect(exists('workspace.project.fixture')).toBe(true);
    expect(exists('workspace.project.live')).toBe(false);
  });
});
