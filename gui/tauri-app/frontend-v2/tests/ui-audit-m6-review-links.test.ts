// M-6 (UI audit 2026-09-29, /tmp/rimloc-ui-bugs.md): on the review tab the
// overview cards («На проверке 0», «Ошибки 0») sat next to a queue chip
// «Все проблемы 45» with no visible connection — the pair read as a glitch.
// Fix: the cards carry a link-note to the queue (with the one-line i18n
// explanation of WHY the numbers differ) and the queue answers with a link
// back to the overview counters.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync } from 'svelte';
import ReviewStub from '../src/lib/components/workspace/ReviewStub.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { review } from '../src/lib/stores/review.svelte';
import { i18n } from '../src/i18n/store.svelte';
import { click, cleanupMounted, exists, mountCmp, q } from './helpers';

describe('M-6: the overview counters and the queue are visibly linked', () => {
  let scrolled: Element[] = [];
  let spy: ReturnType<typeof vi.spyOn> | null = null;

  beforeEach(() => {
    cleanupMounted();
    project.reset();
    review.resetSession();
    i18n.setLocale('ru');
    scrolled = [];
    // jsdom has no scrollIntoView (setup.ts stubs it once globally); spy on
    // it HERE to record which anchor each link scrolls to.
    spy = vi.spyOn(Element.prototype, 'scrollIntoView').mockImplementation(function (this: Element) {
      scrolled.push(this);
    });
  });

  afterEach(() => {
    spy?.mockRestore();
    spy = null;
  });

  it('the overview carries a «см. очередь ниже» note explaining the 0-vs-N pair', () => {
    mountCmp(ReviewStub);
    const note = q('review.overview.queuelink-note');
    expect(exists('review.overview.queueLink')).toBe(true);
    expect(note.textContent).toContain('См. очередь проблем ниже');
    // The one-line explanation of the mismatch (the "0 next to 45" case).
    expect(note.textContent).toContain('числа не обязаны совпадать');
  });

  it('the queue links back to the overview counters', () => {
    mountCmp(ReviewStub);
    // The fixture corpus has open issues → the queue (and its back-link) shows.
    expect(review.active.length).toBeGreaterThan(0);
    expect(exists('review.queue.overviewLink')).toBe(true);
    expect(q('review.queue.overviewLink').textContent).toContain('счётчикам обзора');
  });

  it('both links actually scroll to their counterpart anchors', () => {
    mountCmp(ReviewStub);
    click('review.overview.queueLink');
    click('review.queue.overviewLink');
    expect(scrolled.map((el) => el.id)).toEqual(['review-queue', 'review-overview']);
  });
});
