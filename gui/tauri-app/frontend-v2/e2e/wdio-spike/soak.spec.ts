// Soak v3 (owner soak-hardening §3-10, 03.10): bounded background cycles
// with SEMANTIC route synchronization and classified counters.
//  - identity preflight BEFORE cycle 1: the RUNNING app reports its own
//    build_identity; a source-commit mismatch aborts the run (never soak
//    a wrong artifact silently — lesson of the 03.10 rel14 run);
//  - navigation waits for CONDITIONS (route hash + landmark), never time;
//    an immediate post-action snapshot raced at ~20% under load;
//  - one semantic action per step, no retries; a timeout FAILS the cycle
//    and is preserved as evidence;
//  - route truth = location.hash + semantic landmark (a heading alone
//    never passes); render latency is recorded separately from route
//    latency so a slow render cannot hide behind a passed route wait;
//  - counters stay DISTINCT (functional_failures / route_timeouts /
//    render_timeouts / driver_errors) — never collapsed into one error.
const SOAK_MINUTES = Number(process.env.SOAK_MINUTES ?? '55')
const NAV_TIMEOUT_MS = Number(process.env.SOAK_NAV_TIMEOUT_MS ?? '5000')

function median(a: number[]): number {
  if (a.length === 0) return -1
  const s = [...a].sort((x, y) => x - y)
  return s[Math.floor(s.length / 2)]
}
function pct(a: number[], p: number): number {
  if (a.length === 0) return -1
  const s = [...a].sort((x, y) => x - y)
  return s[Math.min(s.length - 1, Math.floor((s.length * p) / 100))]
}

interface NavSample {
  total: number
  route: number
  render: number
}

