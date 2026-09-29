// geometry.* invariants (mandate 03 §8/§9, manifest T2.invariants) on the real
// screens Home / Workspace / Review, mock transport. Classes, not pixels:
// every assertion compares element rectangles (boundingBox / client rects) or
// DOM scroll extents — a screenshot diff can never pass or fail these tests.
import { expect, test } from '@playwright/test';
import {
  describeBlankTail,
  dismissCoachIfOpen,
  gotoScreen,
  measureScreen,
  type Screen
} from '../helpers/app';

const SCREENS: Screen[] = ['home', 'workspace', 'review'];

for (const screen of SCREENS) {
  test.describe(`geometry @ ${screen}`, () => {
    test.beforeEach(async ({ page }) => {
      await gotoScreen(page, screen);
      if (screen !== 'home') await dismissCoachIfOpen(page);
    });

    // geometry.no_giant_blank_tail — owner-observed "infinite blank scroll"
    // must be caught generically (mandate §8): the last meaningful content in
    // the main area reaches at least 60% of the viewport height, OR the data
    // demonstrably continue below the fold (a real scroll container).
    test('geometry.no_giant_blank_tail: контент доходит до ≥60% viewport или продолжается скроллом', async ({
      page
    }) => {
      const m = await measureScreen(page);
      const reachesThreshold = m.contentBottom >= 0.6 * m.viewportHeight;
      const dataContinue = m.scrollableContainers.length > 0;
      const info = describeBlankTail(m);
      expect(
        reachesThreshold || dataContinue,
        `Гигантский пустой хвост: контент обрывается на ${info}`
      ).toBe(true);
    });

    // geometry.no_unintended_hscroll — the DOCUMENT never scrolls sideways on
    // fixed-workspace screens (inner pane scrollbars are intentional scroll
    // ownership, mandate §9; the page itself must not move).
    test('geometry.no_unintended_hscroll: документ не скроллится по горизонтали', async ({
      page
    }) => {
      const m = await measureScreen(page);
      expect(
        m.docScrollWidth,
        `document.scrollingElement.scrollWidth=${m.docScrollWidth} > clientWidth=${m.docClientWidth}+1 — непреднамеренный горизонтальный скролл страницы`
      ).toBeLessThanOrEqual(m.docClientWidth + 1);
    });

    // geometry.sticky_header_no_cover — the header never overlaps the first
    // content block: header bottom must sit at or above the first content top,
    // and the first content pixel must hit a main-area element, not the header.
    test('geometry.sticky_header_no_cover: шапка не перекрывает первый контент-элемент', async ({
      page
    }) => {
      const m = await measureScreen(page);
      expect(
        m.headerBottom,
        'header.app-header не найден на экране'
      ).not.toBeNaN();
      expect(
        m.firstContentTop,
        'main.app-main без содержимого — нечего проверять'
      ).not.toBeNull();
      expect(
        m.headerBottom,
        `header.bottom=${m.headerBottom.toFixed(1)} перекрывает первый контент-элемент (top=${m.firstContentTop!.toFixed(1)})`
      ).toBeLessThanOrEqual(m.firstContentTop! + 1);

      // Overlap proof: a probe point just inside the first content block must
      // resolve to an element inside main (not the header, not an overlay).
      const coveredByHeader = await page.evaluate(() => {
        const main = document.querySelector('main.app-main')!;
        const first = main.firstElementChild as HTMLElement | null;
        if (!first) return true;
        const r = first.getBoundingClientRect();
        const el = document.elementFromPoint(r.left + Math.min(8, r.width / 2), r.top + 2);
        return !el || !main.contains(el);
      });
      expect(coveredByHeader, 'первый контент-элемент перекрыт не-контентным слоем').toBe(false);
    });
  });
}

// geometry.cta_visible_nonzero — the primary CTA of Home and Workspace has a
// non-zero visible box and sits fully inside the viewport (mandate §8:
// "important controls have non-zero visible area"), no scrolling needed.
test.describe('geometry.cta_visible_nonzero', () => {
  test('Home: главный CTA «Продолжить» видим, ненулевой, во viewport', async ({ page }) => {
    await gotoScreen(page, 'home');
    const cta = page.locator('button.btn-primary[data-testid^="home.recent-continue"]');
    await expect(cta).toBeVisible();
    const box = await cta.boundingBox();
    expect(box, 'главный CTA Home не имеет boundingBox').not.toBeNull();
    expect(box!.width, 'CTA Home нулевой ширины').toBeGreaterThan(0);
    expect(box!.height, 'CTA Home нулевой высоты').toBeGreaterThan(0);
    const vp = page.viewportSize()!;
    expect(
      box!.x,
      `CTA Home вне viewport по X: x=${box!.x}`
    ).toBeGreaterThanOrEqual(-1);
    expect(
      box!.x + box!.width,
      `CTA Home выходит за viewport по X: right=${box!.x + box!.width} (viewport ${vp.width})`
    ).toBeLessThanOrEqual(vp.width + 1);
    expect(
      box!.y,
      `CTA Home вне viewport по Y: y=${box!.y}`
    ).toBeGreaterThanOrEqual(-1);
    expect(
      box!.y + box!.height,
      `CTA Home уходит вниз за viewport: bottom=${box!.y + box!.height} (viewport ${vp.height}) — главная CTA требует скролла`
    ).toBeLessThanOrEqual(vp.height + 1);
  });

  test('Workspace: главный CTA (review/build) видим, ненулевой, во viewport', async ({ page }) => {
    await gotoScreen(page, 'workspace');
    await dismissCoachIfOpen(page);
    // The toolbar routes by state: issues → review CTA, else build CTA.
    const cta = page
      .getByTestId('workspace.cta-review')
      .or(page.getByTestId('workspace.cta-build'));
    await expect(cta).toBeVisible();
    const box = await cta.boundingBox();
    expect(box, 'главный CTA Workspace не имеет boundingBox').not.toBeNull();
    expect(box!.width, 'CTA Workspace нулевой ширины').toBeGreaterThan(0);
    expect(box!.height, 'CTA Workspace нулевой высоты').toBeGreaterThan(0);
    const vp = page.viewportSize()!;
    expect(box!.y + box!.height, `CTA Workspace уходит за viewport: bottom=${box!.y + box!.height} (viewport ${vp.height})`).toBeLessThanOrEqual(vp.height + 1);
    expect(box!.y, `CTA Workspace выше viewport: y=${box!.y}`).toBeGreaterThanOrEqual(-1);
    expect(box!.x, `CTA Workspace левее viewport: x=${box!.x}`).toBeGreaterThanOrEqual(-1);
    expect(box!.x + box!.width, `CTA Workspace правее viewport: right=${box!.x + box!.width}`).toBeLessThanOrEqual(vp.width + 1);
  });
});
