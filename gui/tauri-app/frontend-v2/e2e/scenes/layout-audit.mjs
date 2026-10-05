// D-series layout audit: pairwise overlap of major blocks + doc overflow,
// across routes × themes × locales × viewports. Ancestor/descendant pairs
// are excluded; only SIBLING-level unexpected intersections are violations.
import { chromium } from 'playwright';

const ROUTES = ['home','workspace','review','build','existing','settings','providers','help','diagnostics'];
const THEMES = ['dark','light'];
const LOCALES = ['ru','en'];
const VIEWPORTS = [[1760,900],[1280,800],[1024,760]];

const browser = await chromium.launch({ headless: true });
const findings = [];

function isDescendant(a, b) { return a.contains(b) || b.contains(a); }

for (const vp of VIEWPORTS) {
  for (const theme of THEMES) {
    for (const locale of LOCALES) {
      const ctx = await browser.newContext({ viewport: { width: vp[0], height: vp[1] }, colorScheme: theme });
      await ctx.addInitScript(([t, l]) => {
        localStorage.setItem('rimloc.theme', t);
        localStorage.setItem('rimloc.locale', l);
      }, [theme, locale]);
      const page = await ctx.newPage();
      for (const route of ROUTES) {
        await page.goto(`http://localhost:5199/#/${route}`, { waitUntil: 'networkidle' });
        await page.waitForTimeout(350);
        const res = await page.evaluate(() => {
          const isDescendant = (a, b) => a.contains(b) || b.contains(a);
          // крупные видимые блоки: секции/карточки верхнего уровня
          const els = Array.from(document.querySelectorAll('main section, main .card, section.card, .card'))
            .filter((el) => {
              const cs = getComputedStyle(el);
              if (cs.display === 'none' || cs.visibility === 'hidden') return false;
              // интентциональные плавающие оверлеи — не дефект раскладки:
              // onboarding-коуч и dev-панель
              if (el.closest('[data-testid="onboarding.overlay"], .dev-panel, [data-testid="dev.panel"]')) return false;
              const r = el.getBoundingClientRect();
              return r.width > 120 && r.height > 60;
            });
          const rects = els.map((el) => ({ el, r: el.getBoundingClientRect() }));
          const overlaps = [];
          for (let i = 0; i < rects.length; i++) {
            for (let j = i + 1; j < rects.length; j++) {
              const a = rects[i], b = rects[j];
              if (isDescendant(a.el, b.el)) continue;
              const ox = Math.min(a.r.right, b.r.right) - Math.max(a.r.left, b.r.left);
              const oy = Math.min(a.r.bottom, b.r.bottom) - Math.max(a.r.top, b.r.top);
              if (ox > 8 && oy > 8) {
                overlaps.push({
                  a: (a.el.getAttribute('data-testid') || a.el.className || a.el.tagName).toString().slice(0, 40),
                  b: (b.el.getAttribute('data-testid') || b.el.className || b.el.tagName).toString().slice(0, 40),
                  ox: Math.round(ox), oy: Math.round(oy),
                });
              }
            }
          }
          const hOverflow = document.documentElement.scrollWidth > document.documentElement.clientWidth + 1;
          return { overlaps: overlaps.slice(0, 4), hOverflow };
        });
        if (res.overlaps.length || res.hOverflow) {
          findings.push({ vp: vp.join('x'), theme, locale, route, ...res });
        }
      }
      await ctx.close();
    }
  }
}
await browser.close();
if (findings.length === 0) {
  console.log('LAYOUT AUDIT: clean — 9 маршрутов × 2 темы × 2 локали × 3 вьюпорта, 0 перекрытий, 0 переполнений');
} else {
  console.log(`LAYOUT AUDIT: ${findings.length} находок:`);
  for (const f of findings) {
    console.log(JSON.stringify(f));
  }
}
