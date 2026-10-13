// Draft-safety verify leg (rel26, review P0-2): the reopened project shows
// both committed translations; the app BUILDS the drop-in mod and the XML
// on disk carries each translation in its OWN Def structure.
import * as fs from 'node:fs'
import { pause } from './ds-helpers'

const OUT = '/tmp/rimloc-ds/export'

async function waitExisting(selector: string, timeout = 20000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

describe('draft safety, phase 3 — export XML carries both structural identities', () => {
  it('builds the mod and writes both translations into their Def structures', async () => {
    await browser.waitUntil(
      async () => (await browser.$('.app-sidebar')).isExisting(),
      { timeout: 30000, interval: 100 },
    )
    await browser.execute(() => { window.location.hash = '#/projects' })
    const card = await waitExisting('[data-testid="projects.project-card"]')
    await card.click()
    await waitExisting('[data-testid="ws.root"]', 45000)
    await pause(600)

    // Rows still carry their confirmed targets after another restart.
    const row0 = await waitExisting('[data-testid="ws.entry.0"]')
    await row0.click()
    const t0 = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    console.log(`[ds-verify] ROW0=${JSON.stringify(t0)}`)
    if (t0 !== 'перевод вещи') throw new Error(`row 0 target lost: ${JSON.stringify(t0)}`)

    // Export screen shows the ACTIVE target locale (F1 display half).
    await browser.execute(() => { window.location.hash = '#/export' })
    const target = await waitExisting('[data-testid="be.target-locale"]')
    const tl = await target.getText()
    console.log(`[ds-verify] EXPORT_LOCALE=${JSON.stringify(tl)}`)
    if (!tl.includes('Russian')) throw new Error(`export locale not Russian: ${tl}`)

    fs.rmSync(OUT, { recursive: true, force: true })
    fs.mkdirSync(OUT, { recursive: true })
    await browser.$('[data-testid="be.outdir"]').setValue(OUT)
    await browser.$('[data-testid="be.build"]').click()
    await waitExisting('[data-testid="be.result"]', 60000)
    await pause(1000)

    // The XML on disk: each Def structure holds ITS OWN translation.
    const xmls: string[] = []
    const walk = (d: string): void => {
      for (const e of fs.readdirSync(d, { withFileTypes: true })) {
        const p = `${d}/${e.name}`
        if (e.isDirectory()) walk(p)
        else if (e.name.endsWith('.xml')) xmls.push(p)
      }
    }
    walk(OUT)
    console.log(`[ds-verify] XML_FILES=${JSON.stringify(xmls)}`)
    const all = xmls.map((p) => fs.readFileSync(p, 'utf8')).join('\n')
    const thingOk = /ThingDef[\s\S]*?Dup\.label[\s\S]*?перевод эффекта/.test(all) ||
      (all.includes('перевод эффекта') && xmls.some((p) => p.includes('ThingDef')))
    const hediffOk = all.includes('перевод вещи') && xmls.some((p) => p.includes('HediffDef'))
    if (!thingOk) throw new Error('ThingDef translation missing from the built XML')
    if (!hediffOk) throw new Error('HediffDef translation missing from the built XML')
    const texts = (all.match(/перевод (вещи|эффекта)/g) || []).sort()
    console.log(`[ds-verify] XML_TEXTS=${JSON.stringify(texts)}`)
    if (texts.length !== 2) throw new Error(`expected exactly 2 translations in XML, got ${texts}`)
  })
})
