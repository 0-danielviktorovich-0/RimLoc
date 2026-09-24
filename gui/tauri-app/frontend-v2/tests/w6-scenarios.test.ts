// W6 regression: dev scenario browser and scenario deep-links
// (MOCK_LIVE_ONBOARDING_MANDATE §11).
//   - stable, unique scenario ids with reproducible deep-links;
//   - deep-links apply real setup actions (home modes, demo seeding,
//     multi-target switch, diagnostics bundle) via the hash alone;
//   - unknown scenario ids are ignored honestly;
//   - the browser opens from the dev panel and lists every scenario.
import { describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { SCENARIOS, findScenario, scenarioHref } from '../src/lib/scenarios.svelte';
import { scenarioBrowser } from '../src/lib/scenarios.svelte';
import { onboarding } from '../src/lib/stores/onboarding.svelte';
import { ui } from '../src/lib/stores/ui.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { diagnostics } from '../src/lib/stores/diagnostics.svelte';
import { cleanupMounted, exists, goto, mountCmp, q } from './helpers';

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

  it('unknown scenario ids are ignored without crashing', () => {
    ui.homeMode = 'returning'; // prove the bogus URL changes nothing
    goto('#/home?scenario=bogus/scenario');
    mountCmp(App);
    flushSync();
    expect(exists('home.nomods')).toBe(false);
    expect(ui.homeMode).toBe('returning');
  });

  function languagesActive(): string {
    // The switcher pins reflect the active locale; read the persisted shape.
    const raw = window.localStorage.getItem('rimloc.project.testmod.multitarget.v1');
    const shape = raw ? (JSON.parse(raw) as { active: string }) : { active: '' };
    return shape.active;
  }
});

describe('scenario browser dialog (dev panel entry)', () => {
  it('opens from the dev panel, lists all scenarios, closes', () => {
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
