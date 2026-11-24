// T6 narrow-window lane (#161, parent #155): the packaged AUTOMATION build
// driven at the minimum supported window 980x640 (set through the
// RIMLOC_WINDOW_FRAME env hook in main.rs — browser.setViewportSize cannot
// resize a native Tauri window, GATE_B_STATUS.md). Every check is a measured
// DOM fact through browser.execute(getBoundingClientRect) — the
// react-geometry UI-04 precedent (25/25) — plus native CGWindowList captures
// (shot-window.py) as the visual evidence pair. Background-only: zero
// activations, zero keystrokes (owner P0 rule).
//
// Probes per surface:
//   h-overflow          document scrollWidth > clientWidth (наезд на границы)
//   root-scroll         the whole app shifted (scrollY/X > 4)
//   offscreen-controls  interactive elements FULLY outside the viewport
//   clipped-controls    interactive elements crossing the viewport edge
//   sibling-overlap     normal-flow siblings overlapping >15% of the smaller
//
// Report: $T6_EVIDENCE/narrow-window-report.json (honest acceptance matrix).
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

// Specs load as ESM — no __dirname here.
const HERE = path.dirname(fileURLToPath(import.meta.url))
const REPO = path.resolve(HERE, '..', '..', '..', '..', '..')
const EVIDENCE = process.env.T6_EVIDENCE || '/tmp/rimloc-t6/evidence'
const PIDFILE = process.env.T6_PIDFILE || '/tmp/rimloc-t6/app.pid'
const PY = process.env.T6_PY || '/tmp/rimloc-t6-venv/bin/python3'
const SHOT = process.env.T6_SHOT_PY || path.join(REPO, 'testlab/ui_automation/bin/shot-window.py')
const WINFRAME = process.env.T6_WINFRAME_PY || path.join(REPO, 'testlab/ui_automation/bin/winframe.py')
const MOD = process.env.T6_MOD || '/tmp/rimloc-t6/mod'
const EXPECT_W = 980
const EXPECT_H = 640

interface Finding {
  probe: string
  route: string
  pass: boolean
  detail: string
}
const results: Finding[] = []

function record(probe: string, route: string, pass: boolean, detail = ''): void {
  results.push({ probe, route, pass, detail })
  if (!pass) console.log(`[t6] FAIL ${route} · ${probe}: ${detail}`)
}

