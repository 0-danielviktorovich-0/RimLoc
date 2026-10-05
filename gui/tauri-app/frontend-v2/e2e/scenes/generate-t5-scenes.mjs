// T5 deterministic visual scenes generator (mandate 03 §12): screenshots of the
// real frontend (mock transport, headless) across theme × UI-locale × screen.
// Output: ~/Developing/RimLoc-evidence/t5-scenes/<theme>-<locale>-<screen>.png
// NOT a test: exit 0 when every scene is written; failures name the screen.
// Run: npm run scenes:t5   (vite preview must be running via webServer or start one)
import { chromium } from 'playwright';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';

const BASE = process.env.T5_BASE_URL ?? 'http://localhost:5199';
const OUT = join(homedir(), 'Developing', 'RimLoc-evidence', 't5-scenes');
const THEMES = ['dark', 'light'];
const LOCALES = ['ru', 'en'];
// Viewport axes (T5: narrow/wide). long_text/cjk need mock-data seeds —
// deliberately deferred until a seeded locale fixture exists.
const VIEWPORTS = { normal: { width: 1280, height: 800 }, narrow: { width: 900, height: 800 }, wide: { width: 1760, height: 900 } };
// screen -> hash route of the app shell (mock mode renders every route).
// Glossary/TM/Project are workspace TABS (no dedicated route) — covered by
// the workspace scene; add dedicated scenes when a route appears.
const SCREENS = {
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

const browser = await chromium.launch({ headless: true });
let written = 0;
const failures = [];
try {
  for (const theme of THEMES) {
    for (const locale of LOCALES) {
     for (const [vpName, vp] of Object.entries(VIEWPORTS)) {
      // Viewport axis runs on the reference combo only (dark/ru) to keep the
      // scene count bounded; the theme×locale matrix stays at normal size.
      if (vpName !== 'normal' && !(theme === 'dark' && locale === 'ru')) continue;
      const ctx = await browser.newContext({
        viewport: vp,
        colorScheme: theme === 'light' ? 'light' : 'dark',
      });
      // Seed prefs before app scripts run.
      await ctx.addInitScript(([t, l]) => {
        localStorage.setItem('rimloc.theme', t);
        localStorage.setItem('rimloc.locale', l);
      }, [theme, locale]);
      const page = await ctx.newPage();
      for (const [name, hash] of Object.entries(SCREENS)) {
        const vpSuffix = vpName === 'normal' ? '' : `-${vpName}`;
        const file = join(OUT, `${theme}-${locale}-${name}${vpSuffix}.png`);
        try {
          await page.goto(`${BASE}/${hash}`, { waitUntil: 'networkidle' });
          await page.waitForTimeout(250);
          await page.screenshot({ path: file, fullPage: false });
          written += 1;
        } catch (e) {
          failures.push(`${theme}-${locale}-${name}: ${String(e).slice(0, 120)}`);
        }
      }
      await ctx.close();
     }
    }
  }
} finally {
  await browser.close();
}
console.log(`t5-scenes: ${written} written -> ${OUT}`);
if (failures.length) {
  console.error(failures.join('\n'));
  process.exit(1);
}
