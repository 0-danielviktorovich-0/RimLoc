/**
 * RC IN-GAME RE-ACCEPTANCE — leg 1: build the translation with the REAL
 * RC product over the game's own canary mod, then guarded-export it for
 * mounting as the game's translation mod.
 * Strings: RimLocCanaryGreeting -> «Привет, канареечный мир» and
 * RimLocCanaryBeacon.label -> «Маяк проверки владельца».
 */
import { mkdirSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const GAME_CANARY = '/Applications/RimWorld.app/Mods/RimLocGameCanary'
const EXPORT_DIR = '/tmp/rimloc-rw-lab/rc-export'
const RU_GREETING = 'Привет, канареечный мир'
const RU_LABEL = 'Маяк проверки владельца'

async function waitFor(fn: () => Promise<boolean>, timeoutMs = 15000, intervalMs = 300): Promise<void> {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    if (await fn()) return
    await new Promise(r => setTimeout(r, intervalMs))
  }
  throw new Error('waitFor timed out after ' + timeoutMs + 'ms')
}

async function waitExisting(sel: string, timeout = 30000) {
  const e = await browser.$(sel)
  await e.waitForExist({ timeout })
  return e
}

describe('RC in-game leg 1: translate the game canary and export', () => {
  it('wizard → translate both entries → guarded export', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), { timeout: 60000, interval: 200 })
    await (await browser.$('.app-sidebar a[href="#/projects"]')).click()
    await (await waitExisting('[data-testid="wizard.open"]')).click()
    await (await waitExisting('[data-testid="wizard.path-input"]')).setValue(GAME_CANARY)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]', 60000)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitFor(async () => !(await (await browser.$('[data-testid="wizard.overlay"]')).isExisting()), 10000).catch(() => {})
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="ws.root"]', 300000)
    await waitExisting('.entry-list .entry-row', 60000)

    // translate the Keyed greeting
    await waitFor(async () => {
      const rows = await browser.$$('.entry-list .entry-row')
      for (const r of rows) {
        const txt = (await r.getText()).toLowerCase()
        if (txt.includes('rimloccanarygreeting')) { await r.click(); return true }
      }
      return false
    }, 30000, 500)
    await (await waitExisting('[data-testid="ws.editor-textarea"]', 15000)).setValue(RU_GREETING)
    await browser.$('[data-testid="ws.editor-save"]').click()
    await new Promise(r => setTimeout(r, 900))

    // translate the Def label
    await waitFor(async () => {
      const rows = await browser.$$('.entry-list .entry-row')
      for (const r of rows) {
        const txt = (await r.getText()).toLowerCase()
        if (txt.includes('rimloccanarybeacon.label')) { await r.click(); return true }
      }
      return false
    }, 30000, 500)
    await (await waitExisting('[data-testid="ws.editor-textarea"]', 15000)).setValue(RU_LABEL)
    await browser.$('[data-testid="ws.editor-save"]').click()
    await new Promise(r => setTimeout(r, 900))

    // guarded export
    await (await browser.$('.app-sidebar a[href="#/export"]')).click()
    const outdir = await waitExisting('[data-testid="be.outdir"]', 30000)
    await outdir.setValue(EXPORT_DIR)
    await browser.$('[data-testid="be.export"]').click()
    await waitFor(async () => {
      try {
        const walk = (d: string): string[] => {
          const fs = require('node:fs')
          return fs.readdirSync(d, { withFileTypes: true }).flatMap((e: any) =>
            e.isDirectory() ? walk(join(d, e.name)) : [join(d, e.name)])
        }
        return walk(EXPORT_DIR).some((f: string) => f.endsWith('.xml'))
      } catch { return false }
    }, 60000, 1000)
    console.log('RC_EXPORT_DONE')
  })
})
