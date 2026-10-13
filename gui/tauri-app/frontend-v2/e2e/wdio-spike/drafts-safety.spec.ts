// Draft-safety native E2E, phase 1 (rel26-draft-fixes): drive the REAL
// production app against a collision fixture — two Def types sharing one
// display key must render as two independently editable rows, drafts must
// stay per-row and survive a target-locale round trip inside one session.
// Restart leg: drafts-restart.spec.ts (new WDIO session = new process).
import { mkdirSync, writeFileSync } from 'node:fs'
import * as fs from 'node:fs'
import { switchTargetLocale, commitCurrentRow, pause } from './ds-helpers'

function pause(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

const MOD = '/tmp/rimloc-ds/mod'

async function waitExisting(selector: string, timeout = 20000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

describe('draft safety, phase 1 — collision rows + locale-scoped drafts (live session)', () => {
  before(() => {
    // The data dir itself is wiped by the RUNNER before the session starts
    // (the app is already alive inside a WDIO session here — never rm under
    // it); the fixture is safe to (re)write. The wizard validates the mod
    // root — About/About.xml (ModMetaData) is mandatory, same as the
    // rimloc-wizard-mod canon fixture.
    mkdirSync(`${MOD}/Defs`, { recursive: true })
    mkdirSync(`${MOD}/About`, { recursive: true })
    writeFileSync(
      `${MOD}/About/About.xml`,
      '<?xml version="1.0" encoding="utf-8"?><ModMetaData><name>Draft Safety Fixture</name><author>rimloc-testlab</author><packageId>rimloc.test.draftsafety</packageId><supportedVersions><li>1.6</li></supportedVersions></ModMetaData>',
    )
    writeFileSync(
      `${MOD}/Defs/A_Thing.xml`,
      '<Defs><ThingDef><defName>Dup</defName><label>Thing Dup</label></ThingDef></Defs>',
    )
    writeFileSync(
      `${MOD}/Defs/A_Hediff.xml`,
      '<Defs><HediffDef><defName>Dup</defName><label>Hediff Dup</label></HediffDef></Defs>',
    )
  })

  it('collision fixture → two rows, independent drafts, locale round trip', async () => {
    try {
      await browser.waitUntil(
        async () => (await browser.$('.app-sidebar')).isExisting(),
        { timeout: 60000, interval: 200 },
      )
    } catch {
      // Honest diagnostics instead of a bare timeout: what page did the
      // session actually attach to?
      const url = await browser.getUrl()
      const state = await browser.execute(() => ({
        ready: document.readyState,
        title: document.title,
        bodyLen: document.body ? document.body.innerHTML.length : -1,
      }))
      console.log(`[ds] ATTACH url=${url} state=${JSON.stringify(state)}`)
      throw new Error('app sidebar never appeared — see ATTACH diagnostics')
    }
    await browser.execute(() => { window.location.hash = '#/projects' })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue(MOD)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]')
    await browser.$('[data-testid="wizard.version"]').selectByVisibleText('1.6')
    await browser.$('[data-testid="wizard.next"]').click()
    // Step 3/3 is the review step: the SAME wizard.next button becomes
    // "create" — click it while the overlay is still up.
    await pause(600)
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) {
      await browser.$('[data-testid="wizard.next"]').click()
    }
    try {
      await waitExisting('[data-testid="ws.root"]', 45000)
    } catch {
      const stuck = await browser.$('[data-testid="wizard.overlay"]')
      const err = (await stuck.isExisting()) ? await stuck.getText() : '<overlay gone>'
      console.log(`[ds] WIZARD_STUCK hash=${await browser.getUrl()} text=${JSON.stringify(err.slice(0, 400))}`)
      throw new Error('wizard never reached the workspace — see WIZARD_STUCK')
    }
    await pause(500)

    // --- P0-2: two structural entries sharing the display key ---
    const row0 = await waitExisting('[data-testid="ws.entry.0"]')
    const row1 = await browser.$('[data-testid="ws.entry.1"]')
    if (!row1.isExisting()) throw new Error('collision: expected TWO rows, found <2')
    const label0 = await row0.getText()
    const label1 = await row1.getText()
    console.log(`[ds] ROW0=${JSON.stringify(label0)} ROW1=${JSON.stringify(label1)}`)
    if (!label0.includes('Dup.label') || !label1.includes('Dup.label'))
      throw new Error('collision rows must both show Dup.label')
    const discriminated =
      (label0.includes('ThingDef') && label1.includes('HediffDef')) ||
      (label0.includes('HediffDef') && label1.includes('ThingDef'))
    if (!discriminated) throw new Error('collision rows must show their def-type discriminator')

    // --- independent drafts on the two rows ---
    await row0.click()
    const editor = await waitExisting('[data-testid="ws.editor-textarea"]')
    await editor.setValue('черновик вещи')
    await pause(2200) // debounce 1.5s → durable flush
    await row1.click()
    const editor1 = await waitExisting('[data-testid="ws.editor-textarea"]')
    await editor1.setValue('черновик эффекта')
    await pause(2200)
    await row0.click()
    const back = await browser.$('[data-testid="ws.editor-textarea"]')
    const v = await back.getValue()
    console.log(`[ds] ROW0_DRAFT_AFTER_ROUNDTRIP=${JSON.stringify(v)}`)
    if (v !== 'черновик вещи')
      throw new Error(`row 0 draft clobbered by row 1: ${JSON.stringify(v)}`)

    // --- P0-1: locale switch keeps the slice and restores it back ---
    // Strict isolation on BOTH rows: the foreign slice must be EMPTY.
    await switchTargetLocale(browser, 'uk')
    await row0.click()
    const uk0 = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    await row1.click()
    const uk1 = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    console.log(`[ds] UK_VIEW row0=${JSON.stringify(uk0)} row1=${JSON.stringify(uk1)}`)
    if (uk0 !== '' || uk1 !== '')
      throw new Error(`ru drafts leaked into the uk slice: ${JSON.stringify([uk0, uk1])}`)

    await switchTargetLocale(browser, 'ru')
    await row0.click()
    const ru0 = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    await row1.click()
    const ru1 = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    console.log(`[ds] RU_BACK row0=${JSON.stringify(ru0)} row1=${JSON.stringify(ru1)}`)
    if (!ru0 || !ru1) throw new Error('ru drafts lost after locale round trip')

    // P0-2 (native commits): commit BOTH rows through the real editor —
    // two distinct structural identities, two confirmed translations.
    await row0.click()
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('перевод вещи')
    await commitCurrentRow(browser, { locale: 'Russian', text: 'перевод вещи' })
    await row1.click()
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('перевод эффекта')
    await commitCurrentRow(browser, { locale: 'Russian', text: 'перевод эффекта' })
    await pause(500)
    // Rows now show the confirmed targets (dirty pill gone, target text).
    await row0.click()
    const c0 = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    await row1.click()
    const c1 = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    console.log(`[ds] COMMITTED row0=${JSON.stringify(c0)} row1=${JSON.stringify(c1)}`)
    if (c0 !== 'перевод вещи' || c1 !== 'перевод эффекта')
      throw new Error(`committed translations not on their rows: ${JSON.stringify([c0, c1])}`)
    // The committed drafts left the durable store.
    await pause(1000)
    const mgr = '/tmp/rimloc-ds/data/managed'
    const side = fs.readdirSync(mgr).filter((f) => f.includes('.drafts.json'))[0]
    const store = JSON.parse(fs.readFileSync(`${mgr}/${side}`, 'utf8'))
    console.log(`[ds] STORE_AFTER_COMMITS=${JSON.stringify(Object.keys(store.entries))}`)
    if (Object.keys(store.entries).length !== 0)
      throw new Error('committed drafts must leave the durable store empty')

    // NOTE: phase 1 ENDS ON 'ru' deliberately — phase 2 (a fresh process)
    // proves the persisted target locale + drafts isolation from a restart.

    // sidecar proof: the durable store exists on disk for this project
    await pause(1000)
    // The sidecar lives NEXT to the managed envelope in data/managed/.
    const sidecars = fs
      .readdirSync('/tmp/rimloc-ds/data/managed')
      .filter((f) => f.includes('.drafts.json'))
    console.log(`[ds] SIDECARS=${JSON.stringify(sidecars)}`)
    if (sidecars.length === 0) throw new Error('no drafts sidecar written to the managed root')
  })
})
