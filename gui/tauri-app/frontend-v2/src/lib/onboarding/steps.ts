// Tour step definitions (W6 anchored product tour,
// MOCK_LIVE_ONBOARDING_MANDATE §4/§7). Data + real store actions only:
// the tour guides the user to REAL controls and performs REAL mock actions —
// nothing is faked as "implemented" behind the scenes.
//
// Guided honesty (lead review 024): the final demo step reflects what actually
// happened. `requires` lists guided action ids that must have been performed
// on this pass; when any is missing the step shows its honest fallback copy
// (outstanding work) instead of a fabricated "translation ready" claim.
//
// Anchors are CSS selectors over existing data-testids. A missing anchor is
// never fatal: the tour falls back to a centered card (honest degradation in
// narrow layouts and jsdom, where layout boxes are zero-sized).
import { project } from '../stores/project.svelte';
import { review } from '../stores/review.svelte';
import { router, type RouteId } from '../router.svelte';
import { demoProject } from '../demo/demoProject.svelte';
import type { TourScript } from '../stores/onboarding.svelte';

export interface TourAction {
  /** i18n key of the action button label. */
  labelKey: string;
  testid: string;
  run: () => void;
}

export interface TourStep {
  /** i18n keys: tour.<id>.title / tour.<id>.text unless reuseKeys is set. */
  id: string;
  titleKey: string;
  textKey: string;
  /** Route the step lives on; the tour navigates there when it enters. */
  route?: RouteId;
  /** CSS selector(s) of the anchored control (first match wins). */
  anchor?: string;
  /** Guided action — clicking it performs the real action and advances. */
  action?: TourAction;
  /** Last step shows the finish label instead of Next. */
  final?: boolean;
  /**
   * Guided action ids that the success copy requires. When any is missing,
   * honestTitleKey/honestTextKey replace the success copy.
   */
  requires?: string[];
  honestTitleKey?: string;
  honestTextKey?: string;
}

export const COACH_STEPS: TourStep[] = [
  {
    id: 's1',
    titleKey: 'onboarding.s1.title',
    textKey: 'onboarding.s1.text',
    route: 'workspace',
    anchor: '[data-testid="workspace.table"]'
  },
  {
    id: 's2',
    titleKey: 'onboarding.s2.title',
    textKey: 'onboarding.s2.text',
    route: 'workspace',
    anchor: '[data-testid="workspace.row.keyed-01"]'
  },
  {
    id: 's3',
    titleKey: 'onboarding.s3.title',
    textKey: 'onboarding.s3.text',
    route: 'workspace',
    anchor: '[data-testid="workspace.search"]'
  },
  {
    id: 's4',
    titleKey: 'onboarding.s4.title',
    textKey: 'onboarding.s4.text',
    route: 'workspace',
    anchor: '[data-testid="workspace.cta-review"], [data-testid="workspace.cta-build"]',
    final: true
  }
];

/** The corrected demo target for the intentional placeholder error: keeps
 * every source placeholder, so the derived mismatch genuinely clears. */
const KEYED_06_FIX = '{ENEMYPAWN_nameFull} из {ENEMYFACTION_name} — на вас напали!';

/** Real review fix of the intentional error: open → edit → save, exactly the
 * buttons a user would press (writes through the shared project store). */
function fixKeyed06(): void {
  const issue = review.issues.find((i) => i.id === 'keyed-06:placeholder_mismatch');
  if (!issue) return; // already fixed on this pass — idempotent
  review.open(issue);
  review.startFix(issue);
  review.fixText[issue.id] = KEYED_06_FIX;
  review.saveFix(issue);
}

export const DEMO_STEPS: TourStep[] = [
  {
    id: 'd1',
    titleKey: 'tour.d1.title',
    textKey: 'tour.d1.text',
    route: 'home',
    anchor: '[data-testid="home.entry-new"]',
    action: {
      labelKey: 'tour.d1.action',
      testid: 'tour.action.d1',
      run: () => {
        demoProject.seed();
        router.navigate('workspace');
      }
    }
  },
  {
    id: 'd2',
    titleKey: 'tour.d2.title',
    textKey: 'tour.d2.text',
    route: 'workspace',
    anchor: '[data-testid="workspace.row.keyed-04"]',
    action: {
      labelKey: 'tour.d2.action',
      testid: 'tour.action.d2',
      run: () => project.select('keyed-04')
    }
  },
  {
    id: 'd3',
    titleKey: 'tour.d3.title',
    textKey: 'tour.d3.text',
    route: 'workspace',
    anchor: '[data-testid="workspace.context"]',
    action: {
      labelKey: 'tour.d3.action',
      testid: 'tour.action.d3',
      run: () => {
        // A real isolated demo edit: stage the draft, then save it through
        // the same debounced commit path the editor uses (text mutates,
        // status TODO → translated, origin → human).
        project.setDraft('keyed-04', 'Изгнать {PAWN_nameDef}?');
        project.flushDraft('keyed-04');
      }
    }
  },
  {
    id: 'd4',
    titleKey: 'tour.d4.title',
    textKey: 'tour.d4.text',
    route: 'workspace',
    anchor: '[data-testid="workspace.context"]'
  },
  {
    id: 'd5',
    titleKey: 'tour.d5.title',
    textKey: 'tour.d5.text',
    route: 'workspace',
    anchor: '[data-testid="workspace.row.keyed-06"]',
    action: {
      labelKey: 'tour.d5.action',
      testid: 'tour.action.d5',
      run: () => router.navigate('review')
    }
  },
  {
    id: 'd6',
    titleKey: 'tour.d6.title',
    textKey: 'tour.d6.text',
    route: 'review',
    anchor: '[data-testid="review.categories"]',
    action: {
      labelKey: 'tour.d6.action',
      testid: 'tour.action.d6',
      run: fixKeyed06
    }
  },
  {
    id: 'd7',
    titleKey: 'tour.d7.title',
    textKey: 'tour.d7.text',
    route: 'review',
    anchor: '[data-testid="review.categories"]',
    action: {
      labelKey: 'tour.d7.action',
      testid: 'tour.action.d7',
      run: () => router.navigate('build')
    }
  },
  {
    id: 'd8',
    titleKey: 'tour.d8.title',
    textKey: 'tour.d8.text',
    route: 'build',
    anchor: '[data-testid="build.run"]',
    action: {
      labelKey: 'tour.d8.action',
      testid: 'tour.action.d8',
      // Press the REAL existing demo-build control — the same button a user
      // clicks. No second build engine lives in the tour.
      run: () => {
        (document.querySelector('[data-testid="build.run"]') as HTMLButtonElement | null)?.click();
      }
    }
  },
  {
    id: 'd9',
    titleKey: 'tour.d9.title',
    textKey: 'tour.d9.text',
    route: 'build',
    anchor: '[data-testid="build.done"]',
    final: true,
    // Success copy only when the work actually happened this pass.
    requires: ['d3', 'd6', 'd8'],
    honestTitleKey: 'tour.d9.honest.title',
    honestTextKey: 'tour.d9.honest.text'
  }
];

export function stepsFor(script: TourScript): TourStep[] {
  return script === 'demo' ? DEMO_STEPS : COACH_STEPS;
}