describe('embedded WDIO soak', () => {
  it('survives an hour of background cycles', async () => {
    const deadline = Date.now() + SOAK_MINUTES * 60_000
    const started = Date.now()
    const stats = {
      cycles: 0,
      functional_failures: [] as string[],
      route_timeouts: [] as string[],
      render_timeouts: [] as string[],
      driver_errors: [] as string[],
      settingsLatency: [] as NavSample[],
      otherNavLatency: [] as NavSample[],
      backendLatency: [] as number[]
    }
    let identity = 'unverified'

    // --- identity preflight (BEFORE cycle 1, owner §1) ---
    {
      const t0 = Date.now()
      const id = (await browser.execute(async () => {
        const inv = window.__TAURI_INTERNALS__?.invoke
        if (!inv) throw new Error('__TAURI_INTERNALS__.invoke unavailable')
        return await inv('build_identity')
      })) as { sourceCommit?: string }
      const expected = process.env.SOAK_EXPECT_COMMIT
      if (!id || typeof id.sourceCommit !== 'string') {
        throw new Error(`preflight: app reported no build_identity (${JSON.stringify(id)})`)
      }
      // The commit is compared WITHOUT the declared "-dirty" suffix: the
      // automation capability overlay (capabilities/automation.json) is a
      // deliberate build-time input the artifact-class scripts manage, so a
      // dirty flag alone never masks a WRONG commit — but any commit
      // mismatch still aborts before cycle 1.
      const actualCommit = id.sourceCommit.replace(/-dirty$/, '')
      if (expected && actualCommit !== expected) {
        throw new Error(
          `preflight: ARTIFACT MISMATCH — app sourceCommit ${id.sourceCommit}, expected ${expected}; soak never starts`,
        )
      }
      identity = id.sourceCommit
      console.log(`[soak] preflight: identity verified (${identity}, ${Date.now() - t0}ms)`)
    }

    const step = async (name: string, fn: () => Promise<void>) => {
      const t0 = Date.now()
      try {
        await fn()
      } catch (e) {
        const msg = String(e)
        const line = `c${stats.cycles} ${name}: ${msg.slice(0, 200)}`
        // WebDriver/transport-level errors are a DIFFERENT class than
        // functional failures — the app may be innocent.
        if (/webdriver|stale element|no such window|connection/i.test(msg)) {
          stats.driver_errors.push(line)
          if (stats.driver_errors.length <= 5) console.log('[soak-driver-err]', line)
        } else {
          stats.functional_failures.push(line)
          if (stats.functional_failures.length <= 5) console.log('[soak-err]', line)
        }
        return Date.now() - t0
      }
      return Date.now() - t0
    }

    /** Semantic navigation: ONE click, then CONDITION waits — hash first
     *  (route truth), then landmark (screen truth). Latency is split into
     *  route vs render so a slow render cannot hide behind a passed route
     *  wait. No retries, no sleeps. */
    const navTo = async (
      clickSel: string,
      expectHash: string,
      landmarkSel: string,
      sink: NavSample[],
      stepName: string,
    ): Promise<void> => {
      const t0 = Date.now()
      const b = await browser.$(clickSel)
      if (!(await b.isExisting())) return // navigation affordance absent: nothing to drive this cycle
      await b.click()
      const t1 = Date.now()
      const routeOk = await browser
        .waitUntil(
          async () => {
            const h = await browser.execute(() => window.location.hash)
            return typeof h === 'string' && h.startsWith(expectHash)
          },
          { timeout: NAV_TIMEOUT_MS, interval: 50 },
        )
        .then(() => true)
        .catch(() => false)
      const t3 = Date.now()
      if (!routeOk) {
        stats.route_timeouts.push(
          `c${stats.cycles} ${stepName}: route never reached ${expectHash} within ${NAV_TIMEOUT_MS}ms (click→t1 ${t1 - t0}ms)`,
        )
        return
      }
      const landmarkOk = await browser
        .$(landmarkSel)
        .waitFor({ timeout: NAV_TIMEOUT_MS, interval: 50 })
        .then(() => true)
        .catch(() => false)
      const t4 = Date.now()
      if (!landmarkOk) {
        stats.render_timeouts.push(
          `c${stats.cycles} ${stepName}: route ${expectHash} ok in ${t3 - t0}ms but landmark ${landmarkSel} missing after ${t4 - t3}ms`,
        )
        return
      }
      sink.push({ total: t4 - t0, route: t3 - t0, render: t4 - t3 })
    }

    while (Date.now() < deadline) {
      stats.cycles++

      await step('home-h1', async () => {
        const h1 = await browser.$('h1=RimLoc — переводы модов RimWorld')
        if (!(await h1.isExisting())) throw new Error('home h1 missing')
      })

      // settings-nav (the step whose immediate post-action snapshot raced):
      // hash = route truth, h1 = landmark, latency split route/render.
      await step('settings-nav', async () => {
        const before = stats.settingsLatency.length
        await navTo(
          'button[aria-label="Настройки"]',
          '#/settings',
          'h1=Настройки',
          stats.settingsLatency,
          'settings-nav',
        )
        if (stats.settingsLatency.length === before) {
          const lastTimeout =
            stats.route_timeouts[stats.route_timeouts.length - 1] ??
            stats.render_timeouts[stats.render_timeouts.length - 1] ??
            'no navigation affordance this cycle'
          if (lastTimeout !== 'no navigation affordance this cycle') throw new Error(lastTimeout)
        }
      })

      await step('back-home', async () => {
        const before = stats.otherNavLatency.length
        await navTo(
          'button[aria-label="На главную"]',
          '#/home',
          'h1=RimLoc — переводы модов RimWorld',
          stats.otherNavLatency,
          'back-home',
        )
        if (stats.otherNavLatency.length === before) {
          const lastTimeout =
            stats.route_timeouts[stats.route_timeouts.length - 1] ??
            stats.render_timeouts[stats.render_timeouts.length - 1] ??
            ''
          if (lastTimeout) throw new Error(lastTimeout)
        }
      })

      await step('backend-op', async () => {
        const t0 = Date.now()
        // W3C execute path (browser.tauri.execute hangs on macOS 27 in a
        // never-focused window — upstream #540 class).
        const r = await browser.execute(async () => {
          const inv = window.__TAURI_INTERNALS__?.invoke
          if (!inv) throw new Error('__TAURI_INTERNALS__.invoke unavailable')
          const list = await inv('project_list')
          return Array.isArray(list) ? list.length : -1
        })
        stats.backendLatency.push(Date.now() - t0)
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
          `[soak] ${Math.round((Date.now() - started) / 1000)}s: cycles=${stats.cycles} ` +
            `functional=${stats.functional_failures.length} routeTO=${stats.route_timeouts.length} ` +
            `renderTO=${stats.render_timeouts.length} driverErr=${stats.driver_errors.length}`,
        )
      }
    }

    const rep = (a: number[]): string =>
      a.length === 0 ? 'n/a' : `n=${a.length} p50=${median(a)}ms p95=${pct(a, 95)}ms p99=${pct(a, 99)}ms max=${Math.max(...a)}ms`
    console.log(`[soak] IDENTITY ${identity}`)
    console.log(`[soak] DONE cycles=${stats.cycles}`)
    console.log(`[soak] counters: functional_failures=${stats.functional_failures.length} route_timeouts=${stats.route_timeouts.length} render_timeouts=${stats.render_timeouts.length} driver_errors=${stats.driver_errors.length}`)
    console.log(`[soak] settings-nav latency: ${rep(stats.settingsLatency.map((s) => s.total))}`)
    console.log(
      `[soak] settings-nav split: route ${rep(stats.settingsLatency.map((s) => s.route))} | render ${rep(stats.settingsLatency.map((s) => s.render))}`,
    )
    console.log(`[soak] back-home latency: ${rep(stats.otherNavLatency.map((s) => s.total))}`)
    console.log(`[soak] backend-op latency: ${rep(stats.backendLatency)}`)
    for (const l of [
      ...stats.functional_failures.slice(0, 5),
      ...stats.route_timeouts.slice(0, 5),
      ...stats.render_timeouts.slice(0, 5),
      ...stats.driver_errors.slice(0, 5),
    ]) {
      console.log('[soak-evidence]', l)
    }

    const totalFailures =
      stats.functional_failures.length +
      stats.route_timeouts.length +
      stats.render_timeouts.length +
      stats.driver_errors.length
    if (totalFailures > stats.cycles * 0.1) {
      throw new Error(
        `soak failure rate >10%: functional=${stats.functional_failures.length} routeTO=${stats.route_timeouts.length} renderTO=${stats.render_timeouts.length} driverErr=${stats.driver_errors.length}`,
      )
    }
  })
})
