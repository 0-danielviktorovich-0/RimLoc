// M-7 live regression (2026-09-30): the Project tab of a REAL contract
// project must show the session's real source root — the exact journey
// that exposed the template placeholders on REL-12 (finding M-7).
// Runs on the automation binary via the embedded WDIO channel, fully in
// background (window off-screen, focus restored by the spawn wrapper).

describe('M-7: live Project tab locations', () => {
  it('shows the real source root, never a template', async () => {
    const homeH1 = await browser.$('h1=RimLoc — переводы модов RimWorld')
    await homeH1.waitForExist({ timeout: 20_000 })

    // Open the first recent (selfloc live) project.
    let clicked = false
    for (const sel of ['button[aria-label="Открыть"]', 'button=Открыть']) {
      const b = await browser.$(sel)
      if (await b.isExisting()) {
        await b.click()
        clicked = true
        break
      }
    }
    if (!clicked) {
      await browser.execute(() => {
        const b = Array.from(document.querySelectorAll('button')).find(
          (x) => (x.textContent ?? '').trim() === 'Открыть',
        )
        b?.click()
      })
    }
    const editorH1 = await browser.$('h1=Редактор перевода')
    await editorH1.waitForExist({ timeout: 30_000 })

    // Tab «Проект» (BUTTON role=tab with text).
    await browser.execute(() => {
      const tab = Array.from(document.querySelectorAll<HTMLElement>('[role="tab"], button')).find(
        (el) => (el.textContent ?? '').trim() === 'Проект',
      )
      tab?.click()
    })
    await browser.pause(800)

    const locs = await browser.execute(() => {
      const src = document.querySelector<HTMLElement>(
        '[data-testid="workspace.project.sourceLocation.value"]',
      )
      const out = document.querySelector<HTMLElement>(
        '[data-testid="workspace.project.outputLocation.value"]',
      )
      return {
        source: (src?.textContent ?? '').trim(),
        output: (out?.textContent ?? '').trim(),
      }
    })
    console.log('[m7] locations:', JSON.stringify(locs))

    const markers = ['/Users/<user>', '<mod>', '<publishedfileid>']
    for (const m of markers) {
      if (locs.source.includes(m) || locs.output.includes(m)) {
        throw new Error(`template marker "${m}" leaked into the live Project tab: ${JSON.stringify(locs)}`)
      }
    }
    // Живой проект: реальный путь присутствует (не «—» — конверт H5 пишет
    // source_root с create).
    if (!locs.source || locs.source === '—') {
      throw new Error(`live project must expose its real source root, got: ${JSON.stringify(locs)}`)
    }
    // Вывод выбирается при экспорте — честный «—».
    if (locs.output !== '—') {
      console.log('[m7] WARN: output location expected «—», got:', locs.output)
    }
    console.log('[m7] PASS: real source root on the live Project tab')
  })
})
