// Wave 9 (owner-screenshot defect classes, 2026-09-30): two structural
// regression invariants, built from the owner handoff package (кадры
// 01.37.06 / 01.37.31 / 01.39.14) and mandate §10/§14.
//
//   1. blank-tail class (кадр 01.37.06): the Review content must own the tab
//      height — the section fills the workspace column and the queue list is
//      the stretching scroll container instead of clamping to content height
//      and leaving a dead dark tail below. jsdom cannot lay out flexbox, so
//      the check pins BOTH halves of the contract: the mounted DOM shape
//      (queue ul inside the pane inside the section) and the stretching
//      declarations in the component's <style> block. Removing either half
//      fails this test.
//
//   2. focus budget (мандат §10, «35 табов до вкладки Проект — не приемлемо»):
//      the number of Tab stops between <body> and the workspace tablist is
//      capped at the audited baseline. Today's 15 stops are all real controls
//      with no duplicates: 3 header nav + 3 theme + 2 interface language +
//      3 target-language switcher (button, [+], 1 pinned chip) + 3 toolbar
//      (context toggle, lifecycle CTA, close project) + 1 active tab (roving
//      tabindex keeps the other tabs out). The invariant catches the class
//      regression — new focusable chrome before the tabs — without banning
//      any single control.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { beforeEach, describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { cleanupMounted, goto, mountCmp, q } from './helpers';

const ROOT = join(__dirname, '..');

/** Extract the source rule body for a scoped selector, e.g. '.queue'. */
function styleRule(source: string, selector: string): string {
  const style = source.slice(source.indexOf('<style'));
  const at = style.indexOf(`${selector} {`);
  expect(at, `rule ${selector} { … } must exist in the component <style>`).toBeGreaterThan(-1);
  const open = style.indexOf('{', at);
  const close = style.indexOf('}', open);
  return style.slice(open + 1, close);
}

/** A real Tab stop: naturally focusable, or tabindex >= 0; explicit -1 out. */
function isTabStop(el: Element): boolean {
  const ti = el.getAttribute('tabindex');
  if (ti === '-1') return false;
  if (el.hasAttribute('tabindex')) return true;
  if ((el as HTMLButtonElement).disabled) return false;
  return el.matches('button, a[href], input:not([type="hidden"]), select, textarea');
}

beforeEach(() => {
  cleanupMounted();
  project.reset();
});

describe('wave 9: blank-tail class (Review queue owns the tab height)', () => {
  it('the mounted queue is a ul.queue inside #review-queue inside section.review', () => {
    goto('#/review');
    mountCmp(App);
    flushSync();
    const section = q('workspace.review');
    const pane = section.querySelector('#review-queue');
    expect(pane, 'queue pane must render inside the review section').toBeTruthy();
    const list = pane?.querySelector('ul');
    expect(list?.classList.contains('queue'), 'the pane list carries the queue class').toBe(true);
    // The fixture corpus has open issues → real rows render (not the empty state).
    expect(list?.querySelectorAll('li').length ?? 0).toBeGreaterThan(0);
  });

  it('the queue/section classes carry the stretching declarations', () => {
    const source = readFileSync(join(ROOT, 'src/lib/components/workspace/ReviewStub.svelte'), 'utf8');
    const review = styleRule(source, '.review');
    expect(review).toContain('flex: 1');
    expect(review).toContain('min-height: 0');
    const queue = styleRule(source, '.queue');
    expect(queue).toContain('flex: 1');
    expect(queue).toContain('min-height: 0');
    expect(queue).toContain('overflow-y: auto');
    // Empty-queue case: the meaningful empty state grows into the pane.
    const clear = styleRule(source, '.clear-card');
    expect(clear).toContain('flex: 1');
  });

  it('the other workspace tabs fill the column too (same class, no dead tail)', () => {
    for (const [file, selector] of [
      ['src/lib/components/workspace/GlossaryStub.svelte', '.glossary'],
      ['src/lib/components/workspace/TMStub.svelte', '.tm'],
      ['src/lib/components/screens/ProjectOverview.svelte', '.overview']
    ] as const) {
      const source = readFileSync(join(ROOT, file), 'utf8');
      const rule = styleRule(source, selector);
      expect(rule, `${file}: ${selector} must stretch`).toContain('flex: 1');
      expect(rule, `${file}: ${selector} must allow shrinking`).toContain('min-height: 0');
    }
    // Glossary/TM tables scroll inside a dedicated container…
    for (const file of [
      'src/lib/components/workspace/GlossaryStub.svelte',
      'src/lib/components/workspace/TMStub.svelte'
    ]) {
      const rule = styleRule(readFileSync(join(ROOT, file), 'utf8'), '.scroll');
      expect(rule, `${file}: .scroll must be the table scroll container`).toContain('flex: 1');
      expect(rule).toContain('overflow-y: auto');
    }
  });
});

describe('wave 9: workspace focus budget (mandate §10)', () => {
  it('Tab stops from body to the tablist stay at the audited baseline (≤ 15)', () => {
    goto('#/workspace');
    mountCmp(App);
    flushSync();
    const projectTab = document.querySelector('[data-testid="tabs.project"]');
    expect(projectTab, 'workspace tablist must render').toBeTruthy();
    const stops = [
      ...document.querySelectorAll('button, a[href], input, select, textarea, [tabindex]')
    ].filter(
      (el) => isTabStop(el) && projectTab!.compareDocumentPosition(el) & Node.DOCUMENT_POSITION_PRECEDING
    );
    // Baseline 15 = 8 header (3 nav + 3 theme + 2 lang) + 3 switcher
    // (button, [+], pinned uk) + 3 toolbar (context toggle, CTA, close) +
    // 1 active tab. Every stop was audited 2026-09-30: no button+link
    // duplicates for one action (L-8/M-4 cleanups hold). The cap is the
    // regression line: new focusable chrome before the tabs must come with
    // an explicit decision to raise this number.
    expect(stops.length).toBeLessThanOrEqual(15);
    expect(stops.length).toBeGreaterThanOrEqual(10); // sanity: the mount is real
  });
});
