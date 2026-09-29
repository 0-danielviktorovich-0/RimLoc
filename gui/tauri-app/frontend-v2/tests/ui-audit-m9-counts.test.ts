// M-9 (UI audit 2026-09-29, /tmp/rimloc-ui-bugs.md): after a validation the
// counters rendered as «ОшибкиПредупрежденияИнфо» with orphan numbers «1 0 0»
// — the `.counts` row used `gap: var(--space-5)`, a token that is NOT
// DEFINED in tokens.css, so the gap collapsed to 0 and the label+number
// pairs fused. Fix: real tokens + a divider between the pairs; the DOM keeps
// one <div> per number+label pair (dt/dd together).
import { beforeEach, describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import ContractOps from '../src/lib/components/screens/ContractOps.svelte';
import { clientInstance } from '../src/lib/client/instance.svelte';
import { devMode } from '../src/lib/stores/devmode.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { contractops } from '../src/lib/stores/contractops.svelte';
import { i18n } from '../src/i18n/store.svelte';
import { cleanupMounted, exists, mountCmp, q } from './helpers';

// vitest root is the frontend-v2 package root (see RUN header).
const SRC = readFileSync(
  resolve(process.cwd(), 'src/lib/components/screens/ContractOps.svelte'),
  'utf8'
);

describe('M-9: validate counters render as separated number+label pairs', () => {
  beforeEach(() => {
    cleanupMounted();
    (clientInstance as unknown as { mode: unknown; client: unknown }).mode = null;
    (clientInstance as unknown as { client: unknown }).client = null;
    (project as unknown as { rimloc: unknown }).rimloc = null;
    devMode.enable();
    project.reset();
    contractops.reset();
    i18n.setLocale('en');
  });

  it('each counter is its own pair: label + number never share a text node row', async () => {
    expect(await project.createContractProject('/mods/Demo')).toBe(true);
    expect(await contractops.runValidate()).toBe(true);
    mountCmp(ContractOps, { kind: 'build' });

    expect(exists('contractops.validate.counts')).toBe(true);
    const counts = q('contractops.validate.counts');
    const pairs = counts.querySelectorAll(':scope > div');
    // One div per number+label pair — the pairs are separate elements.
    expect(pairs.length).toBe(3);
    const labels = [...pairs].map((p) => p.querySelector('dt')?.textContent?.trim());
    expect(labels).toEqual(['Errors', 'Warnings', 'Info']);
    // Every pair carries BOTH its label and its number together…
    for (const p of pairs) {
      expect(p.querySelector('dd')?.textContent?.trim()).not.toBe('');
      expect(p.querySelector('dt')?.textContent?.trim() ?? '').not.toBe('');
    }
    // …and the numbers are the real validate result (1 error / 2 warnings on
    // the mock fixture — contract-ops.test.ts teeth).
    const numbers = [...pairs].map((p) => p.querySelector('dd')?.textContent?.trim());
    expect(numbers).toEqual([
      String(contractops.validateResult?.error_count),
      String(contractops.validateResult?.warning_count),
      String(contractops.validateResult?.info_count)
    ]);
  });

  it('the undefined --space-5 gap token is gone; a pair divider rule exists', () => {
    // The fused «ОшибкиПредупрежденияИнфо» root cause: an undefined CSS var.
    expect(SRC).not.toContain('var(--space-5)');
    // The lead's fix shape: dividers + real spacing between the pairs.
    expect(SRC).toContain('.counts div + div');
    expect(SRC).toContain('gap: var(--space-2) var(--space-4)');
  });
});
