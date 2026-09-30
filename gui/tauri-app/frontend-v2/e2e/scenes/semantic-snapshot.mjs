// Semantic UI snapshot (mandate 03 §7): machine-readable observation of the
// real frontend WITHOUT screenshots — per route: focused element, dialogs,
// interactive controls (role/name/testid/enabled/selected/expanded/visible)
// and scroll-container state. Output: JSON to ~/Developing/RimLoc-evidence/
// semantic-snapshots/<route>.json + a combined index. Secondary consumers:
// AI reviewers, design passes, regression diffs (JSON-diff friendly).
// Run: npm run snapshot:semantic  (needs vite on 5199, same as e2e/scenes).
import { chromium } from 'playwright';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';

const BASE = process.env.T5_BASE_URL ?? 'http://localhost:5199';
const OUT = join(homedir(), 'Developing', 'RimLoc-evidence', 'semantic-snapshots');
// M-7 gate: маршруты могут требовать пре-действие — например, вкладку
// «Проект» внутри workspace (контент не смонтирован, пока её не открыть).
const ROUTES_WITH_PRE = {
  'project-tab': { hash: '#/workspace', clickAria: 'Проект' },
};
const ROUTES = {
  home: '',
  workspace: '#/workspace',
  review: '#/review',
  build: '#/build',
  existing: '#/existing',
  settings: '#/settings',
  providers: '#/providers',
  help: '#/help',
  diagnostics: '#/diagnostics',
};

mkdirSync(OUT, { recursive: true });

/** Extract the semantic observation for the current page. Runs in-page. */
function observe() {
  const vis = (el) => {
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0;
  };
  const controls = [];
  const sel = 'button, [role="tab"], [role="switch"], [role="slider"], a[href], input, select, textarea, [role="combobox"], [role="menuitem"], [role="option"]';
  for (const el of document.querySelectorAll(sel)) {
    if (controls.length >= 400) break; // bounded observation, no DOM dump
    const cs = getComputedStyle(el);
    if (cs.display === 'none' || cs.visibility === 'hidden') continue;
    const visible = vis(el);
    if (!visible) continue;
    controls.push({
      role: el.getAttribute('role') ?? el.tagName.toLowerCase(),
      name: (el.getAttribute('aria-label') ?? el.textContent ?? '').trim().slice(0, 80) || null,
      testid: el.getAttribute('data-testid'),
      enabled: !el.disabled !== false ? !el.hasAttribute('disabled') : true,
      selected: el.getAttribute('aria-selected'),
      expanded: el.getAttribute('aria-expanded'),
    });
  }
  // M-7 gate tooth: bounded text observation of value-carrying elements
  // (anything the app marks with data-testid). Without this the snapshot
  // could not see a template placeholder rendered as a VALUE — exactly how
  // finding M-7 slipped past every sensor (2026-09-30).
  const testidTexts = [];
  for (const el of document.querySelectorAll('[data-testid]')) {
    if (testidTexts.length >= 300) break;
    const t = (el.textContent ?? '').trim();
    if (!t) continue;
    const cs = getComputedStyle(el);
    if (cs.display === 'none' || cs.visibility === 'hidden') continue;
    testidTexts.push({ testid: el.getAttribute('data-testid'), text: t.slice(0, 160) });
  }
  const scrollContainers = [];
  for (const el of document.querySelectorAll('*')) {
    if (scrollContainers.length >= 30) break;
    const cs = getComputedStyle(el);
    if (/(auto|scroll)/.test(cs.overflowY) && el.scrollHeight > el.clientHeight + 2) {
      scrollContainers.push({
        testid: el.getAttribute('data-testid'),
        cls: (el.className && String(el.className).slice(0, 40)) || null,
        scrollHeight: el.scrollHeight,
        clientHeight: el.clientHeight,
        atBottomGap: el.scrollHeight - el.scrollTop - el.clientHeight,
      });
    }
  }
  const active = document.activeElement;
  const dialogs = [...document.querySelectorAll('[role="dialog"], dialog')].map((d) => ({
    testid: d.getAttribute('data-testid'),
    name: d.getAttribute('aria-label'),
  }));
  return {
    url: location.hash || '#/',
    lang: document.documentElement.lang,
    theme: document.documentElement.dataset.theme ?? null,
    focused: active && active !== document.body
      ? { role: active.getAttribute('role') ?? active.tagName.toLowerCase(), testid: active.getAttribute('data-testid'), name: (active.getAttribute('aria-label') ?? active.textContent ?? '').trim().slice(0, 60) || null }
      : null,
    dialogs,
    controlsCount: controls.length,
    controls,
    testidTexts,
    scrollContainers,
    documentOverflowX: document.scrollingElement.scrollWidth - document.scrollingElement.clientWidth,
  };
}

const browser = await chromium.launch({ headless: true });
const index = {};
try {
  for (const [name, entry] of Object.entries({ ...ROUTES, ...Object.fromEntries(Object.entries(ROUTES_WITH_PRE).map(([k, v]) => [k, v.hash])) })) {
    const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 }, colorScheme: 'dark' });
    await ctx.addInitScript(() => {
      localStorage.setItem('rimloc.theme', 'dark');
      localStorage.setItem('rimloc.locale', 'ru');
    });
    const page = await ctx.newPage();
    await page.goto(`${BASE}/${entry}`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(200);
    // M-7 gate: пре-клик (например вкладка «Проект») ДО наблюдения —
    // иначе контент вкладки не смонтирован и снимок его не видит.
    const pre = ROUTES_WITH_PRE[name];
    if (pre?.clickAria) {
      // Табы приложения: BUTTON role=tab с текстом (без aria-label) —
      // матчится и aria, и текст.
      const tab = page
        .locator(`button[aria-label="${pre.clickAria}"], [role="tab"]:has-text("${pre.clickAria}")`)
        .first();
      if (await tab.count()) {
        // Playwright-клик перехватывается оверлеем — таб жмём DOM-ом.
        await tab.evaluate((el) => el.click());
        await page.waitForTimeout(400);
      }
    }
    const snap = await page.evaluate(observe);
    writeFileSync(join(OUT, `${name}.json`), JSON.stringify(snap, null, 1));
    index[name] = { controls: snap.controlsCount, dialogs: snap.dialogs.length, scrollContainers: snap.scrollContainers.length, overflowX: snap.documentOverflowX };
    await ctx.close();
  }
} finally {
  await browser.close();
}
writeFileSync(join(OUT, 'index.json'), JSON.stringify(index, null, 1));
console.log(`semantic-snapshots: ${Object.keys(index).length} routes -> ${OUT}`);
