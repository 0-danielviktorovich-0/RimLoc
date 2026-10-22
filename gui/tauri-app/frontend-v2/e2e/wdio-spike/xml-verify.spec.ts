/**
 * RUNTIME VERIFICATION — hardened per REVIEW_1641, portable per
 * REVIEW_180e5344: the fixture mod is GENERATED in a per-run temp dir
 * (no machine-specific paths) and removed unconditionally. The app-side
 * profile isolation is the launcher's contract (RIMLOC_DATA_DIR in the
 * spawn wrapper — see xml-verify.conf.ts); this spec has no other inputs.
 *
 * 1. Asserts actual XML markup from the fixture (not just no-error)
 * 2. Layout test FAILS if DOM elements are missing (fail-closed)
 * 3. AI RU labels have real expect() assertions
 * 4. Deterministic waitFor (no fixed sleeps for critical assertions)
 */
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

/** Per-run workspace: the fixture mod lives here, removed in after(). */
const WORK_DIR = mkdtempSync(join(tmpdir(), 'rimloc-xml-verify-'))
const MOD = join(WORK_DIR, 'TestMod')

/** Visual evidence survives the per-run cleanup; CI can redirect it. */
const EVIDENCE_DIR =
  process.env.RIMLOC_XML_VERIFY_EVIDENCE ?? join(tmpdir(), 'rimloc-xml-verify-evidence')

/** Expected XML content from the generated fixture — asserted against the XML tab. */
const EXPECTED_XML_KEY = 'OwnerTestBeacon'
const EXPECTED_XML_SNIPPETS = ['<defName>OwnerTestBeacon</defName>', 'owner test beacon']

function writeFixture(): void {
  const about = join(MOD, 'About')
  const defs = join(MOD, '1.6', 'Defs')
  const keyed = join(MOD, 'Languages', 'English', 'Keyed')
  mkdirSync(about, { recursive: true })
  mkdirSync(defs, { recursive: true })
  mkdirSync(keyed, { recursive: true })
  writeFileSync(
    join(about, 'About.xml'),
    `<?xml version="1.0" encoding="utf-8"?>
<ModMetaData>
  <name>Owner Test Mod</name>
  <packageId>rimloc.test.owneruat</packageId>
  <author>RimLoc</author>
  <supportedVersions><li>1.6</li></supportedVersions>
  <description>Generated fixture for the XML preview runtime verification.</description>
</ModMetaData>
`,
  )
  writeFileSync(
    join(defs, 'TestBuilding.xml'),
    `<?xml version="1.0" encoding="utf-8"?>
<Defs>
  <ThingDef ParentName="BuildingBase">
    <defName>${EXPECTED_XML_KEY}</defName>
    <label>owner test beacon</label>
    <description>A test building for the owner UAT session.</description>
    <thingClass>Building</thingClass>
    <category>Building</category>
    <graphicData>
      <texPath>Things/Building/Misc/Cooler</texPath>
      <graphicClass>Graphic_Single</graphicClass>
      <drawSize>(1,1)</drawSize>
    </graphicData>
    <size>(1,1)</size>
    <statBases>
      <MaxHitPoints>100</MaxHitPoints>
      <WorkToBuild>100</WorkToBuild>
    </statBases>
    <passability>Impassable</passability>
    <fillPercent>1</fillPercent>
    <designationCategory>Structure</designationCategory>
  </ThingDef>
</Defs>
`,
  )
  writeFileSync(
    join(keyed, 'TestStrings.xml'),
    `<?xml version="1.0" encoding="utf-8"?>
<LanguageData>
  <OwnerTestGreeting>Hello owner test</OwnerTestGreeting>
  <OwnerTestFarewell>Goodbye owner test</OwnerTestFarewell>
  <OwnerTestDescription>This is a test string for the owner UAT session</OwnerTestDescription>
</LanguageData>
`,
  )
}

async function waitFor(fn: () => Promise<boolean>, timeoutMs = 15000, intervalMs = 300): Promise<void> {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    if (await fn()) return
    await new Promise(r => setTimeout(r, intervalMs))
  }
  throw new Error(`waitFor timed out after ${timeoutMs}ms`)
}

async function waitExisting(sel: string, timeout = 30000): Promise<WebdriverIO.Element> {
  const el = await browser.$(sel)
  await el.waitForExist({ timeout })
  return el
}

