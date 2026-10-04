// Multi-target §32: switch target locale in the workspace — the SAME
// snapshot re-maps to the other locale's translations (independent state).
describe('React R1 multi-target (§32)', () => {
  it('переключение цели ре-мапит переводы из того же снапшота', async () => {
    await browser.waitUntil(async () => (await browser.$('[data-testid="home.project-card"]')).isExisting(), { timeout: 30000, interval: 100 })
    // Use the wizard project (known: ru translation was committed in J3).
    await browser.$('a[href="#/home"]').click()
    await new Promise((r) => setTimeout(r, 1200))
    await browser.waitUntil(
      async () => {
        const cards = await browser.$$('[data-testid="home.project-card"]')
        for (const c of cards) {
          if ((await c.getText()).includes('rimloc-wizard-mod')) { await c.click(); return true }
        }
        return false
      },
      { timeout: 30000, interval: 250 },
    )
    await browser.waitUntil(async () => (await browser.$('[data-testid="ws.root"]')).isExisting(), { timeout: 30000, interval: 100 })

    // ru: the committed [R1-smoke] edit is visible
    const switcher = await browser.$('[data-testid="ws.target-locale"]')
    await browser.waitUntil(async () => (await browser.$('[data-testid="ws.target-locale"]')).isExisting(), { timeout: 30000, interval: 100 })
    const ruTextBefore = await browser.$('[data-testid="ws.root"]').getText()
    if (!ruTextBefore.includes('[R1-smoke]')) throw new Error('ru target lacks the committed edit')

    // switch to uk: translations re-map (empty targets — never ru leftovers)
    await browser.execute(() => {
      const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement
      sel.value = 'uk'
      sel.dispatchEvent(new Event('change', { bubbles: true }))
    })
    await new Promise((r) => setTimeout(r, 500))
    const ukText = await browser.$('[data-testid="ws.root"]').getText()
    if (ukText.includes('[R1-smoke]')) throw new Error('uk target shows ru translations — targets are NOT independent')

    // switch back: ru state restored from the same snapshot
    await browser.execute(() => {
      const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement
      sel.value = 'ru'
      sel.dispatchEvent(new Event('change', { bubbles: true }))
    })
    await new Promise((r) => setTimeout(r, 500))
    const ruTextAfter = await browser.$('[data-testid="ws.root"]').getText()
    if (!ruTextAfter.includes('[R1-smoke]')) throw new Error('ru target lost state after round-trip')
  })
})
