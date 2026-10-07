// Seed (rel22): wizard-create a project from /tmp/rimloc-wizard-mod so the
// palette-lm profile carries a live open project for the acceptance suites.
// The session persists in RIMLOC_DATA_DIR — later app launches reopen it,
// which is what the palette/LM route markers expect.
async function waitExisting(selector: string, timeout = 20000): Promise<void> {
  await browser.waitUntil(
    async () => (await browser.$(selector)).isExisting(),
    { timeout, interval: 250 },
  )
}

describe('seed: open project in the isolated profile', () => {
  it('wizard-create from /tmp/rimloc-wizard-mod and reach #/workspace', async () => {
    await waitExisting('.app-sidebar', 30000)
    await browser.execute(() => { window.location.hash = '#/projects' })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue('/tmp/rimloc-wizard-mod')
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]')
    await browser.$('[data-testid="wizard.version"]').selectByVisibleText('1.6')
    await browser.$('[data-testid="wizard.next"]').click()
    await browser.$('[data-testid="wizard.next"]').click()
    await browser.waitUntil(
      async () => (await browser.execute(() => window.location.hash)) === '#/workspace',
      { timeout: 60000, interval: 250 },
    )
    await waitExisting('[data-testid="ws.root"]', 30000)
  })
})
