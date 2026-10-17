// Draft-safety restart leg (rel26, review iteration 2): a fresh app process
// must (a) open the project on the PERSISTED target locale (F1 class),
// (b) show STRICT locale isolation — the uk view holds no ru records,
// (c) keep the two committed structural translations on their exact rows,
// (d) carry them in the durable project envelope with full identities.
// Locale switches go through the SHARED helper (review P0-1 oracle fix:
// the synthetic option-click was a silent no-op and invalidated this
// spec's earlier uk assertion).
import * as fs from 'node:fs'
import { switchTargetLocale, pause } from './ds-helpers'

async function waitExisting(selector: string, timeout = 20000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

function rowDraft(uid: string): Promise<string> {
  return browser.$('[data-testid="ws.editor-textarea"]').getValue()
}

describe('draft safety, phase 2 — restart: persisted locale, isolation, committed identities', () => {
  it('opens on the persisted locale with clean isolation and both commits', async () => {
    await browser.waitUntil(
      async () => (await browser.$('.app-sidebar')).isExisting(),
      { timeout: 30000, interval: 100 },
    )
    await browser.execute(() => { window.location.hash = '#/projects' })
    const card = await waitExisting('[data-testid="projects.project-card"]')
    await card.click()
    await waitExisting('[data-testid="ws.root"]', 45000)
    await pause(800)

    // --- F1 class: the target locale PERSISTED across the restart ---
    // Phase 1 left the project on 'ru'; a fresh session must open on 'ru',
    // not the historical default of some other slice.
    const sel = await (await waitExisting('[data-testid="ws.target-locale"]')).getValue()
    console.log(`[ds-restart] PERSISTED_LOCALE=${JSON.stringify(sel)}`)
    if (sel !== 'ru') throw new Error(`target locale did not persist: ${sel}`)

    // Restore notice for the drafts phase 1 left in the sidecar? The
    // committed phase-1 run ENDED with an EMPTY store — no notice is the
    // honest expectation here. Read via execute: the element may unmount
    // between the existence check and the text read.
    const noticeText = await browser.execute(
      () => document.querySelector('[data-testid="ws.drafts-notice"]')?.textContent ?? '<none>',
    )
    console.log(`[ds-restart] NOTICE=${JSON.stringify(noticeText)}`)

    // --- P0-2: both committed translations live on their EXACT rows ---
    const row0 = await waitExisting('[data-testid="ws.entry.0"]')
    const row1 = await browser.$('[data-testid="ws.entry.1"]')
    await row0.click()
    const t0 = await rowDraft('0')
    await row1.click()
    const t1 = await rowDraft('1')
    console.log(`[ds-restart] ROW0=${JSON.stringify(t0)} ROW1=${JSON.stringify(t1)}`)
    if (t0 !== 'перевод вещи' || t1 !== 'перевод эффекта')
      throw new Error(`committed translations wrong after restart: ${JSON.stringify([t0, t1])}`)

    // --- P0-1 (strict): switch to uk — BOTH rows must show NO ru records;
    // a foreign draft/translation appearing here is data corruption.
    await switchTargetLocale(browser, 'uk')
    await row0.click()
    const uk0 = await rowDraft('0')
    await row1.click()
    const uk1 = await rowDraft('1')
    console.log(`[ds-restart] UK_ISOLATION row0=${JSON.stringify(uk0)} row1=${JSON.stringify(uk1)}`)
    if (uk0 !== '' || uk1 !== '')
      throw new Error(`FOREIGN records in the uk slice after restart: ${JSON.stringify([uk0, uk1])}`)
    await switchTargetLocale(browser, 'ru')
    await row0.click()
    const back0 = await rowDraft('0')
    if (back0 !== 'перевод вещи') throw new Error(`ru target lost: ${JSON.stringify(back0)}`)

    // --- durable envelope carries both full identities ---
    const mgr = '/tmp/rimloc-ds/data/managed'
    const env = fs.readdirSync(mgr).filter((f) => f.endsWith('.rimloc.json'))[0]
    const project = JSON.parse(fs.readFileSync(`${mgr}/${env}`, 'utf8'))
    // The envelope keeps translations at the TOP LEVEL (the DTO nests them
    // under project — the file does not).
    const trs = (project.translations ?? []).filter((t: { locale: string }) => t.locale === 'Russian')
    const ids = trs.map((t: { source_id: { kind: string; key: string; def_type?: string }; text: string }) =>
      `${t.source_id.kind}·${t.source_id.def_type}·${t.source_id.key}=${t.text}`)
    console.log(`[ds-restart] ENVELOPE=${JSON.stringify(ids)}`)
    if (ids.length !== 2) throw new Error(`expected 2 Russian translations, got ${ids.length}`)
    const want = [
      'def_injected·HediffDef·Dup.label=перевод вещи',
      'def_injected·ThingDef·Dup.label=перевод эффекта',
    ]
    for (const w of want) if (!ids.includes(w)) throw new Error(`envelope missing ${w}: ${JSON.stringify(ids)}`)
  })
})
