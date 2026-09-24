// W6 regression: anchored product tour (MOCK_LIVE_ONBOARDING_MANDATE §4-§5, §7).
//   - W1 coach behavior preserved: first workspace open shows it, skip
//     persists ('1' + outcome 'skipped'), Help replay re-shows it;
//   - every step can go Back; progress is visible;
//   - the guided demo script performs REAL actions (seed demo, select row,
//     mark translated, review, build) and records 'completed' at the end.
import { beforeEach, describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { onboarding, COACH_TOTAL, DEMO_TOTAL } from '../src/lib/stores/onboarding.svelte';
import { COACH_STEPS, DEMO_STEPS } from '../src/lib/onboarding/steps';
import { project } from '../src/lib/stores/project.svelte';
import { click, cleanupMounted, exists, goto, mountCmp, q } from './helpers';
import { demoProject } from '../src/lib/demo/demoProject.svelte';

function gotoAndMount(hash: string) {
  goto(hash);
  mountCmp(App);
  flushSync();
}

describe('tour fixtures', () => {
  it('step counts in the store match the step definitions', () => {
    expect(COACH_STEPS).toHaveLength(COACH_TOTAL);
    expect(DEMO_STEPS).toHaveLength(DEMO_TOTAL);
    // Every demo step has i18n keys and a route; coach stays in workspace.
    for (const s of DEMO_STEPS) {
      expect(s.route).toBeTruthy();
    }
  });
});

describe('coach (W1 behavior preserved, now anchored)', () => {
  beforeEach(() => {
    cleanupMounted();
  });

  it('first workspace open shows the coach with progress and disabled Back', () => {
    gotoAndMount('#/workspace');
    expect(exists('onboarding.overlay')).toBe(true);
    expect(exists('onboarding.step-of')).toBe(true);
    const back = q('onboarding.back') as HTMLButtonElement;
    expect(back.disabled).toBe(true);
    // Coach total is the W1 4 steps.
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
    // Outcome cleared by replay: the tour is being taken again.
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

describe('guided demo tour (real actions, no carousel)', () => {
  beforeEach(() => {
    cleanupMounted();
  });

  it('walks Open Demo → row → edit → context → review → validate → build → done', () => {
    gotoAndMount('#/home');
    onboarding.replay('demo');
    flushSync();
    expect(onboarding.open).toBe(true);
    expect(onboarding.script).toBe('demo');
    expect(onboarding.total).toBe(DEMO_TOTAL);

    // d1: open the demo project — seeds and navigates.
    click('tour.action.d1');
    expect(window.location.hash).toBe('#/workspace');
    expect(project.projectName).toBe('RimLoc Demo');
    expect(onboarding.step).toBe(1);

    // d2: select the TODO row for real.
    click('tour.action.d2');
    expect(project.selectedId).toBe('keyed-04');
    expect(onboarding.step).toBe(2);

    // d3: mark it translated (the real store write).
    click('tour.action.d3');
    expect(project.byId('keyed-04')?.status).toBe('translated');
    expect(onboarding.step).toBe(3);

    // d4: context explanation — plain next; Back still works mid-script.
    click('onboarding.back');
    expect(onboarding.step).toBe(2);
    click('tour.action.d3');
    click('onboarding.next');
    expect(onboarding.step).toBe(4);

    // d5: open review (the intentional placeholder error lives there).
    click('tour.action.d5');
    expect(window.location.hash).toBe('#/review');
    expect(onboarding.step).toBe(5);

    // d6: queue explanation, plain next.
    click('onboarding.next');
    expect(onboarding.step).toBe(6);

    // d7: validate → go to build.
    click('tour.action.d7');
    expect(window.location.hash).toBe('#/build');
    expect(onboarding.step).toBe(7);

    // d8: build explanation; plain next reaches the final step…
    click('onboarding.next');
    expect(onboarding.step).toBe(8);
    // …and finishing the last step records completion.
    click('onboarding.next');
    expect(exists('onboarding.overlay')).toBe(false);
    expect(onboarding.outcome).toBe('completed');
    expect(window.localStorage.getItem('rimloc.tour.demo.v1')).toBe('1');
  });

  it('skipping the demo tour records skipped without touching the demo', () => {
    demoProject.leave(); // start clean: skip must not seed anything
    gotoAndMount('#/home');
    onboarding.replay('demo');
    flushSync();
    click('onboarding.skip');
    expect(exists('onboarding.overlay')).toBe(false);
    expect(onboarding.outcome).toBe('skipped');
    expect(project.projectName).toBe('TestMod');
  });
});
