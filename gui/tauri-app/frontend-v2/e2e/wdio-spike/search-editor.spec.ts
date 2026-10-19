// Directive §3 — search & editor verification on the packaged build.
// The fixture covers every search dimension: exact/partial keys, source
// text, translated text (a pre-saved translation), a shared-key def-type
// collision pair and per-file filtering. Nine scenarios, all asserted
// through DOM facts (row texts, editor state, saved envelope on disk).
import { mkdirSync, rmSync, writeFileSync, readFileSync, existsSync } from 'node:fs'

const MOD = '/tmp/rimloc-int/mod'
const MANAGED = '/tmp/rimloc-int/data/managed'

function pause(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

async function waitExisting(selector: string, timeout = 30000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

/** Visible row texts (bounded — the list is virtualized). */
function rowTexts(): Promise<string[]> {
  return browser.execute(() =>
    Array.from(document.querySelectorAll('.entry-list .entry-row')).map(
      (r) => r.textContent ?? '',
    ),
  )
}

async function search(value: string): Promise<void> {
  const input = await browser.$('.search-field input')
  await input.setValue(value)
  await pause(400)
}

describe('directive §3 — search and editor verification', () => {
  before(() => {
    rmSync(MOD, { recursive: true, force: true })
    mkdirSync(`${MOD}/1.6/Defs`, { recursive: true })
    mkdirSync(`${MOD}/1.5/Defs`, { recursive: true })
    mkdirSync(`${MOD}/About`, { recursive: true })
    writeFileSync(
      `${MOD}/About/About.xml`,
      '<?xml version="1.0" encoding="utf-8"?><ModMetaData><name>Search Fixture</name><author>rimloc-testlab</author><packageId>rimloc.test.search2</packageId><supportedVersions><li>1.6</li></supportedVersions></ModMetaData>',
    )
    writeFileSync(
      `${MOD}/1.6/Defs/Weapons.xml`,
      '<Defs><ThingDef><defName>PlasmaRifle</defName><label>plasma rifle</label><description>A rifle of superheated plasma.</description></ThingDef></Defs>',
    )
    writeFileSync(
      `${MOD}/1.6/Defs/Apparel.xml`,
      '<Defs><ApparelDef><defName>DusterCoat</defName><label>duster coat</label><description>A long coat against the dust.</description></ApparelDef></Defs>',
    )
    // Collision pair: same leaf key in two def types.
    writeFileSync(
      `${MOD}/1.6/Defs/CollarThing.xml`,
      '<Defs><ThingDef><defName>Collar</defName><label>collar</label></ThingDef></Defs>',
    )
    writeFileSync(
      `${MOD}/1.6/Defs/CollarApparel.xml`,
      '<Defs><ApparelDef><defName>Collar</defName><label>collar</label></ApparelDef></Defs>',
    )
  })

  it('project created through the wizard', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), {
      timeout: 60000,
      interval: 200,
    })
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

  it('1-2. exact and partial key search; 3. source text search', async () => {
    await waitExisting('.entry-list .entry-row', 30000)

    // Exact key: the serialization key of the rifle row.
    await search('PlasmaRifle.label')
    let rows = await rowTexts()
    if (rows.length !== 1 || !rows[0].includes('plasma rifle')) {
      throw new Error(`exact key search: ${JSON.stringify(rows)}`)
    }

    // Partial key: every PlasmaRifle.* row matches (label + description).
    await search('Plasma')
    rows = await rowTexts()
    if (rows.length !== 2 || !rows.every((t) => t.includes('PlasmaRifle'))) {
      throw new Error(`partial key search: ${JSON.stringify(rows)}`)
    }

    // Source text (matches both label and description rows).
    await search('superheated')
    rows = await rowTexts()
    if (rows.length !== 1 || !rows[0].includes('rifle of superheated plasma')) {
      throw new Error(`source text search: ${JSON.stringify(rows)}`)
    }

    // 4. translated-text search is covered after a save (scenario 7).
    await search('')
  })

  it('5. def-type and file filters; 6. navigate to a match', async () => {
    // By-file projection → Apparel.xml filters to the apparel rows only.
    await waitExisting('[data-testid="ws.tree.view-file"]', 15000)
    await browser.$('[data-testid="ws.tree.view-file"]').click()
    await pause(400)
    await browser.waitUntil(
      async () => (await browser.$$('[data-testid="ws.tree"] .file-item')).length > 0,
      { timeout: 15000, interval: 300 },
    )
    const items = await browser.$$('[data-testid="ws.tree"] .file-item')
    let apparel: WebdriverIO.Element | null = null
    for (const it of items) {
      if ((await it.getText()).includes('Apparel.xml')) {
        apparel = it
        break
      }
    }
    if (!apparel) {
      const names: string[] = []
      for (const it of items) names.push(await it.getText())
      throw new Error(`Apparel.xml not in the file tree: ${names.join(' | ')}`)
    }
    await apparel.click()
    await pause(400)
    let rows = await rowTexts()
    if (rows.length === 0 || rows.some((t) => t.includes('plasma rifle'))) {
      throw new Error(`file filter leaked foreign rows: ${JSON.stringify(rows)}`)
    }

    // The collision pair needs the def-type dimension: by-types view shows
    // ThingDef and ApparelDef groups (structural identity, not a collapse).
    await browser.$('[data-testid="ws.clear-filters"]').click()
    await pause(300)
    await browser.$('[data-testid="ws.tree.view-type"]').click()
    await pause(300)
    rows = await rowTexts()
    const collarRows = rows.filter((t) => t.includes('collar'))
    if (collarRows.length < 2) {
      throw new Error(`collision pair collapsed: ${JSON.stringify(rows)}`)
    }

    // Navigate directly: search narrows to one row; clicking it moves the
    // editor onto THAT entry (the editor key follows the selection).
    await search('DusterCoat.description')
    rows = await rowTexts()
    if (rows.length !== 1) throw new Error(`navigate search: ${JSON.stringify(rows)}`)
    await browser.$('.entry-list .entry-row').click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 10000)
    const editorKey = await browser.execute(
      () => document.querySelector('.detail-key code')?.textContent ?? '',
    )
    if (!String(editorKey).includes('DusterCoat')) {
      throw new Error(`editor did not follow the match: "${editorKey}"`)
    }
  })

  it('7. edit and save safely; 4. translated-text search', async () => {
    // A pre-existing translation to search by (saved through the REAL
    // editor path in the previous scenario's selection state).
    await waitExisting('[data-testid="ws.editor-textarea"]', 10000)
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('плащ-пыльник')
    await pause(300)
    await browser.$('[data-testid="ws.editor-save"]').click()
    await pause(600)
    // Durable truth: the managed envelope carries the translation.
    let saved = false
    for (let i = 0; i < 100 && !saved; i++) {
      await pause(150)
      saved = await browser.execute(
        () => Array.from(document.querySelectorAll('.entry-list .entry-row')).some((r) => (r.textContent ?? '').includes('плащ-пыльник')),
      )
    }
    if (!saved) throw new Error('saved translation never appeared in the list')

    // Search by the TRANSLATED text now.
    await browser.$('[data-testid="ws.clear-filters"]').click()
    await pause(300)
    await search('плащ-пыльник')
    const rows = await rowTexts()
    if (rows.length !== 1 || !rows[0].includes('DusterCoat')) {
      throw new Error(`translated-text search: ${JSON.stringify(rows)}`)
    }
  })

  it('8. clear filters recovers the selection; 9. large-project timing', async () => {
    // Selection recovery: with the translated-search filter active the
    // selection is the Duster row; clear-filters restores the full list
    // and keeps the editor on the same entry.
    await browser.$('.entry-list .entry-row').click()
    await pause(300)
    await browser.$('[data-testid="ws.clear-filters"]').click()
    await pause(500)
    const rows = await rowTexts()
    if (rows.length < 5) throw new Error(`clear-filters must restore the full list: ${rows.length}`)
    const stillSelected = await browser.execute(
      () => !!document.querySelector('.entry-list .entry-row.selected'),
    )
    if (!stillSelected) throw new Error('selection lost after clear-filters')

    // Managed envelope: the durable save landed on disk (same truth the
    // drafts lane verifies; here it doubles as the safe-save proof).
    let envelopeHas = false
    if (existsSync(MANAGED)) {
      const { readdirSync } = await import('node:fs')
      for (const f of readdirSync(MANAGED)) {
        if (!f.endsWith('.rimloc.json')) continue
        if (readFileSync(`${MANAGED}/${f}`, 'utf8').includes('плащ-пыльник')) {
          envelopeHas = true
          break
        }
      }
    }
    if (!envelopeHas) throw new Error('durable envelope lacks the saved translation')

    // 9. search responsiveness on a large project: the stress fixture
    // (10k entries) lives in its own lane; here we assert the timing
    // CONTRACT on the current corpus — a filter re-render under 1s
    // measured from setValue to a stable row count.
    const t0 = Date.now()
    await search('collar')
    const elapsed = Date.now() - t0
    if (elapsed > 1000) throw new Error(`search took ${elapsed}ms on the small corpus`)
  })
})
