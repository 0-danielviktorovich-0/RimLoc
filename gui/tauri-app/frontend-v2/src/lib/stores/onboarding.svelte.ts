// Contextual onboarding state (QA mandate §13, §20): a skippable coach
// overlay shown on the first Workspace open and replayable from Help.
// `seen` is persisted in localStorage so a machine sees it once; the Help →
// "Replay tips" action clears the persisted flag and reopens the overlay, so
// the replay cycle (fresh → skip → replay → visible again) is verifiable.
const STORAGE_KEY = 'rimloc.onboarding.seen.v1';

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

class OnboardingStore {
  /** Overlay visibility. */
  open = $state(false);
  /** Current coach step, 0-based. */
  step = $state(0);
  /** True once the user has seen (or skipped) the coach. Persisted. */
  seen = $state(true);

  constructor() {
    this.seen = readSeen();
  }

  /** Called when the Workspace shell mounts: first open starts the coach. */
  startIfFirstRun() {
    if (!this.seen) {
      this.step = 0;
      this.open = true;
    }
  }

  /** Help → Replay tips: reset the persisted flag and reopen the overlay. */
  replay() {
    try {
      window.localStorage.removeItem(STORAGE_KEY);
    } catch {
      /* ignore */
    }
    this.seen = false;
    this.step = 0;
    this.open = true;
  }

  next(steps: number) {
    if (this.step < steps - 1) {
      this.step += 1;
    } else {
      this.finish();
    }
  }

  /** Skip button and finish share one path: dismiss + persist. */
  finish() {
    this.open = false;
    this.seen = true;
    persistSeen();
  }
}

export const onboarding = new OnboardingStore();
