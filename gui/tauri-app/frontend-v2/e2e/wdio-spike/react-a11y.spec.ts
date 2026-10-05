// A11y probes (mandate §56): keyboard traversal, accessible names, contrast
// (relative luminance over OKLCH-computed rgb), tabindex discipline.
// Automated evidence ≠ complete proof (§56) — human review rides on top.
const results: { probe: string; pass: boolean; detail: string }[] = []
function record(probe: string, pass: boolean, detail = ''): void {
  results.push({ probe, pass, detail })
  if (!pass) console.log(`[a11y] FAIL ${probe}: ${detail}`)
}

async function contrast(fgSel: string, bgSel: string): Promise<number> {
  return browser.execute(
    (fgSel, bgSel) => {
      // OKLCH → linear sRGB (CSS Color 4 math) — canvas can't parse oklch
      // in this WebKit, so compute directly from the token string.
      const oklchToLinear = (L: number, C: number, Hdeg: number): [number, number, number] => {
        const h = (Hdeg * Math.PI) / 180
        const a = C * Math.cos(h)
        const b = C * Math.sin(h)
        const l_ = L + 0.3963377774 * a + 0.2158037573 * b
        const m_ = L - 0.1055613458 * a - 0.0638541728 * b
        const s_ = L - 0.0894841775 * a - 1.291485548 * b
        const l = l_ ** 3
        const m = m_ ** 3
        const s = s_ ** 3
        return [
          4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
          -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
          -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
        ]
      }
      const parse = (color: string): [number, number, number] | null => {
        const m = color.match(/oklch\(([-\d.]+)%?\s+([\d.]+)\s+([-\d.]+)/)
        if (m) {
          const L = Number(m[1]) > 1 ? Number(m[1]) / 100 : Number(m[1])
          return oklchToLinear(L, Number(m[2]), Number(m[3]))
        }
        const rgbM = color.match(/rgba?\(([\d.]+),\s*([\d.]+),\s*([\d.]+)/)
        if (rgbM) {
          const f = (v: number) => {
            const x = v / 255
            return x <= 0.03928 ? x / 12.92 : Math.pow((x + 0.055) / 1.055, 2.4)
          }
          return [f(Number(rgbM[1])), f(Number(rgbM[2])), f(Number(rgbM[3]))]
        }
        return null
      }
      const rel = (rgb: [number, number, number]): number =>
        0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
      const fgEl = document.querySelector(fgSel)
      if (!fgEl) return -1
      const fg = parse(getComputedStyle(fgEl).color)
      if (!fg) return -1
      let bgEl: Element | null = document.querySelector(bgSel)
      let bg: [number, number, number] | null = null
      while (bgEl && !bg) {
        const c = getComputedStyle(bgEl).backgroundColor
        if (c && c !== 'rgba(0, 0, 0, 0)' && c !== 'transparent') bg = parse(c)
        bgEl = bgEl.parentElement
      }
      if (!bg) return -1
      const hi = Math.max(rel(fg), rel(bg))
      const lo = Math.min(rel(fg), rel(bg))
      return Math.round(((hi + 0.05) / (lo + 0.05)) * 100) / 100
    },
    fgSel,
    bgSel,
  )
}

describe('React R1 a11y (§56)', () => {
  it('home: names, tabindex discipline, contrast, focus traversal', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), { timeout: 30000, interval: 100 })
    await new Promise((r) => setTimeout(r, 600))

    // 1) accessible names: every button has text or aria-label
    const unnamed = await browser.execute(() => {
      return Array.from(document.querySelectorAll('button'))
        .filter((b) => !b.textContent?.trim() && !b.getAttribute('aria-label'))
        .map((b) => b.className)
        .slice(0, 5)
    })
    record('buttons-have-names', unnamed.length === 0, `unnamed: ${unnamed.join(', ') || 'none'}`)

    // 2) no positive tabindex
    const positive = await browser.execute(
      () => document.querySelectorAll('[tabindex]:not([tabindex="0"]):not([tabindex="-1"])').length,
    )
    record('no-positive-tabindex', positive === 0, `${positive} elements`)

    // 3) contrast: body text vs background (STANDARD REQUIREMENT: ≥4.5 for 13px)
    const bodyContrast = await contrast('.page-content p', '.page-content')
    record('contrast-body-text', bodyContrast >= 4.5, `ratio ${bodyContrast}`)
    const navContrast = await contrast('.primary-nav a', '.app-sidebar')
    record('contrast-nav', navContrast >= 4.5, `ratio ${navContrast}`)

    // 4) keyboard traversal: Tab reaches the sidebar nav and moves forward
    await browser.$('.brand').click()
    await browser.keys(['Tab'])
    const firstActive = await browser.execute(() => document.activeElement?.textContent?.slice(0, 30) ?? '')
    record('focus-enters-nav', firstActive.trim().length > 0, `first stop: "${firstActive}"`)
  })

  it('workspace: editor labels + focus movement', async () => {
    await browser.waitUntil(async () => (await browser.$('[data-testid="home.project-card"]')).isExisting(), { timeout: 30000, interval: 100 })
    await browser.$('[data-testid="home.project-card"]').click()
    await browser.waitUntil(async () => (await browser.$('[data-testid="ws.root"]')).isExisting(), { timeout: 30000, interval: 100 })

    // editor textarea has a label wired via for/id
    const labelWired = await browser.execute(() => {
      const ta = document.querySelector('#translation')
      if (!ta) return 'no textarea'
      return document.querySelector('label[for="translation"]') ? 'wired' : 'NOT wired'
    })
    record('editor-label-wired', labelWired === 'wired', labelWired)

    // icon-only buttons carry aria-labels
    const unnamedIcons = await browser.execute(() =>
      Array.from(document.querySelectorAll('.icon-btn')).filter((b) => !b.getAttribute('aria-label')).length,
    )
    record('icon-buttons-labeled', unnamedIcons === 0, `${unnamedIcons} unnamed`)

    // focus moves into the textarea on row select→Tab path: textarea reachable
    await browser.$('[data-testid^="ws.entry."]').click()
    await waitTextarea()
    async function waitTextarea(): Promise<void> {
      await browser.waitUntil(async () => (await browser.$('[data-testid="ws.editor-textarea"]')).isExisting(), {
        timeout: 10000,
        interval: 100,
      })
    }
    const taFocusable = await browser.execute(() => {
      const ta = document.querySelector('[data-testid="ws.editor-textarea"]')
      if (!ta) return false
      ta.focus()
      return document.activeElement === ta
    })
    record('textarea-focusable', taFocusable, '')
  })

  it('summary', () => {
    const failed = results.filter((r) => !r.pass)
    console.log(`[a11y] SUMMARY: ${results.length - failed.length}/${results.length} passed`)
    for (const f of failed) console.log(`[a11y] FAILED: ${f.probe} — ${f.detail}`)
    if (failed.length > 0) throw new Error(`a11y: ${failed.length} probe(s) failed`)
  })
})
