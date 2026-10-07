// Multi-target §32, self-contained (rel22 locale-form fix gate): commit a ru
// edit through the REAL editor path (draft → commit → session.apply with the
// folder-contract locale), watch it display, verify the uk phase re-maps
// empty, and the ru state survives the round-trip. No inherited profile
// state — the [R1-smoke] fixture of the old lane died with its data dir.
async function waitExisting(sel: string, timeout = 20000): Promise<void> {
  await browser.waitUntil(async () => (await browser.$(sel)).isExisting(), { timeout, interval: 250 })
}

describe('React R1 multi-target (§32)', () => {
  before(async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), { timeout: 30000, interval: 250 })
    const needProject = await browser.execute(async () => {
      window.location.hash = '#/workspace'
      await new Promise((r) => setTimeout(r, 800))
      return !document.querySelector('[data-testid="ws.root"]')
    })
    if (needProject) {
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
    }
    await waitExisting('[data-testid="ws.root"]', 30000)
    // Deterministic phase: the target switcher starts at the registry default.
    await browser.execute(() => {
      const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement | null
      if (sel) {
        sel.value = 'ru'
        sel.dispatchEvent(new Event('change', { bubbles: true }))
      }
    })
    await new Promise((r) => setTimeout(r, 500))
  })

  it('commit ru → виден → uk пуст → возврат ru сохраняет', async () => {
    // Pick the first entry row and type a marker translation.
    await browser.execute(() => {
      const row = document.querySelector('[data-testid="ws.entry.0"], .entry-row') as HTMLElement | null
      row?.click()
    })
    await waitExisting('[data-testid="ws.editor-textarea"]')
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('[MT-smoke] привет')
    await browser.$('[data-testid="ws.editor-save-next"]').click()
    // commit → persist-before-ack → entries re-map; wait the marker to show.
    await browser.waitUntil(
      async () => (await browser.$('[data-testid="ws.root"]').getText()).includes('[MT-smoke]'),
      { timeout: 20000, interval: 250 },
    )

    // uk phase: the same snapshot re-maps; the ru edit must NOT leak.
    await browser.execute(() => {
      const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement
      sel.value = 'uk'
      sel.dispatchEvent(new Event('change', { bubbles: true }))
    })
    await new Promise((r) => setTimeout(r, 600))
    const ukText = await browser.$('[data-testid="ws.root"]').getText()
    if (ukText.includes('[MT-smoke]')) throw new Error('uk показывает ru-перевод — цели НЕ изолированы')

    // Round-trip: ru state restored from the same snapshot (folder-form hit).
    await browser.execute(() => {
      const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement
      sel.value = 'ru'
      sel.dispatchEvent(new Event('change', { bubbles: true }))
    })
    await new Promise((r) => setTimeout(r, 600))
    const ruAfter = await browser.$('[data-testid="ws.root"]').getText()
    if (!ruAfter.includes('[MT-smoke]')) throw new Error('ru потерял правку после round-trip')
  })
})
