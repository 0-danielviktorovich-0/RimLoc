// perf-v2 (mandate §7): React vs Svelte on IDENTICAL operations.
// HARNESS = driver command wall time. APP_INTERNAL = in-page split:
// hash-route vs landmark render (nav ops), MutationObserver first-DOM-change
// (select/keystroke/search). Raw samples printed per op — the run is killed
// hard at 170s, so EVERY op prints immediately (partial evidence survives).
// n>=10 per op; cold open is n=1 per run by nature (noted honestly).
const FRONT = process.env.PERF_FRONT === 'react' ? 'react' : 'svelte'
const N = Number(process.env.PERF_N ?? '10')
const MOD = '/tmp/rimloc-perf-mod'
const STRESS = '/tmp/rimloc-stress-mod'

interface Op { harness: number[]; route?: number[]; render?: number[]; internal?: number[] }
const R: Record<string, Op> = {}
const op = (name: string): Op => (R[name] = R[name] ?? { harness: [], route: [], render: [], internal: [] })
const dump = (name: string) => {
  const o = R[name]
  const clean = (a?: number[]) => (a && a.length ? a : undefined)
  console.log(`[perf-v2] OP ${name} ${JSON.stringify({ n: o.harness.length, harness: o.harness, route: clean(o.route), render: clean(o.render), internal: clean(o.internal) })}`)
}

// --- front selector table (verified against component sources, see doc) ---
const S = FRONT === 'react'
  ? {
      homeWait: () => browser.$('.app-sidebar'),
      homeRoute: '#/home',
      toSettings: 'a[href="#/settings"]',
      settingsLandmark: () => browser.$('main h1, main h2, [data-testid="ws.root"]'),
      toHome: 'a[href="#/home"]',
      toThird: 'a[href="#/tm"]',
      thirdRoute: '#/tm',
      openProject: async () => {
        await (await browser.$('[data-testid="wizard.open"]')).click()
        await (await browser.$('[data-testid="wizard.path-input"]')).setValue(MOD)
        await (await browser.$('[data-testid="wizard.next"]')).click()
        await (await browser.$('[data-testid="wizard.next"]')).click()
        await (await browser.$('[data-testid="wizard.next"]')).click()
      },
      stressOpen: async () => {
        await (await browser.$('[data-testid="wizard.open"]')).click()
        await (await browser.$('[data-testid="wizard.path-input"]')).setValue(STRESS)
        await (await browser.$('[data-testid="wizard.next"]')).click()
        await (await browser.$('[data-testid="wizard.next"]')).click()
        await (await browser.$('[data-testid="wizard.next"]')).click()
      },
      projectCard: '[data-testid="home.project-card"]',
      row: '[data-testid^="ws.entry."]',
      rowByIdx: (i: number) => `[data-testid="ws.entry.${i}"]`,
      editor: '[data-testid="ws.editor-textarea"]',
      search: '.search-field input',
    }
  : {
      homeWait: () => browser.$('h1=RimLoc — переводы модов RimWorld'),
      homeRoute: '#/home',
      toSettings: 'button[aria-label="Настройки"]',
      settingsLandmark: () => browser.$('h1=Настройки'),
      toHome: 'button[aria-label="На главную"]',
      toThird: 'a[href="#/tm"], [data-testid="nav.tm"]',
      thirdRoute: '#/tm',
      openProject: async () => {
        await (await browser.$('input[data-testid="home.contract.path"]')).setValue(MOD)
        await (await browser.$('[data-testid="home.contract.create"]')).click()
      },
      stressOpen: async () => {
        await (await browser.$('input[data-testid="home.contract.path"]')).setValue(STRESS)
        await (await browser.$('[data-testid="home.contract.create"]')).click()
      },
      projectCard: '[data-testid^="home.contract.open."]',
      row: '[data-testid^="workspace.row."]',
      rowByIdx: (i: number) => `[data-testid^="workspace.row."]:nth-of-type(${i + 1})`,
      editor: '[data-testid^="workspace.editor."]',
      search: '[data-testid="workspace.search"]',
    }

async function waitExisting(sel: () => any, timeout = 8000) {
  await browser.waitUntil(async () => (await sel()).isExisting(), { timeout, interval: 50 })
}

/** Semantic nav: click → hash (route) → landmark (render). Same method as
 *  soak v3 so Svelte numbers are directly comparable with soak history. */
