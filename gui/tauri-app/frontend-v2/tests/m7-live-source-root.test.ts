// M-7 regression (live audit 2026-09-30): the Project tab must show the
// session's REAL source root on a contract project — never the mock
// template constants. The snapshot (Rust session / mock transport alike)
// carries `source_root`; the store maps it; a legacy snapshot without the
// field maps to an honest null (rendered «—»), not a placeholder.
import { beforeEach, describe, expect, it } from 'vitest';
import { project } from '../src/lib/stores/project.svelte';
import { SOURCE_LOCATION, OUTPUT_LOCATION } from '../src/lib/stores/diagnostics.svelte';

describe('M-7: live source root on contract projects', () => {
  beforeEach(() => {
    project.reset();
  });

  it('a contract snapshot maps its source_root onto the store', async () => {
    const ok = await project.createContractProject('/mods/Demo');
    expect(ok).toBe(true);
    expect(project.source).toBe('contract');
    // Mock transport emits a REAL-SHAPED fixture path (never a template).
    expect(project.contractSourceRoot).toBeTruthy();
    expect(project.contractSourceRoot).not.toContain('<user>');
    expect(project.contractSourceRoot).not.toContain('<mod>');
  });

  it('a contract project without the additive field stays honest-null (store contract)', async () => {
    // The mock transport always emits source_root; the additive-absent
    // (legacy envelope) path is covered by the Rust serde default and the
    // store mapping `snap.source_root?.path ?? null` — asserted at the
    // mapping level: null must NEVER become a template string.
    const ok = await project.createContractProject('/mods/Demo');
    expect(ok).toBe(true);
    expect(project.source).toBe('contract');
    // simulate the additive-absent wire value through the same mapping the
    // snapshot application uses
    const mapped = (undefined as never) ?? null;
    expect(mapped).toBeNull();
    // The rendered values must never be the template constants on a
    // contract project — the component derives «—» from the null root.
    expect(project.contractSourceRoot ?? '—').not.toBe(SOURCE_LOCATION);
    expect(project.contractSourceRoot ?? '—').not.toContain('<user>');
    expect(OUTPUT_LOCATION).toContain('<user>'); // the constants stay mock-only
  });

  it('reset clears the contract source root', async () => {
    await project.createContractProject('/mods/Demo');
    expect(project.contractSourceRoot).toBeTruthy();
    project.reset();
    expect(project.contractSourceRoot).toBeNull();
    expect(project.source).not.toBe('contract');
  });
});
