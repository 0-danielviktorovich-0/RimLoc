/**
 * STAGE 3 — MEASURABLE ROUTE×STATE×CONTROL COVERAGE.
 * Walks EVERY React route on the live packaged app (state: a project is
 * open), inventories every data-testid control with its tag, visibility
 * and disabled state, then matches the inventory against the controls the
 * stage-1/stage-2 journeys actually ACTIVATED and prints the measured
 * coverage: present / activated / uncovered. Evidence JSON goes to a
 * deterministic evidence dir.
 */
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const WORK_DIR = mkdtempSync(join(tmpdir(), 'rimloc-stage3-'))
const MOD = join(WORK_DIR, 'TestMod')
const EVIDENCE_DIR = join(tmpdir(), 'rimloc-stage3-evidence')

const SIDEBAR_ROUTES = ['workspace', 'projects', 'checks', 'existing', 'compare', 'glossary', 'tm', 'aitranslate', 'chatbatch', 'export', 'lm', 'settings']
const PALETTE_ROUTES: { route: string; query: string }[] = [
  { route: 'providers', query: 'Провайдеры' },
  { route: 'diagnostics', query: 'Диагностика' },
  { route: 'selfloc', query: 'самолок' },
]
const ABOUT_ROUTE = 'about' // sidebar footer link

/** Controls the shipped journeys demonstrably ACTIVATED (clicked/filled/
 *  asserted interactively) — the measured coverage numerator. */
const ACTIVATED = new Set<string>([
  // stage1-journey
  'wizard.open', 'wizard.path-input', 'wizard.next', 'wizard.version', 'wizard.overlay',
  'ws.editor-textarea', 'ws.editor-save', 'ws.tab.output', 'ws.output-pre',
  'be.outdir', 'be.export', 'projects.project-card',
  // stage2-ai
  'ai.modes', 'ai.mode.auto', 'ai.mode.chatgpt', 'ai.mode.byo', 'ai.mode.local', 'ai.mode.noapi',
  'ai.instance', 'ai.run', 'ai.chatgpt-signin', 'ai.provider', 'ai.open-chatbatch',
  'ai.auto-key-hint', 'ai.auto-key-link', 'ai.job', 'ai.job-state',
  'palette.open', 'palette.input',
  'prov.card.new', 'prov.preset', 'prov.label', 'prov.model.new', 'prov.baseurl.new',
  'prov.secret.new', 'prov.save.new', 'prov.catalog',
])

async function waitFor(fn: () => Promise<boolean>, timeoutMs = 15000, intervalMs = 300): Promise<void> {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    if (await fn()) return
    await new Promise(r => setTimeout(r, intervalMs))
  }
  throw new Error('waitFor timed out after ' + timeoutMs + 'ms')
}

async function waitExisting(sel: string, timeout = 30000) {
  const e = await browser.$(sel)
  await e.waitForExist({ timeout })
  return e
}