async function navMeasure(to: string, route: string, landmark: () => any, name: string): Promise<void> {
  const o = op(name)
  const t0 = Date.now()
  const btn = await browser.$(to)
  if (!(await btn.isExisting())) { console.log(`[perf-v2] NAV_ABSENT ${name} ${to}`); return }
  await btn.click()
  const routeOk = await browser.waitUntil(async () => {
    const h = await browser.execute(() => window.location.hash)
    return typeof h === 'string' && h.startsWith(route)
  }, { timeout: 5000, interval: 30 }).then(() => true).catch(() => false)
  const t1 = Date.now()
  if (!routeOk) { console.log(`[perf-v2] ROUTE_TIMEOUT ${name}`); return }
  const renderOk = await waitExisting(landmark, 5000).then(() => true).catch(() => false)
  const t2 = Date.now()
  o.harness.push(t2 - t0); o.route!.push(t1 - t0); o.render!.push(t2 - t1)
  if (!renderOk) console.log(`[perf-v2] RENDER_TIMEOUT ${name}`)
}

/** One measured op with an in-page MutationObserver split. */
async function pageOp(name: string, armInPage: () => Promise<void>, act: () => Promise<void>, done: () => Promise<boolean>, reset?: () => Promise<void>) {
  const o = op(name)
  for (let i = 0; i < N; i++) {
    if (reset) {
      try { await reset() } catch (e) { console.log(`[perf-v2] RESET_FAIL ${name} ${String(e).slice(0, 120)}`) }
    }
    await armInPage()
    const t0 = Date.now()
    const tPageBefore = await browser.execute(() => performance.now())
    try {
      await act()
    } catch (e) {
      console.log(`[perf-v2] ACT_FAIL ${name} iter=${i} ${String(e).slice(0, 160)}`)
      break
    }
    const ok = await browser.waitUntil(done, { timeout: 5000, interval: 30 }).then(() => true).catch(() => false)
    const t1 = Date.now()
    if (!ok) { console.log(`[perf-v2] TIMEOUT ${name} iter=${i}`); break }
    const firstChange = await browser.execute((b: number) => {
      const evts = (window as any).__pv2 as number[] | undefined
      const after = (evts ?? []).filter((t) => t >= b)
      return after.length ? after[0] - b : -1
    }, tPageBefore)
    o.harness.push(t1 - t0)
    if (firstChange >= 0) o.internal!.push(firstChange)
  }
  dump(name)
}

const arm = () => browser.execute(() => {
  (window as any).__pv2 = []
  const obs = new MutationObserver(() => { (window as any).__pv2.push(performance.now()) })
  obs.observe(document.body, { childList: true, subtree: true, characterData: true })
})

