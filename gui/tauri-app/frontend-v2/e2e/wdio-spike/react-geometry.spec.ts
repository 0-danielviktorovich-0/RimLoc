// Geometry invariants (mandate §50) — programmatic, semantic-first.
// Each probe returns measured facts; failures name the route + probe.
const results: { probe: string; route: string; pass: boolean; detail: string }[] = []

function record(probe: string, route: string, pass: boolean, detail = ''): void {
  results.push({ probe, route, pass, detail })
  if (!pass) console.log(`[geom] FAIL ${route} · ${probe}: ${detail}`)
}

async function probeRoute(route: string, anchor: string): Promise<void> {
  // workspace has no nav link — if its anchor is already up, stay put.
  const firstAnchor = anchor.split(',').map((x) => x.trim())[0]
  if (!(await (await browser.$(firstAnchor)).isExisting())) {
    await browser.$(`a[href="#/${route}"]`).click()
  }
  await browser.waitUntil(
    async () => {
      for (const sel of anchor.split(',').map((x) => x.trim())) {
        if (await (await browser.$(sel)).isExisting()) return true
      }
      return false
    },
    { timeout: 30000, interval: 100 },
  )
  await new Promise((r) => setTimeout(r, 400))
}

async function runLayoutProbes(route: string, anchor: string): Promise<void> {
  await probeRoute(route, anchor)
  const m = await browser.execute(() => {
    const doc = document.documentElement
    const rootScroll = window.scrollY > 4 || window.scrollX > 4
    const hOverflow = doc.scrollWidth > doc.clientWidth + 1
    // blank tail: main content ends >40% above the viewport bottom while the
    // page is otherwise empty below it
    const main = document.querySelector('main')
    let blankTail = false
    if (main) {
      const r = main.getBoundingClientRect()
      blankTail = r.height > 0 && r.bottom < window.innerHeight * 0.5
    }
    // interactive elements inside viewport bounds (x >= 0)
    let offscreenControls = 0
    for (const el of Array.from(document.querySelectorAll('button, a[href], input, select, textarea'))) {
      const b = el.getBoundingClientRect()
      if (b.width > 0 && (b.right < -4 || b.left > window.innerWidth + 4)) offscreenControls++
    }
    return { rootScroll, hOverflow, blankTail, offscreenControls, vw: window.innerWidth }
  })
  record('root-scroll', route, !m.rootScroll, `scrollY/X > 4`)
  record('horizontal-overflow', route, !m.hOverflow, `scrollWidth ${m.vw}+`)
  record('blank-tail', route, !m.blankTail, 'main ends above 50% viewport')
  record('controls-onscreen', route, m.offscreenControls === 0, `${m.offscreenControls} offscreen`)
}

describe('React R1 geometry invariants (§50)', () => {
  it('home: layout invariants', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), { timeout: 30000, interval: 100 })
    await browser.$('a[href="#/home"]').click()
    await new Promise((r) => setTimeout(r, 600))
    await runLayoutProbes('home', '.brand')
  })

  it('workspace: layout invariants + panel containment', async () => {
    await browser.waitUntil(async () => (await browser.$('[data-testid="home.project-card"]')).isExisting(), { timeout: 30000, interval: 100 })
    await browser.$('[data-testid="home.project-card"]').click()
    await browser.waitUntil(async () => (await browser.$('[data-testid="ws.root"]')).isExisting(), { timeout: 30000, interval: 100 })
    await browser.waitUntil(
      async () => (await browser.$$('[data-testid^="ws.entry."]').length) > 0,
      { timeout: 30000, interval: 200 },
    )
    await runLayoutProbes('workspace', '[data-testid="ws.root"]')

    // panel containment: editor pane stays inside the viewport, no overlap
    // with the entries pane (resizable-panels guarantee, but we verify).
    const m = await browser.execute(() => {
      const entries = document.querySelector('[data-testid="ws.entries"]')?.getBoundingClientRect()
      const editor = document.querySelector('[data-testid="ws.editor"]')?.getBoundingClientRect()
      const overlap = entries && editor ? entries.right > editor.left + 1 : false
      const inside = editor ? editor.right <= window.innerWidth + 1 && editor.top >= 0 : true
      return { overlap, inside }
    })
    record('panels-no-overlap', 'workspace', !m.overlap, 'entries.right > editor.left')
    record('editor-inside-viewport', 'workspace', m.inside, 'editor outside viewport')
  })

  it('checks + glossary + export: layout invariants', async () => {
    await runLayoutProbes('checks', '[data-testid="checks.findings"], .passed-state, .metrics-band')
    await runLayoutProbes('glossary', '[data-testid="gl.table"]')
    await runLayoutProbes('export', '.build-grid')
  })

  it('dark theme: same invariants on workspace', async () => {
    const toggle = await browser.$('[data-testid="theme-toggle"]')
    if (await toggle.isExisting()) {
      await toggle.click()
      await new Promise((r) => setTimeout(r, 300))
      // workspace has no nav link — reach it through the project card
      await browser.$('a[href="#/home"]').click()
      await browser.waitUntil(async () => (await browser.$('[data-testid="home.project-card"]')).isExisting(), { timeout: 30000, interval: 100 })
      await browser.$('[data-testid="home.project-card"]').click()
      await browser.waitUntil(async () => (await browser.$('[data-testid="ws.root"]')).isExisting(), { timeout: 30000, interval: 100 })
      const m = await browser.execute(() => {
        const doc = document.documentElement
        return {
          dark: document.documentElement.classList.contains('dark'),
          hOverflow: doc.scrollWidth > doc.clientWidth + 1,
          rootScroll: window.scrollY > 4,
        }
      })
      record('dark-applied', 'workspace', m.dark, 'no .dark class')
      record('dark-horizontal-overflow', 'workspace', !m.hOverflow, '')
      record('dark-root-scroll', 'workspace', !m.rootScroll, '')
      await toggle.click()
      await new Promise((r) => setTimeout(r, 200))
    }
  })

  it('summary', () => {
    const failed = results.filter((r) => !r.pass)
    console.log(`[geom] SUMMARY: ${results.length - failed.length}/${results.length} passed`)
    for (const f of failed) console.log(`[geom] FAILED: ${f.route} · ${f.probe} — ${f.detail}`)
    if (failed.length > 0) throw new Error(`geometry: ${failed.length} invariant(s) failed`)
  })
})