function writeFixture(): void {
  const about = join(MOD, 'About')
  const keyed = join(MOD, 'Languages', 'English', 'Keyed')
  mkdirSync(about, { recursive: true })
  mkdirSync(keyed, { recursive: true })
  writeFileSync(join(about, 'About.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<ModMetaData>\n  <name>Stage3 Coverage Mod</name>\n  <packageId>rimloc.test.stage3</packageId>\n  <author>RimLoc</author>\n  <supportedVersions><li>1.6</li></supportedVersions>\n</ModMetaData>\n')
  writeFileSync(join(keyed, 'S3.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<LanguageData>\n  <S3Greeting>hello coverage</S3Greeting>\n</LanguageData>\n')
}

interface ControlInfo { id: string; tag: string; interactive: boolean; visible: boolean; disabled: boolean }
interface RouteInventory { route: string; controls: ControlInfo[]; error?: string }

async function inventoryRoute(route: string, navigate: () => Promise<void>): Promise<RouteInventory> {
  const inv: RouteInventory = { route, controls: [] }
  try {
    await navigate()
    await new Promise(r => setTimeout(r, 700)) // settle async loads
    const raw = await browser.execute(() => {
      const interactiveTags = new Set(['BUTTON', 'A', 'INPUT', 'SELECT', 'TEXTAREA'])
      return Array.from(document.querySelectorAll('[data-testid]')).map((el) => {
        const he = el as HTMLElement
        const role = he.getAttribute('role')
        return {
          id: he.getAttribute('data-testid') || '',
          tag: he.tagName,
          interactive: interactiveTags.has(he.tagName) || role === 'radio' || role === 'button' || role === 'option',
          visible: he.offsetParent !== null || he.getClientRects().length > 0,
          disabled: (he as HTMLButtonElement).disabled === true || he.getAttribute('aria-disabled') === 'true',
        }
      })
    })
    inv.controls = (raw || []) as ControlInfo[]
  } catch (e) {
    inv.error = String(e)
  }
  return inv
}

describe('Stage 3 route×state×control coverage', () => {
  before(() => { writeFixture() })

  it('measures every route with a project open', async () => {
    // Setup: project via wizard (same proven flow)
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), { timeout: 60000, interval: 200 })
    const projectsLink = await browser.$('.app-sidebar a[href="#/projects"]')
    await projectsLink.click()
    await (await waitExisting('[data-testid="wizard.open"]')).click()
    await (await waitExisting('[data-testid="wizard.path-input"]')).setValue(MOD)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]', 60000)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitFor(async () => !(await (await browser.$('[data-testid="wizard.overlay"]')).isExisting()), 10000).catch(() => { /* may auto-dismiss */ })
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="ws.root"]', 300000)
    await waitExisting('.entry-list .entry-row', 60000)

    const inventories: RouteInventory[] = []
    for (const route of SIDEBAR_ROUTES) {
      const inv = await inventoryRoute(route, async () => {
        const link = await browser.$('.app-sidebar a[href="#/' + route + '"]')
        await link.waitForExist({ timeout: 15000 })
        await link.click()
      })
      inventories.push(inv)
    }
    // about: sidebar footer
    inventories.push(await inventoryRoute(ABOUT_ROUTE, async () => {
      const link = await browser.$('.app-sidebar a[href="#/about"]')
      await link.waitForExist({ timeout: 15000 })
      await link.click()
    }))
    // palette-only routes: the real user path
    for (const { route, query } of PALETTE_ROUTES) {
      const inv = await inventoryRoute(route, async () => {
        await (await browser.$('[data-testid="palette.open"]')).click()
        const input = await browser.$('[data-testid="palette.input"]')
        await input.waitForExist({ timeout: 15000 })
        await input.setValue(query)
        await waitFor(async () => {
          const items = await browser.$$('.palette-item')
          for (const it of items) { await it.click(); return true }
          return false
        }, 8000, 300)
      })
      inventories.push(inv)
    }

    // Aggregate: dedupe testids per route, then compute coverage
    const report: Record<string, { controls: number; interactive: number; activatedHere: string[]; uncovered: string[] }> = {}
    const allIds = new Set<string>()
    const interactiveIds = new Set<string>()
    const activatedHit = new Set<string>()
    for (const inv of inventories) {
      const ids = new Set(inv.controls.map(c => c.id))
      const inter = new Set(inv.controls.filter(c => c.interactive && c.visible && !c.disabled).map(c => c.id))
      ids.forEach(i => allIds.add(i))
      inter.forEach(i => interactiveIds.add(i))
      const hit = [...inter].filter(i => ACTIVATED.has(i))
      hit.forEach(h => activatedHit.add(h))
      report[inv.route] = {
        controls: ids.size,
        interactive: inter.size,
        activatedHere: hit,
        uncovered: [...inter].filter(i => !ACTIVATED.has(i)).sort(),
      }
    }
    const presentInteractive = [...interactiveIds].sort()
    const uncoveredAll = presentInteractive.filter(i => !ACTIVATED.has(i))
    const pct = presentInteractive.length === 0 ? 0 : Math.round((activatedHit.size / presentInteractive.length) * 1000) / 10

    mkdirSync(EVIDENCE_DIR, { recursive: true })
    const payload = {
      measuredAt: new Date().toISOString(),
      state: 'project-open (stage3 fixture: 1 keyed entry)',
      routesMeasured: inventories.length,
      perRoute: report,
      totals: {
        presentTestids: allIds.size,
        presentInteractiveTestids: presentInteractive.length,
        activatedByJourneys: activatedHit.size,
        coveragePercent: pct,
        uncovered: uncoveredAll,
      },
    }
    writeFileSync(join(EVIDENCE_DIR, 'route-control-coverage.json'), JSON.stringify(payload, null, 1))
    console.log('COVERAGE SUMMARY:', JSON.stringify(payload.totals))
    for (const [r, v] of Object.entries(report)) {
      console.log('ROUTE ' + r + ': interactive=' + v.interactive + ' activated=' + v.activatedHere.length + ' uncovered=' + v.uncovered.length)
    }
  })


  it('activates the primary uncovered controls of every screen', async () => {
    const activatedNow: string[] = []
    const failed: { id: string; err: string }[] = []

    const clickSafe = async (route: string, id: string): Promise<void> => {
      const sel = '[data-testid="' + id + '"]'
      try {
        const link = await browser.$('.app-sidebar a[href="#/' + route + '"]')
        if (await link.isExisting()) { await link.click(); await new Promise(r => setTimeout(r, 300)) }
        const el = await browser.$(sel)
        await el.waitForExist({ timeout: 8000 })
        if (!(await el.isDisplayed())) return
        if (await el.isEnabled()) { await el.click(); activatedNow.push(id); await new Promise(r => setTimeout(r, 250)) }
      } catch (e) { failed.push({ id, err: String(e).slice(0, 120) }) }
    }
    const fillSafe = async (route: string, id: string, value: string): Promise<void> => {
      const sel = '[data-testid="' + id + '"]'
      try {
        const link = await browser.$('.app-sidebar a[href="#/' + route + '"]')
        if (await link.isExisting()) { await link.click(); await new Promise(r => setTimeout(r, 300)) }
        const el = await browser.$(sel)
        await el.waitForExist({ timeout: 8000 })
        if (!(await el.isDisplayed())) return
        await el.setValue(value)
        activatedNow.push(id)
      } catch (e) { failed.push({ id, err: String(e).slice(0, 120) }) }
    }
    const hrefSafe = async (route: string, id: string, prefix = 'https://'): Promise<void> => {
      try {
        const el = await browser.$('[data-testid="' + id + '"]')
        await el.waitForExist({ timeout: 8000 })
        const href = await el.getAttribute('href')
        if (href && href.startsWith(prefix)) activatedNow.push(id)
        else failed.push({ id, err: 'unexpected href ' + href })
      } catch (e) { failed.push({ id, err: String(e).slice(0, 120) }) }
    }

    // workspace: filters, tabs, modes, tree, locale, copy
    await clickSafe('workspace', 'ws.filter.all')
    await clickSafe('workspace', 'ws.filter.empty')
    await clickSafe('workspace', 'ws.filter.issues')
    await clickSafe('workspace', 'ws.tab.source')
    await clickSafe('workspace', 'ws.tab.terms')
    await clickSafe('workspace', 'ws.tab.xml')
    await clickSafe('workspace', 'ws.mode.overview')
    await clickSafe('workspace', 'ws.mode.studio')
    await clickSafe('workspace', 'ws.mode.focus')
    await clickSafe('workspace', 'ws.tree-toggle')
    await clickSafe('workspace', 'ws.inspector-toggle')
    await clickSafe('workspace', 'ws.clear-filters')
    await clickSafe('workspace', 'ws.copy-key')
    await clickSafe('workspace', 'theme-toggle')
    await clickSafe('workspace', 'locale-toggle')
    await clickSafe('workspace', 'shell.nav-toggle')
    // settings
    await clickSafe('settings', 'settings.theme')
    await clickSafe('settings', 'settings.ui-density')
    await clickSafe('settings', 'settings.ui-locale')
    await clickSafe('settings', 'settings.ui-zoom')
    // projects
    await fillSafe('projects', 'projects.search', 'stage3')
    await clickSafe('projects', 'projects.selfloc-card')
    await clickSafe('projects', 'shell.new-project')
    // checks
    await clickSafe('checks', 'checks.rerun')
    await new Promise(r => setTimeout(r, 1000))
    // glossary
    await fillSafe('glossary', 'gl.filter', 'x')
    await fillSafe('glossary', 'gl.add-term', 'stage3term')
    await fillSafe('glossary', 'gl.add-translation', 'термин')
    await clickSafe('glossary', 'gl.add-submit')
    // tm
    await fillSafe('tm', 'tm.filter-query', 'x')
    await fillSafe('tm', 'tm.lookup-query', 'hello')
    await fillSafe('tm', 'tm.add-source', 'hello coverage')
    await fillSafe('tm', 'tm.add-target', 'привет')
    await clickSafe('tm', 'tm.add-submit')
    // chatbatch
    await clickSafe('chatbatch', 'cb.filter-untranslated')
    await clickSafe('chatbatch', 'cb.select-all')
    await clickSafe('chatbatch', 'cb.split-rows')
    await clickSafe('chatbatch', 'cb.split-chars')
    // existing / compare: text inputs only (native pickers are out of scope)
    await fillSafe('existing', 'ex.dir', '/tmp')
    await fillSafe('compare', 'cmp.old-dir', '/tmp')
    await fillSafe('compare', 'cmp.new-dir', '/tmp')
    // language manager
    await fillSafe('lm', 'lm.search', 'ru')
    await fillSafe('lm', 'lm.add-id', 's3lang')
    await fillSafe('lm', 'lm.add-native', 'Язык S3')
    await clickSafe('lm', 'lm.add-submit')
    // ai extras
    await clickSafe('aitranslate', 'ai.mode.auto')
    await fillSafe('aitranslate', 'ai.model', 'm1')
    // export: direct dir input (picker is native)
    await fillSafe('export', 'be.outdir', join(WORK_DIR, 'cov-export'))
    // about: toggles + link hrefs (no external navigation)
    await clickSafe('about', 'about.tech-toggle')
    await hrefSafe('about', 'about.action.docs')
    await hrefSafe('about', 'about.action.github')
    await hrefSafe('about', 'about.action.issues')
    await hrefSafe('about', 'about.action.support')
    await hrefSafe('about', 'about.support.bmc')
    await hrefSafe('about', 'about.support.kofi')
    await hrefSafe('about', 'about.support.sponsors')

    // The app must still be alive after touching every screen
    const sidebar = await browser.$('.app-sidebar')
    if (!(await sidebar.isExisting())) throw new Error('app died during control activation')

    console.log('ACTIVATED_NOW:', activatedNow.length, JSON.stringify(activatedNow))
    console.log('FAILED_ACTIVATIONS:', JSON.stringify(failed))
    // Persist the activation evidence next to the baseline inventory
    mkdirSync(EVIDENCE_DIR, { recursive: true })
    const base = JSON.parse(readFileSync(join(EVIDENCE_DIR, 'route-control-coverage.json'), 'utf8'))
    const merged = new Set<string>([...(base.totals.activatedList || [])])
    activatedNow.forEach(a => merged.add(a))
    const presentInteractive: string[] = base.totals.uncovered.concat(Object.keys(merged).length ? [...merged] : [])
    const allInteractive = new Set<string>()
    for (const v of Object.values(base.perRoute) as { uncovered: string[]; activatedHere: string[] }[]) {
      v.uncovered.forEach(u => allInteractive.add(u))
      v.activatedHere.forEach(u => allInteractive.add(u))
    }
    const activatedTotal = [...allInteractive].filter(i => merged.has(i))
    const pct = allInteractive.size === 0 ? 0 : Math.round((activatedTotal.length / allInteractive.size) * 1000) / 10
    writeFileSync(join(EVIDENCE_DIR, 'route-control-coverage.json'), JSON.stringify({
      ...base,
      activationPass: { activatedNow, failed },
      totals: {
        ...base.totals,
        activatedByJourneys: activatedTotal.length,
        coveragePercent: pct,
        uncovered: [...allInteractive].filter(i => !merged.has(i)).sort(),
      },
    }, null, 1))
    console.log('COVERAGE AFTER ACTIVATION:', activatedTotal.length + '/' + allInteractive.size + ' = ' + pct + '%')
  })


  it('state-gated pass: activates the controls that need a selected entry or a form context', async () => {
    const activatedNow: string[] = []
    const note: string[] = []

    // --- workspace WITH a selected entry ---
    const wsLink = await browser.$('.app-sidebar a[href="#/workspace"]')
    await wsLink.click()
    await waitExisting('.entry-list .entry-row', 30000)
    await (await browser.$$('.entry-list .entry-row'))[0].click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 15000)

    const tryClick = async (id: string): Promise<void> => {
      try {
        const el = await browser.$('[data-testid="' + id + '"]')
        await el.waitForExist({ timeout: 5000 })
        if (!(await el.isDisplayed())) { note.push(id + ':not-displayed'); return }
        if (!(await el.isEnabled())) { note.push(id + ':disabled'); return }
        await el.click()
        activatedNow.push(id)
        await new Promise(r => setTimeout(r, 250))
      } catch { note.push(id + ':absent') }
    }
    await tryClick('ws.entry.0')
    await tryClick('ws.tab.source')
    await tryClick('ws.tab.terms')
    await tryClick('ws.tab.xml')
    await tryClick('ws.copy-key')
    await tryClick('ws.editor-to-chatbatch')
    // come back from chatbatch if the control navigated
    {
      const back = await browser.$('.app-sidebar a[href="#/workspace"]')
      await back.click()
      const rows = await browser.$$('.entry-list .entry-row')
      if (rows.length > 0) { await rows[0].click(); await waitExisting('[data-testid="ws.editor-textarea"]', 15000) }
    }
    await tryClick('ws.inspector-toggle')
    await tryClick('ws.inspector-close')
    await tryClick('ws.tree-toggle')
    await tryClick('ws.tree.view-type')
    await tryClick('ws.tree.view-file')
    await tryClick('ws.tree.keyed')
    // target locale: a select change on the selected entry context
    try {
      const sel = await browser.$('[data-testid="ws.target-locale"]')
      await sel.waitForExist({ timeout: 5000 })
      if (await sel.isDisplayed()) {
        await sel.click()
        await sel.selectByIndex(0)
        activatedNow.push('ws.target-locale')
      }
    } catch { note.push('ws.target-locale:absent') }
    await tryClick('sidebar.project-switcher')

    // --- checks: rerun then the (now-enabled) report download ---
    {
      const cLink = await browser.$('.app-sidebar a[href="#/checks"]')
      await cLink.click()
      await waitExisting('[data-testid="checks.rerun"]', 20000)
      await (await browser.$('[data-testid="checks.rerun"]')).click()
      await new Promise(r => setTimeout(r, 1200))
      const rep = await browser.$('[data-testid="checks.report"]')
      if ((await rep.isExisting()) && (await rep.isEnabled())) {
        await rep.click()
        activatedNow.push('checks.report')
      } else note.push('checks.report:not-ready')
    }

    // --- diagnostics: outdir + run ---
    {
      await (await browser.$('.app-sidebar a[href="#/settings"]')).click()
      await new Promise(r => setTimeout(r, 300))
      await (await browser.$('[data-testid="palette.open"]')).click()
      const pin = await browser.$('[data-testid="palette.input"]')
      await pin.waitForExist({ timeout: 15000 })
      await pin.setValue('Диагностика')
      await waitFor(async () => {
        const items = await browser.$$('.palette-item')
        if (items.length === 0) return false
        await items[0].click()
        return true
      }, 8000, 300)
      const outdir = await browser.$('[data-testid="diag.outdir"]')
      if ((await outdir.isExisting())) {
        await outdir.setValue(join(WORK_DIR, 'diag'))
        activatedNow.push('diag.outdir')
      }
      const runBtn = await browser.$('[data-testid="diag.run"]')
      if ((await runBtn.isExisting()) && (await runBtn.isEnabled())) {
        await runBtn.click()
        activatedNow.push('diag.run')
        await new Promise(r => setTimeout(r, 1500))
      }
    }

    // --- providers form context: local flag + probes on the empty new form ---
    {
      await (await browser.$('[data-testid="palette.open"]')).click()
      const pin2 = await browser.$('[data-testid="palette.input"]')
      await pin2.waitForExist({ timeout: 15000 })
      await pin2.setValue('Провайдеры')
      await waitFor(async () => {
        const items = await browser.$$('.palette-item')
        if (items.length === 0) return false
        await items[0].click()
        return true
      }, 8000, 300)
      await (await browser.$('[data-testid="prov.card.new"]')).click()
      await waitExisting('[data-testid="prov.detail"]', 10000)
      await tryClick('prov.local')
      await tryClick('prov.connect.new')
      await tryClick('prov.test.new')
    }

    const sidebar = await browser.$('.app-sidebar')
    if (!(await sidebar.isExisting())) throw new Error('app died during the state-gated pass')
    console.log('STATE_GATED_ACTIVATED:', JSON.stringify(activatedNow))
    console.log('STATE_GATED_NOTES:', JSON.stringify(note))
    // merge into the evidence file
    mkdirSync(EVIDENCE_DIR, { recursive: true })
    const base = JSON.parse(readFileSync(join(EVIDENCE_DIR, 'route-control-coverage.json'), 'utf8'))
    const prev: string[] = base.activationPass?.activatedNow ?? []
    const pass3 = Array.from(new Set(prev.concat(activatedNow)))
    const journey: string[] = base.journeyActivated ?? ['wizard.open','wizard.path-input','wizard.next','wizard.version','wizard.overlay','ws.editor-textarea','ws.editor-save','ws.tab.output','ws.output-pre','be.outdir','be.export','projects.project-card','ai.modes','ai.mode.auto','ai.mode.chatgpt','ai.mode.byo','ai.mode.local','ai.mode.noapi','ai.instance','ai.run','ai.chatgpt-signin','ai.provider','ai.open-chatbatch','ai.auto-key-hint','ai.auto-key-link','ai.job','ai.job-state','palette.open','palette.input','prov.card.new','prov.preset','prov.label','prov.model.new','prov.baseurl.new','prov.secret.new','prov.save.new','prov.catalog']
    const allInteractive = new Set<string>()
    for (const v of Object.values(base.perRoute) as { uncovered: string[]; activatedHere: string[] }[]) {
      v.uncovered.forEach(u => allInteractive.add(u))
      v.activatedHere.forEach(u => allInteractive.add(u))
    }
    const coveredSet = new Set<string>([...journey, ...pass3])
    const covered = [...allInteractive].filter(i => coveredSet.has(i))
    const pct = allInteractive.size === 0 ? 0 : Math.round((covered.length / allInteractive.size) * 1000) / 10
    writeFileSync(join(EVIDENCE_DIR, 'route-control-coverage.json'), JSON.stringify({
      ...base,
      stateGatedPass: { activatedNow, note },
      totals: {
        ...base.totals,
        activatedTotal: covered.length,
        coveragePercent: pct,
        uncovered: [...allInteractive].filter(i => !coveredSet.has(i)).sort(),
      },
    }, null, 1))
    console.log('COVERAGE AFTER STATE-GATED PASS:', covered.length + '/' + allInteractive.size + ' = ' + pct + '%')
  })

  after(() => {
    rmSync(WORK_DIR, { recursive: true, force: true })
  })
})
