// T6 phase 1 (background-only): REAL export of the live project through
// the installed release artifact into an ISOLATED /tmp output, then the
// output is structurally validated (separate step) against RimWorld
// language-folder conventions. No game launch, no foreground, no source
// writes — the source mod stays read-only.
describe('T6 phase 1: real export to isolated output', () => {
  it('exports the live project translation', async () => {
    const homeH1 = await browser.$('h1=RimLoc — переводы модов RimWorld')
    await homeH1.waitForExist({ timeout: 20_000 })
    let clicked = false
    for (const sel of ['button[aria-label="Открыть"]', 'button=Открыть']) {
      const b = await browser.$(sel)
      if (await b.isExisting()) { await b.click(); clicked = true; break }
    }
    if (!clicked) {
      await browser.execute(() => {
        Array.from(document.querySelectorAll('button')).find(
          (x) => (x.textContent ?? '').trim() === 'Открыть')?.click()
      })
    }
    await browser.$('h1=Редактор перевода').waitForExist({ timeout: 30_000 })
    await browser.execute(() => { location.hash = '#/build' })
    await browser.pause(900)

    const field = await browser.$('[data-testid="contractops.export.outdir"]')
    if (!(await field.isExisting())) throw new Error('live build panel not present')
    await field.setValue('/tmp/rimloc-t6-out/Russian')
    await browser.pause(200)
    const run = await browser.$('[data-testid="contractops.export.run"]')
    if (!(await run.isEnabled())) throw new Error('run disabled for an absolute path — unexpected')
    await run.click()
    // export runs the real pipeline; wait for the honest result
    const outcome = await browser.waitUntil(async () => {
      const ok = await browser.$('[data-testid="contractops.export.result"]')
      if (await ok.isExisting()) return 'ok'
      const err = await browser.$('[data-testid="contractops.export.error"]')
      if (await err.isExisting()) return 'error: ' + (await err.getText())
      return false
    }, { timeout: 30_000, interval: 500 })
    console.log('[t6] export outcome:', outcome)
    if (!outcome.startsWith('ok')) throw new Error(outcome)
  })

  it('builds the drop-in mod package (game-loadable ModMetaData)', async () => {
    const field = await browser.$('[data-testid="contractops.buildmod.outdir"]')
    if (!(await field.isExisting())) throw new Error('build-mod card not present')
    await field.setValue('/tmp/rimloc-t6-mod')
    await browser.pause(200)
    const run = await browser.$('[data-testid="contractops.buildmod.run"]')
    if (!(await run.isEnabled())) throw new Error('build-mod run disabled for absolute path')
    await run.click()
    const outcome = await browser.waitUntil(async () => {
      const ok = await browser.$('[data-testid="contractops.buildmod.result"]')
      if (await ok.isExisting()) return 'ok'
      const err = await browser.$('[data-testid="contractops.buildmod.error"]')
      if (await err.isExisting()) return 'error: ' + (await err.getText())
      return false
    }, { timeout: 30_000, interval: 500 })
    console.log('[t6] build-mod outcome:', outcome)
    if (!outcome.startsWith('ok')) throw new Error(outcome)
  })
})
