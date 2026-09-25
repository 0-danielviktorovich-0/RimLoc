// Audit P1-5 regression: the handshake CapabilityReport must reach the UI —
//   - the store caches the report and answers strict support questions;
//   - on a REAL contract project the build/diagnostics CTAs degrade honestly
//     (disabled + reason) while the J slices have not landed;
//   - demo/fixture projects keep their marked demo flows (no gating).
import { beforeEach, describe, expect, it } from 'vitest';
import { capability, CAP_BUILD, CAP_DIAGNOSTICS } from '../src/lib/client/capability.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { buildState } from '../src/lib/mock/buildState.svelte';
import { diagnostics } from '../src/lib/stores/diagnostics.svelte';
import App from '../src/App.svelte';
import { cleanupMounted, click, exists, goto, mountCmp, q } from './helpers';

describe('capability store over the mock handshake', () => {
  beforeEach(() => {
    capability.reset();
    project.reset();
    buildState.reset();
    diagnostics.reset();
  });

  it('caches the handshake report and answers strictly from `supported`', async () => {
    expect(capability.state(CAP_BUILD)).toBe(null); // unknown before ensure()
    await capability.ensure();
    expect(capability.error).toBe(null);
    expect(capability.state('project_create')).toBe(true);
    // The honest slice boundary: build/diagnostics are NOT wired yet.
    expect(capability.state(CAP_BUILD)).toBe(false);
    expect(capability.state(CAP_DIAGNOSTICS)).toBe(false);
    expect(capability.reason(CAP_BUILD)).toBeTruthy();
    // Idempotent: a second ensure does not disturb the cached report.
    await capability.ensure();
    expect(capability.state(CAP_BUILD)).toBe(false);
  });

  it('disables the build CTA on a contract project, honest note on Build', async () => {
    await capability.ensure();
    const ok = await project.createContractProject('/mods/Demo');
    expect(ok).toBe(true);
    expect(project.source).toBe('contract');

    goto('#/workspace');
    mountCmp(App);
    const cta = q('workspace.cta-build') as HTMLButtonElement;
    expect(cta.disabled).toBe(true);
    // ru is the default test locale; the backend reason rides along.
    expect(cta.title).toContain('контракт-слайс');
    expect(cta.title).toContain('build/export');
    cleanupMounted();

    goto('#/build');
    mountCmp(App);
    expect((q('build.run') as HTMLButtonElement).disabled).toBe(true);
    expect(exists('build.capability-note')).toBe(true);
    // The demo build engine must not start from a gated button.
    click('build.run');
    expect(buildState.phase).toBe('idle');
  });

  it('keeps the demo/fixture build flow ungated', async () => {
    await capability.ensure();
    project.reset(); // fixture source (the explicit demo dataset)
    goto('#/build');
    mountCmp(App);
    // The route renders the run button enabled: fixture/demo projects keep
    // their marked demo build flow even though the capability is absent.
    expect((q('build.run') as HTMLButtonElement).disabled).toBe(false);
    expect(exists('build.capability-note')).toBe(false);
  });
});
