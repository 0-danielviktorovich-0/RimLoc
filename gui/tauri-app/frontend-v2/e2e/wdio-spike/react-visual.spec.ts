// Visual evidence capture (§80): real app, real data, light+dark workspace
// frames for the independent visual critic. Background-only.
describe('React R1 visual evidence', () => {
  it('captures home + workspace (light/dark) on live data', async () => {
    const out = '/tmp/rimloc-r1-visual'
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), { timeout: 30000, interval: 100 })
    // light home
    let shot = await browser.takeScreenshot()
    await browser.saveScreenshot(`${out}/home-light.png`)
    // dark toggle
    const darkBtn = await browser.$('button[aria-label*="Темная"], button[aria-label*="Тёмная"]')
    if (await darkBtn.isExisting()) {
      await darkBtn.click()
      await new Promise((r) => setTimeout(r, 300))
      shot = await browser.takeScreenshot()
      await browser.saveScreenshot(`${out}/home-dark.png`)
      await darkBtn.click() // back to light
      await new Promise((r) => setTimeout(r, 300))
    }
    // open first project → workspace
    await browser.waitUntil(async () => (await browser.$('[data-testid="home.project-card"]')).isExisting(), { timeout: 30000, interval: 100 })
    await browser.$('[data-testid="home.project-card"]').click()
    await browser.waitUntil(async () => (await browser.$('[data-testid="ws.root"]')).isExisting(), { timeout: 30000, interval: 100 })
    await browser.waitUntil(
      async () => (await browser.$$('[data-testid^="ws.entry."]').length) > 0,
      { timeout: 30000, interval: 200 },
    )
    await browser.$('[data-testid^="ws.entry."]').click()
    await browser.$('[data-testid="ws.editor-textarea"]').click()
    await new Promise((r) => setTimeout(r, 400))
    shot = await browser.takeScreenshot()
    await browser.saveScreenshot(`${out}/workspace-light.png`)
    if (await darkBtn.isExisting()) {
      await darkBtn.click()
      await new Promise((r) => setTimeout(r, 300))
      await browser.saveScreenshot(`${out}/workspace-dark.png`)
    }
  })
})
