// W6 regression: dev scenario browser and scenario deep-links
// (MOCK_LIVE_ONBOARDING_MANDATE §11; lead 024 dev-gate).
//   - stable, unique scenario ids with reproducible deep-links;
//   - deep-links apply real setup actions — but ONLY in dev/testing mode;
//   - with the mode off, any ?scenario= value (known/unknown/malicious) is
//     ignored without touching a single store;
//   - the browser dialog opens from the dev panel, which is itself dev-gated.
import { beforeEach, describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { SCENARIOS, findScenario, scenarioHref } from '../src/lib/scenarios.svelte';
import { scenarioBrowser } from '../src/lib/scenarios.svelte';
import { devMode } from '../src/lib/stores/devmode.svelte';
import { onboarding } from '../src/lib/stores/onboarding.svelte';
import { ui } from '../src/lib/stores/ui.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { diagnostics } from '../src/lib/stores/diagnostics.svelte';
import { cleanupMounted, exists, goto, mountCmp, q } from './helpers';
import { demoProject } from '../src/lib/demo/demoProject.svelte';

beforeEach(() => {
  cleanupMounted();
  // Tests drive the gate explicitly; import.meta.env.DEV is true under
  // vitest, so the disabled cases must flip the store off by hand.
  devMode.enable();
});

describe('scenario registry', () => {
  it('has unique stable ids in group/id form', () => {
    const ids = SCENARIOS.map((s) => s.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const id of ids) {
      expect(id).toMatch(/^[a-z]+\/[a-z-]+$/);
    }
  });

  it('deep-links reproduce each scenario URL deterministically', () => {
    for (const scenario of SCENARIOS) {
      expect(scenarioHref(scenario)).toBe(`#/${scenario.route}?scenario=${scenario.id}`);
    }
  });
});

describe('scenario deep-links apply real state', () => {
  it('home/first-run switches the home view via URL alone', () => {
    ui.homeMode = 'returning';
    goto('#/home?scenario=home/first-run');
    mountCmp(App);
    flushSync();
    expect(ui.homeMode).toBe('first-run');
    expect(exists('home.hint')).toBe(true);
  });

  it('home/no-mods renders the three-way empty state via URL alone', () => {
    goto('#/home?scenario=home/no-mods');
    mountCmp(App);
    flushSync();
    expect(ui.homeMode).toBe('no-mods');
    expect(exists('home.nomods.demo')).toBe(true);
    expect(exists('home.nomods.folder')).toBe(true);
    expect(exists('home.nomods.configure')).toBe(true);
  });

  it('home/demo seeds the bundled demo via URL alone', () => {
    goto('#/home?scenario=home/demo');
    mountCmp(App);
    flushSync();
    expect(project.projectName).toBe('RimLoc Demo');
    expect(project.isDemo).toBe(true);
  });

  it('workspace/multi-target switches the active target to ja', () => {
    goto('#/workspace?scenario=workspace/multi-target');
    mountCmp(App);
    flushSync();
    expect(languagesActive()).toBe('ja');
    expect(project.isDemo).toBe(false); // standard dataset, ja target active
  });

  it('review/issues opens the review queue on the demo dataset', () => {
    goto('#/review?scenario=review/issues');
    mountCmp(App);
    flushSync();
    expect(project.projectName).toBe('RimLoc Demo');
    expect(exists('workspace.review')).toBe(true);
  });

  it('diagnostics/bundle-preview shows the sanitized bundle', () => {
    goto('#/diagnostics?scenario=diagnostics/bundle-preview');
    mountCmp(App);
    flushSync();
    expect(exists('diagnostics.bundle')).toBe(true);
    expect(diagnostics.bundle?.counts.redacted).toBeGreaterThan(0);
  });

  it('onboarding/demo-tour starts the guided tour on Home', () => {
    goto('#/home?scenario=onboarding/demo-tour');
    mountCmp(App);
    flushSync();
    expect(onboarding.open).toBe(true);
    expect(onboarding.script).toBe('demo');
  });

  it('cold-boot workspace/demo-tour seeds the demo and stays in the workspace (tail fix)', () => {
    // Cold boot: the deep-link is in the hash BEFORE any component mounts.
    goto('#/workspace?scenario=workspace/demo-tour');
    mountCmp(App);
    flushSync();
    expect(project.projectName).toBe('RimLoc Demo');
    expect(onboarding.open).toBe(true);
    expect(onboarding.script).toBe('demo');
    // The Home step is skipped (demo already seeded): tour sits on the row step.
    expect(onboarding.step).toBe(1);
    expect(exists('onboarding.overlay')).toBe(true);
    // The init order must not bounce the scenario to Home.
    expect(window.location.hash).toContain('#/workspace');
    expect(window.location.hash).not.toBe('#/home');
  });

  it('unknown scenario ids are ignored without crashing', () => {
    ui.homeMode = 'returning'; // prove the bogus URL changes nothing
    goto('#/home?scenario=bogus/scenario');
    mountCmp(App);
    flushSync();
    expect(exists('home.nomods')).toBe(false);
    expect(ui.homeMode).toBe('returning');
  });

  it('with dev mode OFF, a known scenario deep-link mutates nothing (lead 024)', () => {
    devMode.disable();
    demoProject.leave(); // cancel demo leakage from earlier tests in this file
    const snapshot = JSON.stringify(project.entries);
    ui.homeMode = 'returning';
    goto('#/home?scenario=home/no-mods');
    mountCmp(App);
    flushSync();
    // The deep-link is inert: no home-mode switch, no seeding, no reset.
    expect(ui.homeMode).toBe('returning');
    expect(project.projectName).toBe('TestMod');
    expect(project.isDemo).toBe(false);
    expect(JSON.stringify(project.entries)).toBe(snapshot);
    // Re-enabling applies the pending deep-link on the next navigation pass.
    devMode.enable();
    goto('#/workspace');
    goto('#/home?scenario=home/no-mods');
    flushSync();
    expect(ui.homeMode).toBe('no-mods');
  });

  it('with dev mode OFF, diagnostics deep-link does not build a bundle', () => {
    devMode.disable();
    goto('#/diagnostics?scenario=diagnostics/bundle-preview');
    mountCmp(App);
    flushSync();
    expect(diagnostics.bundle).toBeNull();
  });

  function languagesActive(): string {
    // The switcher pins reflect the active locale; read the persisted shape.
    const raw = window.localStorage.getItem('rimloc.project.testmod.multitarget.v1');
    const shape = raw ? (JSON.parse(raw) as { active: string }) : { active: '' };
    return shape.active;
  }
});

describe('scenario browser dialog (dev panel entry)', () => {
  it('dev panel and scenario picker are dev-gated (hidden when mode is off)', () => {
    devMode.disable();
    goto('#/home');
    mountCmp(App);
    expect(exists('dev.panel')).toBe(false);
    expect(exists('dev.scenarios')).toBe(false);
    // The honesty chip stays — audit M-2: it now names the transport
    // (mock mode, no project open → mock-transport badge).
    expect(exists('transport-mock-badge')).toBe(true);
  });

  it('opens from the dev panel, lists all scenarios, closes', () => {
    devMode.enable();
    goto('#/home');
    mountCmp(App);
    // Dev panel is a closed <details> by default — open it first.
    (document.querySelector('[data-testid="dev.panel"]') as HTMLDetailsElement | null)?.setAttribute('open', '');
    click_('dev.scenarios');
    expect(exists('scenarios.dialog')).toBe(true);
    expect(document.querySelectorAll('[data-testid^="scenarios.item."]').length).toBe(SCENARIOS.length);
    click_('scenarios.close');
    expect(exists('scenarios.dialog')).toBe(false);
  });

  it('running the home/no-mods entry applies the scenario', () => {
    goto('#/home');
    mountCmp(App);
    (document.querySelector('[data-testid="dev.panel"]') as HTMLDetailsElement | null)?.setAttribute('open', '');
    click_('dev.scenarios');
    click_('scenarios.item.home/no-mods');
    expect(ui.homeMode).toBe('no-mods');
    expect(exists('scenarios.dialog')).toBe(false); // picker closes after pick
  });

  // Local click that also flushes the scenario store effects.
  function click_(testid: string) {
    q(testid).click();
    flushSync();
  }
});

describe('registry lookup helpers', () => {
  it('finds a scenario by id and returns undefined for unknown ones', () => {
    expect(findScenario('home/no-mods')?.route).toBe('home');
    expect(findScenario('no/such-id')).toBeUndefined();
  });

  it('dialog starts closed (scenario browser stays out of the product UI)', () => {
    expect(scenarioBrowser.open).toBe(false);
  });
});
