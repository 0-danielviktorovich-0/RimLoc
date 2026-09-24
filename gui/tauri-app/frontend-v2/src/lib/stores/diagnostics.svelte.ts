// Diagnostics store (W5): reactive phase machine over the mock diagnostics
// layer. Owns the controlled-failure replay (idle → running → diagnosed) and
// the sanitized support-bundle preview, so both the Help screen and the
// #/diagnostics deep screen read the same single source of truth.

import {
  buildAiPrompt,
  buildCausalContext,
  buildRawBundle,
  sanitizeBundle,
  type BundlePreview,
  type CausalContext
} from '../mock/diagnostics';
import { providers } from './providers.svelte';
import { i18n } from '../../i18n/store.svelte';
import { languages } from '../languages/store.svelte';
import { registry } from '../languages/registry';

/**
 * Mock locations the bundle shows with home-path normalization. Synthetic
 * generic fixtures on purpose: no real username/company/workshop id of the
 * owner may appear in code or tests (the live pipeline binds real paths).
 */
export const SOURCE_LOCATION = `/Users/<user>/Library/Application Support/Steam/steamapps/workshop/content/294100/<publishedfileid>`;
export const OUTPUT_LOCATION = '/Users/<user>/Projects/<mod>/Languages';

/** Replay steps shown while the scenario runs. */
export interface ScenarioStep {
  id: 'inventory' | 'validate' | 'causal';
  labelKey: string;
  state: 'pending' | 'active' | 'done';
}

export type DiagnosticsPhase = 'idle' | 'running' | 'diagnosed' | 'bundled';

const STEP_MS = 480;

function initialSteps(): ScenarioStep[] {
  return [
    { id: 'inventory', labelKey: 'diagnostics.step.inventory', state: 'pending' },
    { id: 'validate', labelKey: 'diagnostics.step.validate', state: 'pending' },
    { id: 'causal', labelKey: 'diagnostics.step.causal', state: 'pending' }
  ];
}

// Exported for regression tests (fresh instances, fake timers); the app
// uses the singleton below.
export class DiagnosticsStore {
  phase = $state<DiagnosticsPhase>('idle');
  steps = $state<ScenarioStep[]>(initialSteps());
  causal = $state<CausalContext | null>(null);
  bundle = $state<BundlePreview | null>(null);
  aiPrompt = $state('');

  private timers: ReturnType<typeof setTimeout>[] = [];

  private clearTimers() {
    for (const timer of this.timers) clearTimeout(timer);
    this.timers = [];
  }

  /** Drop everything (screen unmount / explicit reset). */
  reset() {
    this.clearTimers();
    this.phase = 'idle';
    this.steps = initialSteps();
    this.causal = null;
    this.bundle = null;
    this.aiPrompt = '';
  }

  /**
   * Replay the controlled known failure: run the pipeline, let the validator
   * fail on keyed-04, assemble the structured causal context. Store reads the
   * live provider/locale state so the context reflects the actual session.
   *
   * A rerun clears the previous causal context FIRST: while the new run is
   * 'running', no stale operation id / bundle / prompt may remain readable or
   * copyable as if it were the fresh result.
   */
  runScenario() {
    this.clearTimers();
    this.phase = 'running';
    this.steps = initialSteps();
    this.causal = null;
    this.bundle = null;
    this.aiPrompt = '';

    const [inventory, validate, causal] = this.steps;
    this.timers.push(
      setTimeout(() => {
        inventory.state = 'done';
        validate.state = 'active';
      }, STEP_MS)
    );
    this.timers.push(
      setTimeout(() => {
        validate.state = 'done';
        causal.state = 'active';
      }, STEP_MS * 2)
    );
    this.timers.push(
      setTimeout(() => {
        this.causal = buildCausalContext({
          providerCounts: providers.counts(),
          sourceLocale: languages.sourceLocale,
          targetLocale: languages.activeLocale,
          uiLocale: i18n.locale
        });
        causal.state = 'done';
        this.phase = 'diagnosed';
      }, STEP_MS * 3)
    );
  }

  /**
   * Build the sanitized bundle preview from the current causal context (or
   * the last-error default when the scenario has not been replayed yet).
   */
  prepareBundle() {
    const ctx = this.causal;
    const raw = buildRawBundle(ctx, { sourceLocation: SOURCE_LOCATION, outputLocation: OUTPUT_LOCATION });
    this.bundle = sanitizeBundle(raw);
    this.aiPrompt = buildAiPrompt(this.bundle);
    this.phase = 'bundled';
  }
}

export const diagnostics = new DiagnosticsStore();
