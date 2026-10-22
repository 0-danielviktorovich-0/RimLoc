// A2 (Issue #9): the REAL RimLoc GUI translates a LIVE mod string
// (VFE Security VFES_Turret_AntiAir.label "anti-air turret") and the
// guarded exporter builds the translation mod — in an ISOLATED RimLoc
// data dir so the owner's projects stay untouched. Part 2 (shell/bridge)
// mounts the export into the isolated GAME profile and proves the label
// renders in Russian in-game.
import { pause } from './search-editor.util'

const MOD = '/Applications/RimWorld.app/Mods/1845154007'
const OUT = '/tmp/rimloc-rw-lab/a2-export'

async function waitExisting(selector: string, timeout = 30000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

describe('A2 — real GUI translation of a live mod string', () => {
  it('wizard → search → edit → durable save → guarded export', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), {
      timeout: 60000,
      interval: 200,
    })
    await browser.execute(() => {
      window.location.hash = '#/projects'
    })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue(MOD)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]', 60000)
    await browser.$('[data-testid="wizard.version"]').selectByVisibleText('1.6')
    await browser.$('[data-testid="wizard.next"]').click()
    await pause(800)
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) {
      await browser.$('[data-testid="wizard.next"]').click()
    }
    await waitExisting('[data-testid="ws.root"]', 300000)

    // Find the REAL string through the workspace search.
    await waitExisting('.entry-list .entry-row', 60000)
    const input = await browser.$('.search-field input')
    await input.setValue('anti-air turret')
    await pause(900)
    let rows: string[] = []
    for (let i = 0; i < 60; i++) {
      rows = (await browser.execute(() =>
        Array.from(document.querySelectorAll('.entry-list .entry-row')).map(
          (r) => r.textContent ?? '',
        ),
      )) as string[]
      if (rows.length >= 1 && rows[0].includes('anti-air turret')) break
      await pause(300)
    }
    if (rows.length < 1 || !rows[0].includes('anti-air turret')) {
      throw new Error(`canary row not found: ${JSON.stringify(rows.slice(0, 2))}`)
    }
    await browser.$('.entry-list .entry-row').click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 15000)

    await browser.$('[data-testid="ws.editor-textarea"]').setValue('ПВО турель (RimLoc)')
    await pause(300)
    await browser.$('[data-testid="ws.editor-save"]').click()
    let saved = false
    for (let i = 0; i < 150 && !saved; i++) {
      await pause(200)
      saved = await browser.execute(() =>
        Array.from(document.querySelectorAll('.entry-list .entry-row')).some((r) =>
          (r.textContent ?? '').includes('ПВО турель (RimLoc)'),
        ),
      )
    }
    if (!saved) throw new Error('the live-mod translation never reached the list')

    await browser.$('[data-testid="ws.clear-filters"]').click()
    await pause(400)
    await browser.execute(() => {
      window.location.hash = '#/export'
    })
    await waitExisting('[data-testid="be.outdir"]', 30000)
    await browser.$('[data-testid="be.outdir"]').setValue(OUT)
    await browser.$('[data-testid="be.build"]').click()
    await browser.waitUntil(
      async () => {
        const strongs = await browser.execute(() => {
          const blocks = Array.from(document.querySelectorAll('.context-content'))
          const last = blocks[blocks.length - 1]
          return Array.from(last?.querySelectorAll('strong') ?? []).map(
            (s) => s.textContent ?? '',
          )
        })
        return Array.isArray(strongs) && strongs.length >= 2 && Number(strongs[0]) >= 1
      },
      { timeout: 300000, interval: 2000 },
    )
  })
})
