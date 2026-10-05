// Visual evidence capture (§80): real app, real data — home, workspace,
// checks, glossary; light+dark for key screens. Background-only.
describe('React R1 visual evidence', () => {
  it('captures home + workspace + checks + glossary (light/dark)', async () => {
    const out = '/tmp/rimloc-r1-visual'
    await waitVisual('.app-sidebar')
    await snap('home-light')
    const darkBtn = () => browser.$('[data-testid="theme-toggle"]')
    if (await (await darkBtn()).isExisting()) {
      await (await darkBtn()).click()
      await pause(300)
      await snap('home-dark')
      await (await darkBtn()).click()
      await pause(300)
    }
    await waitVisual('[data-testid="home.project-card"]')
    await (await browser.$('[data-testid="home.project-card"]')).click()
    await waitVisual('[data-testid="ws.root"]')
    await waitVisual('[data-testid^="ws.entry."]')
    await (await browser.$('[data-testid^="ws.entry."]').catch(() => browser.$$('[data-testid^="ws.entry."]').then((a) => a[0]))).click()
    await pause(400)
    await snap('workspace-light')
    if (await (await darkBtn()).isExisting()) {
      await (await darkBtn()).click()
      await pause(300)
      await snap('workspace-dark')
      await (await darkBtn()).click()
      await pause(200)
    }
    // checks
    await (await browser.$('a[href="#/checks"]')).click()
    await pause(1200)
    await snap('checks-light')
    // glossary
    await (await browser.$('a[href="#/glossary"]')).click()
    await pause(1200)
    await snap('glossary-light')

    async function snap(name: string): Promise<void> {
      const b64 = await browser.takeScreenshot()
      await browser.saveScreenshot(`${out}/${name}.png`)
      void b64
    }
    async function pause(ms: number): Promise<void> {
      await new Promise((r) => setTimeout(r, ms))
    }
    async function waitVisual(sel: string): Promise<void> {
      await browser.waitUntil(async () => (await browser.$(sel)).isExisting(), { timeout: 30000, interval: 100 })
    }
  })
})
