// Audit P1-5 regression: the handshake CapabilityReport must reach the UI.
// Final night wave update: the Rust report (and the synced mock) now SUPPORT
// validate/build/diagnostics — the gates OPEN, the live panels render, and
// the honest-degradation path is exercised against a simulated unsupported
// report (the next honest "not yet" capability).
import { beforeEach, describe, expect, it } from 'vitest';
import { capability, CAP_BUILD, CAP_DIAGNOSTICS, CAP_VALIDATE } from '../src/lib/client/capability.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { buildState } from '../src/lib/mock/buildState.svelte';
import App from '../src/App.svelte';
import { cleanupMounted, click, exists, goto, mountCmp, q } from './helpers';

describe('capability store over the mock handshake', () => {
  beforeEach(() => {
    capability.reset();
    project.reset();
    buildState.reset();
  });

  it('caches the handshake report and answers strictly from `supported`', async () => {
    expect(capability.state(CAP_BUILD)).toBe(null); // unknown before ensure()
    await capability.ensure();
    expect(capability.error).toBe(null);
    expect(capability.state('project_create')).toBe(true);
    // Final night wave: validate/build/diagnostics ARE supported.
    expect(capability.state(CAP_VALIDATE)).toBe(true);
    expect(capability.state(CAP_BUILD)).toBe(true);
    expect(capability.state(CAP_DIAGNOSTICS)).toBe(true);
    // Strictly the supported list: a capability named in NEITHER list is
    // unsupported (the remaining honest slice boundary).
    expect(capability.state('source_inspector_actions')).toBe(false);
    expect(capability.reason('source_inspector_actions')).toBeTruthy();
    // Idempotent: a second ensure does not disturb the cached report.
    await capability.ensure();
    expect(capability.state(CAP_BUILD)).toBe(true);
  });

  it('opens the LIVE contract build panel when the capability is supported', async () => {
    await capability.ensure();
    const ok = await project.createContractProject('/mods/Demo');
    expect(ok).toBe(true);
    expect(project.source).toBe('contract');

    goto('#/workspace');
    mountCmp(App);
    expect((q('workspace.cta-build') as HTMLButtonElement).disabled).toBe(false);
    cleanupMounted();

    goto('#/build');
    mountCmp(App);
    // The live panel replaced the demo engine entirely.
    expect(exists('contractops.build')).toBe(true);
    expect(exists('contractops.validate.run')).toBe(true);
    expect(exists('contractops.export.run')).toBe(true);
    expect(exists('build.run')).toBe(false);
  });

  it('still degrades honestly when a capability is unsupported', async () => {
    // Simulate the pre-J report: build_export not yet supported.
    capability.report = {
      contract_version: 1,
      supported: ['project_create'],
      unsupported: [{ capability: CAP_BUILD, reason: 'next slice: safe build/export' }]
    };
    const ok = await project.createContractProject('/mods/Demo');
    expect(ok).toBe(true);
    expect(capability.state(CAP_BUILD)).toBe(false);

    goto('#/build');
    mountCmp(App);
    // Demo engine branch: gated run button + honest note.
    expect(exists('contractops.build')).toBe(false);
    expect((q('build.run') as HTMLButtonElement).disabled).toBe(true);
    expect(exists('build.capability-note')).toBe(true);
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
