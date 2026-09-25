// W-built regression: the client instance gate — tauri when the bridge is
// present, mock only with an explicit dev/demo opt-in, and an honest
// configuration error otherwise (never a silent mock default).
import { describe, expect, it, beforeEach, afterEach, vi } from 'vitest';
import { clientInstance, ClientConfigError } from '../src/lib/client/instance.svelte';
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
