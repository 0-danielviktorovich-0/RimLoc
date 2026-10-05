// Night audit §7 follow-up (worker W-B): honesty of the Review overview and
// the Settings example data.
//   - fixture mode: the mandate example counters stay, the demo badge covers
//     them (header), and NO partial-overview note appears;
//   - contract mode: counters are computed from the loaded snapshot
//     (pending-review, validation issues), while source-change tracking and
//     the glossary check have no dimension in the v1 snapshot — they render
//     as an explicit "—" with a visible partial note, never a faked zero;
//   - Settings: the static install list carries a permanent example-data note
//     and the logs hint names the real app-data log path.
import { beforeEach, describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { review } from '../src/lib/stores/review.svelte';
import { buildState } from '../src/lib/mock/buildState.svelte';
import { cleanupMounted, exists, goto, mountCmp, q } from './helpers';

function text(testid: string): string {
  return q(testid).textContent ?? '';
}

beforeEach(() => {
  cleanupMounted();
  project.reset();
  review.resetSession();
  buildState.reset();
});

describe('review overview honesty', () => {
  it('fixture mode keeps the mandate example counters with no partial note', async () => {
    await project.createContractProject('/mods/Demo').then(() => project.reset()); // sanity: contract path exists
    goto('#/review');
    mountCmp(App);
    flushSync();
    expect(exists('workspace.review')).toBe(true);
    expect(text('review.overview.needsReview')).toContain('173');
    expect(text('review.overview.glossaryConflicts')).toContain('11');
    expect(exists('review.overview.partial')).toBe(false);
  });

  it('contract mode computes counters from the snapshot and dashes the rest', async () => {
    const ok = await project.createContractProject('/mods/Demo');
    expect(ok).toBe(true);
    expect(project.source).toBe('contract');
    goto('#/review');
    mountCmp(App);
    flushSync();
    expect(exists('workspace.review')).toBe(true);
    // Mock corpus: 0 entries in needs_review, 1 with validation issues —
    // the overview must show THESE numbers, not 173/36.
    expect(text('review.overview.needsReview')).toContain('0');
    expect(text('review.overview.errors')).toContain('1');
    expect(text('review.overview.needsReview')).not.toContain('173');
    // No dimension in the v1 snapshot → explicit dash + visible partial note.
    expect(exists('review.overview.sourceChanged.unavailable')).toBe(true);
    expect(exists('review.overview.glossaryConflicts.unavailable')).toBe(true);
    expect(text('review.overview.sourceChanged')).not.toContain('24');
    expect(text('review.overview.partial')).toBeTruthy();
  });
});

describe('settings honesty', () => {
  it('the install list carries a permanent example-data note', () => {
    goto('#/settings');
    mountCmp(App);
    flushSync();
    q('settings.nav.rimworld').click();
    flushSync();
    expect(exists('settings.rimworld.exampleNote')).toBe(true);
  });

  it('the logs hint names the real app-data log path, not ~/RimLoc/logs', () => {
    goto('#/settings');
    mountCmp(App);
    flushSync();
    q('settings.nav.advanced').click();
    flushSync();
    q('settings.btn.openLogs').click();
    flushSync();
    const note = document.body.textContent ?? '';
    expect(note).toContain('com.rimloc.gui/RimLoc/logs/gui.log');
    expect(note).not.toContain('~/RimLoc/logs');
  });
});
