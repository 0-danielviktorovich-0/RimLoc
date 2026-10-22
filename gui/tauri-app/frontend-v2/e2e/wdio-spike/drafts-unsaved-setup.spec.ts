// Unsaved-drafts restart, run 1 (review iteration 2 P0-1): create FOUR
// UNCOMMITTED drafts — two ru (different Def types sharing one display
// key), switch to uk (both rows strictly empty), two uk drafts — and end
// the session WITHOUT committing anything. The durable sidecar must hold
// all four at locale-scoped composite keys. drafts-unsaved-verify.spec
// (fresh process) raises and verifies them.
import * as fs from 'node:fs'
import { switchTargetLocale, pause } from './ds-helpers'

const MGR = '/tmp/rimloc-ds/data/managed'
const SNAP = '/tmp/rimloc-ds/unsaved-sidecar-before.json'

async function waitExisting(selector: string, timeout = 20000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

describe('unsaved drafts, run 1 — four drafts, two locales, NO commits', () => {
  before(() => {
    // The collision fixture (About.xml canon): ThingDef/Dup + HediffDef/Dup.
    fs.mkdirSync('/tmp/rimloc-ds/mod/Defs', { recursive: true })
    fs.mkdirSync('/tmp/rimloc-ds/mod/About', { recursive: true })
    fs.writeFileSync(
      '/tmp/rimloc-ds/mod/About/About.xml',
      '<?xml version="1.0" encoding="utf-8"?><ModMetaData><name>Unsaved Fixture</name><author>rimloc-testlab</author><packageId>rimloc.test.unsaved</packageId><supportedVersions><li>1.6</li></supportedVersions></ModMetaData>',
    )
    fs.writeFileSync(
      '/tmp/rimloc-ds/mod/Defs/A_Thing.xml',
      '<Defs><ThingDef><defName>Dup</defName><label>Thing Dup</label></ThingDef></Defs>',
    )
    fs.writeFileSync(
      '/tmp/rimloc-ds/mod/Defs/A_Hediff.xml',
      '<Defs><HediffDef><defName>Dup</defName><label>Hediff Dup</label></HediffDef></Defs>',
    )
  })

  it('creates ru+uk drafts on both collision rows and ends without committing', async () => {
    await browser.waitUntil(
      async () => (await browser.$('.app-sidebar')).isExisting(),
      { timeout: 30000, interval: 100 },
    )
    await browser.execute(() => { window.location.hash = '#/projects' })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue('/tmp/rimloc-ds/mod')
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]')
    await browser.$('[data-testid="wizard.version"]').selectByVisibleText('1.6')
    await browser.$('[data-testid="wizard.next"]').click()
    await pause(600)
    if (await browser.$('[data-testid="wizard.overlay"]').isExisting()) {
      await browser.$('[data-testid="wizard.next"]').click()
    }
    await waitExisting('[data-testid="ws.root"]', 45000)
    await pause(500)

    const row0 = await waitExisting('[data-testid="ws.entry.0"]')
    const row1 = await browser.$('[data-testid="ws.entry.1"]')

    // ru drafts on both rows (debounce flush waits).
    await row0.click()
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('ru черновик вещи')
    await pause(2000)
    await row1.click()
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('ru черновик эффекта')
    await pause(2000)

    // Switch to uk: both rows STRICTLY empty, then two uk drafts.
    await switchTargetLocale(browser, 'uk')
    await row0.click()
    const ukEmpty0 = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    await row1.click()
    const ukEmpty1 = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    console.log(`[ds-unsaved] UK_EMPTY row0=${JSON.stringify(ukEmpty0)} row1=${JSON.stringify(ukEmpty1)}`)
    if (ukEmpty0 !== '' || ukEmpty1 !== '')
      throw new Error(`ru drafts leaked into uk: ${JSON.stringify([ukEmpty0, ukEmpty1])}`)
    await row0.click()
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('uk чернетка речі')
    await pause(2000)
    await row1.click()
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('uk чернетка ефекту')
    await pause(2000)

    // Durable truth BEFORE the kill: the sidecar holds all four at
    // locale-scoped composite keys; snapshot it for the evidence pack.
    const side = fs.readdirSync(MGR).filter((f) => f.includes('.drafts.json'))[0]
    const store = JSON.parse(fs.readFileSync(`${MGR}/${side}`, 'utf8'))
    const keys = Object.keys(store.entries).sort()
    console.log(`[ds-unsaved] STORE_BEFORE=${JSON.stringify(keys)}`)
    fs.writeFileSync(SNAP, JSON.stringify(store, null, 1))
    const want = [
      'ru::def_injected·HediffDef·Dup.label',
      'ru::def_injected·ThingDef·Dup.label',
      'uk::def_injected·HediffDef·Dup.label',
      'uk::def_injected·ThingDef·Dup.label',
    ]
    for (const w of want) if (!keys.includes(w)) throw new Error(`store missing ${w}: ${keys}`)
    // NOTHING may be committed: the envelope translations must stay empty.
    const env = fs.readdirSync(MGR).filter((f) => f.endsWith('.rimloc.json'))[0]
    const project = JSON.parse(fs.readFileSync(`${MGR}/${env}`, 'utf8'))
    if ((project.translations ?? []).length !== 0)
      throw new Error('setup must not commit anything')
    console.log('[ds-unsaved] SETUP_OK — session ends WITHOUT commits')
  })
})
