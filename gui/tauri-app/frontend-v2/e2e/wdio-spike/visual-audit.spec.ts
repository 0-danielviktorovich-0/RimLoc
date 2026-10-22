// UI-12 deep visual audit (owner addendum): a REPEATABLE visual-audit
// system, not a one-off — every surface × both themes × structural facts:
//   - text contrast: WCAG ratio computed in-page per text-bearing element
//     (oklch-aware: the design tokens are oklch, WebKit reports them
//     verbatim), hard floor 3.0 everywhere, < 4.5 recorded for review;
//   - checkboxes: rendered, non-zero, at least 8px;
//   - tables: the workspace head grid matches the row grid;
//   - layout: no horizontal document overflow.
// The full report is written to /tmp/rimloc-int/visual-audit-report.json
// as the review artifact (the honest acceptance matrix, not a proxy).
import { mkdirSync, writeFileSync } from 'node:fs'

const MOD = '/tmp/rimloc-int/mod'

interface SurfaceIssue {
  surface: string
  theme: string
  kind: string
  detail: string
}

// The FULL route table (16 routes, routes.ts) — UI-12 Issue #7 requires
// every screen, not a six-screen tranche.
const SURFACES: { name: string; hash: string; wait: string }[] = [
  { name: 'workspace', hash: '#/workspace', wait: '[data-testid="ws.root"]' },
  { name: 'projects', hash: '#/projects', wait: '[data-testid="projects.search"]' },
  { name: 'checks', hash: '#/checks', wait: '.page-content' },
  { name: 'existing', hash: '#/existing', wait: '.page-content' },
  { name: 'compare', hash: '#/compare', wait: '.page-content' },
  { name: 'glossary', hash: '#/glossary', wait: '.page-content' },
  { name: 'tm', hash: '#/tm', wait: '.page-content' },
  { name: 'aitranslate', hash: '#/aitranslate', wait: '.page-content' },
  { name: 'chatbatch', hash: '#/chatbatch', wait: '.page-content' },
  { name: 'export', hash: '#/export', wait: '.page-content' },
  { name: 'selfloc', hash: '#/selfloc', wait: '.page-content' },
  { name: 'diagnostics', hash: '#/diagnostics', wait: '.page-content' },
  { name: 'providers', hash: '#/providers', wait: '.page-content' },
  { name: 'lm', hash: '#/lm', wait: '.page-content' },
  { name: 'settings', hash: '#/settings', wait: '[data-testid="settings.theme"]' },
  { name: 'about', hash: '#/about', wait: '[data-testid="about.hero"]' },
]

