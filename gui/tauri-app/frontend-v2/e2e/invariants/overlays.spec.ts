// overlays.inside_viewport (mandate 03 §11, manifest overlays.inside_viewport).
// Overlays that actually open in mock mode (found by reading the shell):
//   1. OnboardingCoach — the product tour auto-starts on the first Workspace
//      mount in a fresh context (onboarding.svelte.ts: startIfFirstRun);
//      renders [data-testid=onboarding.overlay] role=dialog aria-modal, fixed
//      inset 0, with an anchored card that must stay on-screen on every step.
//   2. LanguageManager — opened semantically: Workspace toolbar
//      languages.switcher.button → languages.switcher.add; renders
//      [data-testid=languages.manager.dialog] role=dialog aria-modal.
// Assertion class: boundingBox of the dialog card fully inside the viewport —
// geometry, not pixels (no screenshots).
import { expect, test } from '@playwright/test';
import { dismissCoachIfOpen, gotoScreen } from '../helpers/app';

const COACH_STEPS = 4; // COACH_TOTAL in onboarding.svelte.ts

/** The step-of line is localized ("Подсказка 1 из 4" / "Tip 1 of 4"), so the
 * current step is parsed from the digits — locale-independent. */
async function currentCoachStep(page: import('@playwright/test').Page, expected: number): Promise<void> {
  const stepOf = page.getByTestId('onboarding.step-of');
  await expect(stepOf).toBeVisible();
  await expect
    .poll(
      async () => {
        const nums = (await stepOf.innerText()).match(/\d+/g)?.map(Number) ?? [];
        return nums[0] ?? -1;
      },
      { message: `коуч должен показывать шаг ${expected}` }
    )
    .toBe(expected);
}

function assertBoxInsideViewport(
  box: { x: number; y: number; width: number; height: number },
  vp: { width: number; height: number },
  what: string
): void {
  expect(box.width, `${what}: нулевая ширина`).toBeGreaterThan(0);
  expect(box.height, `${what}: нулевая высота`).toBeGreaterThan(0);
  expect(box.x, `${what}: левее viewport (x=${box.x})`).toBeGreaterThanOrEqual(-1);
  expect(box.y, `${what}: выше viewport (y=${box.y})`).toBeGreaterThanOrEqual(-1);
  expect(
    box.x + box.width,
    `${what}: выходит за правый край (right=${box.x + box.width}, viewport ${vp.width})`
  ).toBeLessThanOrEqual(vp.width + 1);
  expect(
    box.y + box.height,
    `${what}: уходит за нижний край (bottom=${box.y + box.height}, viewport ${vp.height})`
  ).toBeLessThanOrEqual(vp.height + 1);
}

test('overlays.inside_viewport: коуч-тур открывается и остаётся во viewport на всех шагах', async ({
  page
}) => {
  await gotoScreen(page, 'workspace');
  const overlay = page.getByTestId('onboarding.overlay');
  await expect(overlay).toBeVisible();
  // The overlay IS the modal dialog (same element carries role=dialog).
  await expect(overlay).toHaveAttribute('role', 'dialog');
  await expect(overlay).toHaveAttribute('aria-modal', 'true');
  await expect(page.getByTestId('onboarding.step-of')).toBeVisible();

  const vp = page.viewportSize()!;
  const overlayBox = await overlay.boundingBox();
  expect(overlayBox).not.toBeNull();
  assertBoxInsideViewport(overlayBox!, vp, 'оверлей тура');

  for (let step = 1; step <= COACH_STEPS; step += 1) {
    await currentCoachStep(page, step);
    const card = overlay.locator('.card');
    const cardBox = await card.boundingBox();
    expect(cardBox, `шаг ${step}: карточка тура без boundingBox`).not.toBeNull();
    assertBoxInsideViewport(cardBox!, vp, `карточка тура, шаг ${step}`);
    // Highlighted anchor inside viewport too (coach marks must not cover their
    // target off-screen — mandate §11). The spotlight rect is the TARGET rect
    // plus the product's own 6px padding (OnboardingCoach.measure), and the
    // editor table legitimately touches the viewport bottom edge — so the
    // highlight may overshoot by exactly that padding, never more. The CARD
    // (the actual coach mark) stays strict above.
    const anchor = page.getByTestId('onboarding.anchor');
    if (await anchor.isVisible().catch(() => false)) {
      const anchorBox = await anchor.boundingBox();
      if (anchorBox) {
        expect(anchorBox.width, `подсветка якоря, шаг ${step}: нулевая ширина`).toBeGreaterThan(0);
        expect(anchorBox.height, `подсветка якоря, шаг ${step}: нулевая высота`).toBeGreaterThan(0);
        expect(
          anchorBox.y + anchorBox.height,
          `подсветка якоря, шаг ${step}: ниже viewport больше собственного пэддинга (bottom=${anchorBox.y + anchorBox.height}, viewport ${vp.height})`
        ).toBeLessThanOrEqual(vp.height + 7);
      }
    }
    if (step < COACH_STEPS) await page.getByTestId('onboarding.next').click();
  }

  // Skip closes the overlay — Back/Skip/Next controls present (mandate §11).
  await page.getByTestId('onboarding.back').click(); // last step → Back works
  await currentCoachStep(page, COACH_STEPS - 1);
  await page.getByTestId('onboarding.skip').click();
  await expect(overlay).toHaveCount(0);
});

test('overlays.inside_viewport: диалог менеджера языков внутри viewport', async ({ page }) => {
  await gotoScreen(page, 'workspace');
  await dismissCoachIfOpen(page);

  await page.getByTestId('languages.switcher.button').click();
  const menu = page.getByTestId('languages.switcher.menu');
  await expect(menu).toBeVisible();
  await page.getByTestId('languages.switcher.add').click();

  const dialog = page.getByTestId('languages.manager.dialog');
  await expect(dialog).toBeVisible();
  const vp = page.viewportSize()!;
  const box = await dialog.boundingBox();
  expect(box, 'диалог менеджера языков без boundingBox').not.toBeNull();
  assertBoxInsideViewport(box!, vp, 'диалог менеджера языков');

  // Escape closes the transient UI (mandate §10: Escape closes transient UI).
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
});
