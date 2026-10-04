// React-lane smoke (mandate §68: RUN it, don't reason it): boot the REAL
// React app against the REAL backend (isolated data copy), open a managed
// project, verify the representative workspace on live entries, commit an
// edit through the REAL save&next path, verify the durable revision moved.
// Semantic controls only; background-only; zero global input.
/** wdio v9 removed element.waitFor — the ONLY sanctioned wait in this spec
 *  is a bounded existence condition (see soak v3 lesson). */
async function waitExisting(sel: string, timeout = 30000): Promise<void> {
  await browser.waitUntil(async () => (await browser.$(sel)).isExisting(), {
    timeout,
    interval: 100,
  })
}

describe('React R1 lane smoke (real backend, isolated data)', () => {
  it('boots → lists live projects → opens one → edits → commits → revision bumps', async () => {
    // 1) The R1 shell rendered (React mounted, sidebar visible).
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), { timeout: 30000, interval: 100 })
    const brand = await browser.$('.brand').getText()
    if (!brand.includes('RimLoc')) throw new Error(`brand text wrong: ${brand}`)

    // 2) Home lists the LIVE managed projects (isolated copy).
    await browser.waitUntil(async () => (await browser.$('[data-testid="home.project-card"]')).isExisting(), { timeout: 30000, interval: 100 })
    const cards = await browser.$$('[data-testid="home.project-card"]')
    if (cards.length < 1) throw new Error('no live project cards on Home')

    // 3) Open the first project → workspace with real entries.
    await cards[0]!.click()
    await browser.waitUntil(async () => (await browser.$('[data-testid="ws.root"]')).isExisting(), { timeout: 30000, interval: 100 })
    await browser.waitUntil(
      async () => (await browser.$$('[data-testid^="ws.entry."]').length) > 0,
      { timeout: 30000, interval: 200 },
    )
    const rows = await browser.$$('[data-testid^="ws.entry."]')
    if (rows.length < 1) throw new Error('workspace rendered no entries')

    // 4) Revision before the edit (footer carries `rev N`).
    const footerBefore = await browser.$('.ws-footer').getText()
    const revBefore = Number(/rev (\d+)/.exec(footerBefore)?.[1] ?? '-1')

    // 5) Select first row → edit → save & next (real apply intent path).
    await rows[0]!.click()
    const ta = await browser.$('[data-testid="ws.editor-textarea"]')
    await browser.waitUntil(async () => (await browser.$('[data-testid="ws.editor-textarea"]')).isExisting(), { timeout: 10000, interval: 100 })
    const original = await ta.getValue()
    const edited = `${original} [R1-smoke]`
    await ta.setValue(edited)
    await browser.$('[data-testid="ws.editor-save-next"]').click()

    // 6) The commit acked → durable revision bumped.
    await browser.waitUntil(
      async () => {
        const footer = await browser.$('.ws-footer').getText()
        const rev = Number(/rev (\d+)/.exec(footer)?.[1] ?? '-1')
        return rev > revBefore
      },
      { timeout: 30000, interval: 250 },
    )

    // 7) The edited text persisted in the fresh snapshot rows.
    await browser.waitUntil(
      async () => {
        const pageText = await browser.$('[data-testid="ws.root"]').getText()
        return pageText.includes('[R1-smoke]')
      },
      { timeout: 15000, interval: 250 },
    )
  })
})

describe('React R1 lane smoke — checks & glossary (live contract)', () => {
  it('checks: живой валидатор отдаёт отчёт по открытому проекту', async () => {
    await waitExisting('[data-testid="ws.root"]', 30000)
    await browser.$('a[href="#/checks"]').click()
    await waitExisting('[data-testid="checks.findings"]', 30000)
    await browser.waitUntil(
      async () => (await browser.$('[data-testid="checks.findings"]').getText()).length > 0 || (await browser.$('.passed-state').isExisting()),
      { timeout: 30000, interval: 250 },
    )
    const metrics = await browser.$('.metrics-band').getText()
    if (!/0|1|2|3|4|5|6|7|8|9/.test(metrics)) throw new Error('metrics band empty')
  })

  it('glossary: добавление термина через живой CRUD (persist-before-ack)', async () => {
    await browser.$('a[href="#/glossary"]').click()
    await waitExisting('[data-testid="gl.add-term"]', 30000)
    const stamp = `r1-${Date.now()}`
    await browser.$('[data-testid="gl.add-term"]').setValue(stamp)
    await browser.$('[data-testid="gl.add-translation"]').setValue('проверка')
    await browser.$('[data-testid="gl.add-submit"]').click()
    await browser
      .waitUntil(async () => (await browser.$(`[data-testid="gl.row.${stamp}"]`)).isExisting(), {
        timeout: 30000,
        interval: 250,
      })
      .catch(() => {
        throw new Error('glossary row did not appear after live upsert')
      })
  })
})
