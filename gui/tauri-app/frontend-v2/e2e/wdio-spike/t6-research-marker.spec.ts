// T6-2b: reopen the T6 project, set a DefInjected research marker, rebuild.
const OUT = '/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main/testlab/run/t6/mod-package'
describe('T6-2b: research DefInjected marker', () => {
  it('opens project, marks research, rebuilds', async () => {
    const homeH1 = await browser.$('h1=RimLoc — переводы модов RimWorld')
    await homeH1.waitForExist({ timeout: 20_000 })
    // открыть проект из recents (первая карточка Открыть)
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
    const rowIds = await browser.execute(() =>
      Array.from(document.querySelectorAll<HTMLElement>('[data-testid^="workspace.row."]')).map(
        (el) => el.getAttribute('data-testid')!.replace('workspace.row.', ''),
      ),
    )
    console.log('[t6b] rows:', rowIds.length)
    const research = rowIds.filter((r) => /research/i.test(r))
    console.log('[t6b] research rows:', research.slice(0, 5))
    const target = research[0] ?? rowIds.find((r) => /def_injected/i.test(r))
    if (!target) throw new Error('no def_injected rows at all')
    // edit с ретраями
    for (let attempt = 1; attempt <= 4; attempt++) {
      await browser.execute((tid) => {
        const row = document.querySelector(`[data-testid="workspace.row.${CSS.escape(tid)}"]`)
        row?.scrollIntoView({ block: 'center' })
        const cell = row?.querySelector('.cell.target') as HTMLElement | null
        cell?.click()
      }, target)
      await browser.pause(400)
      const editor = await browser.$(`[data-testid="workspace.editor.${target}"]`)
      if (await editor.isExisting()) {
        await editor.setValue('RIMLOC-T6 MARKER definjected research')
        await browser.keys('Enter')
        await browser.pause(800)
        console.log('[t6b] marker set on', target)
        break
      }
      await browser.pause(1200)
    }
    // build-mod
    await browser.execute(() => { location.hash = '#/build' })
    await browser.pause(900)
    const outField = await browser.$('[data-testid="contractops.buildmod.outdir"]')
    await outField.setValue(OUT)
    await browser.pause(200)
    await browser.$('[data-testid="contractops.buildmod.run"]').click()
    const outcome = await browser.waitUntil(async () => {
      const ok = await browser.$('[data-testid="contractops.buildmod.result"]')
      if (await ok.isExisting()) return 'ok'
      const err = await browser.$('[data-testid="contractops.buildmod.error"]')
      if (await err.isExisting()) return 'error: ' + (await err.getText())
      return false
    }, { timeout: 60_000, interval: 500 })
    console.log('[t6b] build outcome:', outcome)
  })
})