describe(`perf-v2 ${FRONT}`, () => {
  it('cold open + nav + list + select + keystroke + search', async () => {
    // --- cold open (n=1 per run): spec start → first home landmark ---
    const coldT0 = Date.now()
    await waitExisting(S.homeWait, 30000)
    const cold = Date.now() - coldT0
    R['cold_open'] = { harness: [cold] }
    // external cold open: bash passed PERF_T0 (epoch ms of `npx wdio` launch)
    const extT0 = Number(process.env.PERF_T0 ?? '0')
    const ext = extT0 > 0 ? Date.now() - extT0 : -1
    console.log(`[perf-v2] COLD_OPEN ${JSON.stringify({ front: FRONT, harnessMs: cold, externalWallMs: ext, note: 'externalWall = npx wdio старт → лендмарк найден (включает node-бут, spawn приложения, драйвер, сессию); harnessMs = хвост после готовности сессии' })}`)

    // --- window perf-mark probe (React T0-T4 availability) ---
    if (FRONT === 'react') {
      const probe = await browser.execute(() => ({
        keys: Object.getOwnPropertyNames(window).filter((k) => /perf|rimloc|__tauri|wdio/i.test(k)),
      }))
      console.log(`[perf-v2] WINDOW_PROBE ${JSON.stringify(probe)}`)
    }

    // --- setup (NOT measured): open the shared 30-entry project once ---
    try {
      await S.openProject()
      await waitExisting(() => browser.$(S.row), 60000)
      console.log('[perf-v2] SETUP_OK project open, rows visible')
    } catch (e) {
      console.log(`[perf-v2] SETUP_FAIL ${String(e).slice(0, 300)}`)
      throw e
    }
    // back home so list_open is measurable through the same affordance
    await navMeasure(S.toHome, S.homeRoute, S.homeWait, 'setup_home')

    // --- nav circuit: settings → home → third, N rounds ---
    for (let i = 0; i < N; i++) {
      await navMeasure(S.toSettings, '#/settings', S.settingsLandmark, 'nav_settings')
      await navMeasure(S.toHome, S.homeRoute, S.homeWait, 'nav_home')
    }
    dump('nav_settings'); dump('nav_home')
    await navMeasure(S.toThird, S.thirdRoute, S.settingsLandmark, 'nav_third')
    dump('nav_third')
    await navMeasure(S.toHome, S.homeRoute, S.homeWait, 'setup_home2')

    // --- list_open: project card → first row (n=N) ---
    const backHome = async () => {
      const h = await browser.execute(() => window.location.hash)
      if (!String(h).startsWith('#/home')) {
        const b = await browser.$(S.toHome)
        if (await b.isExisting()) { await b.click(); await waitExisting(S.homeWait, 5000) }
      }
    }
    await pageOp('list_open',
      arm,
      async () => { await (await browser.$(S.projectCard)).click() },
      async () => (await browser.$(S.row)).isExisting(),
      backHome,
    )

    // --- select_row: row click → UI reaction (n=N) ---
    // React: single click opens the editor pane (selection == editor exists).
    // Svelte: single click selects (row.class 'selected'); the editor opens on
    // dblclick — per-front semantics recorded as-is in the doc.
    const selectDone = FRONT === 'react'
      ? async () => (await browser.$(S.editor)).isExisting()
      : async () => ((await (await browser.$(S.row)).getAttribute('class')) ?? '').includes('selected')
    await pageOp('select_row',
      arm,
      async () => { await (await browser.$(S.row)).click() },
      selectDone,
      async () => { /* re-click re-triggers */ },
    )

    // --- keystroke: addValue('x') → editor value non-empty (n=N) ---
    const edSel = S.editor
    const openEditor = FRONT === 'react'
      ? async () => {
          const ed = await browser.$(edSel)
          if (await ed.isExisting()) await ed.setValue('')
        }
      : async () => {
          const ed = await browser.$(edSel)
          if (await ed.isExisting()) await ed.setValue('')
          await (await browser.$(S.row)).doubleClick()
          await waitExisting(() => browser.$(edSel), 5000)
        }
    await pageOp('keystroke',
      arm,
      async () => { await (await browser.$(edSel)).addValue('x') },
      async () => ((await (await browser.$(edSel)).getValue()) as string ?? '').length > 0,
      openEditor as () => Promise<void>,
    )

    // --- search: query → narrowed row count (n=N) ---
    await pageOp('search',
      arm,
      async () => { await (await browser.$(S.search)).setValue('R1_Perf_29') },
      async () => (await browser.$$(S.row.replace('^=', '^='))).length <= 2,
      async () => { await (await browser.$(S.search)).setValue('') },
    )
  })

  it('react-stress decomposition: 10k open + selection split (PERF_DECOMP=1)', async function () {
    if (process.env.PERF_DECOMP !== '1' || FRONT !== 'react') { this.skip(); return }
    // decom starts from wherever it 1 ended (workspace) → home first
    const h = await browser.execute(() => window.location.hash)
    if (!String(h).startsWith('#/home')) {
      await (await browser.$('a[href="#/home"]')).click()
      await waitExisting(S.homeWait, 8000)
    }
    const dT0 = Date.now()
    await S.stressOpen()
    await browser.waitUntil(async () => (await browser.$$(S.row)).length >= 1, { timeout: 60000, interval: 250 })
    const createToPaint = Date.now() - dT0
    console.log(`[perf-v2] DECOMP create→first-row-paint(10k) ${createToPaint}ms`)
    const footer = await browser.$('.list-footer').getText().catch(() => 'n/a')
    console.log(`[perf-v2] DECOMP footer ${footer.replace(/\n/g, ' | ')}`)
    // selection ×5 with in-page split (222ms context: same shape as stress spec)
    const o = op('decomp_select_10k')
    for (let i = 0; i < 5; i++) {
      await arm()
      const tPage = await browser.execute(() => performance.now())
      const t0 = Date.now()
      await (await browser.$('[data-testid="ws.entry.0"]')).click()
      const ok = await browser.waitUntil(async () => (await browser.$('[data-testid="ws.editor-textarea"]')).isExisting(), { timeout: 5000, interval: 30 }).then(() => true).catch(() => false)
      const t1 = Date.now()
      if (!ok) { console.log(`[perf-v2] TIMEOUT decomp_select iter=${i}`); break }
      const inPage = await browser.execute((b: number) => {
        const evts = (window as any).__pv2 as number[] | undefined
        const after = (evts ?? []).filter((t) => t >= b)
        return after.length ? after[0] - b : -1
      }, tPage)
      o.harness.push(t1 - t0)
      if (inPage >= 0) o.internal!.push(inPage)
    }
    dump('decomp_select_10k')
  })
})
