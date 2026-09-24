// Tour step definitions (W6 anchored product tour,
// MOCK_LIVE_ONBOARDING_MANDATE §4/§7). Data + real store actions only:
// the tour guides the user to REAL controls and performs REAL mock actions —
// nothing is faked as "implemented" behind the scenes.
//
// Anchors are CSS selectors over existing data-testids. A missing anchor is
// never fatal: the tour falls back to a centered card (honest degradation in
// narrow layouts and jsdom, where layout boxes are zero-sized).
import { project } from '../stores/project.svelte';
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
      run: () => project.setStatus('keyed-04', 'translated')
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
    anchor: '[data-testid="tabs.review"]',
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
    anchor: '[data-testid="review.categories"]'
  },
  {
    id: 'd7',
    titleKey: 'tour.d7.title',
    textKey: 'tour.d7.text',
    route: 'review',
    anchor: '[data-testid="workspace.cta-review"], [data-testid="workspace.cta-build"]',
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
    anchor: '[data-testid="build.run"]'
  },
  {
    id: 'd9',
    titleKey: 'tour.d9.title',
    textKey: 'tour.d9.text',
    route: 'build',
    anchor: '[data-testid="build.done"]',
    final: true
  }
];

export function stepsFor(script: TourScript): TourStep[] {
  return script === 'demo' ? DEMO_STEPS : COACH_STEPS;
}