function pause(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

async function waitExisting(selector: string, timeout = 30000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

/** Native window capture through BOTH CG paths (Quartz + screencapture -l). */
function nativeShot(name: string): void {
  try {
    const pid = readFileSync(PIDFILE, 'utf8').trim()
    execFileSync(PY, [SHOT, pid, path.join(EVIDENCE, name)], { timeout: 25000 })
    console.log(`[t6] native shot ${name}`)
  } catch (e) {
    console.log(`[t6] native shot ${name} FAILED: ${(e as Error).message}`)
  }
}

async function webShot(name: string): Promise<void> {
  try {
    await browser.saveScreenshot(path.join(EVIDENCE, `${name}.web.png`))
  } catch (e) {
    console.log(`[t6] web shot ${name} FAILED: ${(e as Error).message}`)
  }
}

/** The app pid published by spawn-wrapper-t6.sh (0 when unreadable). */
function appPid(): number {
  try {
    return parseInt(readFileSync(PIDFILE, 'utf8').trim(), 10)
  } catch {
    return 0
  }
}

// ---------------------------------------------------------------- measurement
const MEASURE_JS = () => {
  const vw = window.innerWidth
  const vh = window.innerHeight
  const doc = document.documentElement
  const off: string[] = []
  const clipped: string[] = []
  const overlaps: string[] = []
  const label = (el: Element): string => {
    const tid = el.getAttribute('data-testid')
    if (tid) return `#${tid}`
    const c = (el as HTMLElement).className
    const cs = typeof c === 'string' && c ? `.${c.split(/\s+/).slice(0, 2).join('.')}` : ''
    const text = (el.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 24)
    return el.tagName.toLowerCase() + cs + (text ? ` "${text}"` : '')
  }
  const shown = (cs: CSSStyleDeclaration): boolean =>
    cs.display !== 'none' && cs.visibility !== 'hidden' && parseFloat(cs.opacity || '1') > 0.01

  // Reachable = some scrollable ancestor (or the document) can bring the
  // element into view. Below-the-fold content in a scrollable container is
  // normal UX, not a defect; unreachable (clipped by overflow:hidden)
  // content IS. Horizontally parked skip-links (left ≈ -9999px) are an
  // intentional a11y pattern, likewise exempt.
  const reachable = (el: Element, b: DOMRect): boolean => {
    if (b.left < -3000 || b.right > vw + 3000) return true // parked skip-link
    const verticalOut = b.bottom < -4 || b.top > vh + 4
    const horizontalOut = b.right < -4 || b.left > vw + 4
    if (!verticalOut && !horizontalOut) return true
    let cur: HTMLElement | null = el.parentElement
    while (cur) {
      const cs = getComputedStyle(cur)
      if (verticalOut && /(auto|scroll)/.test(cs.overflowY) && cur.scrollHeight > cur.clientHeight + 2) return true
      if (horizontalOut && /(auto|scroll)/.test(cs.overflowX) && cur.scrollWidth > cur.clientWidth + 2) return true
      cur = cur.parentElement
    }
    if (verticalOut && doc.scrollHeight > doc.clientHeight + 2) return true
    if (horizontalOut && doc.scrollWidth > doc.clientWidth + 2) return true
    return false
  }

  for (const el of Array.from(
    document.querySelectorAll(
      'button, a[href], input, select, textarea, [role="button"], [role="tab"], [role="switch"], [role="menuitem"]',
    ),
  )) {
    const cs = getComputedStyle(el)
    if (!shown(cs)) continue
    const b = el.getBoundingClientRect()
    if (b.width <= 1 || b.height <= 1) continue
    if (b.right < -4 || b.left > vw + 4 || b.bottom < -4 || b.top > vh + 4) {
      if (!reachable(el, b)) off.push(label(el))
      continue
    }
    if (b.left < -2 || b.right > vw + 2) clipped.push(label(el))
  }

  // Sibling overlap: normal-flow block siblings of one parent that overlap
  // by >15% of the smaller element's area. Parent-child nesting and
  // absolutely/fixed-positioned overlays are excluded (overlays are
  // intentional layers, flow siblings colliding is a layout bug).
  const seen = new Set<string>()
  for (const parent of Array.from(document.querySelectorAll('body *'))) {
    if (getComputedStyle(parent).display === 'contents') continue
    const kids = Array.from(parent.children).filter((k) => {
      const cs = getComputedStyle(k)
      if (!shown(cs)) return false
      if (cs.position === 'absolute' || cs.position === 'fixed') return false
      const r = k.getBoundingClientRect()
      return r.width * r.height >= 4000
    })
    if (kids.length < 2) continue
    for (let i = 0; i < kids.length; i++) {
      const a = kids[i].getBoundingClientRect()
      for (let j = i + 1; j < kids.length; j++) {
        const b = kids[j].getBoundingClientRect()
        const ix = Math.min(a.right, b.right) - Math.max(a.left, b.left)
        const iy = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top)
        if (ix <= 2 || iy <= 2) continue
        const ia = ix * iy
        const smaller = Math.min(a.width * a.height, b.width * b.height)
        if (ia > 0.15 * smaller) {
          const key = `${label(kids[i])} × ${label(kids[j])}`
          if (!seen.has(key)) {
            seen.add(key)
            overlaps.push(`${key} (${Math.round((ia / smaller) * 100)}%)`)
          }
        }
      }
    }
  }

  return {
    vw,
    vh,
    hOverflow: doc.scrollWidth > doc.clientWidth + 1,
    rootScroll: window.scrollY > 4 || window.scrollX > 4,
    off: [...new Set(off)].slice(0, 8),
    clipped: [...new Set(clipped)].slice(0, 8),
    overlaps: overlaps.slice(0, 8),
  }
}

type Measure = ReturnType<typeof MEASURE_JS>

function recordSurfaceProbes(name: string, m: Measure): void {
  record('h-overflow', name, !m.hOverflow, `scrollWidth > ${m.vw}`)
  record('root-scroll', name, !m.rootScroll, 'scrollY/X > 4')
  record(
    'offscreen-controls',
    name,
    m.off.length === 0,
    m.off.length ? `fully outside: ${m.off.join(', ')}` : '',
  )
  record(
    'clipped-controls',
    name,
    m.clipped.length === 0,
    m.clipped.length ? `crosses edge: ${m.clipped.join(', ')}` : '',
  )
  record(
    'sibling-overlap',
    name,
    m.overlaps.length === 0,
    m.overlaps.length ? m.overlaps.join(' | ') : '',
  )
}

async function measureSurface(name: string, hash: string, waitSel: string, shotName?: string): Promise<void> {
  await browser.execute((h: string) => {
    window.location.hash = h
  }, hash)
  await waitExisting(waitSel, 30000)
  await pause(800) // layout/settle margin — rect probes read post-transition state
  const m = (await browser.execute(MEASURE_JS)) as Measure
  recordSurfaceProbes(name, m)
  if (shotName) {
    await webShot(shotName)
    nativeShot(shotName)
  }
}

// The FULL route table (routes.ts) — the narrow window is the MINIMUM
// supported size, so every surface must hold its layout there.
const SURFACES: { name: string; hash: string; wait: string; shot?: string }[] = [
  { name: 'home', hash: '#/home', wait: '.brand', shot: '01-home' },
  { name: 'projects', hash: '#/projects', wait: '[data-testid="projects.search"]' },
  { name: 'workspace', hash: '#/workspace', wait: '[data-testid="ws.root"]', shot: '02-workspace' },
  { name: 'checks', hash: '#/checks', wait: '.page-content', shot: '03-checks' },
  { name: 'existing', hash: '#/existing', wait: '.page-content' },
  { name: 'compare', hash: '#/compare', wait: '.page-content' },
  { name: 'glossary', hash: '#/glossary', wait: '.page-content', shot: '04-glossary' },
  { name: 'tm', hash: '#/tm', wait: '.page-content' },
  { name: 'aitranslate', hash: '#/aitranslate', wait: '.page-content' },
  { name: 'chatbatch', hash: '#/chatbatch', wait: '.page-content' },
  { name: 'export', hash: '#/export', wait: '.page-content' },
  { name: 'selfloc', hash: '#/selfloc', wait: '.page-content' },
  { name: 'diagnostics', hash: '#/diagnostics', wait: '.page-content' },
  { name: 'providers', hash: '#/providers', wait: '.page-content' },
  { name: 'lm', hash: '#/lm', wait: '.page-content' },
  { name: 'settings', hash: '#/settings', wait: '[data-testid="settings.theme"]', shot: '05-settings' },
  { name: 'about', hash: '#/about', wait: '[data-testid="about.hero"]', shot: '06-about' },
]

// ---------------------------------------------------------------------- lane
describe('T6 narrow window 980x640 — native-AX regression lane', () => {
  before(() => {
    mkdirSync(EVIDENCE, { recursive: true })
    // Same well-formed versioned fixture the visual-audit lane uses.
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

  it('window frame is exactly 980x640, on-display, not offscreen', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), {
      timeout: 60000,
      interval: 200,
    })
    await pause(700)
    const vp = await browser.execute(() => ({
      iw: window.innerWidth,
      ih: window.innerHeight,
      dpr: window.devicePixelRatio,
    }))
    // The webview viewport excludes the native title bar: at a 980x640
    // window the content area measured 980x608 (32px chrome) — the width
    // must match exactly, the height is window height minus title bar.
    const TITLE_BAR = 32
    record(
      'viewport-980x640',
      'window',
      Math.abs(vp.iw - EXPECT_W) <= 1 &&
        vp.ih >= EXPECT_H - TITLE_BAR - 2 &&
        vp.ih <= EXPECT_H - 8,
      `innerWidth=${vp.iw} innerHeight=${vp.ih} dpr=${vp.dpr} (content area = window minus ~${TITLE_BAR}px title bar)`,
    )

    const pid = appPid()
    if (!pid || !existsSync(WINFRAME)) {
      record('native-frame-980x640', 'window', false, `pid=${pid} winframe=${WINFRAME}`)
      return
    }
    const out = execFileSync(PY, [WINFRAME, String(pid)], { timeout: 15000 }).toString()
    const frame = /frame=(-?\d+),(-?\d+),(\d+)x(\d+)/.exec(out)
    // [^\]] — a bare [^]] in JS is the match-any class, not "not-]"
    const screensRaw = /screens=\[([^\]]*)\]/.exec(out)
    if (!frame) {
      record('native-frame-980x640', 'window', false, `winframe output: ${out.trim()}`)
      return
    }
    const [, fx, fy, fw, fh] = frame.map(Number) as unknown as number[]
    record(
      'native-frame-980x640',
      'window',
      fw === EXPECT_W && fh === EXPECT_H,
      `native frame=${fx},${fy},${fw}x${fh}`,
    )
    let onDisplay = false
    if (screensRaw) {
      for (const s of screensRaw[1].split('|')) {
        const [sx, sy, rest] = s.split(',')
        const [sw, sh] = rest.split('x').map(Number)
        if (
          fx >= Number(sx) &&
          fy >= Number(sy) &&
          fx + fw <= Number(sx) + sw &&
          fy + fh <= Number(sy) + sh
        ) {
          onDisplay = true
          break
        }
      }
    }
    record('native-on-display', 'window', onDisplay, out.replace(/\n/g, ' ').trim())
    nativeShot('00-window-native')
  })

  it('project opened through the wizard', async () => {
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

  for (const surface of SURFACES) {
    it(`narrow layout: ${surface.name}`, async () => {
      await measureSurface(surface.name, surface.hash, surface.wait, surface.shot)
    })
  }

  // Workspace panel containment (react-geometry precedent): the two panes
  // must not collide and the editor must stay inside the viewport.
  it('narrow layout: workspace panel containment', async () => {
    await browser.execute(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]')
    await waitExisting('[data-testid^="ws.entry."]')
    await pause(400)
    const m = await browser.execute(() => {
      const entries = document.querySelector('[data-testid="ws.entries"]')?.getBoundingClientRect()
      const editor = document.querySelector('[data-testid="ws.editor"]')?.getBoundingClientRect()
      const overlap = entries && editor ? entries.right > editor.left + 1 : false
      const inside = editor
        ? editor.right <= window.innerWidth + 1 && editor.top >= 0
        : true
      return { overlap, inside }
    })
    record('panels-no-overlap', 'workspace', !m.overlap, 'entries.right > editor.left')
    record('editor-inside-viewport', 'workspace', m.inside, 'editor outside viewport')
  })

  it('dark theme: same invariants at 980x640', async () => {
    await browser.execute(() => {
      window.location.hash = '#/settings'
    })
    await waitExisting('[data-testid="settings.theme"]')
    await browser.execute(() => {
      const select = document.querySelector('[data-testid="settings.theme"]') as HTMLSelectElement
      const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')!.set!
      setter.call(select, 'dark')
      select.dispatchEvent(new Event('change', { bubbles: true }))
    })
    await pause(500)
    for (const [name, hash, waitSel] of [
      ['dark-workspace', '#/workspace', '[data-testid="ws.root"]'],
      ['dark-settings', '#/settings', '[data-testid="settings.theme"]'],
    ] as const) {
      await browser.execute((h: string) => {
        window.location.hash = h
      }, hash)
      await waitExisting(waitSel)
      await pause(500)
      const m = (await browser.execute(MEASURE_JS)) as Measure
      recordSurfaceProbes(name, m)
      await webShot(name)
      nativeShot(name)
    }
    // back to light so the fixture profile stays predictable
    await browser.execute(() => {
      const select = document.querySelector('[data-testid="settings.theme"]') as HTMLSelectElement
      const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')!.set!
      setter.call(select, 'light')
      select.dispatchEvent(new Event('change', { bubbles: true }))
    })
  })

  it('summary: report written, zero layout defects', async () => {
    const totals = {
      probes: results.length,
      passed: results.filter((r) => r.pass).length,
      failed: results.filter((r) => !r.pass).length,
      surfaces: SURFACES.map((s) => s.name),
      window: { expectW: EXPECT_W, expectH: EXPECT_H },
    }
    writeFileSync(
      path.join(EVIDENCE, 'narrow-window-report.json'),
      JSON.stringify({ totals, results }, null, 2),
    )
    const failed = results.filter((r) => !r.pass)
    console.log(`[t6] SUMMARY: ${totals.passed}/${totals.probes} passed`)
    for (const f of failed) console.log(`[t6] FAILED: ${f.route} · ${f.probe} — ${f.detail}`)
    if (failed.length > 0) {
      throw new Error(`narrow-window: ${failed.length} layout defect(s) at 980x640`)
    }
  })
})
