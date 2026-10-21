/**
 * Build the canary translation through the REAL product (packaged build):
 * wizard over the game-valid canary → translate keyed + def label →
 * guarded export → the artifact the in-game acceptance mounts.
 */
const MOD = '/Applications/RimWorld.app/Mods/RimLocGameCanary'
const OUT = '/tmp/rimloc-rw-lab/canary-export'

async function pause(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}
async function waitExisting(selector: string, timeout = 30000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

describe('canary: real-product export for in-game acceptance', () => {
  it('wizard → translate both entries → export', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), {
      timeout: 60000, interval: 200,
    })
    await browser.execute(() => { window.location.hash = '#/projects' })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue(MOD)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]', 60000)
    await browser.$('[data-testid="wizard.next"]').click()
    await pause(800)
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) {
      await browser.$('[data-testid="wizard.next"]').click()
    }
    await waitExisting('[data-testid="ws.root"]', 300000)
    await waitExisting('.entry-list .entry-row', 60000)

    // Translate the keyed greeting.
    let input = await browser.$('.search-field input')
    await input.setValue('Hello canary world')
    await pause(900)
    await browser.$('.entry-list .entry-row').click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('Привет, канареечный мир')
    await browser.$('[data-testid="ws.editor-save"]').click()
    let saved = false
    for (let i = 0; i < 150 && !saved; i++) {
      await pause(200)
      saved = await browser.execute(() =>
        Array.from(document.querySelectorAll('.entry-list .entry-row')).some((r) =>
          (r.textContent ?? '').includes('Привет, канареечный мир')))
    }
    if (!saved) throw new Error('keyed translation did not land')

    // Translate the def label (canary beacon).
    input = await browser.$('.search-field input')
    await input.setValue('canary beacon')
    await pause(900)
    await browser.$('.entry-list .entry-row').click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('Канареечный маяк')
    await browser.$('[data-testid="ws.editor-save"]').click()
    saved = false
    for (let i = 0; i < 150 && !saved; i++) {
      await pause(200)
      saved = await browser.execute(() =>
        Array.from(document.querySelectorAll('.entry-list .entry-row')).some((r) =>
          (r.textContent ?? '').includes('Канареечный маяк')))
    }
    if (!saved) throw new Error('def-label translation did not land')

    // Guarded export.
    await browser.execute(() => { window.location.hash = '#/export' })
    await waitExisting('[data-testid="be.outdir"]', 30000)
    await browser.$('[data-testid="be.outdir"]').setValue(OUT)
    await browser.$('[data-testid="be.build"]').click()
    await browser.waitUntil(async () => {
      const strongs = await browser.execute(() => {
        const blocks = Array.from(document.querySelectorAll('.context-content'))
        const last = blocks[blocks.length - 1]
        return Array.from(last?.querySelectorAll('strong') ?? []).map((s) => s.textContent ?? '')
      })
      return Array.isArray(strongs) && strongs.length >= 2 && Number(strongs[0]) >= 1
    }, { timeout: 300000, interval: 2000 })
  })
})
