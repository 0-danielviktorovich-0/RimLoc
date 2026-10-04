describe('diag', () => {
  it('dumps workspace dom head', async () => {
    await browser.waitUntil(async () => (await browser.$('[data-testid="home.project-card"]')).isExisting(), { timeout: 30000, interval: 100 })
    await browser.$('[data-testid="home.project-card"]').click()
    await browser.waitUntil(async () => (await browser.$('[data-testid="ws.root"]')).isExisting(), { timeout: 30000, interval: 100 })
    const info = await browser.execute(() => {
      const rc = document.querySelector('.route-content')
      const wh = document.querySelector('.workspace-heading')
      const page = document.querySelector('.ws-page')
      return {
        rcChildren: rc ? [...rc.children].map((c) => c.className) : null,
        whExists: !!wh,
        whText: wh ? wh.textContent?.slice(0, 60) : null,
        whHeight: wh ? wh.getBoundingClientRect().height : null,
        pageChildren: page ? [...page.children].map((c) => c.className) : null,
        hash: window.location.hash,
      }
    })
    console.log('[diag]', JSON.stringify(info))
  })
})
