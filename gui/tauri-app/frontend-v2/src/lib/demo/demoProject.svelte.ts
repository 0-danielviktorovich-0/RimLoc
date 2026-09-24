// Demo project (W6, MOCK_LIVE_ONBOARDING_MANDATE §6/§8): a bundled,
// RimLoc-owned SYNTHETIC fixture that lets onboarding happen without
// Steam/RimWorld/Workshop. It covers Keyed, DefInjected, TKey/multi-context,
// placeholders, glossary conflicts, an intentional validation problem,
// review/sourceChanged entries and multiple targets — the existing pristine
// mock inventory is exactly that dataset, so the demo seeds it under an
// explicit demo identity.
//
// Safety contract:
//   - In-memory ONLY: seeding mutates reactive stores, never the filesystem;
//     the demo cannot touch a real source/output directory or another
//     project's data (there are none — but the boundary stays explicit).
//   - Deterministic reset: seed() always produces the same state from the
//     pristine fixtures (same entries, same targets, same revisions).
//   - Clearly marked: the demo carries its own name and the isDemo flag so
//     it can never be confused with a real recent project.
import { project } from '../stores/project.svelte';
import { languages } from '../languages/store.svelte';
import { diagnostics } from '../stores/diagnostics.svelte';
import { review } from '../stores/review.svelte';
import { buildState } from '../mock/buildState.svelte';

/** Synthetic identity of the bundled demo project. */
export const DEMO_PROJECT_NAME = 'RimLoc Demo';

class DemoProjectStore {
  /** True while the workspace is showing the bundled demo. */
  active = $state(false);

  /**
   * Seed the demo project from the pristine fixtures. Deterministic: every
   * call yields the identical state (entries, targets, revisions, selection
   * cleared), so screenshots and E2E runs are reproducible.
   */
  seed() {
    project.reset();
    languages.initFromPristine();
    diagnostics.reset();
    // Demo isolation (027): a fresh pass also starts with clean review
    // session state and no half-finished mock build.
    review.resetSession();
    buildState.reset();
    project.projectName = DEMO_PROJECT_NAME;
    project.isDemo = true;
    this.active = true;
  }

  /** The demo's own reset: re-seed to the exact starting state. */
  resetDemo() {
    this.seed();
  }

  /** Leave the demo: back to the plain pristine (non-demo) project state. */
  leave() {
    project.reset();
    this.active = false;
  }
}

export const demoProject = new DemoProjectStore();
