// M-10 (UI audit 2026-09-29, /tmp/rimloc-ui-bugs.md): an empty out dir +
// «Собрать и записать» gave NO visible reaction. Part 1 (already on main,
// kept honest): the run button is disabled for an empty/non-absolute out
// dir. Part 2 (this fix): a refusal AFTER the request went out surfaces as
// a role=alert ops-error INSIDE the export card — never as silence.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import ContractOps from '../src/lib/components/screens/ContractOps.svelte';
import { clientInstance } from '../src/lib/client/instance.svelte';
import { devMode } from '../src/lib/stores/devmode.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { contractops } from '../src/lib/stores/contractops.svelte';
import { cleanupMounted, exists, mountCmp, q } from './helpers';

describe('M-10: an export refusal is visible inside the export card', () => {
  beforeEach(() => {
    cleanupMounted();
    (clientInstance as unknown as { mode: unknown; client: unknown }).mode = null;
    (clientInstance as unknown as { client: unknown }).client = null;
    (project as unknown as { rimloc: unknown }).rimloc = null;
    devMode.enable();
    project.reset();
    contractops.reset();
  });
  afterEach(cleanupMounted);

  it('the run button stays gated on an absolute dir; a refused run surfaces in the card', async () => {
    expect(await project.createContractProject('/mods/Demo')).toBe(true);
    mountCmp(ContractOps, { kind: 'build' });
    const input = q('contractops.export.outdir') as HTMLInputElement;
    const run = q('contractops.export.run') as HTMLButtonElement;

    // Part 1 of the fix: empty → no request at all.
    expect(input.value).toBe('');
    expect(run.disabled).toBe(true);

    // Part 2: an ABSOLUTE dir that the backend refuses (inside the source
    // tree) → the typed refusal is visible IN the export card, role=alert.
    input.value = '/mods/Demo/Languages';
    input.dispatchEvent(new Event('input'));
    flushSync();
    expect(run.disabled).toBe(false);
    run.click();
    await vi.waitFor(() => {
      expect(exists('contractops.export.error')).toBe(true);
    });
    const card = q('contractops.export');
    const err = q('contractops.export.error');
    expect(card.contains(err)).toBe(true);
    expect(err.getAttribute('role')).toBe('alert');
    expect(err.textContent).toContain('guard_output_denied');
    // The section-level alert still carries the same truth (unchanged path).
    expect(q('contractops.error').textContent).toContain('guard_output_denied');
  });

  it('a relative dir keeps the button disabled — no request, no card error', async () => {
    expect(await project.createContractProject('/mods/Demo')).toBe(true);
    mountCmp(ContractOps, { kind: 'build' });
    const input = q('contractops.export.outdir') as HTMLInputElement;
    const run = q('contractops.export.run') as HTMLButtonElement;
    input.value = 'RimLoc-Export';
    input.dispatchEvent(new Event('input'));
    flushSync();
    expect(run.disabled).toBe(true);
    expect(exists('contractops.export.error')).toBe(false);
  });

  it('the error clears on the next attempt', async () => {
    expect(await project.createContractProject('/mods/Demo')).toBe(true);
    mountCmp(ContractOps, { kind: 'build' });
    const input = q('contractops.export.outdir') as HTMLInputElement;
    const run = q('contractops.export.run') as HTMLButtonElement;
    input.value = '/mods/Demo/Languages';
    input.dispatchEvent(new Event('input'));
    flushSync();
    run.click();
    await vi.waitFor(() => {
      expect(exists('contractops.export.error')).toBe(true);
    });
    // A retry with a writable dir clears the stale refusal and succeeds.
    input.value = '/tmp/rimloc-export-m10';
    input.dispatchEvent(new Event('input'));
    flushSync();
    run.click();
    await vi.waitFor(() => {
      expect(exists('contractops.export.result')).toBe(true);
    });
    expect(exists('contractops.export.error')).toBe(false);
  });
});
