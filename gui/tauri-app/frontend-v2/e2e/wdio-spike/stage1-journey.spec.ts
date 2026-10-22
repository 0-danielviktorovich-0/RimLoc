/**
 * STAGE 1 — CORE PRODUCT ACCEPTANCE JOURNEY (packaged automation artifact).
 * One continuous user story, every step asserted:
 * wizard create → translate two entries → save → reopen via sidebar →
 * translation persisted → checks screen → output XML shows the RU text →
 * export writes LanguageData files verified on disk.
 * Structured as a minimal delta from the proven xml-verify lane.
 */
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const WORK_DIR = mkdtempSync(join(tmpdir(), 'rimloc-stage1-'))
const MOD = join(WORK_DIR, 'TestMod')
const EXPORT_DIR = join(WORK_DIR, 'export')
const RU_LABEL = 'Маяк проверки владельца'
const RU_GREETING = 'Привет, канареечный мир'

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
  const defs = join(MOD, '1.6', 'Defs')
  const keyed = join(MOD, 'Languages', 'English', 'Keyed')
  mkdirSync(about, { recursive: true })
  mkdirSync(defs, { recursive: true })
  mkdirSync(keyed, { recursive: true })
  writeFileSync(
    join(about, 'About.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<ModMetaData>\n  <name>Stage1 Journey Mod</name>\n  <packageId>rimloc.test.stage1</packageId>\n  <author>RimLoc</author>\n  <supportedVersions><li>1.6</li></supportedVersions>\n  <description>Fixture for the stage-1 journey.</description>\n</ModMetaData>\n',
  )
  writeFileSync(
    join(defs, 'JourneyBuilding.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<Defs>\n  <ThingDef ParentName="BuildingBase">\n    <defName>Stage1Beacon</defName>\n    <label>stage one beacon</label>\n    <description>A test building for the journey.</description>\n    <thingClass>Building</thingClass>\n    <category>Building</category>\n    <graphicData>\n      <texPath>Things/Building/Misc/Cooler</texPath>\n      <graphicClass>Graphic_Single</graphicClass>\n      <drawSize>(1,1)</drawSize>\n    </graphicData>\n    <size>(1,1)</size>\n    <statBases><MaxHitPoints>100</MaxHitPoints><WorkToBuild>100</WorkToBuild></statBases>\n    <passability>Impassable</passability>\n    <fillPercent>1</fillPercent>\n    <designationCategory>Structure</designationCategory>\n  </ThingDef>\n</Defs>\n',
  )
  writeFileSync(
    join(keyed, 'JourneyStrings.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<LanguageData>\n  <Stage1Greeting>Hello canary world</Stage1Greeting>\n  <Stage1Farewell>Goodbye canary world</Stage1Farewell>\n</LanguageData>\n',
  )
}

function walkXml(d: string): string[] {
  try {
    return readdirSync(d, { withFileTypes: true }).flatMap(e =>
      e.isDirectory() ? walkXml(join(d, e.name)) : [join(d, e.name)],
    )
  } catch {
    return []
  }
}

describe('Stage 1 core journey', () => {
  before(() => { writeFixture() })

  it('wizard creates the project', async () => {
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
  })

  it('translates the Def label and the Keyed greeting, saving both', async () => {
    await waitFor(async () => {
      const rows = await browser.$$('.entry-list .entry-row')
      for (const r of rows) {
        const txt = (await r.getText()).toLowerCase()
        if (txt.includes('stage1beacon.label')) { await r.click(); return true }
      }
      return false
    }, 30000, 500)
    const ta = await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
    await ta.setValue(RU_LABEL)
    await (await browser.$('[data-testid="ws.editor-save"]')).click()
    await new Promise(r => setTimeout(r, 1000))

    await waitFor(async () => {
      const rows = await browser.$$('.entry-list .entry-row')
      for (const r of rows) {
        const txt = (await r.getText()).toLowerCase()
        if (txt.includes('stage1greeting')) { await r.click(); return true }
      }
      return false
    }, 30000, 500)
    const ta2 = await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
    await ta2.setValue(RU_GREETING)
    await (await browser.$('[data-testid="ws.editor-save"]')).click()
    await new Promise(r => setTimeout(r, 1000))
    await waitFor(async () => {
      const list = await browser.$('.entry-list')
      const t = await list.getText()
      return t.includes(RU_LABEL) && t.includes(RU_GREETING)
    }, 20000, 400)
  })

  it('reopen through Projects — translations persist', async () => {
    const projectsLink = await browser.$('.app-sidebar a[href="#/projects"]')
    await projectsLink.click()
    await waitExisting('[data-testid="projects.project-card"]', 30000)
    await (await browser.$$('[data-testid="projects.project-card"]'))[0].click()
    await waitExisting('[data-testid="ws.root"]', 120000)
    await waitExisting('.entry-list .entry-row', 60000)
    await waitFor(async () => {
      const list = await browser.$('.entry-list')
      const t = await list.getText()
      return t.includes(RU_LABEL) && t.includes(RU_GREETING)
    }, 30000, 500)
  })

  it('checks screen runs and reports', async () => {
    const checksLink = await browser.$('.app-sidebar a[href="#/checks"]')
    await checksLink.click()
    await waitFor(async () => (await (await browser.$('[data-testid="checks.rerun"], [data-testid="checks.report"]')).isExisting()), 20000, 400)
    await waitFor(async () => {
      const main = await browser.$('main')
      const t = await main.getText()
      return t.length > 50
    }, 20000, 400)
  })

  it('output XML tab shows the translated content', async () => {
    const wsLink = await browser.$('.app-sidebar a[href="#/workspace"]')
    await wsLink.click()
    await waitExisting('.entry-list .entry-row', 30000)
    await waitFor(async () => {
      const rows = await browser.$$('.entry-list .entry-row')
      for (const r of rows) {
        const txt = (await r.getText()).toLowerCase()
        if (txt.includes('stage1beacon.label')) { await r.click(); return true }
      }
      return false
    }, 30000, 500)
    await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
    const outTab = await browser.$('[data-testid="ws.tab.output"]')
    await outTab.click()
    await waitFor(async () => {
      const pre = await browser.$('[data-testid="ws.output-pre"]')
      if (!(await pre.isExisting())) return false
      const t = await pre.getText()
      return t.includes(RU_LABEL)
    }, 20000, 400)
  })

  it('export writes verified LanguageData files', async () => {
    const buildLink = await browser.$('.app-sidebar a[href="#/export"]')
    await buildLink.click()
    const outdir = await waitExisting('[data-testid="be.outdir"]', 30000)
    await outdir.setValue(EXPORT_DIR)
    await (await browser.$('[data-testid="be.export"]')).click()
    await waitFor(() => Promise.resolve(walkXml(EXPORT_DIR).length > 0), 60000, 500)
    const files = walkXml(EXPORT_DIR).filter(f => f.endsWith('.xml'))
    console.log('EXPORTED FILES:', JSON.stringify(files))
    const all = files.map(f => readFileSync(f, 'utf8')).join('\n')
    expect(all).toContain(RU_LABEL)
    expect(all).toContain(RU_GREETING)
    expect(all).toContain('<LanguageData>')
  })

  after(() => {
    rmSync(WORK_DIR, { recursive: true, force: true })
  })
})
