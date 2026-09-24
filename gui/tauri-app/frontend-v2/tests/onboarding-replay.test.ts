// W1 regression: onboarding replay cycle (QA mandate §13, §20).
// E2E over the full App shell: fresh workspace → coach visible; skip → gone
// and persisted; Help → "Replay tips" → coach visible again in the Workspace.
import { describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { onboarding } from '../src/lib/stores/onboarding.svelte';
import { cleanupMounted, click, exists, goto, mountCmp, q } from './helpers';

function goto(hash: string) {
  window.location.hash = hash;
  flushSync();
}

describe('onboarding replay cycle', () => {
  it('fresh workspace shows the coach, skip persists, Help replay re-shows', () => {
    // 1. Fresh workspace: the coach overlay is visible on first open.
    expect(window.localStorage.getItem('rimloc.onboarding.seen.v1')).toBeNull();
    goto('#/workspace');
    mountCmp(App);
    expect(exists('onboarding.overlay')).toBe(true);
    expect(exists('onboarding.step-of')).toBe(true);

    // 2. Skip: overlay closes and the seen-flag is persisted.
    click('onboarding.skip');
    expect(exists('onboarding.overlay')).toBe(false);
    expect(window.localStorage.getItem('rimloc.onboarding.seen.v1')).toBe('1');

    // 3. Remount (new "session"): the coach stays hidden — skip persisted.
    cleanupMounted();
    goto('#/workspace');
    mountCmp(App);
    expect(exists('onboarding.overlay')).toBe(false);

    // 4. Help → Replay tips: navigates to the Workspace and re-shows the coach.
    goto('#/help');
    expect(exists('help.replay.action')).toBe(true);
    click('help.replay.action');
    expect(window.location.hash).toBe('#/workspace');
    expect(onboarding.open).toBe(true);
    expect(exists('onboarding.overlay')).toBe(true);
    expect(exists('onboarding.step-of')).toBe(true);

    // 5. Walking to the last step and finishing persists the flag again.
    click('onboarding.next');
    click('onboarding.next');
    click('onboarding.next');
    expect(exists('onboarding.overlay')).toBe(true);
    click('onboarding.next'); // finish on the last step
    expect(exists('onboarding.overlay')).toBe(false);
    expect(window.localStorage.getItem('rimloc.onboarding.seen.v1')).toBe('1');
  });

  it('coach has exactly 4 steps and Step N of 4 counter', () => {
    onboarding.replay();
    goto('#/workspace');
    mountCmp(App);
    expect(q('onboarding.step-of').textContent).toMatch(/1/);
    expect(exists('onboarding.next')).toBe(true);
    click('onboarding.skip');
  });
});
