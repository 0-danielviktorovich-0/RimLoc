// Shared semantic helpers for the T2a invariant suite.
// Every locator here is semantic (role / data-testid / structural container) —
// never coordinates, never pixels (AUTONOMOUS_ACCEPTANCE_ARCHITECTURE, principle
// "Semantic-first"). Geometry facts come from boundingBox / getBoundingClientRect.
import { expect, type Page } from '@playwright/test';

export type Screen = 'home' | 'workspace' | 'review';

/** Open a screen by its hash route. The app boots in mock mode on the vite dev
 * server (import.meta.env.DEV → devMode → client mode 'mock'). */
export async function gotoScreen(page: Page, screen: Screen): Promise<void> {
  await page.goto(`/#/${screen}`);
  await expect(page.getByRole('banner')).toBeVisible();
  await expect(page.locator('main.app-main')).toBeVisible();
}

/** The first Workspace mount starts the product coach (fresh browser context =
 * fresh localStorage = coach opens). Skip it so geometry/focus tests measure
 * the underlying screen, not the tour overlay. The mount is asynchronous
 * (Svelte effect after navigation), so WAIT for the overlay to either appear
 * or stay absent before deciding — an immediate isVisible() races the mount.
 * `coachAlreadySeen` shortens the wait for subsequent mounts in the same
 * browser context (the skip persists the seen flag). */
export async function dismissCoachIfOpen(page: Page, coachAlreadySeen = false): Promise<void> {
  const overlay = page.getByTestId('onboarding.overlay');
  await overlay
    .waitFor({ state: 'visible', timeout: coachAlreadySeen ? 300 : 4_000 })
    .catch(() => {
      /* coach legitimately absent (seen flag already persisted) */
    });
  if (await overlay.isVisible().catch(() => false)) {
    await page.getByTestId('onboarding.skip').click();
  }
  await expect(overlay).toHaveCount(0);
}

export interface ScreenMetrics {
  /** Viewport-relative bottom edge of the deepest meaningful content. */
  contentBottom: number;
  viewportWidth: number;
  viewportHeight: number;
  /** Containers inside main with overflow auto/scroll AND scrollHeight >
   * clientHeight — the "data continue below the fold" signal. */
  scrollableContainers: string[];
  docScrollWidth: number;
  docClientWidth: number;
  /** Top edge of the first content block under the header. */
  firstContentTop: number;
  /** Bottom edge of the app header (role=banner). */
  headerBottom: number;
}

interface MeaningfulProbe {
  contentBottom: number;
  scrollableContainers: string[];
  firstContentTop: number | null;
}

/**
 * Measure one screen in the live DOM. "Meaningful element" = visible, non-zero
 * box, and either own text (direct text node) or a form/media control —
 * structural wrappers (divs/spans with no own content) don't count, so a tall
 * empty wrapper can never fake a pass.
 */
export async function measureScreen(page: Page): Promise<ScreenMetrics> {
  return page.evaluate((): ScreenMetrics => {
    const main = document.querySelector('main.app-main');
    if (!main) throw new Error('main.app-main не найден — экран не смонтирован (буст-гейт?)');

    const probe: MeaningfulProbe = { contentBottom: 0, scrollableContainers: [], firstContentTop: null };

    for (const el of Array.from(main.querySelectorAll<HTMLElement>('*'))) {
      const cs = getComputedStyle(el);
      if (cs.display === 'none' || cs.visibility === 'hidden') continue;
      const rect = el.getBoundingClientRect();
      if (rect.width <= 0 || rect.height <= 0) continue;

      const ownText = Array.from(el.childNodes)
        .filter((n) => n.nodeType === Node.TEXT_NODE)
        .some((n) => (n.textContent ?? '').trim().length > 0);
      const interactive = ['BUTTON', 'INPUT', 'TEXTAREA', 'SELECT', 'IMG'].includes(el.tagName);
      if (ownText || interactive) {
        probe.contentBottom = Math.max(probe.contentBottom, rect.bottom);
      }

      // Vertical scroll ownership: data continue below the fold.
      if (/(auto|scroll)/.test(cs.overflowY) && el.scrollHeight > el.clientHeight + 1) {
        probe.scrollableContainers.push(
          `${el.tagName.toLowerCase()}` + (el.classList.length ? `.${Array.from(el.classList).join('.')}` : '')
        );
      }
    }

    const firstChild = main.firstElementChild as HTMLElement | null;
    const header = document.querySelector('header.app-header');
    const se = document.scrollingElement!;

    return {
      contentBottom: probe.contentBottom,
      viewportWidth: window.innerWidth,
      viewportHeight: window.innerHeight,
      scrollableContainers: probe.scrollableContainers,
      docScrollWidth: se.scrollWidth,
      docClientWidth: (se as HTMLElement).clientWidth,
      firstContentTop: firstChild ? firstChild.getBoundingClientRect().top : null,
      headerBottom: header ? header.getBoundingClientRect().bottom : Number.NaN
    };
  });
}

export function describeBlankTail(m: ScreenMetrics): string {
  const pct = Math.round((m.contentBottom / m.viewportHeight) * 100);
  return (
    `contentBottom=${m.contentBottom.toFixed(0)}px (${pct}% от viewport ${m.viewportHeight}px); ` +
    `скроллящиеся контейнеры в main: ${m.scrollableContainers.length ? m.scrollableContainers.join(', ') : 'нет'}`
  );
}
