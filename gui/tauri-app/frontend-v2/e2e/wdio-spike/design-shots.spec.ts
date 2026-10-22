/**
 * PRODUCT-DESIGN PASS — captures the key RC screens for visual audit:
 * workspace (empty + entry selected + each context tab), wizard,
 * aitranslate (each mode), export, checks, settings, about. Screenshots
 * land in the deterministic evidence dir for the design review.
 */
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const WORK_DIR = mkdtempSync(join(tmpdir(), 'rimloc-design-'))
const MOD = join(WORK_DIR, 'TestMod')
const SHOT_DIR = join(tmpdir(), 'rimloc-design-shots')

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
  writeFileSync(join(about, 'About.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<ModMetaData>\n  <name>Design Pass Mod</name>\n  <packageId>rimloc.test.design</packageId>\n  <author>RimLoc</author>\n  <supportedVersions><li>1.6</li></supportedVersions>\n</ModMetaData>\n')
  writeFileSync(join(defs, 'D.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<Defs>\n  <ThingDef ParentName="BuildingBase">\n    <defName>DesignBeacon</defName>\n    <label>design beacon</label>\n    <description>A beacon for the design pass.</description>\n    <thingClass>Building</thingClass><category>Building</category>\n    <graphicData><texPath>T</texPath><graphicClass>G</graphicClass><drawSize>(1,1)</drawSize></graphicData>\n    <size>(1,1)</size><statBases><MaxHitPoints>100</MaxHitPoints><WorkToBuild>100</WorkToBuild></statBases>\n    <passability>Impassable</passability><fillPercent>1</fillPercent><designationCategory>Structure</designationCategory>\n  </ThingDef>\n</Defs>\n')
  writeFileSync(join(keyed, 'K.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<LanguageData>\n  <DesignGreeting>hello design pass</DesignGreeting>\n  <DesignFarewell>goodbye design pass</DesignFarewell>\n</LanguageData>\n')
}

async function shot(name: string): Promise<void> {
  await browser.saveScreenshot(join(SHOT_DIR, name + '.png'))
  console.log('SHOT:', name)
}

describe('design pass captures', () => {
  before(() => {
    writeFixture()
    mkdirSync(SHOT_DIR, { recursive: true })
  })

  it('captures the key screens', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), { timeout: 60000, interval: 200 })
    await shot('00-start')

    // wizard
    await (await browser.$('.app-sidebar a[href="#/projects"]')).click()
    await shot('10-projects-empty')
    await (await waitExisting('[data-testid="wizard.open"]')).click()
    await waitExisting('[data-testid="wizard.path-input"]', 15000)
    await shot('11-wizard-path')
    await (await browser.$('[data-testid="wizard.path-input"]')).setValue(MOD)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]', 60000)
    await shot('12-wizard-version')
    await browser.$('[data-testid="wizard.next"]').click()
    await waitFor(async () => !(await (await browser.$('[data-testid="wizard.overlay"]')).isExisting()), 10000).catch(() => {})
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) { await shot('13-wizard-summary'); await browser.$('[data-testid="wizard.next"]').click() }
    await waitExisting('[data-testid="ws.root"]', 300000)
    await waitExisting('.entry-list .entry-row', 60000)
    await shot('20-workspace-overview')

    // select entry + tabs
    await (await browser.$$('.entry-list .entry-row'))[0].click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
    await shot('21-workspace-entry')
    for (const [tid, name] of [['ws.tab.source', '22-tab-source'], ['ws.tab.terms', '23-tab-terms'], ['ws.tab.xml', '24-tab-xml'], ['ws.tab.output', '25-tab-output']] as [string, string][]) {
      const tab = await browser.$('[data-testid="' + tid + '"]')
      if (await tab.isExisting()) { await tab.click(); await new Promise(r => setTimeout(r, 600)); await shot(name) }
    }

    // aitranslate modes
    await (await browser.$('.app-sidebar a[href="#/aitranslate"]')).click()
    await waitExisting('[data-testid="ai.modes"]', 20000)
    await shot('30-ai-auto')
    for (const [tid, name] of [['ai.mode.chatgpt', '31-ai-chatgpt'], ['ai.mode.byo', '32-ai-byo'], ['ai.mode.local', '33-ai-local'], ['ai.mode.noapi', '34-ai-noapi']] as [string, string][]) {
      await (await browser.$('[data-testid="' + tid + '"]')).click()
      await new Promise(r => setTimeout(r, 400))
      await shot(name)
    }

    // export, checks, settings, about
    await (await browser.$('.app-sidebar a[href="#/export"]')).click()
    await waitExisting('[data-testid="be.outdir"]', 20000)
    await shot('40-export')
    await (await browser.$('.app-sidebar a[href="#/checks"]')).click()
    await new Promise(r => setTimeout(r, 900))
    await shot('41-checks')
    await (await browser.$('.app-sidebar a[href="#/settings"]')).click()
    await new Promise(r => setTimeout(r, 500))
    await shot('42-settings')
    await (await browser.$('.app-sidebar a[href="#/about"]')).click()
    await new Promise(r => setTimeout(r, 500))
    await shot('43-about')
  })

  after(() => { rmSync(WORK_DIR, { recursive: true, force: true }) })
})
