// r4-e2e phase 4: probe the restart-default-locale chain — fresh instance,
// reopen from recents, go STRAIGHT to export and build (no re-switch).
// Observation for the target-locale-persistence finding. TEMPORARY.
import fs from 'node:fs'
import assert from 'node:assert'

async function waitExisting(sel: string, timeout = 30000): Promise<void> {
  await browser.waitUntil(async () => (await browser.$(sel)).isExisting(), {
    timeout,
    interval: 250,
  })
}
async function pause(ms: number): Promise<void> {
  await new Promise((r) => setTimeout(r, ms))
}
async function js<T>(fn: () => T): Promise<T> {
  return browser.execute(fn) as unknown as T
}

describe('r4-e2e phase4: build straight after restart (locale reset chain)', () => {
  it('reopen → export screen default locale → build', async () => {
    await waitExisting('.app-sidebar', 40000)
    await js(() => {
      window.location.hash = '#/projects'
    })
    await waitExisting('[data-testid="home.project-card"]')
    await browser.$('[data-testid="home.project-card"]').click()
    await waitExisting('[data-testid="ws.root"]', 60000)
    await pause(600)
    const wsState = await js(() => ({
      select: (document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement)?.value ?? '',
      eyebrow: document.querySelector('.heading-eyebrow')?.textContent ?? '',
    }))
    console.log(`[r4] §P4 after reopen select=${wsState.select} eyebrow=${JSON.stringify(wsState.eyebrow)}`)
    // straight to export WITHOUT re-switching
    await js(() => {
      window.location.hash = '#/export'
    })
    await waitExisting('[data-testid="be.outdir"]')
    const exportDefault = await js(() => ({
      eyebrow: document.querySelector('.heading-eyebrow')?.textContent ?? '',
      note: document.querySelector('.safe-label')?.textContent ?? '',
    }))
    console.log(`[r4] §P4 export screen eyebrow=${JSON.stringify(exportDefault.eyebrow)}`)
    fs.rmSync('/tmp/r4-e2e/out-ru', { recursive: true, force: true })
    await browser.$('[data-testid="be.outdir"]').setValue('/tmp/r4-e2e/out-ru')
    await browser.$('[data-testid="be.build"]').click()
    await browser.waitUntil(
      async () =>
        (await browser.execute(
          () =>
            Boolean(document.querySelector('[data-testid="be.result"] .context-content')) ||
            Boolean(document.querySelector('[data-testid="be.error"]')),
        )) as boolean,
      { timeout: 120000, interval: 400 },
    )
    const res = await js(() => ({
      result: document.querySelector('[data-testid="be.result"]')?.textContent ?? '',
      error: document.querySelector('[data-testid="be.error"]')?.textContent ?? '',
    }))
    console.log(`[r4] §P4 BUILD result=${JSON.stringify(res.result)} error=${JSON.stringify(res.error)}`)
    const tree: string[] = []
    const walk = (d: string): void => {
      for (const e of fs.readdirSync(d, { withFileTypes: true })) {
        const p = `${d}/${e.name}`
        if (e.isDirectory()) walk(p)
        else tree.push(p.replace('/tmp/r4-e2e/out-ru/', ''))
      }
    }
    if (fs.existsSync('/tmp/r4-e2e/out-ru')) walk('/tmp/r4-e2e/out-ru')
    console.log(`[r4] §P4 out-ru tree: ${JSON.stringify(tree)}`)
    const keyFile = tree.find((f) => f.includes('Keyed'))
    if (keyFile) {
      const c = fs.readFileSync(`/tmp/r4-e2e/out-ru/${keyFile}`, 'utf8')
      console.log(`[r4] §P4 keyed file size=${c.length} hasJa=${c.includes('Mod設定')} sample=${JSON.stringify(c.slice(0, 200))}`)
    }
  })
})
