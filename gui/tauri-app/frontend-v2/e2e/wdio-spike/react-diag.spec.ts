describe('diag locale switch', () => {
  it('dumps switcher state after select', async () => {
    await browser.waitUntil(async () => (await browser.$('[data-testid="home.project-card"]')).isExisting(), { timeout: 30000, interval: 100 })
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
    await new Promise((r) => setTimeout(r, 800))
    await browser.$('[data-testid="ws.target-locale"]').selectByAttribute('value', 'uk')
    await new Promise((r) => setTimeout(r, 600))
    const info = await browser.execute(() => {
      const sel = document.querySelector('[data-testid="ws.target-locale"]')
      const rows = [...document.querySelectorAll('[data-testid^="ws.entry."]')]
      return {
        selectValue: sel ? (sel as HTMLSelectElement).value : 'none',
        firstRowText: rows[0]?.textContent?.slice(0, 120) ?? 'no rows',
        smokeVisible: document.body.textContent?.includes('[R1-smoke]') ?? false,
      }
    })
    console.log('[diag-mt]', JSON.stringify(info))
  })
})
