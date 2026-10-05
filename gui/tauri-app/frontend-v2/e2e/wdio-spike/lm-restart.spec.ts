// LM persistence across RESTART: этот спек исполняется в НОВОЙ сессии WDIO —
// приложение перезапущено (новый процесс), localStorage WKWebView пережил
// рестарт. Проверяем qa-persist из lm-acceptance.spec.ts и удаляем его.
function pause(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

describe('LM §10: персистентность пользовательского языка через рестарт', () => {
  it('qa-persist пережил перезапуск приложения; после проверки удаляем (cleanup)', async () => {
    await browser.waitUntil(
      async () => (await browser.$('.app-sidebar')).isExisting(),
      { timeout: 30000, interval: 100 },
    )
    await browser.execute(() => { window.location.hash = '#/lm' })
    await browser.waitUntil(
      async () => (await browser.$('[data-testid="lm.table"]')).isExisting(),
      { timeout: 15000, interval: 100 },
    )
    await pause(300)
    const present = await browser.$('[data-testid="lm.row.qa-persist"]').isExisting()
    const stored = await browser.execute(() => localStorage.getItem('rimloc.languages.user.v1') ?? '<absent>')
    console.log(`[lm-restart] QA_PERSIST_PRESENT=${present} STORED=${stored}`)
    if (!present) throw new Error('qa-persist НЕ пережил рестарт приложения')
    // cleanup: удаляем свой язык, не оставляя мусора в профиле webview
    await browser.execute(() => {
      const row = document.querySelector('[data-testid="lm.row.qa-persist"]')
      const btn = row?.querySelector('button') as HTMLElement | null
      btn?.click()
    })
    await pause(500)
    const gone = !(await browser.$('[data-testid="lm.row.qa-persist"]').isExisting())
    console.log(`[lm-restart] CLEANUP_DELETED=${gone}`)
    if (!gone) throw new Error('cleanup не удалил qa-persist')
  })
})
