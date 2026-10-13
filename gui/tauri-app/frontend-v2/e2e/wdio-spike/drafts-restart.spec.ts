// Draft-safety restart leg (rel26-draft-fixes): the drafts written by
// drafts-safety.spec.ts must come back after a full app restart — restore
// notice raised, both collision rows still independent, drafts on their rows.
function pause(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

async function waitExisting(selector: string, timeout = 20000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

describe('draft safety, phase 2 — drafts survive an app restart', () => {
  it('restore notice + drafts on both collision rows after relaunch', async () => {
    await browser.waitUntil(
      async () => (await browser.$('.app-sidebar')).isExisting(),
      { timeout: 30000, interval: 100 },
    )
    await browser.execute(() => { window.location.hash = '#/projects' })
    const card = await waitExisting('[data-testid="projects.project-card"]')
    await card.click()
    await waitExisting('[data-testid="ws.root"]', 45000)
    await pause(800)

    // Honest restore notice: the sidecar had two drafts for the ru slice.
    const notice = await browser.$('[data-testid="ws.drafts-notice"]')
    const noticeText = notice.isExisting() ? await notice.getText() : '<absent>'
    console.log(`[ds-restart] NOTICE=${JSON.stringify(noticeText)}`)
    if (!notice.isExisting()) throw new Error('restore notice did not appear after restart')

    // Row 0 keeps its draft; row 1 keeps the other one.
    const row0 = await waitExisting('[data-testid="ws.entry.0"]')
    await row0.click()
    const editor = await waitExisting('[data-testid="ws.editor-textarea"]')
    const v0 = await editor.getValue()
    console.log(`[ds-restart] ROW0=${JSON.stringify(v0)}`)
    if (v0 !== 'черновик вещи') throw new Error(`row 0 draft not restored: ${JSON.stringify(v0)}`)

    // Find the OTHER row (inventory order is stable: 0 and 1).
    const row1 = await browser.$('[data-testid="ws.entry.1"]')
    await row1.click()
    const editor1 = await waitExisting('[data-testid="ws.editor-textarea"]')
    const v1 = await editor1.getValue()
    console.log(`[ds-restart] ROW1=${JSON.stringify(v1)}`)
    if (v1 !== 'черновик эффекта') throw new Error(`row 1 draft not restored: ${JSON.stringify(v1)}`)

    // Locale round trip AFTER restart: the ru drafts stay put, uk stays empty.
    const localeSelect = await waitExisting('[data-testid="ws.target-locale"]')
    await localeSelect.selectByAttribute('value', 'uk')
    await pause(400)
    const uk = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    await localeSelect.selectByAttribute('value', 'ru')
    await pause(400)
    const ru = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    console.log(`[ds-restart] UK_AFTER_RESTART=${JSON.stringify(uk)} RU_BACK=${JSON.stringify(ru)}`)
    if (ru !== 'черновик эффекта')
      throw new Error(`draft lost after post-restart locale round trip: ${JSON.stringify(ru)}`)
  })
})
