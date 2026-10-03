// honesty.no_mock_action_in_prod — STATUS class (wave-10 ask §g): in mock mode
// EVERY screen must visibly carry the honesty badge — either
// [data-testid=transport-mock-badge] ("Mock transport (demo mode)", the
// transport truth when no project is open) or [data-testid=mock-badge]
// ("Demo data (mock)", the demo-project identity). The badge is the marker
// that separates honest mock UI from a silent mock default; a screen WITHOUT
// the badge in mock mode is a honesty regression.
// (The reverse half of the manifest invariant — an actionable control must
// require the LIVE contract — needs the Tauri IPC surface and is T2b scope.)
import { expect, test } from '@playwright/test';
import { dismissCoachIfOpen, gotoScreen } from '../helpers/app';

/** All routes of the hash router (router.svelte.ts ROUTES). */
const ALL_ROUTES = [
  'home',
  'wizard',
  'workspace',
  'review',
  'build',
  'existing',
  'chat',
  'settings',
  'providers',
  'help',
  'diagnostics',
  'glossary',
  'tm'
] as const;

const BADGE = '[data-testid=transport-mock-badge], [data-testid=mock-badge]';

test('honesty: бейдж мока виден на каждом экране mock-режима', async ({ page }) => {
  test.setTimeout(90_000); // 13 маршрутов × goto + одноразовый коуч-скип
  const seen: { route: string; badge: string }[] = [];
  const PROJECT_ROUTES = new Set(['workspace', 'review', 'build']);
  let coachAlreadySeen = false;

  for (const route of ALL_ROUTES) {
    await gotoScreen(page, route as (typeof ALL_ROUTES)[number]);
    // Project routes mount the Workspace → the first-run coach opens once per
    // context; dismiss so the badge is measured undimmed. Subsequent mounts
    // skip the long wait — the seen flag is persisted by the skip itself.
    if (PROJECT_ROUTES.has(route)) {
      await dismissCoachIfOpen(page, coachAlreadySeen);
      coachAlreadySeen = true;
    } else {
      await expect(page.getByTestId('onboarding.overlay')).toHaveCount(0);
    }
    const badge = page.locator(BADGE);
    await expect(
      badge,
      `на #/${route} в mock-режиме нет ни transport-mock-badge, ни mock-badge — экран претендует на живые данные`
    ).toBeVisible();
    const testid = await badge.getAttribute('data-testid');
    seen.push({ route, badge: testid ?? 'unknown' });
  }

  // The route→badge listing is the STATUS output of this invariant: attach it
  // to the report so the wave log records which screen shows which marker.
  await test.info().attach('mock-badge-screens.json', {
    body: JSON.stringify(seen, null, 2),
    contentType: 'application/json'
  });
  // Sanity: the honest marker appeared everywhere we visited.
  expect(seen).toHaveLength(ALL_ROUTES.length);
});

test('honesty: глоссарий в mock — живой таблицы нет, бейдж честности виден (wave 13)', async ({ page }) => {
  // Wave 13 (glossary MOCK→LIVE): on a CONTRACT project the workspace
  // glossary tab renders the live table over project_glossary; in mock mode
  // it MUST stay the honest stub — durable project state cannot be faked,
  // so no term table with add/edit/delete controls may appear.
  await gotoScreen(page, 'home');
  await page.getByTestId('home.demo.open').click();
  await dismissCoachIfOpen(page);
  await page.getByTestId('tabs.glossary').click();
  await expect(
    page.getByTestId('workspace.glossary-stub'),
    'в mock-режиме глоссарий обязан оставаться честным стабом'
  ).toBeVisible();
  await expect(
    page.getByTestId('workspace.glossary.demo-badge'),
    'стаб глоссария без бейджа Demo data претендует на живые данные'
  ).toBeVisible();
  await expect(page.getByTestId('workspace.glossary-live')).toHaveCount(0);
  await expect(page.getByTestId('workspace.glossary.add-form')).toHaveCount(0);
});

test('honesty: демо-проект помечен бейджем Demo data (mock) в шапке', async ({ page }) => {
  await gotoScreen(page, 'home');
  // Open the bundled demo project: the workspace must carry the DEMO badge
  // (mock-badge), not just the transport badge — the dataset is synthetic and
  // says so (MOCK_LIVE_ONBOARDING_MANDATE §1, MockBadge.svelte).
  await page.getByTestId('home.demo.open').click();
  await dismissCoachIfOpen(page);
  await expect(
    page.getByTestId('mock-badge'),
    'демо-проект открыт, но шапка не показывает Demo data (mock)'
  ).toBeVisible();
});
