// W6 regression: anchored product tour (MOCK_LIVE_ONBOARDING_MANDATE §4-§5, §7;
// lead review 024 — truthful guided flow).
//   - W1 coach behavior preserved: first workspace open shows it, skip
//     persists ('1' + outcome 'skipped'), Help replay re-shows it;
//   - every step can go Back; progress is visible;
//   - the guided demo script performs REAL actions (seed demo, pick a row,
//     edit+save through the editor path, fix the intentional review error via
//     the real inline fix, run the real demo-build control);
//   - pressing Next through everything NEVER claims "translation ready": the
//     final step reads the action ledger and shows honest outstanding copy.
import { beforeEach, describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { onboarding, COACH_TOTAL, DEMO_TOTAL } from '../src/lib/stores/onboarding.svelte';
import { COACH_STEPS, DEMO_STEPS } from '../src/lib/onboarding/steps';
import { project } from '../src/lib/stores/project.svelte';
import { review } from '../src/lib/stores/review.svelte';
import { demoProject } from '../src/lib/demo/demoProject.svelte';
import { click, cleanupMounted, exists, goto, mountCmp, q } from './helpers';

function gotoAndMount(hash: string) {
  goto(hash);
  mountCmp(App);
  flushSync();
}

describe('tour fixtures', () => {
  it('step counts in the store match the step definitions', () => {
    expect(COACH_STEPS).toHaveLength(COACH_TOTAL);
    expect(DEMO_STEPS).toHaveLength(DEMO_TOTAL);
    for (const s of DEMO_STEPS) {
      expect(s.route).toBeTruthy();
    }
  });
});

describe('coach (W1 behavior preserved, now anchored)', () => {
  beforeEach(() => {
    cleanupMounted();
    onboarding.open = false;
  });

  it('first workspace open shows the coach with progress and disabled Back', () => {
    gotoAndMount('#/workspace');
    expect(exists('onboarding.overlay')).toBe(true);
    expect(exists('onboarding.step-of')).toBe(true);
    const back = q('onboarding.back') as HTMLButtonElement;
    expect(back.disabled).toBe(true);
    expect(q('onboarding.step-of').textContent).toContain('1 из 4');
  });

  it('Back navigates steps backwards', () => {
    gotoAndMount('#/workspace');
    click('onboarding.next');
    expect(onboarding.step).toBe(1);
    click('onboarding.back');
    expect(onboarding.step).toBe(0);
  });

  it('skip persists seen + outcome; remount stays hidden; Help replay re-shows', () => {
    gotoAndMount('#/workspace');
    expect(exists('onboarding.overlay')).toBe(true);
    click('onboarding.skip');
    expect(exists('onboarding.overlay')).toBe(false);
    expect(window.localStorage.getItem('rimloc.onboarding.seen.v1')).toBe('1');
    expect(onboarding.outcome).toBe('skipped');
    expect(window.localStorage.getItem('rimloc.onboarding.outcome.v1')).toBe('skipped');

    cleanupMounted();
    gotoAndMount('#/workspace');
    expect(exists('onboarding.overlay')).toBe(false);

    goto('#/help');
    expect(exists('help.replay.action')).toBe(true);
    click('help.replay.action');
    expect(window.location.hash).toBe('#/workspace');
    expect(onboarding.open).toBe(true);
    expect(window.localStorage.getItem('rimloc.onboarding.outcome.v1')).toBeNull();
  });

  it('finishing through the last step records completed', () => {
    gotoAndMount('#/workspace');
    for (let i = 0; i < COACH_TOTAL; i++) click('onboarding.next');
    expect(exists('onboarding.overlay')).toBe(false);
    expect(onboarding.outcome).toBe('completed');
    expect(window.localStorage.getItem('rimloc.onboarding.outcome.v1')).toBe('completed');
  });
});

describe('guided demo tour (real actions, ledger-gated ending)', () => {
  beforeEach(() => {
    cleanupMounted();
    onboarding.open = false;
  });

  it('full action path mutates real state and earns the success ending', async () => {
    gotoAndMount('#/home');
    onboarding.replay('demo');
    flushSync();
    expect(onboarding.open).toBe(true);
    expect(onboarding.script).toBe('demo');

    // d1: open the demo project — seeds and navigates.
    click('tour.action.d1');
    expect(window.location.hash).toBe('#/workspace');
    expect(project.projectName).toBe('RimLoc Demo');
    expect(onboarding.step).toBe(1);

    // d2: select the TODO row for real.
    click('tour.action.d2');
    expect(project.selectedId).toBe('keyed-04');
    expect(onboarding.step).toBe(2);

    // d3: a real edit+save through the editor path (draft → commit).
    click('tour.action.d3');
    await new Promise((r) => setTimeout(r, 500));
    const edited = project.byId('keyed-04');
    expect(edited?.target).toBe('Изгнать {PAWN_nameDef}?');
    expect(edited?.status).toBe('translated');
    expect(onboarding.step).toBe(3);

    // d4: plain next; Back still works mid-script.
    click('onboarding.next');
    expect(onboarding.step).toBe(4);

    // d5: the intentional error opens the review queue.
    click('tour.action.d5');
    expect(window.location.hash).toBe('#/review');
    expect(onboarding.step).toBe(5);

    // d6: the real inline fix closes the issue and mutates the text.
    const beforeFix = review.issues.length;
    click('tour.action.d6');
    expect(review.resolutions['keyed-06:placeholder_mismatch']).toBe('fixed');
    expect(project.byId('keyed-06')?.target).toContain('{ENEMYPAWN_nameFull}');
    expect(review.issues.length).toBeLessThanOrEqual(beforeFix);
    expect(onboarding.step).toBe(6);

    // d7 → build; d8: presses the REAL demo-build control.
    click('tour.action.d7');
    expect(window.location.hash).toBe('#/build');
    expect(onboarding.step).toBe(7);
    expect(exists('build.run')).toBe(true);
    click('tour.action.d8');
    // The real mock build takes ~1.4 s to reach its done state.
    await new Promise((r) => setTimeout(r, 1800));
    expect(exists('build.done')).toBe(true);
    expect(onboarding.step).toBe(8);

    // d9: the work happened — the success copy is earned.
    click('onboarding.next');
    expect(exists('onboarding.overlay')).toBe(false);
    expect(onboarding.outcome).toBe('completed');
    expect(onboarding.missingGuided(['d3', 'd6', 'd8'])).toEqual([]);
  });

  it('Next-through without any work shows honest outstanding copy, never "ready"', async () => {
    gotoAndMount('#/home');
    // Start from a truly clean slate: earlier tests in this file fix the
    // intentional error and seed the demo — undo all of it.
    demoProject.leave();
    project.reset();
    review.resolutions = {};
    onboarding.replay('demo');
    flushSync();
    // 8 plain Nexts land on the final step with an empty ledger…
    for (let i = 0; i < DEMO_TOTAL - 1; i++) click('onboarding.next');
    expect(onboarding.step).toBe(DEMO_TOTAL - 1);
    // …and the honest variant shows instead of a success claim.
    expect(exists('onboarding.honest-note')).toBe(true);
    const title = document.getElementById('onboarding-title')?.textContent ?? '';
    expect(title).toContain('не всё сделано');
    expect(title).not.toContain('перевод готов');
    // The underlying claims are false: no edit, no fix, no build run.
    expect(project.byId('keyed-04')?.target).toBe('');
    expect(review.resolutions['keyed-06:placeholder_mismatch']).toBeUndefined();
    expect(exists('build.done')).toBe(false);

    click('onboarding.next');
    expect(onboarding.outcome).toBe('completed'); // the TOUR completed…
    // …but the honest pass never seeded or mutated the demo.
    expect(project.projectName).toBe('TestMod');
  });

  it('skipping the demo tour records skipped without touching the demo', () => {
    demoProject.leave();
    gotoAndMount('#/home');
    onboarding.replay('demo');
    flushSync();
    click('onboarding.skip');
    expect(exists('onboarding.overlay')).toBe(false);
    expect(onboarding.outcome).toBe('skipped');
    expect(project.projectName).toBe('TestMod');
  });
});