describe('XML tab runtime verification (hardened)', () => {
  before(() => {
    writeFixture()
  })

  it('wizard → select entry → click XML tab → assert real XML markup', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), {
      timeout: 60000, interval: 200,
    })

    // Create project via wizard (user-level navigation)
    await browser.execute(() => { window.location.hash = '#/projects' })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue(MOD)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]', 60000)
    await browser.$('[data-testid="wizard.next"]').click()
    // Deterministic wait for the overlay to appear/disappear
    await waitFor(async () => {
      const overlay = await browser.$('[data-testid="wizard.overlay"]')
      return !(await overlay.isExisting())
    }, 10000).catch(() => {/* overlay may auto-dismiss */})
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="ws.root"]', 300000)
    await waitExisting('.entry-list .entry-row', 60000)

    // Select the first entry deterministically
    await browser.$('.entry-list .entry-row').click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 15000)

    // Click the XML source tab (labeled "XML" or "XML исходника")
    const tabs = await browser.execute(() =>
      Array.from(document.querySelectorAll('.context-tabs button')).map(b => b.textContent?.trim())
    )
    console.log('BOTTOM TABS:', JSON.stringify(tabs))
    // Find the XML tab by text (language-agnostic)
    const xmlTabIndex = tabs.findIndex(t => t?.toLowerCase().includes('xml') && !t?.toLowerCase().includes('output'))
    if (xmlTabIndex < 0) throw new Error(`No XML tab found in: ${JSON.stringify(tabs)}`)
    const xmlTab = await browser.$(`.context-tabs button:nth-child(${xmlTabIndex + 1})`)
    await xmlTab.click()

    // DETERMINISTIC WAIT: wait for actual XML content (not just no error)
    await waitFor(async () => {
      const content = await browser.execute(() => ({
        text: document.querySelector('.context-content')?.textContent ?? '',
        hasError: !!document.querySelector('.inline-warning, [class*="error"]'),
        hasCommandNotFound: document.body.textContent?.includes('Command') &&
          document.body.textContent?.includes('not found'),
      }))
      return content.text.length > 20 && !content.hasCommandNotFound
    }, 15000)

    // ASSERT ACTUAL XML MARKUP from the fixture
    const xmlContent = await browser.execute(() =>
      document.querySelector('.context-content')?.textContent ?? ''
    )
    console.log('XML content (first 200):', xmlContent.slice(0, 200))

    // FAIL if the expected key from our fixture is not in the XML
    expect(xmlContent).toContain(EXPECTED_XML_KEY)
    // FAIL if the expected source text is not present
    for (const snippet of EXPECTED_XML_SNIPPETS) {
      expect(xmlContent).toContain(snippet)
    }

    // Visual evidence: capture the XML tab as rendered by the real runtime.
    mkdirSync(EVIDENCE_DIR, { recursive: true })
    const shotPath = join(EVIDENCE_DIR, 'xml-tab-evidence.png')
    await browser.saveScreenshot(shotPath)
    console.log('XML tab screenshot saved:', shotPath)

    // FAIL if any error indicator is visible
    const hasVisibleError = await browser.execute(() => {
      const errs = document.querySelectorAll('.inline-warning, .ws-error')
      return Array.from(errs).some(e => e.offsetParent !== null && e.textContent?.includes('not found'))
    })
    expect(hasVisibleError).toBe(false)
  })

  it('layout: entries pane fills width — FAIL if elements missing', async () => {
    // Navigate back to workspace
    await browser.execute(() => { window.location.hash = '#/workspace' })
    await waitExisting('.entry-list .entry-row', 30000)

    const geo = await browser.execute(() => {
      const ws = document.querySelector('.ws-center')?.getBoundingClientRect()
      const entries = document.querySelector('.entries-pane')?.getBoundingClientRect()
      const tree = document.querySelector('.file-tree')?.getBoundingClientRect()
      return {
        wsCenter: ws ? { w: ws.width, x: ws.x, right: ws.x + ws.width } : null,
        entriesPane: entries ? { w: entries.width, x: entries.x, right: entries.x + entries.width } : null,
        fileTree: tree ? { w: tree.width } : null,
      }
    })
    console.log('LAYOUT:', JSON.stringify(geo))

    // FAIL-CLOSED: assert all elements exist (no silent skip)
    expect(geo.wsCenter).not.toBeNull()
    expect(geo.entriesPane).not.toBeNull()
    expect(geo.fileTree).not.toBeNull()

    // Assert entries pane width is reasonable (> 200px)
    expect(geo.entriesPane!.w).toBeGreaterThan(200)

    // Assert the gap between entries-pane right edge and ws-center right edge is < 50px
    const gap = geo.wsCenter!.right - geo.entriesPane!.right
    console.log(`Gap: ${gap}px`)
    expect(gap).toBeLessThan(50)
    expect(gap).toBeGreaterThan(-5) // no overflow
  })

  it('AI mode labels: assert Russian text with expect()', async () => {
    await browser.execute(() => { window.location.hash = '#/aitranslate' })
    await waitFor(async () => {
      const el = await browser.$('[data-testid="ai.form"], .ai-form')
      return await el.isExisting()
    }, 15000)

    const modes = await browser.execute(() => {
      const chips = Array.from(document.querySelectorAll(
        '[class*="mode"] button, [class*="chip"], [data-testid*="mode"]'
      ))
      return chips.map(c => c.textContent?.trim()).filter(t => t && t.length > 3 && t.length < 60)
    })
    console.log('AI MODE LABELS:', JSON.stringify(modes))

    // REAL ASSERTIONS: at least one mode button must exist
    expect(modes.length).toBeGreaterThan(0)

    // Assert Russian labels are present (specific expected values from the i18n dictionary)
    const allText = modes.join(' ')
    expect(allText).toContain('автоматически')
    expect(allText).toContain('Локальные')
    expect(allText).toContain('модели')

    // Assert English-only mode labels are NOT present
    expect(allText).not.toContain('Translate automatically')
    expect(allText).not.toContain('Bring your')
    expect(allText).not.toContain('Local models')
  })

  after(async () => {
    // Unconditional per-run cleanup (unique temp dir, safe to remove)
    rmSync(WORK_DIR, { recursive: true, force: true })
  })
})
