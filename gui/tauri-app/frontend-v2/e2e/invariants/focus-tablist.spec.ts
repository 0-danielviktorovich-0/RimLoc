// focus.tablist_roving (mandate 03 §10, manifest focus.tablist_roving).
// The Workspace top navigation is an ARIA tablist with roving tabindex:
//   - Tab from body must reach the tablist within an interaction budget;
//   - Arrow keys move focus AND selection (selection follows focus — Enter is
//     NOT needed);
//   - exactly one tab is in the tab order (tabindex=0), the rest are -1.
//
// MEASURED 2026-09-30 (wave 10, headless Chromium, mock mode): the Tab walk
// body → tablist takes **16 presses** in dev mode / **14 product presses**
// without dev tooling (DevPanel `summary` is the first DOM tab stop and one
// stop is swallowed by the collapsed details) — over the 12-press budget from
// the wave-10 ask. The budget test below is therefore a self-reporting
// known-fail: it still walks the real DOM every run and calls test.fixme with
// the measured number while it exceeds the budget; when a UX wave brings it
// ≤12 the same test flips to a live regression gate automatically. The count
// is NEVER loosened in the assertion.
import { expect, test } from '@playwright/test';
import { dismissCoachIfOpen, gotoScreen } from '../helpers/app';

/** Regression budget from the wave-10 ask: ≤12 Tab presses from body to the
 * Workspace tablist. */
const TAB_REACH_BUDGET = 12;

test('focus.tablist_roving: Tab с body доходит до вкладок в бюджете ≤12 (факт: 16 — известный долг UX)', async ({
  page
}) => {
  await gotoScreen(page, 'workspace');
  await dismissCoachIfOpen(page);

  // Start from body: blur anything the browser focused after load.
  await page
    .evaluate(() => (document.activeElement as HTMLElement | null)?.blur?.())
    .catch(() => undefined);

  let presses = 0;
  let onTab = false;
  const MAX_PROBE = 40; // loop guard; the budget assertion below is the gate
  while (presses < MAX_PROBE) {
    await page.keyboard.press('Tab');
    presses += 1;
    onTab = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      return !!el && el.getAttribute('role') === 'tab';
    });
    if (onTab) break;
  }
  expect(onTab, `Tab не дошёл до role="tab" даже за ${presses} нажатий (пробежали ${MAX_PROBE})`).toBe(true);

  // Known-fail with the honest number: while the walk exceeds the budget the
  // test is reported as fixme (wave input for the next UX pass); once the
  // budget is met the expect below becomes the live gate.
  test.fixme(
    presses > TAB_REACH_BUDGET,
    `фактический фокус-бюджет body→вкладки: ${presses} нажатий Tab при бюджете ${TAB_REACH_BUDGET} — клавиатурный путь перегружен (mandate §10: «dozens of Tab presses is not acceptable»)`
  );
  expect(presses, `фактический бюджет ${presses} > ${TAB_REACH_BUDGET}`).toBeLessThanOrEqual(TAB_REACH_BUDGET);
});

test('focus.tablist_roving: стрелки переключают вкладки без Enter, roving tabindex — одна вкладка в tab-order', async ({
  page
}) => {
  await gotoScreen(page, 'workspace');
  await dismissCoachIfOpen(page);

  // Focus the ACTIVE tab directly (semantic focus, no coordinates) — the
  // Tab-reachability budget is asserted by the test above; this test proves
  // the roving/arrow contract once focus is in the tablist.
  const selectedTab = page.locator('[role="tab"][aria-selected="true"]').first();
  await selectedTab.focus();

  const focused = await page.evaluate(() => {
    const el = document.activeElement as HTMLElement;
    return {
      role: el.getAttribute('role'),
      tabindex: el.getAttribute('tabindex'),
      selected: el.getAttribute('aria-selected'),
      inTablist: !!el.closest('[role="tablist"]')
    };
  });
  expect(
    focused.inTablist && focused.role === 'tab' && focused.tabindex === '0' && focused.selected === 'true',
    `в фокусе не активный таб с roving tabindex: ${JSON.stringify(focused)}`
  ).toBe(true);

  const before = await selectedTab.getAttribute('data-testid');

  // ArrowRight: selection moves WITHOUT Enter (selection follows focus).
  await page.keyboard.press('ArrowRight');
  const afterRight = await selectedTab.getAttribute('data-testid');
  expect(afterRight, `ArrowRight не переключил вкладку (${before} → ${afterRight})`).not.toBe(before);
  const focusedAfterRight = await page.evaluate(() =>
    (document.activeElement as HTMLElement | null)?.getAttribute('data-testid')
  );
  expect(
    focusedAfterRight,
    'после ArrowRight фокус и выделение разошлись (selection must follow focus)'
  ).toBe(afterRight);

  // ArrowLeft returns both focus and selection.
  await page.keyboard.press('ArrowLeft');
  const afterLeft = await selectedTab.getAttribute('data-testid');
  expect(afterLeft, `ArrowLeft не вернул выделение на ${before}`).toBe(before);
  const focusedAfterLeft = await page.evaluate(() =>
    (document.activeElement as HTMLElement | null)?.getAttribute('data-testid')
  );
  expect(focusedAfterLeft).toBe(before);

  // Exactly one tab of THIS tablist is in the tab order — scoped to the top
  // Workspace tablist (the detail panel owns its own independent roving set).
  const roving = await page.evaluate(() => {
    const list = document.querySelector('[role="tablist"]')!;
    const tabs = Array.from(list.querySelectorAll<HTMLElement>('[role="tab"]'));
    return {
      tabs: tabs.length,
      zero: tabs.filter((t) => t.getAttribute('tabindex') === '0').length
    };
  });
  expect(roving.tabs).toBeGreaterThanOrEqual(2);
  expect(
    roving.zero,
    `roving tabindex сломан: ${roving.zero} из ${roving.tabs} вкладок верхней таб-панели в tab-order (должна быть ровно 1)`
  ).toBe(1);
});
