// Soak (frontier directive §11): 55 minutes of repeated semantic cycles —
// idle periods, navigation, project open, tab switch, scroll, backend ops,
// repeated queries. Doctrine §6: element references are REACQUIRED every
// cycle (no cached handles); failures are recorded, not fatal. Latency per
// step feeds the soak report. Non-interference is asserted by the outer
// monitor (frontmost/pointer/clipboard), as in the spike.
const SOAK_MINUTES = Number(process.env.SOAK_MINUTES ?? '55')

describe('embedded WDIO soak', () => {
  it('survives an hour of background cycles', async () => {
    const deadline = Date.now() + SOAK_MINUTES * 60_000
    const started = Date.now()
    const stats = { cycles: 0, errors: [] as string[], latencies: [] as number[] }
    const step = async (name: string, fn: () => Promise<void>) => {
      const t0 = Date.now()
      try {
        await fn()
        stats.latencies.push(Date.now() - t0)
      } catch (e) {
        const line = `c${stats.cycles} ${name}: ${String(e).slice(0, 200)}`
        // первая ошибка каждого шага — сразу в лог (атрибуция без вскрытия)
        if (!stats.errors.some((x) => x.includes(name))) console.log('[soak-err]', line)
        stats.errors.push(line)
      }
    }

    while (Date.now() < deadline) {
      stats.cycles++
      // --- reacquired every cycle (§6) ---
      await step('home-h1', async () => {
        const h1 = await browser.$('h1=RimLoc — переводы модов RimWorld')
        if (!(await h1.isExisting())) throw new Error('home h1 missing')
      })
      await step('settings-nav', async () => {
        const b = await browser.$('button[aria-label="Настройки"]')
        if (await b.isExisting()) {
          await b.click()
          const h = await browser.$('h1=Настройки')
          if (!(await h.isExisting())) throw new Error('settings h1 missing')
        }
      })
      await step('back-home', async () => {
        const b = await browser.$('button[aria-label="На главную"]')
        if (await b.isExisting()) await b.click()
      })
      await step('backend-op', async () => {
        // browser.tauri.execute hangs on this artifact: the plugin's
        // /wdio/eval uses callAsyncJavaScript, which macOS 27 WebKit
        // defers in a never-focused window (upstream issue #540 class).
        // The W3C execute path works in the same window — drive the live
        // backend through __TAURI_INTERNALS__ instead.
        const r = await browser.execute(async () => {
          const inv = window.__TAURI_INTERNALS__?.invoke
          if (!inv) throw new Error('__TAURI_INTERNALS__.invoke unavailable')
          const list = await inv('project_list')
          return Array.isArray(list) ? list.length : -1
        })
        if (r !== 8) throw new Error(`project_list count ${r} != 8`)
      })
      if (stats.cycles % 5 === 0) {
        await step('scroll-probe', async () => {
          await browser.execute(() => {
            const el = Array.from(document.querySelectorAll<HTMLElement>('*')).find(
              (e) => e.scrollHeight > e.clientHeight + 40,
            )
            if (el) el.scrollTop = 150
          })
        })
        // idle stretch between measurement bursts
        await new Promise((r) => setTimeout(r, 20_000))
      }
      if (stats.cycles % 10 === 0) {
        console.log(
          `[soak] ${Math.round((Date.now() - started) / 1000)}s: cycles=${stats.cycles} errors=${stats.errors.length} p50=${median(stats.latencies)}ms`,
        )
      }
    }
    console.log(
      `[soak] DONE cycles=${stats.cycles} errors=${stats.errors.length}/${stats.cycles} p50=${median(stats.latencies)}ms p95=${pct(stats.latencies, 95)}ms`,
    )
    if (stats.errors.length > stats.cycles * 0.1) {
      throw new Error(`soak error rate >10%: ${stats.errors.slice(0, 5).join(' | ')}`)
    }
  })
})

function median(xs: number[]): number {
  if (!xs.length) return -1
  return pct(xs, 50)
}
function pct(xs: number[], p: number): number {
  if (!xs.length) return -1
  const s = [...xs].sort((a, b) => a - b)
  return s[Math.min(s.length - 1, Math.floor((s.length * p) / 100))]
}
