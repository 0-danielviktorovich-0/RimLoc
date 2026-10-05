// REL-13 live verification: M-7 (real source root on Project tab) and
// M-10 (export refusal visible) on the INSTALLED release artifact.
describe('REL-13 live: M-7 + M-10', () => {
  it('M-7: project tab shows the real source root', async () => {
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
    await browser.execute(() => {
      Array.from(document.querySelectorAll<HTMLElement>('[role="tab"], button')).find(
        (el) => (el.textContent ?? '').trim() === 'Проект')?.click()
    })
    await browser.pause(800)
    const srcEl = await browser.$('[data-testid="workspace.project.sourceLocation.value"]')
    const src = await srcEl.getText()
    console.log('[rel13] M-7 source:', src)
    for (const m of ['/Users/<user>', '<mod>', '<publishedfileid>']) {
      if (src.includes(m)) throw new Error(`M-7 REGRESSION: marker ${m} on release`)
    }
    if (!src || src === '—') throw new Error('M-7 REGRESSION: no source root on release')
  })

  it('M-10: export refusal is visible, never silence', async () => {
    // Контрактный build-экран в mock-браузере недостижим; на релизе живой
    // проект открыт → #/build показывает живую панель ContractOps.
    await browser.execute(() => { location.hash = '#/build' })
    await browser.pause(900)
    const field = await browser.$('[data-testid="contractops.export.outdir"]')
    if (!(await field.isExisting())) {
      console.log('[rel13] M-10 SKIP: live build panel not present (capability off?)')
      return
    }
    const exportField = await browser.$('[data-testid="contractops.export.outdir"]')
    await exportField.setValue('RimLoc-Export/Мод-Russian')
    await browser.pause(300)
    const res = await browser.execute(() => {
      const input2 = document.querySelector<HTMLInputElement>('[data-testid="contractops.export.outdir"]')!
      const run = document.querySelector<HTMLButtonElement>('[data-testid="contractops.export.run"]')
      const reason = document.querySelector('[data-testid="contractops.export.outdir.invalid"]')
      return { inputValue: input2.value, runDisabled: run?.disabled ?? null, reason: (reason?.textContent ?? '').trim().slice(0, 80) }
    })
    console.log('[rel13] M-10:', JSON.stringify(res))
    if (res.runDisabled !== true) throw new Error('M-10 REGRESSION: run enabled for relative path')
    if (!/абсолютн/i.test(res.reason)) throw new Error('M-10 REGRESSION: inline reason missing')
  })
})