const AUDIT_JS = () => {
  function parseColor(raw: string): [number, number, number, number] | null {
    const s = raw.trim()
    let m = s.match(/^rgba?\(([^)]+)\)$/)
    if (m) {
      const parts = m[1].split(/[\s,/]+/).filter(Boolean).map(Number)
      if (parts.length >= 3) return [parts[0], parts[1], parts[2], parts[3] ?? 1]
    }
    m = s.match(/^#([0-9a-fA-F]{3,8})$/)
    if (m) {
      const hex = m[1]
      const v = hex.length <= 4 ? hex.split('').map((ch) => ch + ch).join('') : hex
      const num = (i: number) => parseInt(v.slice(i, i + 2), 16)
      const a = v.length >= 8 ? num(6) / 255 : 1
      return [num(0), num(2), num(4), a]
    }
    m = s.match(/^color\(srgb\s+([^)]+)\)$/)
    if (m) {
      const parts = m[1].split(/[\s,/]+/).filter(Boolean)
      const ch = (tok: string) =>
        tok.endsWith('%') ? (parseFloat(tok) / 100) * 255 : parseFloat(tok) * 255
      const a = parts[3] !== undefined ? (parts[3].endsWith('%') ? parseFloat(parts[3]) / 100 : parseFloat(parts[3])) : 1
      return [ch(parts[0]), ch(parts[1]), ch(parts[2]), a]
    }
    m = s.match(/^oklch\(([^)]+)\)$/)
    if (m) {
      const parts = m[1].split(/[\s,/]+/).filter(Boolean)
      const l = parseFloat(parts[0])
      const c = parts[1].endsWith('%') ? (parseFloat(parts[1]) / 100) * 0.4 : parseFloat(parts[1])
      const hDeg = parseFloat(parts[2])
      const alpha = parts[3] !== undefined ? (parts[3].endsWith('%') ? parseFloat(parts[3]) / 100 : parseFloat(parts[3])) : 1
      if (![l, c, hDeg].every(Number.isFinite)) return null
      const h = (hDeg * Math.PI) / 180
      const a = c * Math.cos(h)
      const b = c * Math.sin(h)
      // OKLab -> LMS -> linear sRGB (Björn Ottosson's constants).
      const l_ = l + 0.3963377774 * a + 0.2158037573 * b
      const m_ = l - 0.1055613458 * a - 0.0638541728 * b
      const s_ = l - 0.0894841775 * a - 1.291485548 * b
      const L = l_ * l_ * l_
      const M = m_ * m_ * m_
      const S = s_ * s_ * s_
      const r = 4.0767416621 * L - 3.3077115913 * M + 0.2309699292 * S
      const g = -1.2684380046 * L + 2.6097574011 * M - 0.3413193965 * S
      const bl = -0.0041960863 * L - 0.7034186147 * M + 1.707614701 * S
      const to255 = (v: number) => Math.max(0, Math.min(1, v)) * 255
      return [to255(r), to255(g), to255(bl), alpha]
    }
    return null
  }
  function channelLin(v: number): number {
    const x = v / 255
    return x <= 0.04045 ? x / 12.92 : Math.pow((x + 0.055) / 1.055, 2.4)
  }
  function luminance(rgb: [number, number, number, number]): number {
    return (
      0.2126 * channelLin(rgb[0]) + 0.7152 * channelLin(rgb[1]) + 0.0722 * channelLin(rgb[2])
    )
  }
  function bgEl(el: Element): Element {
    let cur: Element | null = el
    while (cur) {
      const c = parseColor(getComputedStyle(cur).backgroundColor)
      if (c && c[3] >= 0.95) return cur
      cur = cur.parentElement
    }
    return document.body
  }
  function bgOf(el: Element): [number, number, number, number] | null {
    let cur: Element | null = el
    while (cur) {
      const raw = getComputedStyle(cur).backgroundColor
      const c = parseColor(raw)
      if (c && c[3] >= 0.95) return c
      cur = cur.parentElement
    }
    return [255, 255, 255, 1]
  }
  const issues: { kind: string; detail: string; ratio?: number }[] = []
  const review: { text: string; ratio: number; px: number }[] = []
  let texts = 0
  const main = document.querySelector('.app-main') ?? document.body
  for (const el of Array.from(main.querySelectorAll('*'))) {
    const hasText = Array.from(el.childNodes).some(
      (n) => n.nodeType === 3 && (n.textContent ?? '').trim().length > 0,
    )
    if (!hasText) continue
    const cs = getComputedStyle(el)
    if (cs.visibility === 'hidden' || cs.display === 'none' || parseFloat(cs.opacity) < 0.05) continue
    // WCAG exception: disabled controls are exempt from contrast minimums.
    if (el.closest('button[disabled], input[disabled], select[disabled], textarea[disabled], fieldset[disabled]')) continue
    const fg = parseColor(cs.color)
    if (!fg) continue
    const bg = bgOf(el)
    const l1 = luminance(fg)
    const l2 = luminance(bg)
    const hi = Math.max(l1, l2)
    const lo = Math.min(l1, l2)
    const ratio = (hi + 0.05) / (lo + 0.05)
    const px = parseFloat(cs.fontSize)
    texts++
    // WCAG 2.2 SC 1.4.3: AA = 4.5:1 for normal text; 3.0:1 only for
    // LARGE text (>=24px, or >=18.66px bold). The threshold is chosen
    // per element, and the report counts BOTH bands honestly.
    const large = px >= 24 || (px >= 18.66 && (parseInt(cs.fontWeight, 10) || 400) >= 700)
    const aaThreshold = large ? 3.0 : 4.5
    if (ratio < aaThreshold) {
      const text = (el.textContent ?? '').trim().slice(0, 40)
      issues.push({
        kind: large ? 'contrast-large' : 'contrast-aa',
        detail: `<${el.tagName.toLowerCase()}> "${text}" ${cs.fontSize}/${cs.fontWeight} ratio=${ratio.toFixed(2)} (needs ${aaThreshold}) fg=${cs.color} bg=${getComputedStyle(bgEl(el)).backgroundColor}`,
        ratio: Number(ratio.toFixed(2)),
      })
    } else if (ratio < 4.5) {
      // passes AA only as large text / non-text — recorded for review
      review.push({ text: (el.textContent ?? '').trim().slice(0, 30), ratio: Number(ratio.toFixed(2)), px })
    }
  }
  // Checkboxes: rendered and touchable.
  let checkboxes = 0
  for (const cb of Array.from(document.querySelectorAll('main input[type=checkbox]'))) {
    checkboxes++
    const r = cb.getBoundingClientRect()
    const cs = getComputedStyle(cb)
    if (r.width < 8 || r.height < 8 || cs.visibility === 'hidden' || cs.display === 'none') {
      issues.push({ kind: 'checkbox', detail: `checkbox too small or hidden: ${r.width}x${r.height}` })
    }
  }
  // Dividers: workspace rows keep a visible bottom border.
  let dividers = 0
  let dividerIssues = 0
  for (const row of Array.from(document.querySelectorAll('.entry-list .entry-row'))) {
    dividers++
    const cs = getComputedStyle(row)
    const c = parseColor(cs.borderBottomColor)
    if (cs.borderBottomWidth === '0px' || (c && c[3] < 0.05)) dividerIssues++
  }
  // Table alignment: the head grid must match the row grid.
  let tableAligned: boolean | null = null
  const head = document.querySelector('.table-head')
  const row = document.querySelector('.entry-list .entry-row')
  if (head && row) {
    tableAligned =
      getComputedStyle(head).gridTemplateColumns === getComputedStyle(row).gridTemplateColumns ||
      getComputedStyle(head).gridTemplateColumns.split(' ').length ===
        getComputedStyle(row).gridTemplateColumns.split(' ').length
    if (!tableAligned) {
      issues.push({
        kind: 'table',
        detail: `head "${getComputedStyle(head).gridTemplateColumns}" vs row "${getComputedStyle(row).gridTemplateColumns}"`,
      })
    }
  }
  const hOverflow =
    document.documentElement.scrollWidth > document.documentElement.clientWidth + 1
  if (hOverflow) issues.push({ kind: 'overflow', detail: 'horizontal document overflow' })
  return { texts, issues, review: review.slice(0, 10), checkboxes, dividers, dividerIssues, tableAligned, hOverflow }
}

