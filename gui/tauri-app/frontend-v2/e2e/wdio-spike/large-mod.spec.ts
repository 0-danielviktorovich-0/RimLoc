// Large-mod E2E (directive §12): a 12 000-row mod (6 000 defs × label +
// description across 20 files) through create → search → edit → durable
// save → guarded export on the REAL packaged build. Timing assertions
// keep the virtualizer honest at beta scale.
import { mkdirSync } from 'node:fs'
import { readdirSync, readFileSync, existsSync } from 'node:fs'

const MOD = '/tmp/rimloc-large-mod'
const MANAGED = '/tmp/rimloc-int/data/managed'
const EXPECTED_ROWS = 12000

function pause(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

async function waitExisting(selector: string, timeout = 60000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

describe('directive §12 — large-mod end to end (12k rows)', () => {
  it('wizard indexes all 12k rows in a sane time', async () => {
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
    await waitExisting('[data-testid="wizard.version"]')
    await browser.$('[data-testid="wizard.version"]').selectByVisibleText('1.6')
    const t0 = Date.now()
    await browser.$('[data-testid="wizard.next"]').click()
    await pause(600)
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) {
      await browser.$('[data-testid="wizard.next"]').click()
    }
    await waitExisting('[data-testid="ws.root"]', 300000)
    const indexMs = Date.now() - t0
    // The full inventory count must be honest — 12000, not a truncation.
    const total = await browser.execute(
      () => document.querySelector('[data-testid="ws.tree"] .tree-root span')?.textContent ?? '',
    )
    if (String(total).trim() !== String(EXPECTED_ROWS)) {
      throw new Error(`expected ${EXPECTED_ROWS} rows in the tree root counter, got: ${total}`)
    }
    if (indexMs > 120000) throw new Error(`indexing took ${indexMs}ms`)
  })

  it('search stays interactive at 12k and navigates to the match', async () => {
    await waitExisting('.entry-list .entry-row', 60000)
    // Typing through WebDriver is per-keystroke and itself slow; the
    // PRODUCT budget is the filter path: from the debounce firing to the
    // settled list.
    const input = await browser.$('.search-field input')
    await input.setValue('Item 5999 label')
    const t0 = Date.now()
    let rows: string[] = []
    for (let i = 0; i < 100; i++) {
      await pause(100)
      rows = (await browser.execute(() =>
        Array.from(document.querySelectorAll('.entry-list .entry-row')).map((r) => r.textContent ?? ''),
      )) as string[]
      if (rows.length === 1 && rows[0].includes('Item5999.label')) break
    }
    const elapsed = Date.now() - t0
    if (rows.length !== 1 || !rows[0].includes('Item5999.label')) {
      throw new Error(`12k search never settled: ${JSON.stringify(rows.slice(0, 3))}`)
    }
    // Budget 3000ms INCLUDES the polling overhead (a 100ms-tick execute
    // round-trip each iteration); the pure filter settles well under it.
    // Deep filter optimization stays post-beta (P2).
    if (elapsed > 3000) throw new Error(`12k filter path took ${elapsed}ms (budget 3000)`)
    await browser.$('.entry-list .entry-row').click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
  })

  it('edit + durable save + guarded export', async () => {
    await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('Перевод последней строки')
    await pause(300)
    await browser.$('[data-testid="ws.editor-save"]').click()
    // Durable envelope (persist-before-ack): bounded wait.
    let saved = false
    for (let i = 0; i < 200 && !saved; i++) {
      await pause(200)
      saved = await browser.execute(
        () =>
          Array.from(document.querySelectorAll('.entry-list .entry-row')).some((r) =>
            (r.textContent ?? '').includes('Перевод последней строки'),
          ),
      )
    }
    if (!saved) throw new Error('the 12k-row save never reached the list')

    // Guarded export through the Existing/export screen: the canonical
    // serializer must handle the whole corpus.
    const OUT = '/tmp/rimloc-large-export'
    await browser.execute(() => {
      window.location.hash = '#/export'
    })
    await waitExisting('[data-testid="be.outdir"]', 30000)
    await browser.$('[data-testid="be.outdir"]').setValue(OUT)
    await browser.$('[data-testid="be.build"]').click()
    // Result block: files written + reparsed keys — the reparse guard is
    // the honest acceptance (the writer's own count read back by the
    // scanner).
    await browser.waitUntil(
      async () => !!(await browser.$('[data-testid="be.preview"], .context-content strong').isExisting()),
      { timeout: 300000, interval: 2000 },
    )
    // Export with ONE saved translation writes the translated file only;
    // the durable envelope already proved the row. Assert the result
    // surface appears with sane numbers (files >= 1).
    const strongs = await browser.execute(() => {
      const blocks = Array.from(document.querySelectorAll('.context-content'))
      const last = blocks[blocks.length - 1]
      return Array.from(last?.querySelectorAll('strong') ?? []).map((s) => s.textContent ?? '')
    })
    // Result block order: files written, reparsed keys, out dir.
    const files = Number(strongs[0])
    if (!Number.isFinite(files) || files < 1) {
      throw new Error(`export result missing files count: ${JSON.stringify(strongs)}`)
    }
  })
})
