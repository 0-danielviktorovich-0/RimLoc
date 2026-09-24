// Dev scenario browser (W6, MOCK_LIVE_ONBOARDING_MANDATE §11): a dev-only
// registry of stable scenario IDs with deep-links (`#/<route>?scenario=<id>`).
// Scenarios reuse the existing fixtures and perform REAL store actions and
// navigation — nothing pretends to be implemented that is not. This is
// tooling outside the product UI (opened from the dev panel); it does not
// affect the future production MockTransport guard, which stays a CI/xtask
// concern after the freeze.
import { ui } from './stores/ui.svelte';
import { devMode } from './stores/devmode.svelte';
import { router, type RouteId } from './router.svelte';
import { project } from './stores/project.svelte';
import { languages } from './languages/store.svelte';
import { onboarding } from './stores/onboarding.svelte';
import { diagnostics } from './stores/diagnostics.svelte';
import { demoProject } from './demo/demoProject.svelte';

export type ScenarioGroup =
  | 'home'
  | 'wizard'
  | 'workspace'
  | 'review'
  | 'build'
  | 'settings'
  | 'onboarding'
  | 'diagnostics';

export interface Scenario {
  /** Stable id, also the deep-link payload: `#/<route>?scenario=<id>`. */
  id: string;
  group: ScenarioGroup;
  route: RouteId;
  labelKey: string;
  /** Real setup actions; navigation to `route` happens automatically. */
  run?: () => void;
}

function seedWorkspace(): void {
  project.reset();
  languages.initFromPristine();
}

export const SCENARIOS: Scenario[] = [
  // home
  { id: 'home/first-run', group: 'home', route: 'home', labelKey: 'scenarios.s.home/first-run', run: () => { ui.homeMode = 'first-run'; ui.homeState = 'ready'; } },
  { id: 'home/returning', group: 'home', route: 'home', labelKey: 'scenarios.s.home/returning', run: () => { ui.homeMode = 'returning'; ui.homeState = 'ready'; } },
  { id: 'home/no-mods', group: 'home', route: 'home', labelKey: 'scenarios.s.home/no-mods', run: () => { ui.homeMode = 'no-mods'; ui.homeState = 'ready'; } },
  { id: 'home/demo', group: 'home', route: 'workspace', labelKey: 'scenarios.s.home/demo', run: () => demoProject.seed() },
  // wizard (branch preselection is read by the wizard from the scenario id)
  { id: 'wizard/mod', group: 'wizard', route: 'wizard', labelKey: 'scenarios.s.wizard/mod' },
  { id: 'wizard/base-game', group: 'wizard', route: 'wizard', labelKey: 'scenarios.s.wizard/base-game' },
  { id: 'wizard/dlc', group: 'wizard', route: 'wizard', labelKey: 'scenarios.s.wizard/dlc' },
  { id: 'wizard/language-pack', group: 'wizard', route: 'wizard', labelKey: 'scenarios.s.wizard/language-pack' },
  // workspace
  { id: 'workspace/standard', group: 'workspace', route: 'workspace', labelKey: 'scenarios.s.workspace/standard', run: seedWorkspace },
  {
    id: 'workspace/multi-target',
    group: 'workspace',
    route: 'workspace',
    labelKey: 'scenarios.s.workspace/multi-target',
    run: () => {
      seedWorkspace();
      languages.setActive('ja');
    }
  },
  {
    id: 'workspace/demo-tour',
    group: 'workspace',
    route: 'workspace',
    labelKey: 'scenarios.s.workspace/demo-tour',
    run: () => {
      demoProject.seed();
      onboarding.startDemoTour();
    }
  },
  // review / build
  { id: 'review/issues', group: 'review', route: 'review', labelKey: 'scenarios.s.review/issues', run: () => demoProject.seed() },
  { id: 'build/demo', group: 'build', route: 'build', labelKey: 'scenarios.s.build/demo', run: () => demoProject.seed() },
  // settings
  { id: 'settings/rimworld', group: 'settings', route: 'settings', labelKey: 'scenarios.s.settings/rimworld' },
  { id: 'settings/providers', group: 'settings', route: 'providers', labelKey: 'scenarios.s.settings/providers' },
  // onboarding
  {
    id: 'onboarding/coach',
    group: 'onboarding',
    route: 'workspace',
    labelKey: 'scenarios.s.onboarding/coach',
    run: () => onboarding.replay('coach')
  },
  {
    id: 'onboarding/demo-tour',
    group: 'onboarding',
    route: 'home',
    labelKey: 'scenarios.s.onboarding/demo-tour',
    run: () => onboarding.replay('demo')
  },
  // diagnostics
  {
    id: 'diagnostics/bundle-preview',
    group: 'diagnostics',
    route: 'diagnostics',
    labelKey: 'scenarios.s.diagnostics/bundle-preview',
    run: () => diagnostics.prepareBundle()
  }
];

class ScenarioBrowserStore {
  /** Dev-panel dialog visibility. */
  open = $state(false);
  /** Last scenario applied via deep-link (avoids re-running on the same hash). */
  lastApplied = $state<string | null>(null);

  show() {
    this.open = true;
  }

  hide() {
    this.open = false;
  }
}

export const scenarioBrowser = new ScenarioBrowserStore();

/** Stable deep-link for a scenario. */
export function scenarioHref(scenario: Scenario): string {
  return `#/${scenario.route}?scenario=${scenario.id}`;
}

/** Run a scenario: its setup, then the canonical navigation. Dev-only guard
 * lives here too so a programmatic caller cannot bypass the mode. */
export function runScenario(scenario: Scenario): void {
  if (!devMode.enabled) return;
  scenario.run?.();
  router.navigate(scenario.route);
}

/** Find a scenario by stable id (null when unknown). */
export function findScenario(id: string): Scenario | undefined {
  return SCENARIOS.find((s) => s.id === id);
}

// ---------------------------------------------------------------------------
// Deep-link runner (module scope, registered at import time — BEFORE any
// component effects): `#/<route>?scenario=<id>` applies on load and on every
// hashchange. Unknown ids are ignored. Running here guarantees the scenario
// setup wins over component-level navigation (e.g. the first-run coach would
// otherwise rewrite the hash and destroy the query).
// ---------------------------------------------------------------------------
let lastSeenHash = '';

function applyScenarioFromHash(): void {
  // Dev/testing gate: when the mode is off, ANY ?scenario= value — known,
  // unknown or malicious — is ignored without touching a single store.
  if (!devMode.enabled) return;
  const hash = window.location.hash;
  if (hash === lastSeenHash) return;
  lastSeenHash = hash;
  const query = hash.split('?')[1];
  if (!query) return;
  const id = new URLSearchParams(query).get('scenario');
  if (!id) return;
  const scenario = findScenario(id);
  if (!scenario) return;
  scenarioBrowser.lastApplied = id;
  runScenario(scenario);
}

if (typeof window !== 'undefined') {
  applyScenarioFromHash();
  window.addEventListener('hashchange', applyScenarioFromHash);
}
