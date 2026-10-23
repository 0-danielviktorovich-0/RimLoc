/**
 * STAGE 5 — ISSUE #2 SCENARIO B: standalone translation pack.
 * Open an existing translation-pack mod WITHOUT any original mod as a
 * project (the pack is itself a valid mod: About + Languages/Russian),
 * see its strings, edit, save, close, reopen — edits persist — then
 * export a new pack and verify it on disk.
 */
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const WORK_DIR = mkdtempSync(join(tmpdir(), 'rimloc-stage5-'))
const PACK = join(WORK_DIR, 'PackMod')
const EXPORT_DIR = join(WORK_DIR, 'export')
const EDITED_GREETING = 'Привет из пака · правка оракула'

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

function writePack(): void {
  const about = join(PACK, 'About')
  const keyed = join(PACK, 'Languages', 'Russian', 'Keyed')
  const definj = join(PACK, 'Languages', 'Russian', 'DefInjected', 'ThingDef')
  mkdirSync(about, { recursive: true }); mkdirSync(keyed, { recursive: true }); mkdirSync(definj, { recursive: true })
  writeFileSync(join(about, 'About.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<ModMetaData>\n  <name>Canary Russian Pack</name>\n  <packageId>rimloc.canarypack</packageId>\n  <author>RimLoc</author>\n  <supportedVersions><li>1.6</li></supportedVersions>\n  <description>Standalone Russian translation pack fixture.</description>\n</ModMetaData>\n')
  writeFileSync(join(keyed, 'TestStrings.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<LanguageData>\n  <PackGreeting>Привет из пака</PackGreeting>\n  <PackFarewell>Пока из пака</PackFarewell>\n</LanguageData>\n')
  writeFileSync(join(definj, 'PackBeacon.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<LanguageData>\n  <PackBeacon.label>Маяк пака</PackBeacon.label>\n  <PackBeacon.description>Описание маяка из пака.</PackBeacon.description>\n</LanguageData>\n')
}


/** React-controlled select: native value setter + a real change event. */
async function setSelect(id: string, value: string): Promise<void> {
  await browser.execute((tid, val) => {
    const s = document.querySelector('[data-testid="' + tid + '"]') as HTMLSelectElement | null
    if (!s) return
    const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')?.set
    setter?.call(s, val)
    s.dispatchEvent(new Event('change', { bubbles: true }))
  }, id, value)
}

function walkXml(d: string): string[] {
  try {
    return readdirSync(d, { withFileTypes: true }).flatMap(e =>
      e.isDirectory() ? walkXml(join(d, e.name)) : [join(d, e.name)])
  } catch { return [] }
}

describe('Stage 5B — standalone translation pack', () => {
  before(() => { writePack() })

  it('wizard opens the pack WITHOUT an original mod', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), { timeout: 60000, interval: 200 })
    await (await browser.$('.app-sidebar a[href="#/projects"]')).click()
    await (await waitExisting('[data-testid="wizard.open"]')).click()
    await (await waitExisting('[data-testid="wizard.path-input"]')).setValue(PACK)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]', 60000)
    // THE pack-mode step: the source language is the pack's own language
    // (Russian) — the pack carries no English source to scan.
    await waitExisting('[data-testid="wizard.source-locale"]', 15000)
    await setSelect('wizard.source-locale', 'ru')
    await browser.$('[data-testid="wizard.next"]').click()
    await waitFor(async () => !(await (await browser.$('[data-testid="wizard.overlay"]')).isExisting()), 10000).catch(() => {})
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="ws.root"]', 300000)
    await waitFor(async () => (await browser.$$('.entry-list .entry-row')).length >= 4, 60000, 400)
  })

  it('sees pack strings and edits the greeting', async () => {
    await waitFor(async () => {
      const rows = await browser.$$('.entry-list .entry-row')
      for (const r of rows) {
        if ((await r.getText()).includes('PackGreeting')) { await r.click(); return true }
      }
      return false
    }, 30000, 500)
    const ta = await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
    await ta.setValue(EDITED_GREETING)
    await (await browser.$('[data-testid="ws.editor-save"]')).click()
    await waitFor(async () => {
      const list = await browser.$('.entry-list')
      return (await list.getText()).includes(EDITED_GREETING)
    }, 15000, 400)
  })

  it('reopen through Projects — the edit persists', async () => {
    await (await browser.$('.app-sidebar a[href="#/projects"]')).click()
    await waitExisting('[data-testid="projects.project-card"]', 30000)
    await (await browser.$$('[data-testid="projects.project-card"]'))[0].click()
    await waitExisting('[data-testid="ws.root"]', 120000)
    await waitExisting('.entry-list .entry-row', 60000)
    await waitFor(async () => {
      const list = await browser.$('.entry-list')
      return (await list.getText()).includes(EDITED_GREETING)
    }, 30000, 500)
  })

  it('exports a new pack and the edited string lands on disk', async () => {
    await (await browser.$('.app-sidebar a[href="#/export"]')).click()
    const outdir = await waitExisting('[data-testid="be.outdir"]', 30000)
    await outdir.setValue(EXPORT_DIR)
    await browser.$('[data-testid="be.export"]').click()
    await waitFor(() => Promise.resolve(existsSync(EXPORT_DIR) && walkXml(EXPORT_DIR).some(f => f.includes('Keyed'))), 60000, 800)
    const keyedFiles = walkXml(EXPORT_DIR).filter(f => f.includes('Keyed'))
    console.log('STAGE5 EXPORTED KEYED:', JSON.stringify(keyedFiles))
    const all = keyedFiles.map(f => readFileSync(f, 'utf8')).join('\n')
    expect(all).toContain(EDITED_GREETING)
    expect(all).toContain('PackGreeting')
  })

  after(() => { rmSync(WORK_DIR, { recursive: true, force: true }) })
})