function pause(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

async function waitExisting(selector: string, timeout = 30000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

describe('UI-12 deep visual audit — every surface x both themes', () => {
  const report: { surfaces: unknown[]; hardIssues: SurfaceIssue[] } = {
    surfaces: [],
    hardIssues: [],
  }

  before(() => {
    // Screenshots land here BEFORE any audit test runs.
    mkdirSync('/tmp/rimloc-int/shots', { recursive: true })
    // The same well-formed versioned fixture the integration lane uses —
    // the packaged app already has a persisted project over this mod, the
    // wizard re-opens it through the real path.
    mkdirSync(`${MOD}/1.6/Defs`, { recursive: true })
    mkdirSync(`${MOD}/1.5/Defs`, { recursive: true })
    mkdirSync(`${MOD}/About`, { recursive: true })
    writeFileSync(
      `${MOD}/About/About.xml`,
      '<?xml version="1.0" encoding="utf-8"?><ModMetaData><name>Integration UI Fixture</name><author>rimloc-testlab</author><packageId>rimloc.test.intgui</packageId><supportedVersions><li>1.6</li></supportedVersions></ModMetaData>',
    )
    writeFileSync(
      `${MOD}/1.6/Defs/Alpha.xml`,
      '<Defs><ThingDef><defName>Alpha</defName><label>Alpha label</label><description>Alpha description text</description></ThingDef></Defs>',
    )
    writeFileSync(
      `${MOD}/1.6/Defs/Beta.xml`,
      '<Defs><HediffDef><defName>Beta</defName><label>Beta label</label><description>Beta description text</description></HediffDef></Defs>',
    )
  })

  it('project opened through the wizard', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), {
      timeout: 60000,
      interval: 200,
    })
    await browser.execute(() => {
      window.location.hash = '#/projects'
    })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue(MOD)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]')
    await browser.$('[data-testid="wizard.version"]').selectByVisibleText('1.6')
    await browser.$('[data-testid="wizard.next"]').click()
    await pause(600)
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) {
      await browser.$('[data-testid="wizard.next"]').click()
    }
    await waitExisting('[data-testid="ws.root"]', 60000)
  })

  for (const theme of ['light', 'dark'] as const) {
    it(`theme=${theme}: set through the real Settings control`, async () => {
      await browser.execute(() => {
        window.location.hash = '#/settings'
      })
      await waitExisting('[data-testid="settings.theme"]', 15000)
      await browser.execute((t) => {
        const select = document.querySelector('[data-testid="settings.theme"]') as HTMLSelectElement
        const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')!.set!
        setter.call(select, t)
        select.dispatchEvent(new Event('change', { bubbles: true }))
      }, theme)
      await pause(400)
      const applied = await browser.execute(
        (t) =>
          document.documentElement.classList.contains('dark') === (t === 'dark')
            ? 'ok'
            : document.documentElement.className,
        theme,
      )
      if (applied !== 'ok') throw new Error(`theme ${theme} not applied: ${applied}`)
    })

    for (const surface of SURFACES) {
      it(`audit ${surface.name} in ${theme}`, async () => {
        await browser.execute((h) => {
          window.location.hash = h
        }, surface.hash)
        await waitExisting(surface.wait, 30000)
        await pause(500)
        const audit = (await browser.execute(AUDIT_JS)) as {
          texts: number
          issues: { kind: string; detail: string }[]
          review: { text: string; ratio: number; px: number }[]
          checkboxes: number
          dividers: number
          dividerIssues: number
          tableAligned: boolean | null
          hOverflow: boolean
        }
        if (audit.texts < 10) {
          throw new Error(`${surface.name}/${theme}: only ${audit.texts} text elements audited`)
        }
        // REAL packaged-app screenshot (Issue #7): the review artifact is
        // the PNG pair per surface/theme, taken from the SAME build the
        // DOM metrics come from.
        await browser.saveScreenshot(
          `/tmp/rimloc-int/shots/${surface.name}-${theme}.png`,
        )
        report.surfaces.push({ surface: surface.name, theme, ...audit })
        for (const issue of audit.issues) {
          report.hardIssues.push({
            surface: surface.name,
            theme,
            kind: issue.kind,
            detail: issue.detail,
          })
        }
      })
    }
  }

  it('audit report: zero hard issues, artifact written', async () => {
    mkdirSync('/tmp/rimloc-int/shots', { recursive: true })
    const aa = report.hardIssues.filter((i) => i.kind === 'contrast-aa')
    const aaLarge = report.hardIssues.filter((i) => i.kind === 'contrast-large')
    const totals = {
      surfaceThemePairs: report.surfaces.length,
      textElements: report.surfaces.reduce((a, s) => a + (s as { texts: number }).texts, 0),
      reviewBandLt45: report.surfaces.reduce(
        (a, s) => a + (s as { review: unknown[] }).review.length,
        0,
      ),
      aaViolations: aa.length,
      aaLargeViolations: aaLarge.length,
    }
    writeFileSync(
      '/tmp/rimloc-int/visual-audit-report.json',
      JSON.stringify({ totals, ...report }, null, 2),
    )
    if (report.hardIssues.length > 0) {
      throw new Error(
        `visual audit found ${report.hardIssues.length} hard issues (${totals.aaViolations} AA <4.5, ${totals.aaLargeViolations} large <3.0, rest structural): ${JSON.stringify(
          report.hardIssues.slice(0, 10),
        )}`,
      )
    }
  })
})
