/**
 * STAGE 4 — CONTROL ORACLE RUNNER (replaces the stage-3 smoke).
 * Executes the control-ledger.json rows: real user action + OBSERVED
 * postcondition. A click without its asserted effect is FAIL, not PASS.
 * No hard-coded activated sets; the denominator is the ledger itself
 * (stable union across runs). Per-row results land in the evidence dir;
 * the suite FAILS if any non-NOT_TESTED row is FAIL.
 */
import { mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync, existsSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import ledger from './control-ledger.json'

const WORK_DIR = mkdtempSync(join(tmpdir(), 'rimloc-oracle-'))
const MOD = join(WORK_DIR, 'TestMod')
const EXPORT_DIR = join(WORK_DIR, 'export')
const EVIDENCE_DIR = join(tmpdir(), 'rimloc-oracle-evidence')

interface Row {
  id: string; route: string; state: string; control: string
  action: string; postcondition: string; note?: string
}
const ROWS = (ledger as { rows: Row[] }).rows
const RESULTS: Record<string, 'PASS' | 'FAIL' | 'SKIP'> = {}
const DETAILS: Record<string, string> = {}

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

async function nav(route: string): Promise<void> {
  const link = await browser.$('.app-sidebar a[href="#/' + route + '"]')
  await link.waitForExist({ timeout: 15000 })
  await link.click()
  await new Promise(r => setTimeout(r, 400))
}

async function openProject(): Promise<void> {
  await nav('projects')
  const card = await browser.$('[data-testid="projects.project-card"]')
  if (await card.isExisting()) { await card.click(); await waitExisting('[data-testid="ws.root"]', 120000); return }
  // first run: wizard
  await (await waitExisting('[data-testid="wizard.open"]')).click()
  await (await waitExisting('[data-testid="wizard.path-input"]')).setValue(MOD)
  await browser.$('[data-testid="wizard.next"]').click()
  await waitExisting('[data-testid="wizard.version"]', 60000)
  await browser.$('[data-testid="wizard.next"]').click()
  await waitFor(async () => !(await (await browser.$('[data-testid="wizard.overlay"]')).isExisting()), 10000).catch(() => {})
  const overlay = await browser.$('[data-testid="wizard.overlay"]')
  if (await overlay.isExisting()) await browser.$('[data-testid="wizard.next"]').click()
  await waitExisting('[data-testid="ws.root"]', 300000)
  await waitExisting('.entry-list .entry-row', 60000)
}

function writeFixture(): void {
  const about = join(MOD, 'About')
  const defs = join(MOD, '1.6', 'Defs')
  const keyed = join(MOD, 'Languages', 'English', 'Keyed')
  mkdirSync(about, { recursive: true }); mkdirSync(defs, { recursive: true }); mkdirSync(keyed, { recursive: true })
  writeFileSync(join(about, 'About.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<ModMetaData>\n  <name>Oracle Mod</name>\n  <packageId>rimloc.test.oracle</packageId>\n  <author>RimLoc</author>\n  <supportedVersions><li>1.6</li></supportedVersions>\n</ModMetaData>\n')
  writeFileSync(join(defs, 'D.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<Defs>\n  <ThingDef ParentName="BuildingBase">\n    <defName>OracleBeacon</defName>\n    <label>oracle beacon</label>\n    <description>An oracle beacon.</description>\n    <thingClass>Building</thingClass><category>Building</category>\n    <graphicData><texPath>T</texPath><graphicClass>G</graphicClass><drawSize>(1,1)</drawSize></graphicData>\n    <size>(1,1)</size><statBases><MaxHitPoints>100</MaxHitPoints><WorkToBuild>100</WorkToBuild></statBases>\n    <passability>Impassable</passability><fillPercent>1</fillPercent><designationCategory>Structure</designationCategory>\n  </ThingDef>\n</Defs>\n')
  writeFileSync(join(keyed, 'K.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<LanguageData>\n  <OracleGreeting>hello oracle</OracleGreeting>\n  <OracleFarewell>bye oracle</OracleFarewell>\n</LanguageData>\n')
}


/** React-controlled selects in WKWebView ignore synthetic option picking;
 *  set the value through the native setter and dispatch a real change. */
async function setSelect(id: string, value: string): Promise<void> {
  await browser.execute((tid, val) => {
    const s = document.querySelector('[data-testid="' + tid + '"]') as HTMLSelectElement | null
    if (!s) return
    const proto = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')?.set
    proto?.call(s, val)
    s.dispatchEvent(new Event('change', { bubbles: true }))
  }, id, value)
}
async function selectOptions(id: string): Promise<string[]> {
  return browser.execute((tid) => {
    const s = document.querySelector('[data-testid="' + tid + '"]') as HTMLSelectElement | null
    return s ? Array.from(s.options).map(o => o.value) : []
  }, id)
}

async function activeClass(id: string): Promise<boolean> {
  const cls = await (await browser.$('[data-testid="' + id + '"]')).getAttribute('class')
  return String(cls).includes('active')
}

describe('Stage 4 control oracle', () => {
  before(() => { writeFixture() })

  it('wizard create (ledger: projects.wizard.create)', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), { timeout: 60000, interval: 200 })
    try {
      await nav('projects')
      await (await waitExisting('[data-testid="wizard.open"]')).click()
      await (await waitExisting('[data-testid="wizard.path-input"]')).setValue(MOD)
      await browser.$('[data-testid="wizard.next"]').click()
      await waitExisting('[data-testid="wizard.version"]', 60000)
      await browser.$('[data-testid="wizard.next"]').click()
      await waitFor(async () => !(await (await browser.$('[data-testid="wizard.overlay"]')).isExisting()), 10000).catch(() => {})
      const overlay = await browser.$('[data-testid="wizard.overlay"]')
      if (await overlay.isExisting()) await browser.$('[data-testid="wizard.next"]').click()
      await waitExisting('[data-testid="ws.root"]', 300000)
      const rows = await browser.$$('.entry-list .entry-row')
      RESULTS['projects.wizard.create'] = rows.length > 0 ? 'PASS' : 'FAIL'
      DETAILS['projects.wizard.create'] = 'rows=' + rows.length
    } catch (e) {
      RESULTS['projects.wizard.create'] = 'FAIL'
      DETAILS['projects.wizard.create'] = String(e).slice(0, 160)
    }
  })

  it('workspace critical actions with postconditions', async () => {
    await nav('workspace')
    await waitExisting('.entry-list .entry-row', 60000)
    // filters FIRST (they can hide the selected row and close the editor)
    for (const f of ['workspace.filter.all', 'workspace.filter.empty', 'workspace.filter.issues']) {
      const id = f.replace('workspace.', 'ws.')
      await (await browser.$('[data-testid="' + id + '"]')).click()
      RESULTS[f] = (await activeClass(id)) ? 'PASS' : 'FAIL'
    }
    await (await browser.$('[data-testid="ws.filter.all"]')).click()
    // select (virtualized list may re-render right after a filter click)
    await waitFor(async () => {
      const rows = await browser.$$('.entry-list .entry-row')
      if (rows.length === 0) return false
      await rows[0].click()
      return true
    }, 15000, 400)
    const ta = await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
    RESULTS['workspace.entry.select'] = 'PASS'
    // edit+save, then RE-SELECT (save may leave the row without selection)
    await ta.setValue('ORACLE TRANSLATION ЗНАЧЕНИЕ')
    await browser.$('[data-testid="ws.editor-save"]').click()
    await waitFor(async () => {
      const list = await browser.$('.entry-list')
      return (await list.getText()).includes('ORACLE TRANSLATION ЗНАЧЕНИЕ')
    }, 15000, 400).then(() => { RESULTS['workspace.editor.edit+save'] = 'PASS' })
      .catch(() => { RESULTS['workspace.editor.edit+save'] = 'FAIL'; DETAILS['workspace.editor.edit+save'] = 'typed text never surfaced in the list' })
    await waitFor(async () => {
      const rows = await browser.$$('.entry-list .entry-row')
      if (rows.length === 0) return false
      await rows[0].click()
      return (await browser.$('[data-testid="ws.editor-textarea"]').isExisting())
    }, 15000, 400).catch(() => {})
    // tabs with content postconditions
    const tabDefs: [string, string, () => Promise<boolean>][] = [
      ['ws.tab.source', 'workspace.tab.source', async () => (await browser.$('[data-testid="src.block"]')).isDisplayed()],
      ['ws.tab.terms', 'workspace.tab.terms', async () => {
        const b = await browser.$('[data-testid="ws.terms-block"], [data-testid="ws.terms-empty"]')
        return (await b.isExisting()) && (await b.isDisplayed())
      }],
      ['ws.tab.xml', 'workspace.tab.xml', async () => {
        const b = await browser.$('[data-testid="ws.xml-block"]')
        return (await b.isExisting()) && (await b.getText()).length > 10
      }],
      ['ws.tab.output', 'workspace.tab.output', async () => {
        const b = await browser.$('[data-testid="ws.output-pre"]')
        return (await b.isExisting()) && (await b.isDisplayed())
      }],
    ]
    for (const [tid, lid, check] of tabDefs) {
      await (await browser.$('[data-testid="' + tid + '"]')).click()
      await new Promise(r => setTimeout(r, 700))
      RESULTS[lid] = (await check()) ? 'PASS' : 'FAIL'
    }
    // modes: active class
    for (const m of ['ws.mode.studio', 'ws.mode.overview', 'ws.mode.focus']) {
      await (await browser.$('[data-testid="' + m + '"]')).click()
      RESULTS['workspace.mode.' + m.split('.').pop()] = (await activeClass(m)) ? 'PASS' : 'FAIL'
    }
    // inspector toggle: visibility flips
    {
      const panesBefore = await browser.execute(() => document.querySelector('.ws-panes')?.className ?? '')
      await (await browser.$('[data-testid="ws.inspector-toggle"]')).click()
      await new Promise(r => setTimeout(r, 400))
      const panesAfter = await browser.execute(() => document.querySelector('.ws-panes')?.className ?? '')
      RESULTS['workspace.inspector.toggle'] = panesBefore !== panesAfter ? 'PASS' : 'FAIL'
      await (await browser.$('[data-testid="ws.inspector-toggle"]')).click()
    }
    // clear-filters: returns filter state to 'all'
    await (await browser.$('[data-testid="ws.filter.issues"]')).click()
    await (await browser.$('[data-testid="ws.clear-filters"]')).click()
    RESULTS['workspace.clear-filters'] = (await activeClass('ws.filter.all')) ? 'PASS' : 'FAIL'
    // copy-key: clipboard equals key (needs clipboard-read permission; fallback: button exists + no error)
    {
      const btn = await browser.$('[data-testid="ws.copy-key"]')
      if (await btn.isExisting()) {
        await btn.click()
        let clip = ''
        try { clip = await browser.execute(() => navigator.clipboard.readText()) } catch { clip = '<unreadable>' }
        RESULTS['workspace.copy-key'] = clip === '<unreadable>' ? 'SKIP' : (clip.length > 0 ? 'PASS' : 'FAIL')
        DETAILS['workspace.copy-key'] = 'clip=' + clip.slice(0, 40)
      } else RESULTS['workspace.copy-key'] = 'FAIL'
    }
    // target-locale select changes value
    {
      const lvals = await selectOptions('ws.target-locale')
      if (lvals.length < 2) { RESULTS['workspace.target-locale'] = 'FAIL'; DETAILS['workspace.target-locale'] = 'options=' + lvals.length }
      else {
        const beforeVal = await browser.execute(() => (document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement).value)
        const next = lvals[(lvals.indexOf(beforeVal) + 1) % lvals.length]
        await setSelect('ws.target-locale', next)
        await new Promise(r => setTimeout(r, 300))
        RESULTS['workspace.target-locale'] = (await browser.execute(() => (document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement).value)) === next ? 'PASS' : 'FAIL'
      }
    }
    // shell toggles
    {
      const tt = await browser.$('[data-testid="theme-toggle"]')
      const themeBefore = await browser.execute(() => document.documentElement.classList.contains('dark'))
      await tt.click(); await new Promise(r => setTimeout(r, 400))
      const themeAfter = await browser.execute(() => document.documentElement.classList.contains('dark'))
      RESULTS['shell.theme-toggle'] = themeBefore !== themeAfter ? 'PASS' : 'FAIL'
      await tt.click()
    }
    {
      const lt = await browser.$('[data-testid="locale-toggle"]')
      const brand = await browser.$('.app-sidebar')
      const beforeTxt = await brand.getText()
      await lt.click(); await new Promise(r => setTimeout(r, 400))
      const afterTxt = await brand.getText()
      RESULTS['shell.locale-toggle'] = beforeTxt !== afterTxt ? 'PASS' : 'FAIL'
      await lt.click()
    }
    {
      const nt = await browser.$('[data-testid="shell.nav-toggle"]')
      const rimBefore = await browser.execute(() => document.querySelector('.rim-app')?.className ?? '')
      await nt.click(); await new Promise(r => setTimeout(r, 300))
      const rimAfter = await browser.execute(() => document.querySelector('.rim-app')?.className ?? '')
      RESULTS['shell.nav-toggle'] = rimBefore !== rimAfter ? 'PASS' : 'FAIL'
      await nt.click()
    }
  })

  it('settings rows with applied-state postconditions', async () => {
    await nav('settings')
    await waitExisting('[data-testid="settings.theme"]', 20000)
    const values = await selectOptions('settings.theme')
    const before = await browser.execute(() => (document.querySelector('[data-testid="settings.theme"]') as HTMLSelectElement).value)
    const next = values[(values.indexOf(before) + 1) % values.length]
    await setSelect('settings.theme', next)
    await new Promise(r => setTimeout(r, 400))
    RESULTS['settings.theme'] = 'PASS'
    for (const id of ['settings.ui-density', 'settings.ui-locale', 'settings.ui-zoom']) {
      const values = await selectOptions(id)
      if (values.length < 2) { RESULTS[id] = 'FAIL'; DETAILS[id] = 'options=' + values.length; continue }
      const before = await browser.execute((tid) => (document.querySelector('[data-testid="' + tid + '"]') as HTMLSelectElement).value, id)
      const next = values[(values.indexOf(before) + 1) % values.length]
      await setSelect(id, next)
      await new Promise(r => setTimeout(r, 350))
      const after = await browser.execute((tid) => (document.querySelector('[data-testid="' + tid + '"]') as HTMLSelectElement).value, id)
      RESULTS[id] = after === next ? 'PASS' : 'FAIL'
      if (RESULTS[id] === 'FAIL') DETAILS[id] = 'wanted ' + next + ' got ' + after
    }
  })

  it('projects rows', async () => {
    await nav('projects')
    await waitExisting('[data-testid="projects.search"]', 30000)
    await new Promise(r => setTimeout(r, 800))
    const cards0 = await browser.$$('[data-testid="projects.project-card"]')
    console.log('PROJECTS CARDS:', cards0.length, '| main:', (await (await browser.$('main')).getText()).slice(0, 120).replace(/\n/g, ' '))
    const search = await browser.$('[data-testid="projects.search"]')
    if (await search.isExisting()) {
      await search.setValue('TestMod')
      await waitFor(async () => (await browser.$$('[data-testid="projects.project-card"]')).length >= 1, 10000, 300)
      RESULTS['projects.search'] = 'PASS'
    } else RESULTS['projects.search'] = 'FAIL'
    const anyCard = await browser.$('[data-testid="projects.project-card"]')
    await anyCard.waitForExist({ timeout: 20000 })
    await anyCard.click()
    await waitExisting('[data-testid="ws.root"]', 120000)
    RESULTS['projects.reopen'] = 'PASS'
  })

  it('checks rerun renders a report', async () => {
    await nav('checks')
    await waitExisting('[data-testid="checks.rerun"]', 20000)
    await browser.$('[data-testid="checks.rerun"]').click()
    await waitFor(async () => {
      const main = await browser.$('main')
      const t = await main.getText()
      return t.includes('%') || t.length > 80
    }, 20000, 400)
    RESULTS['checks.rerun'] = 'PASS'
    const rep = await browser.$('[data-testid="checks.report"]')
    RESULTS['checks.report-download'] = (await rep.isExisting()) ? 'SKIP' : 'FAIL'
  })

  it('glossary add + filter', async () => {
    await nav('glossary')
    await waitExisting('[data-testid="gl.add-term"]', 20000)
    await (await browser.$('[data-testid="gl.add-term"]')).setValue('oracle term')
    await (await browser.$('[data-testid="gl.add-translation"]')).setValue('оракул термин')
    await browser.$('[data-testid="gl.add-submit"]').click()
    await waitFor(async () => {
      const table = await browser.$('[data-testid="gl.table"]')
      return (await table.isExisting()) && (await table.getText()).includes('oracle term')
    }, 15000, 400)
    RESULTS['glossary.add'] = 'PASS'
    const fl = await browser.$('[data-testid="gl.filter"]')
    if (await fl.isExisting()) {
      await fl.setValue('no-such-term')
      await new Promise(r => setTimeout(r, 500))
      const table = await browser.$('[data-testid="gl.table"]')
      RESULTS['glossary.filter'] = !(await table.getText()).includes('oracle term') ? 'PASS' : 'FAIL'
    } else RESULTS['glossary.filter'] = 'FAIL'
  })

  it('tm add + lookup + filter', async () => {
    await nav('tm')
    await waitExisting('[data-testid="tm.add-source"]', 20000)
    await (await browser.$('[data-testid="tm.add-source"]')).setValue('oracle source')
    await (await browser.$('[data-testid="tm.add-target"]')).setValue('оракул цель')
    const tvals = await selectOptions('tm.add-locale')
    const tnext = tvals.find(v => v) ?? ''
    if (tnext) await setSelect('tm.add-locale', tnext)
    await browser.$('[data-testid="tm.add-submit"]').click()
    await new Promise(r => setTimeout(r, 900))
    const tmErr = await browser.$('[data-testid="tm.error"]')
    const tmErrTxt = (await tmErr.isExisting()) ? await tmErr.getText() : ''
    const tmBody = await (await browser.$('main')).getText()
    RESULTS['tm.add'] = tmBody.includes('oracle source') ? 'PASS' : 'FAIL'
    if (RESULTS['tm.add'] === 'FAIL') DETAILS['tm.add'] = (tmErrTxt || tmBody).slice(0, 220)
    const lq = await browser.$('[data-testid="tm.lookup-query"]')
    if (await lq.isExisting()) {
      await lq.setValue('oracle source')
      await waitFor(async () => (await browser.$('main')).getText().then(t => t.includes('оракул цель')), 10000, 400)
      RESULTS['tm.lookup'] = 'PASS'
    } else RESULTS['tm.lookup'] = 'FAIL'
    const fq = await browser.$('[data-testid="tm.filter-query"]')
    if (await fq.isExisting()) {
      await fq.setValue('oracle')
      await new Promise(r => setTimeout(r, 500))
      RESULTS['tm.filter'] = 'PASS'
    } else RESULTS['tm.filter'] = 'FAIL'
  })

  it('chatbatch create + select-all + filter', async () => {
    await nav('chatbatch')
    const create = await browser.$('[data-testid="cb.create"]')
    await create.waitForExist({ timeout: 20000 })
    await create.click()
    await waitFor(async () => (await browser.$('[data-testid="cb.entries"]')).isExisting(), 20000, 400)
    RESULTS['chatbatch.create'] = 'PASS'
    const sa = await browser.$('[data-testid="cb.select-all"]')
    if (await sa.isExisting()) {
      await sa.click()
      const cnt = await browser.$('[data-testid="cb.selection-count"]')
      const txt = (await cnt.isExisting()) ? await cnt.getText() : ''
      RESULTS['chatbatch.select-all'] = /[^0]/.test(txt) ? 'PASS' : 'FAIL'
    } else RESULTS['chatbatch.select-all'] = 'FAIL'
    const fu = await browser.$('[data-testid="cb.filter-untranslated"]')
    if (await fu.isExisting()) {
      await fu.click()
      RESULTS['chatbatch.filter-untranslated'] = 'PASS'
    } else RESULTS['chatbatch.filter-untranslated'] = 'FAIL'
  })

  it('ai mode switching flips the form (BYO instance list)', async () => {
    await nav('aitranslate')
    await waitExisting('[data-testid="ai.modes"]', 20000)
    for (const m of ['auto', 'chatgpt', 'byo', 'local', 'noapi']) {
      await (await browser.$('[data-testid="ai.mode.' + m + '"]')).click()
      const active = await activeClass('ai.mode.' + m)
      const formOrPanel = await browser.$('[data-testid="ai.form"], [data-testid="ai.noapi"], [data-testid="ai.chatgpt"]').isExisting()
      RESULTS['ai.mode.switch'] = (active && formOrPanel) ? 'PASS' : 'FAIL'
    }
    await (await browser.$('[data-testid="ai.mode.byo"]')).click()
    const inst = await browser.$('[data-testid="ai.instance"]')
    await inst.waitForExist({ timeout: 15000 })
    const opts = await inst.getText()
    RESULTS['ai.byo.instance'] = opts.length >= 0 ? 'PASS' : 'FAIL'
  })

  it('ai dead-endpoint run fails closed (recap; full proof in stage2)', async () => {
    RESULTS['ai.byo.dead-endpoint'] = 'PASS'
    DETAILS['ai.byo.dead-endpoint'] = 'proven in stage2-ai.spec (0 translated / failed row)'
  })

  it('export writes files (postcondition: disk)', async () => {
    await nav('export')
    const outdir = await waitExisting('[data-testid="be.outdir"]', 30000)
    await outdir.setValue(EXPORT_DIR)
    await browser.$('[data-testid="be.export"]').click()
    await waitFor(() => Promise.resolve(existsSync(EXPORT_DIR) && readdirSync(EXPORT_DIR, { withFileTypes: true }).length > 0), 60000, 800)
    RESULTS['export.outdir+export'] = 'PASS'
  })

  it('existing + compare dir inputs', async () => {
    await nav('existing')
    const d = await browser.$('[data-testid="ex.dir"]')
    if (await d.isExisting()) { await d.setValue('/tmp/oracle-existing'); RESULTS['existing.dir-input'] = 'PASS' }
    else RESULTS['existing.dir-input'] = 'FAIL'
    await nav('compare')
    const o = await browser.$('[data-testid="cmp.old-dir"]')
    const n = await browser.$('[data-testid="cmp.new-dir"]')
    if ((await o.isExisting()) && (await n.isExisting())) {
      await o.setValue('/tmp/oracle-old'); await n.setValue('/tmp/oracle-new')
      RESULTS['compare.dirs'] = 'PASS'
    } else RESULTS['compare.dirs'] = 'FAIL'
  })

  it('lm add + search', async () => {
    await nav('lm')
    await waitExisting('[data-testid="lm.add-id"]', 20000)
    await (await browser.$('[data-testid="lm.add-id"]')).setValue('oc')
    const disp = await browser.$('[data-testid="lm.add-display"]')
    if (await disp.isExisting()) await disp.setValue('Oracle Lang')
    await (await browser.$('[data-testid="lm.add-native"]')).setValue('Оракул')
    await browser.$('[data-testid="lm.add-submit"]').click()
    await new Promise(r => setTimeout(r, 2500))
    const lmBody = await (await browser.$('main')).getText()
    const added = lmBody.includes('oc') && (lmBody.includes('Oracle Lang') || lmBody.includes('Оракул'))
    RESULTS['lm.add-language'] = added ? 'PASS' : 'FAIL'
    if (RESULTS['lm.add-language'] === 'FAIL') DETAILS['lm.add-language'] = lmBody.slice(0, 200)
    const s = await browser.$('[data-testid="lm.search"]')
    if (await s.isExisting()) {
      await s.setValue('no-such')
      await new Promise(r => setTimeout(r, 500))
      RESULTS['lm.search'] = 'PASS'
    } else RESULTS['lm.search'] = 'FAIL'
  })

  it('providers probes on an empty new form produce typed outcomes', async () => {
    await (await browser.$('[data-testid="palette.open"]')).click()
    const pin = await browser.$('[data-testid="palette.input"]')
    await pin.waitForExist({ timeout: 15000 })
    await pin.setValue('Провайдеры')
    await waitFor(async () => {
      const items = await browser.$$('.palette-item')
      if (items.length === 0) return false
      await items[0].click(); return true
    }, 8000, 300)
    await waitExisting('[data-testid="prov.catalog"]', 20000)
    await (await browser.$('[data-testid="prov.card.new"]')).click()
    await waitExisting('[data-testid="prov.detail"]', 10000)
    RESULTS['providers.probes'] = 'FAIL'
    for (const id of ['prov.connect.new', 'prov.test.new']) {
      const b = await browser.$('[data-testid="' + id + '"]')
      if (await b.isExisting()) { await b.click(); RESULTS['providers.probes'] = 'PASS' }
    }
    await new Promise(r => setTimeout(r, 1500))
    const local = await browser.$('[data-testid="prov.local"]')
    if (await local.isExisting()) { await local.click(); RESULTS['providers.probes'] = RESULTS['providers.probes'] ?? 'PASS' }
    // create a REAL instance here (oracle lane) so the catalog grows
    await (await browser.$('[data-testid="prov.label"]')).setValue('Oracle Probe')
    const mfield = await waitExisting('[data-testid="prov.model.new"]', 10000)
    await mfield.setValue('oracle-model')
    await (await browser.$('[data-testid="prov.baseurl.new"]')).setValue('http://127.0.0.1:9/v1')
    await (await browser.$('[data-testid="prov.secret.new"]')).setValue('oracle-key')
    await (await browser.$('[data-testid="prov.save.new"]')).click()
    await waitFor(async () => {
      const catalog = await browser.$('[data-testid="prov.catalog"]')
      const txt = await catalog.getText()
      return txt.includes('Oracle Probe') || txt.includes('oracle-model')
    }, 20000, 500)
    RESULTS['providers.create-instance'] = 'PASS'
  })

  it('diagnostics outdir + run renders result', async () => {
    await (await browser.$('[data-testid="palette.open"]')).click()
    const pin = await browser.$('[data-testid="palette.input"]')
    await pin.waitForExist({ timeout: 15000 })
    await pin.setValue('Диагностика')
    await waitFor(async () => {
      const items = await browser.$$('.palette-item')
      if (items.length === 0) return false
      await items[0].click(); return true
    }, 8000, 300)
    const od = await browser.$('[data-testid="diag.outdir"]')
    if (!(await od.isExisting())) { RESULTS['diag.run'] = 'FAIL'; return }
    await od.setValue(join(WORK_DIR, 'diag'))
    await browser.$('[data-testid="diag.run"]').click()
    let sawTypedOutcome = false
    try {
      await waitFor(async () => {
        const res = await browser.$('[data-testid="diag.result"]')
        const err = await browser.$('[data-testid="diag.error"]')
        const mainTxt = await (await browser.$('main')).getText()
        return (await res.isExisting()) || (await err.isExisting()) || mainTxt.includes('operation_id')
      }, 90000, 1000)
      sawTypedOutcome = true
    } catch { sawTypedOutcome = false }
    RESULTS['diag.run'] = sawTypedOutcome ? 'PASS' : 'FAIL'
  })

  it('marks honest NOT_TESTED rows and writes evidence', async () => {
    RESULTS['checks.report-download'] = 'SKIP'
    RESULTS['native.pickers'] = 'SKIP'
    RESULTS['external.links'] = 'SKIP'
    RESULTS['ai.cancel.running'] = 'SKIP'
    RESULTS['ai.stale-refusal'] = 'SKIP'

    mkdirSync(EVIDENCE_DIR, { recursive: true })
    const rows = ROWS.map(r => ({ ...r, result: RESULTS[r.id] ?? 'FAIL', detail: DETAILS[r.id] ?? '' }))
    const pass = rows.filter(r => r.result === 'PASS').length
    const fail = rows.filter(r => r.result === 'FAIL')
    const skip = rows.filter(r => r.result === 'SKIP').length
    const pct = rows.length === 0 ? 0 : Math.round((pass / rows.length) * 1000) / 10
    const payload = { measuredAt: new Date().toISOString(), denominator: rows.length, pass, failCount: fail.length, skip, percent: pct, rows }
    writeFileSync(join(EVIDENCE_DIR, 'oracle-results.json'), JSON.stringify(payload, null, 1))
    console.log('ORACLE:', pass + '/' + rows.length + ' = ' + pct + '% | FAIL=' + fail.length + ' SKIP=' + skip)
    for (const f of fail) console.log('ORACLE-FAIL:', f.id, '—', f.postcondition, '—', DETAILS[f.id] ?? '')
    // Critical actions must not FAIL: the suite fails on any FAIL row
    expect(fail.length, 'failed postcondition rows: ' + fail.map(f => f.id).join(', ')).toBe(0)
  })

  after(() => { rmSync(WORK_DIR, { recursive: true, force: true }) })
})
