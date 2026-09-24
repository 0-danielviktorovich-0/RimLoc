// W6 regression: anchored product tour (MOCK_LIVE_ONBOARDING_MANDATE §4-§5, §7;
// lead 024 + 027 — truthful guided flow with completion evidence).
//   - W1 coach behavior preserved: first workspace open shows it, skip
//     persists ('1' + outcome 'skipped'), Help replay re-shows it;
//   - every step can go Back; progress is visible;
//   - the guided demo script performs REAL actions (seed demo, pick a row,
//     edit+save through the editor path, fix the intentional review error via
//     the real inline fix, run the REAL shared mock build engine);
//   - a guided action counts ONLY when it verifiably completed: pending shows
//     as pending, reset/cancel yields no completion, stale completions after
//     Skip/replay are discarded, and skipping the demo open step guards every
//     later mutation;
//   - Next-through without work shows honest outstanding copy, never
//     "translation ready".
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { onboarding, COACH_TOTAL, DEMO_TOTAL } from '../src/lib/stores/onboarding.svelte';
import { COACH_STEPS, DEMO_STEPS } from '../src/lib/onboarding/steps';
import { project } from '../src/lib/stores/project.svelte';
import { review } from '../src/lib/stores/review.svelte';
import { buildState } from '../src/lib/mock/buildState.svelte';
import { languages } from '../src/lib/languages/store.svelte';
import { demoProject } from '../src/lib/demo/demoProject.svelte';
import { click, cleanupMounted, exists, goto, mountCmp, q } from './helpers';

function gotoAndMount(hash: string) {
  goto(hash);
  mountCmp(App);
  flushSync();
}

/** Flush the microtask chain of an async guided action (mark + advance)
 * and settle the DOM render that follows. */
async function settle() {
  for (let i = 0; i < 6; i++) await Promise.resolve();
  flushSync();
}

