// Onboarding / product-tour state (QA mandate §13, §20; W6:
// MOCK_LIVE_ONBOARDING_MANDATE §4-§5, §7).
//
// Two scripts share one engine:
//   - 'coach' — the 4-step first-open coach in the Workspace (W1 behavior
//     preserved: shown once, skip persists, Help replay re-shows it);
//   - 'demo'  — the guided demo flow (Open Demo → pick a row → edit →
//     Context → Review → Validate → Build demo → done): real actions on the
//     bundled demo project, never a text carousel.
//
// Persistence: first run = no flags; skip and completion both set the W1 seen
// flag ('1', kept for W1-test compatibility) plus a separate outcome record
// ('skipped' | 'completed') so the UI can tell them apart.
const STORAGE_KEY = 'rimloc.onboarding.seen.v1';
const OUTCOME_KEY = 'rimloc.onboarding.outcome.v1';
const DEMO_TOUR_KEY = 'rimloc.tour.demo.v1';

export type TourScript = 'coach' | 'demo';
export type TourOutcome = 'completed' | 'skipped';

/** Step counts per script (the step definitions live in onboarding/steps.ts). */
export const COACH_TOTAL = 4;
export const DEMO_TOTAL = 9;

function readSeen(): boolean {
  try {
    return window.localStorage.getItem(STORAGE_KEY) === '1';
  } catch {
    // Storage unavailable (private mode etc.) — never nag.
    return true;
  }
}

function persistSeen() {
  try {
    window.localStorage.setItem(STORAGE_KEY, '1');
  } catch {
    /* ignore */
  }
}

function readOutcome(): TourOutcome | null {
  try {
    const v = window.localStorage.getItem(OUTCOME_KEY);
    return v === 'completed' || v === 'skipped' ? v : null;
  } catch {
    return null;
  }
}

function persistOutcome(outcome: TourOutcome) {
  try {
    window.localStorage.setItem(OUTCOME_KEY, outcome);
  } catch {
    /* ignore */
  }
}

class OnboardingStore {
  /** Tour overlay visibility. */
  open = $state(false);
  /** Current step, 0-based, within the active script. */
  step = $state(0);
  /** True once the user has seen (or skipped) the coach. Persisted. */
  seen = $state(true);
  /** Which script is on screen. */
  script = $state<TourScript>('coach');
  /** Step count of the active script (drives progress + last-step detection). */
  total = $state(COACH_TOTAL);
  /** Outcome of the last coach run. Persisted. */
  outcome = $state<TourOutcome | null>(readOutcome());
  /** Whether the guided demo tour was ever started (persisted). */
  demoTourSeen = $state(false);
  /**
   * Guided-action ledger for the RUNNING tour: step id -> the action actually
   * COMPLETED successfully (recorded only after the awaited result, never at
   * click start). The final demo step reads this ledger: success copy is only
   * allowed when the work really happened. Reset on every script start; not
   * persisted — it describes one pass.
   */
  guidedDone = $state<Record<string, boolean>>({});
  /**
   * Bumped by every script start: async guided actions compare their captured
   * pass id against this when they finally complete — a stale completion
   * arriving after Skip/replay must not mark or advance the new pass.
   */
  passId = $state(0);

  constructor() {
    this.seen = readSeen();
    try {
      this.demoTourSeen = window.localStorage.getItem(DEMO_TOUR_KEY) === '1';
    } catch {
      this.demoTourSeen = false;
    }
  }

  /** Called when the Workspace shell mounts: first open starts the coach.
   * Never hijacks a tour that is already on screen (the guided demo script
   * legitimately passes through the Workspace). */
  startIfFirstRun() {
    if (!this.seen && !this.open) {
      this.startScript('coach');
    }
  }

  /** Open a script at step 0. The guided ledger starts empty — a fresh pass
   * must earn its success copy with real actions. A new passId invalidates
   * async completions that belonged to a previous pass. */
  startScript(script: TourScript) {
    this.script = script;
    this.total = script === 'coach' ? COACH_TOTAL : DEMO_TOTAL;
    this.step = 0;
    this.guidedDone = {};
    this.passId += 1;
    this.open = true;
  }

  /** Record that a guided action was actually performed on this pass. */
  markGuided(stepId: string) {
    this.guidedDone[stepId] = true;
  }

  /** Guided ids from `requires` that were NOT performed on this pass. */
  missingGuided(requires: readonly string[]): string[] {
    return requires.filter((id) => !this.guidedDone[id]);
  }

  /**
   * Help → Replay: reset the persisted flags and re-show the requested tour
   * (default keeps the W1 coach behavior). The demo tour also replays on
   * demand — it is teaching material, not a nag.
   */
  replay(script: TourScript = 'coach') {
    try {
      window.localStorage.removeItem(STORAGE_KEY);
      window.localStorage.removeItem(OUTCOME_KEY);
    } catch {
      /* ignore */
    }
    this.seen = false;
    this.outcome = null;
    this.startScript(script);
    if (script === 'demo') this.markDemoTourStarted();
  }

  /** Guided demo tour entry (Home demo card / Help / scenario browser). */
  startDemoTour() {
    this.markDemoTourStarted();
    this.startScript('demo');
  }

  private markDemoTourStarted() {
    this.demoTourSeen = true;
    try {
      window.localStorage.setItem(DEMO_TOUR_KEY, '1');
    } catch {
      /* ignore */
    }
  }

  /** Anchor navigation: every step except the first can go Back. */
  back() {
    if (this.step > 0) this.step -= 1;
  }

  next() {
    if (this.step < this.total - 1) {
      this.step += 1;
    } else {
      this.complete();
    }
  }

  /** Finished through the last step: recorded as completed. */
  complete() {
    this.open = false;
    this.seen = true;
    this.outcome = 'completed';
    persistSeen();
    persistOutcome('completed');
  }

  /** Skip button: dismiss + persist as skipped (W1 semantics kept). */
  finish() {
    this.open = false;
    this.seen = true;
    this.outcome = 'skipped';
    persistSeen();
    persistOutcome('skipped');
  }
}

export const onboarding = new OnboardingStore();
