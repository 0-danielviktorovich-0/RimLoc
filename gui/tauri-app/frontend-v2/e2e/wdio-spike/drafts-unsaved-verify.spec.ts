// Unsaved-drafts restart, run 2 (review iteration 2 P0-1 + P1-1): a FRESH
// app process must open the project on the PERSISTED non-default locale
// (uk), raise all FOUR uncommitted drafts from the sidecar, keep strict
// locale isolation, and survive a locale round trip. The uk drafts are
// then committed and exported — the built XML must land under
// Languages/Ukrainian with both texts (F1 with a NON-default locale).
import * as fs from 'node:fs'
import { switchTargetLocale, commitCurrentRow, pause } from './ds-helpers'

const MGR = '/tmp/rimloc-ds/data/managed'
const SNAP_AFTER = '/tmp/rimloc-ds/unsaved-sidecar-after-restart.json'
const OUT = '/tmp/rimloc-ds/export-uk'

async function waitExisting(selector: string, timeout = 20000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

describe('unsaved drafts, run 2 — restart raises four drafts on the persisted uk locale', () => {
  it('uk persisted, four drafts restored, round trip exact, commits + xml export', async () => {
    await browser.waitUntil(
      async () => (await browser.$('.app-sidebar')).isExisting(),
      { timeout: 30000, interval: 100 },
    )
    await browser.execute(() => { window.location.hash = '#/projects' })
    const card = await waitExisting('[data-testid="projects.project-card"]')
    await card.click()
    await waitExisting('[data-testid="ws.root"]', 45000)
    await pause(800)

    // --- P1-1: the persisted locale is UK — NOT the ru default ---
    const sel = await (await waitExisting('[data-testid="ws.target-locale"]')).getValue()
    console.log(`[ds-unsaved2] PERSISTED_LOCALE=${JSON.stringify(sel)}`)
    if (sel !== 'uk') throw new Error(`locale persistence broken: expected uk, got ${sel}`)

    // Honest restore notice (the sidecar held four drafts).
    const noticeText = await browser.execute(
      () => document.querySelector('[data-testid="ws.drafts-notice"]')?.textContent ?? '<none>',
    )
    console.log(`[ds-unsaved2] NOTICE=${JSON.stringify(noticeText)}`)
    if (noticeText === '<none>')
      throw new Error('no restore notice despite four durable drafts')

    // Both rows carry the UK drafts.
    const row0 = await waitExisting('[data-testid="ws.entry.0"]')
    const row1 = await browser.$('[data-testid="ws.entry.1"]')
    await row0.click()
    const uk0 = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    await row1.click()
    const uk1 = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    console.log(`[ds-unsaved2] UK_DRAFTS row0=${JSON.stringify(uk0)} row1=${JSON.stringify(uk1)}`)
    if (uk0 !== 'uk чернетка речі' || uk1 !== 'uk чернетка ефекту')
      throw new Error(`uk drafts wrong after restart: ${JSON.stringify([uk0, uk1])}`)

    // Round trip: ru drafts intact too — all four values exact.
    await switchTargetLocale(browser, 'ru')
    await row0.click()
    const ru0 = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    await row1.click()
    const ru1 = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    console.log(`[ds-unsaved2] RU_DRAFTS row0=${JSON.stringify(ru0)} row1=${JSON.stringify(ru1)}`)
    if (ru0 !== 'ru черновик вещи' || ru1 !== 'ru черновик эффекта')
      throw new Error(`ru drafts wrong after restart: ${JSON.stringify([ru0, ru1])}`)

    // Sidecar AFTER the restart (for the evidence pack).
    const side = fs.readdirSync(MGR).filter((f) => f.includes('.drafts.json'))[0]
    fs.writeFileSync(SNAP_AFTER, fs.readFileSync(`${MGR}/${side}`))

    // Back to uk; commit BOTH uk drafts; the XML export must use the uk dir.
    await switchTargetLocale(browser, 'uk')
    await row0.click()
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('uk переклад речі')
    await commitCurrentRow(browser, { locale: 'Ukrainian', text: 'uk переклад речі' })
    await row1.click()
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('uk переклад ефекту')
    await commitCurrentRow(browser, { locale: 'Ukrainian', text: 'uk переклад ефекту' })

    await browser.execute(() => { window.location.hash = '#/export' })
    const target = await waitExisting('[data-testid="be.target-locale"]')
    const tl = await target.getText()
    console.log(`[ds-unsaved2] EXPORT_LOCALE=${JSON.stringify(tl)}`)
    if (!tl.includes('Ukrainian')) throw new Error(`export locale not Ukrainian: ${tl}`)
    fs.rmSync(OUT, { recursive: true, force: true })
    fs.mkdirSync(OUT, { recursive: true })
    await browser.$('[data-testid="be.outdir"]').setValue(OUT)
    await browser.$('[data-testid="be.build"]').click()
    await waitExisting('[data-testid="be.result"]', 60000)
    await pause(1000)
    const xmls: string[] = []
    const walk = (d: string): void => {
      for (const e of fs.readdirSync(d, { withFileTypes: true })) {
        const p = `${d}/${e.name}`
        if (e.isDirectory()) walk(p)
        else if (e.name.endsWith('.xml')) xmls.push(p)
      }
    }
    walk(OUT)
    const all = xmls.map((p) => fs.readFileSync(p, 'utf8')).join('\n')
    const texts = (all.match(/uk переклад (речі|ефекту)/g) || []).sort()
    console.log(`[ds-unsaved2] UK_XML=${JSON.stringify({ files: xmls.length, texts })}`)
    if (texts.length !== 2) throw new Error(`Ukrainian XML texts wrong: ${texts}`)
    if (!xmls.some((p) => p.includes('Ukrainian')))
      throw new Error('export did not write under Languages/Ukrainian')
  })
})