/** Click a guided action and settle its synchronous continuation. */
async function clickA(id: string) {
  click(id);
  await settle();
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

describe('guided demo tour (completion evidence, not click-start)', () => {
  beforeEach(() => {
    cleanupMounted();
    onboarding.open = false;
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('full action path mutates real state and earns the success ending', async () => {
    gotoAndMount('#/home');
    onboarding.replay('demo');
    flushSync();
    expect(onboarding.open).toBe(true);
    expect(onboarding.script).toBe('demo');

    // d1: open the demo project — seeds and navigates.
    click('tour.action.d1');
    await settle();
    expect(window.location.hash).toBe('#/workspace');
    expect(project.projectName).toBe('RimLoc Demo');
    expect(onboarding.step).toBe(1);

    // d2: select the TODO row for real.
    click('tour.action.d2');
    await settle();
    expect(project.selectedId).toBe('keyed-04');
    expect(onboarding.step).toBe(2);

    // d3: the real edit+save — while pending nothing is recorded or advanced.
    click('tour.action.d3');
    expect(project.saveStates['keyed-04']).toBe('saving');
    expect(onboarding.guidedDone['d3']).toBeUndefined();
    expect(onboarding.step).toBe(2);
    await vi.advanceTimersByTimeAsync(500); // the save lands
    const edited = project.byId('keyed-04');
    expect(edited?.target).toBe('Изгнать {PAWN_nameDef}?');
    expect(edited?.status).toBe('translated');
    expect(onboarding.step).toBe(3);

    // d4: plain next; Back still works mid-script.
    click('onboarding.next');
    expect(onboarding.step).toBe(4);

    // d5: the intentional error opens the review queue.
    click('tour.action.d5');
    await settle();
    expect(window.location.hash).toBe('#/review');
    expect(onboarding.step).toBe(5);

    // d6: the real inline fix closes the issue and mutates the text.
    const beforeFix = review.issues.length;
    click('tour.action.d6');
    await settle();
    expect(review.resolutions['keyed-06:placeholder_mismatch']).toBe('fixed');
    expect(project.byId('keyed-06')?.target).toContain('{ENEMYPAWN_nameFull}');
    expect(review.issues.length).toBeLessThanOrEqual(beforeFix);
    expect(onboarding.step).toBe(6);

    // d7 → build; d8: starts the REAL shared mock build engine.
    click('tour.action.d7');
    await settle();
    expect(window.location.hash).toBe('#/build');
    expect(onboarding.step).toBe(7);
    expect(exists('build.run')).toBe(true);
    click('tour.action.d8');
    // Pending is visible: build phase, no completion recorded, no advance.
    expect(buildState.phase).toBe('building');
    expect(onboarding.guidedDone['d8']).toBeUndefined();
    expect(onboarding.step).toBe(7);
    await vi.advanceTimersByTimeAsync(1500); // the build really completes
    expect(buildState.phase).toBe('done');
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
    demoProject.leave(); // start clean: Next-through must not seed anything
    project.reset();
    review.resetSession();
    buildState.reset(); // no leftover "done" from an earlier pass
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

describe('pending-save isolation (lead 034)', () => {
  beforeEach(() => {
    cleanupMounted();
    onboarding.open = false;
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('a newer draft supersedes an older staged commit', async () => {
    project.reset();
    project.setDraft('keyed-04', 'черновик A');
    const first = project.flushDraft('keyed-04'); // epoch 1, fires at 350ms
    project.setDraft('keyed-04', 'черновик B'); // epoch 2 + autosave at 800ms
    await vi.advanceTimersByTimeAsync(400); // older commit fires: stale, no write
    expect(project.byId('keyed-04')?.target).toBe('');
    // Newer draft: autosave at 800ms + its own save latency at ~1150ms.
    await vi.advanceTimersByTimeAsync(1000);
    expect(project.byId('keyed-04')?.target).toBe('черновик B');
    await expect(first).resolves.toBe(false);
  });

  it('a locale switch cancels the staged commit so text stays in its locale', async () => {
    project.reset();
    languages.initFromPristine();
    project.setDraft('keyed-04', 'ТестX — только для ru');
    const pending = project.flushDraft('keyed-04');
    languages.setActive('uk'); // cancels pending commits BEFORE substituting data
    await expect(pending).resolves.toBe(false);
    await vi.advanceTimersByTimeAsync(1000); // cancelled timer must not fire
    // The ru capture-back holds the text; the uk dataset never receives it.
    expect(project.byId('keyed-04')?.target).not.toContain('ТестX');
    expect(languages.targets['ru']?.entries['keyed-04']?.target).toBe('ТестX — только для ru');
    expect(languages.targets['uk']?.entries['keyed-04']?.target).not.toContain('ТестX');
  });

  it('leaving the demo mid-build aborts the build: no inherited done state', async () => {
    gotoAndMount('#/home');
    demoProject.seed();
    onboarding.replay('demo');
    flushSync();
    // Jump to the build step through real actions.
    await clickA('tour.action.d1');
    await clickA('tour.action.d2');
    await clickA('tour.action.d3');
    await vi.advanceTimersByTimeAsync(600);
    click('onboarding.next'); // d4
    await clickA('tour.action.d5');
    await clickA('tour.action.d6');
    await clickA('tour.action.d7');
    // Start the build, then LEAVE the demo while it is in flight.
    await clickA('tour.action.d8');
    expect(buildState.phase).toBe('building');
    demoProject.leave();
    expect(buildState.phase).toBe('idle');
    await vi.advanceTimersByTimeAsync(3000); // the in-flight run resolves stale
    // Ordinary project restored; nothing recorded, no success, no advance.
    expect(project.projectName).toBe('TestMod');
    expect(project.byId('keyed-04')?.target).toBe('');
    expect(onboarding.guidedDone['d8']).toBeUndefined();
    expect(onboarding.step).toBe(7);
  });
});

describe('build completion evidence (lead 027)', () => {
  beforeEach(() => {
    cleanupMounted();
    onboarding.open = false;
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  async function startDemoTourAtBuild() {
    gotoAndMount('#/home');
    demoProject.seed();
    onboarding.replay('demo');
    flushSync(); // open + step 0 rendered before the first action click
    // Walk the real guided actions up to d8 (index 7): d1..d3, d5..d7 are
    // action steps (no plain Next), d4 is plain.
    await clickA('tour.action.d1');
    await clickA('tour.action.d2');
    await clickA('tour.action.d3'); // pending save — flushed below
    click('onboarding.next'); // → d4
    click('onboarding.next'); // → d5
    await clickA('tour.action.d5');
    console.log('DBG5 ' + JSON.stringify({ step: onboarding.step, hash: window.location.hash, overlay: exists('onboarding.overlay') }));
    await clickA('tour.action.d6');
    console.log('DBG6 ' + JSON.stringify({ step: onboarding.step, overlay: exists('onboarding.overlay') }));
    await clickA('tour.action.d7');
    flushSync();
    expect(onboarding.step).toBe(7);
  }

  it('right after the action the build is pending and d8 is NOT earned', async () => {
    await startDemoTourAtBuild();
    click('tour.action.d8');
    flushSync();
    expect(buildState.phase).toBe('building');
    expect(onboarding.guidedDone['d8']).toBeUndefined();
    expect(onboarding.step).toBe(7);
    await vi.advanceTimersByTimeAsync(1500);
    expect(buildState.phase).toBe('done');
    expect(onboarding.guidedDone['d8']).toBe(true);
    expect(onboarding.step).toBe(8);
  });

  it('reset mid-build yields no completion and no advance', async () => {
    await startDemoTourAtBuild();
    click('tour.action.d8');
    vi.advanceTimersByTime(500); // build in flight…
    buildState.reset(); // …then cancelled (Test again / demo reset)
    await vi.advanceTimersByTimeAsync(3000);
    expect(buildState.phase).toBe('idle');
    expect(onboarding.guidedDone['d8']).toBeUndefined();
    expect(onboarding.step).toBe(7); // stayed; user may press the action again
    // Retry after the cancel works and completes.
    click('tour.action.d8');
    await vi.advanceTimersByTimeAsync(1500);
    expect(onboarding.guidedDone['d8']).toBe(true);
    expect(onboarding.step).toBe(8);
  });

  it('Skip during a pending build discards the stale completion', async () => {
    await startDemoTourAtBuild();
    click('tour.action.d8'); // pending
    click('onboarding.skip'); // dismiss the tour mid-flight
    expect(onboarding.outcome).toBe('skipped');
    await vi.advanceTimersByTimeAsync(3000); // the build would finish NOW
    // The stale completion belongs to a dead pass: nothing marked, nothing advanced.
    expect(onboarding.guidedDone['d8']).toBeUndefined();
    expect(onboarding.open).toBe(false);
    // A NEW pass starts with an empty ledger even though the old build finished.
    demoProject.seed();
    onboarding.replay('demo');
    flushSync();
    expect(onboarding.missingGuided(['d3', 'd6', 'd8'])).toEqual(['d3', 'd6', 'd8']);
  });

  it('skipping the demo-open step guards every mutating action', async () => {
    gotoAndMount('#/home');
    project.reset();
    review.resetSession();
    onboarding.replay('demo');
    flushSync();
    // d1 skipped: plain Next past the open step.
    click('onboarding.next');
    expect(onboarding.step).toBe(1);
    // d2 cannot even select a row of the ordinary project.
    await clickA('tour.action.d2');
    expect(project.selectedId).toBeNull();
    expect(exists('onboarding.action-failed')).toBe(true);
    // Plain Next to d3; its edit refuses to mutate the ordinary TestMod.
    click('onboarding.next');
    expect(onboarding.step).toBe(2);
    click('tour.action.d3');
    await vi.advanceTimersByTimeAsync(2000);
    expect(project.byId('keyed-04')?.target).toBe(''); // ordinary TestMod untouched
    expect(onboarding.guidedDone['d3']).toBeUndefined();
    expect(onboarding.step).toBe(2); // nothing advanced, nothing recorded
  });
});
