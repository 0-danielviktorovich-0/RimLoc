// Palette acceptance (mandate §9) — REAL rel17 react artifact app, isolated
// lane (port 4469, RIMLOC_DATA_DIR=/tmp/palette-lm/data). Keyboard probes
// dispatch window KeyboardEvents (the app's contract is the window keydown
// handler in App.tsx); a page-side __keylog records what actually arrived.
// Every state assertion RECORDS its evidence (console + screenshots to
// /tmp/palette-lm/shots/) instead of failing silently.
import { execSync } from 'node:child_process'

const SHOTS = '/tmp/palette-lm/shots'

function pause(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

async function waitExisting(sel: string, timeout = 30000): Promise<void> {
  await browser.waitUntil(async () => (await browser.$(sel)).isExisting(), { timeout, interval: 100 })
}

let shotN = 0
async function shot(name: string): Promise<void> {
  shotN += 1
  const file = `${SHOTS}/${String(shotN).padStart(2, '0')}-${name}.png`
  try {
    await browser.saveScreenshot(file)
  } catch {
    try {
      execSync(`screencapture -x -R348,70,980,640 '${file}'`)
    } catch {
      console.log(`[palette-acc] screenshot unavailable: ${name}`)
    }
  }
}

/** Install a page-side keydown recorder so we can prove which events arrive. */
async function installKeylog(): Promise<void> {
  await browser.execute(() => {
    const w = window as unknown as { __keylog?: string[] }
    if (!w.__keylog) {
      w.__keylog = []
      window.addEventListener(
        'keydown',
        (e) => {
          w.__keylog!.push(`${e.key}${e.metaKey ? '+meta' : ''}`)
        },
        true,
      )
    }
  })
}

async function keylog(): Promise<string[]> {
  return browser.execute(() => (window as unknown as { __keylog?: string[] }).__keylog ?? [])
}

async function overlayCount(): Promise<number> {
  return browser.execute(() => document.querySelectorAll('.palette-overlay').length)
}

async function inputValue(): Promise<string> {
  const el = await browser.$('[data-testid="palette.input"]')
  return (await el.isExisting()) ? ((await el.getValue()) as string) : '<no-input>'
}

async function hash(): Promise<string> {
  return browser.execute(() => window.location.hash)
}

/** Synthetic window-level keyboard event (the app listens on window). */
async function pressCombo(key: string, mod: boolean): Promise<void> {
  const before = await keylog()
  await browser.execute(
    (k, m) => {
      window.dispatchEvent(
        new KeyboardEvent('keydown', { key: k, metaKey: m, ctrlKey: false, bubbles: true }),
      )
    },
    key,
    mod,
  )
  await pause(600)
  const after = await keylog()
  const arrived = after.length > before.length
  if (!arrived) console.log(`[palette-acc] WARN: ${key} не дошёл до страницы`)
  return
}

/** Deterministically closed state; recovers and REPORTS stuck overlays. */
async function ensureClosed(context: string): Promise<void> {
  for (let i = 0; i < 4 && (await overlayCount()) > 0; i++) {
    await pressCombo('Escape', false)
    await pause(800)
    if ((await overlayCount()) === 0) break
    console.log(`[palette-acc] ${context}: Escape не закрыл (попытка ${i + 1}) — пробую клик по оверлею`)
    await browser.execute(() => {
      const o = document.querySelector('.palette-overlay') as HTMLElement | null
      o?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    })
    await pause(800)
  }
  const stuck = await overlayCount()
  if (stuck > 0) {
    console.log(`[palette-acc] ${context}: ПАЛИТРА ЗАЛИПА ОТКРЫТОЙ (overlay ×${stuck})`)
    await shot(`stuck-overlay-${context}`)
    throw new Error(`палитра не закрывается ни Escape, ни кликом (${context})`)
  }
}

async function openPalette(context: string): Promise<void> {
  await ensureClosed(context)
  await pressCombo('k', true)
  const opened = await browser.waitUntil(async () => (await overlayCount()) > 0, {
    timeout: 6000,
    interval: 150,
  }).catch(() => false)
  if (!opened) {
    console.log(`[palette-acc] ${context}: Cmd+K не открыл — повтор`)
    await pressCombo('k', true)
    await pause(1000)
    if ((await overlayCount()) === 0) throw new Error(`Cmd+K не открывает палитру (${context})`)
  }
  // Обход задокументированного бага «запрос переживает закрытие» (см.
  // отдельный тест): сбрасываем фильтр, чтобы список был полным.
  const inp = await browser.$('[data-testid="palette.input"]')
  if ((await inp.getValue()) !== '') {
    console.log(`[palette-acc] ${context}: запрос пережил закрытие — сбрасываю (доказано отдельным тестом)`)
    await inp.setValue('')
    await pause(250)
  }
}

async function paletteItems(): Promise<string[]> {
  return browser.execute(() =>
    [...document.querySelectorAll('.palette-item')].map((b) => (b as HTMLElement).innerText.trim()),
  )
}

async function clickItemByLabel(label: string): Promise<boolean> {
  return browser.execute((lbl) => {
    const btn = [...document.querySelectorAll('.palette-item')].find(
      (b) => (b as HTMLElement).innerText.trim() === lbl,
    ) as HTMLElement | undefined
    if (!btn) return false
    btn.click()
    return true
  }, label)
}

/** Keyboard-active option: highlight class + aria-selected + the input's
 *  aria-activedescendant. This IS the navigation model's observable state. */
async function activeOption(): Promise<{ index: number; label: string; ad: string }> {
  return browser.execute(() => {
    const items = [...document.querySelectorAll('.palette-item')]
    const idx = items.findIndex((b) => (b as HTMLElement).classList.contains('palette-item-active'))
    const input = document.querySelector('[data-testid="palette.input"]') as HTMLInputElement | null
    return {
      index: idx,
      label: idx >= 0 ? (items[idx] as HTMLElement).innerText.trim() : '<none>',
      ad: input?.getAttribute('aria-activedescendant') ?? '<none>',
    }
  })
}

async function activeElementInfo(): Promise<string> {
  return browser.execute(() => {
    const a = document.activeElement
    if (!a) return 'null'
    return `${a.tagName.toLowerCase()}${a.className ? `.${String(a.className).split(' ')[0]}` : ''}`
  })
}

// The ARTIFACT palette (commit 53bd1aa) had 6 commands; the palette-nav lane
// brought 7; the palette/LM MUST-FIX lane brought the FULL route coverage
// (14); the wave integration dropped «Инструменты» (W0: dead nav point) —
// the composition test below expects exactly this list (13).
const COMMANDS: Array<{ label: string; hash: string; marker?: string }> = [
  { label: 'Строки перевода', hash: '#/home', marker: '[data-testid="wizard.open"]' },
  { label: 'Проекты', hash: '#/projects', marker: '[data-testid="wizard.open"]' },
  { label: 'Проверки', hash: '#/checks', marker: '[data-testid="checks.findings"], [data-testid="checks.rerun"], .narrow-page .btn-primary[href="#/home"]' },
  { label: 'Импорт существующего перевода', hash: '#/existing', marker: '[data-testid="ex.dir"], [data-testid="ex.error"], .narrow-page .btn-primary[href="#/home"]' },
  { label: 'Сравнение версий', hash: '#/compare', marker: '[data-testid="cmp.old-dir"]' }, // Compare-экран ЖИВОЙ (wave-integration): форма diff двух корней модов
  { label: 'Глоссарий', hash: '#/glossary', marker: '[data-testid="gl.table"], [data-testid="gl.error"], .narrow-page .btn-primary[href="#/home"]' },
  { label: 'Память переводов', hash: '#/tm', marker: '[data-testid="tm.count"], [data-testid="tm.filter-query"], [data-testid="tm.filter-status"], [data-testid="tm.error"]' },
  { label: 'Сборка и экспорт', hash: '#/export', marker: '[data-testid="be.outdir"], .narrow-page .btn-primary[href="#/home"]' },
  { label: 'Самоперевод RimLoc', hash: '#/selfloc', marker: '[data-testid="selfloc.open"], [data-testid="selfloc.error"], .narrow-page .btn-primary[href="#/home"]' },
  { label: 'Диагностика', hash: '#/diagnostics', marker: '[data-testid="diag.outdir"], [data-testid="diag.error"], .narrow-page .btn-primary[href="#/home"]' },
  { label: 'AI-провайдеры', hash: '#/providers', marker: '[data-testid="prov.catalog"], [data-testid="prov.error"], .narrow-page .btn-primary[href="#/home"]' },
  { label: 'Языки', hash: '#/lm', marker: '[data-testid="lm.table"]' },
  { label: 'Настройки', hash: '#/settings', marker: '[data-testid="settings.theme"]' },
]

before(async () => {
  await waitExisting('.app-sidebar', 30000)
  await installKeylog()
})

afterEach(async function () {
  const title = this.currentTest?.title ?? 'test'
  try {
    await ensureClosed(`afterEach:${title.slice(0, 24)}`)
  } catch (e) {
    console.log(`[palette-acc] afterEach: ${e}`)
  }
})

describe('Palette §9: открытие и закрытие', () => {
  it('Cmd+K открывает палитру, автофокус в поле поиска', async () => {
    await openPalette('open')
    const focus = await activeElementInfo()
    const items = await paletteItems()
    console.log(`[palette-acc] OPEN focus=${focus} items=${items.length}`)
    if (!focus.includes('palette-input')) throw new Error(`после Cmd+K фокус: ${focus}, не в поле поиска`)
    await shot('palette-open-cmdk')
  })

  it('повторный Cmd+K закрывает (toggle)', async () => {
    await openPalette('toggle')
    await pressCombo('k', true)
    await pause(800)
    const n = await overlayCount()
    if (n !== 0) throw new Error(`повторный Cmd+K не закрыл палитру (overlay ×${n})`)
    await shot('palette-toggle-closed')
  })

  it('Escape закрывает палитру', async () => {
    await openPalette('escape')
    await pressCombo('Escape', false)
    let closedAt = -1
    for (const delay of [500, 1000, 2000, 4000, 8000]) {
      await pause(delay === 500 ? 500 : delay - (closedAt > 0 ? 0 : 0))
      if ((await overlayCount()) === 0) { closedAt = delay; break }
    }
    console.log(`[palette-acc] ESCAPE_CLOSE_DELAY_MS=${closedAt}`)
    if (closedAt < 0) throw new Error('Escape не закрывает палитру за 8с')
    await shot('palette-escape-closed')
  })

  it('клик по фону (оверлею) закрывает палитру', async () => {
    await openPalette('overlay-click')
    await browser.execute(() => {
      const o = document.querySelector('.palette-overlay') as HTMLElement | null
      o?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    })
    await pause(800)
    const n = await overlayCount()
    if (n !== 0) throw new Error(`клик по оверлею не закрыл палитру (overlay ×${n})`)
    await shot('palette-overlayclick-closed')
  })

  it('реальная доставка клавиш (browser.keys Meta+k) — проба драйвера', async () => {
    await openPalette('realkeys-pre')
    await pressCombo('Escape', false)
    await pause(500)
    let viaKeys = false
    let err = ''
    try {
      await browser.keys(['Meta', 'k'])
      await pause(800)
      viaKeys = (await overlayCount()) > 0
    } catch (e) {
      err = String(e)
    }
    console.log(`[palette-acc] REAL_KEYS_OPEN=${viaKeys}${err ? ` err=${err}` : ''}`)
    if (viaKeys) {
      await pressCombo('Escape', false)
      await pause(500)
    }
    await shot('palette-realkeys-probe')
  })
})

describe('Palette §9: состав команд и поиск', () => {
  it('состав: ровно 13 команд (tools удалён, полное покрытие маршрутов)', async () => {
    await openPalette('list')
    const items = await paletteItems()
    console.log(`[palette-acc] ITEMS=${JSON.stringify(items)}`)
    const expected = COMMANDS.map((c) => c.label)
    for (const e of expected) {
      if (!items.includes(e)) throw new Error(`нет команды «${e}»`)
    }
    if (items.length !== expected.length) {
      throw new Error(`лишние команды: ${JSON.stringify(items)}`)
    }
    await shot('palette-full-list')
  })

  it('поиск по названию фильтрует список («перев» → 4 после расширения состава)', async () => {
    await openPalette('filter')
    await browser.$('[data-testid="palette.input"]').setValue('перев')
    await pause(400)
    const items = await paletteItems()
    console.log(`[palette-acc] FILTER перьев=${JSON.stringify(items)}`)
    // Полный состав 13 команд: «перев» матчит 4 (метки палитры через t(),
    // значения при дефолтной локали ru — те же литералы).
    const EXPECT_PEREV = ['Строки перевода', 'Импорт существующего перевода', 'Память переводов', 'Самоперевод RimLoc']
    if (items.length !== EXPECT_PEREV.length || !EXPECT_PEREV.every((e) => items.some((i) => i.includes(e)))) {
      throw new Error(`фильтр «перев» дал ${JSON.stringify(items)}, ждали ${JSON.stringify(EXPECT_PEREV)}`)
    }
    await shot('palette-filter-perev')
  })

  it('запрос СБРАСЫВАЕТСЯ при закрытии (MUST-FIX закрыт: палитра не открывается предотфильтрованной)', async () => {
    await ensureClosed('reopen-query-pre')
    await openPalette('reopen-query')
    await browser.$('[data-testid="palette.input"]').setValue('глосс')
    await pause(300)
    const q1 = await inputValue()
    if (q1 !== 'глосс') throw new Error(`сеттинг query не дошёл: ${JSON.stringify(q1)}`)
    await pressCombo('Escape', false)
    await pause(800)
    if ((await overlayCount()) > 0) throw new Error('Escape не закрыл — нечего проверять')
    await pressCombo('k', true)
    await pause(800)
    const q2 = await inputValue()
    const items = await paletteItems()
    console.log(`[palette-acc] QUERY_RESET q1=${JSON.stringify(q1)} q2=${JSON.stringify(q2)} items=${items.length}`)
    if (q2 !== '') throw new Error(`запрос пережил закрытие: ${JSON.stringify(q2)} — MUST-FIX не закрыт`)
    if (items.length !== COMMANDS.length) {
      throw new Error(`палитра открылась предотфильтрованной: ${items.length} вместо ${COMMANDS.length}`)
    }
    await shot('palette-query-reset')
  })

  it('без результатов — «Ничего не найдено»', async () => {
    await openPalette('empty')
    await browser.$('[data-testid="palette.input"]').setValue('языки-переключение')
    await pause(400)
    const empty = await browser.execute(() => document.querySelector('.palette-empty')?.textContent ?? '<нет>')
    console.log(`[palette-acc] EMPTY_STATE=${JSON.stringify(empty)}`)
    if (String(empty).includes('<нет>')) throw new Error('нет состояния «Ничего не найдено»')
    await shot('palette-empty-results')
  })

  it('МАНДАТ закрыт: команда «Языки» в палитре есть и ведёт на #/lm', async () => {
    await openPalette('lm-cmd')
    await browser.$('[data-testid="palette.input"]').setValue('Языки')
    await pause(400)
    const items = await paletteItems()
    if (!items.some((i) => i.includes('Языки'))) {
      throw new Error(`команда «Языки» не найдена: ${JSON.stringify(items)}`)
    }
    await pressCombo('Enter', false)
    await pause(1000)
    const hash = await browser.execute(() => window.location.hash)
    if (String(hash) !== '#/lm') throw new Error(`Enter по «Языки» дал ${String(hash)}, ждали #/lm`)
    await shot('palette-lm-command')
  })
})

describe('Palette §9: route navigation по каждой команде', () => {
  for (const c of COMMANDS) {
    it(`«${c.label}» → ${c.hash}`, async () => {
      await ensureClosed(`route-pre:${c.label}`)
      await browser.execute(() => { window.location.hash = '#/home' })
      await pause(400)
      await openPalette(`route:${c.label}`)
      const clicked = await clickItemByLabel(c.label)
      if (!clicked) throw new Error(`команда «${c.label}» не найдена для клика`)
      // навигация
      let navOk = false
      for (let i = 0; i < 20 && !navOk; i++) {
        await pause(250)
        navOk = (await hash()) === c.hash
      }
      // закрытие палитры после выполнения команды
      let closedOk = false
      for (let i = 0; i < 20 && !closedOk; i++) {
        await pause(250)
        closedOk = (await overlayCount()) === 0
      }
      const nOverlay = await overlayCount()
      console.log(
        `[palette-acc] ROUTE «${c.label}» hash=${await hash()} navOk=${navOk} closedOk=${closedOk} overlay=${nOverlay}`,
      )
      if (!navOk) throw new Error(`команда «${c.label}» не перевела на ${c.hash}`)
      if (!closedOk) {
        await shot(`BUG-palette-stays-open-${c.hash.replace(/[#\/]/g, '')}`)
        throw new Error(`БАГ: после «${c.label}» палитра осталась открытой (overlay ×${nOverlay})`)
      }
      if (c.marker) {
        // Роут обновляется hashchange-событием/400мс-поллом в фоне — даём
        // экрану время смонтироваться, прежде чем искать маркер.
        const parts = c.marker.split(', ')
        let found = false
        for (let i = 0; i < 24 && !found; i++) {
          await pause(250)
          for (const p of parts) {
            if (await browser.$(p).isExisting()) { found = true; break }
          }
        }
        if (!found) throw new Error(`маршрут ${c.hash} открыт, но экран не смонтировался (маркер ${c.marker} за 6с)`)
      }
      await shot(`palette-route-${c.hash.replace(/[#\/]/g, '')}`)
    })
  }
})

describe('Palette §9: клавиатура, фокус, конфликты', () => {
  // MUST-FIX #4 из приёмки: навигация стрелками и запуск Enter'ом. Пробы
  // гоняют ТУ же кодовую дорожку, что и живой пользователь: window-level
  // keydown (pressCombo) — хук useCommandPalette слушает именно его.
  it('ArrowDown/ArrowUp подсвечивают активный пункт (класс + aria)', async () => {
    await openPalette('nav-arrows')
    let a = await activeOption()
    if (a.index !== 0) throw new Error(`исходный активный пункт #${a.index}, ожидается #0`)
    await pressCombo('ArrowDown', false)
    await pressCombo('ArrowDown', false)
    a = await activeOption()
    console.log(`[palette-acc] NAV after 2×↓ index=${a.index} ariaAD=${a.ad} label=${a.label}`)
    if (a.index !== 2) throw new Error(`после 2×ArrowDown активен #${a.index}, ожидается #2`)
    if (a.ad !== 'palette-opt-2') throw new Error(`aria-activedescendant=${a.ad}, ожидается palette-opt-2`)
    await shot('palette-nav-arrows-down')
    await pressCombo('ArrowUp', false)
    a = await activeOption()
    if (a.index !== 1) throw new Error(`после ArrowUp активен #${a.index}, ожидается #1`)
    await shot('palette-nav-arrows-up')
  })

  it('стрелки цикличны, Home/End — первый/последний', async () => {
    await openPalette('nav-wrap')
    const n = (await paletteItems()).length
    await pressCombo('End', false)
    let a = await activeOption()
    if (a.index !== n - 1) throw new Error(`End дал #${a.index}, ожидается #${n - 1}`)
    await shot('palette-nav-end')
    await pressCombo('ArrowDown', false)
    a = await activeOption()
    if (a.index !== 0) throw new Error(`после End+↓ (цикл вниз) активен #${a.index}, ожидается #0`)
    await pressCombo('ArrowUp', false)
    a = await activeOption()
    if (a.index !== n - 1) throw new Error(`после ↑ с первого (цикл вверх) активен #${a.index}, ожидается #${n - 1}`)
    await pressCombo('Home', false)
    a = await activeOption()
    if (a.index !== 0) throw new Error(`Home дал #${a.index}, ожидается #0`)
    console.log(`[palette-acc] NAV_WRAP items=${n}: End→#${n - 1}, циклы ok, Home→#0`)
    await shot('palette-nav-wrap')
  })

  it('Enter запускает активную команду: маршрут + закрытие палитры', async () => {
    await ensureClosed('enter-pre')
    await browser.execute(() => { window.location.hash = '#/home' })
    await pause(400)
    await openPalette('nav-enter')
    await pressCombo('ArrowDown', false)
    const a = await activeOption()
    const target = COMMANDS.find((c) => c.label === a.label)
    if (!target) throw new Error(`активная команда «${a.label}» не из списка COMMANDS`)
    await pressCombo('Enter', false)
    let navOk = false
    let closedOk = false
    for (let i = 0; i < 20 && !(navOk && closedOk); i++) {
      await pause(250)
      navOk = (await hash()) === target.hash
      closedOk = (await overlayCount()) === 0
    }
    console.log(`[palette-acc] NAV_ENTER «${a.label}» hash=${await hash()} navOk=${navOk} closedOk=${closedOk}`)
    if (!navOk) throw new Error(`Enter не перевёл на ${target.hash} (активной была «${a.label}»)`)
    if (!closedOk) throw new Error('Enter выполнил команду, но палитра осталась открытой')
    await shot('palette-nav-enter-runs')
  })

  it('фильтр сбрасывает активный пункт на первый', async () => {
    await openPalette('nav-filter-reset')
    await pressCombo('ArrowDown', false)
    await pressCombo('ArrowDown', false)
    let a = await activeOption()
    if (a.index !== 2) throw new Error(`до фильтра активен #${a.index}, ожидается #2`)
    await browser.$('[data-testid="palette.input"]').setValue('перев')
    await pause(400)
    const items = await paletteItems()
    a = await activeOption()
    console.log(`[palette-acc] NAV_FILTER_RESET items=${JSON.stringify(items)} active=#${a.index} ariaAD=${a.ad}`)
    if (items.length !== 4) throw new Error(`фильтр «перев» дал ${items.length} команд (в HEAD их 4)`)
    if (a.index !== 0) throw new Error(`после фильтра активен #${a.index}, ожидается #0`)
    if (a.ad !== 'palette-opt-0') throw new Error(`aria-activedescendant=${a.ad}, ожидается palette-opt-0`)
    await shot('palette-nav-filter-reset')
  })

  it('hover мышью синхронизирует активный пункт', async () => {
    await openPalette('nav-hover')
    await pressCombo('ArrowDown', false)
    let a = await activeOption()
    if (a.index !== 1) throw new Error(`после ↓ активен #${a.index}, ожидается #1`)
    // React синтезирует onMouseEnter из делегированного mouseover —
    // всплывающий mouseover по пункту и есть программная имитация наведения.
    await browser.execute(() => {
      const item = document.querySelectorAll('.palette-item')[0] as HTMLElement | undefined
      item?.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }))
    })
    await pause(300)
    a = await activeOption()
    console.log(`[palette-acc] NAV_HOVER mouseover→#0, активен #${a.index}`)
    if (a.index !== 0) throw new Error(`hover не синхронизировал курсор: активен #${a.index}, ожидается #0`)
    await shot('palette-nav-hover')
  })

  it('фокус после закрытия: куда возвращается', async () => {
    await ensureClosed('focus-pre')
    await browser.$('a[href="#/checks"]').click()
    await pause(500)
    const before = await activeElementInfo()
    await openPalette('focus')
    const inPalette = await activeElementInfo()
    await pressCombo('Escape', false)
    await pause(800)
    const after = await activeElementInfo()
    console.log(`[palette-acc] FOCUS before=${before} inPalette=${inPalette} afterEsc=${after}`)
    await shot('palette-focus-after-close')
    if (after === 'body' && before !== 'body') {
      console.log('[palette-acc] FINDING: фокус НЕ восстановлен (ушёл в body)')
    }
  })

  it('Cmd+K внутри input-поля: палитра открывается поверх поля', async () => {
    await ensureClosed('cmdk-pre')
    await browser.execute(() => { window.location.hash = '#/projects' })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue('/tmp/конфликт-проверка')
    const focusInField = await activeElementInfo()
    await pressCombo('k', true)
    const opened = (await overlayCount()) > 0
    await shot('palette-cmdk-inside-input')
    // Палитра открыта ПОВЕРХ мастера: Escape закрывает только палитру?
    await pressCombo('Escape', false)
    await pause(800)
    const wizardStill = await browser.$('[data-testid="wizard.path-input"]').isExisting()
    const after = await activeElementInfo()
    let fieldVal: string = '<closed>'
    if (wizardStill) fieldVal = await browser.$('[data-testid="wizard.path-input"]').getValue()
    console.log(
      `[palette-acc] CMDK_IN_INPUT focus=${focusInField} opened=${opened} afterEsc=${after} wizardStill=${wizardStill} fieldValue=${JSON.stringify(fieldVal)}`,
    )
    if (!opened) throw new Error('Cmd+K из input-поля не открыл палитру (это тоже результат — записан)')
    // Закрыть мастер, чтобы не мешать следующим тестам (кнопка «Отмена»).
    if (wizardStill) {
      await browser.execute(() => {
        const btn = document.querySelector('.dialog-actions button') as HTMLElement | null
        btn?.click()
      })
      await pause(400)
    }
  })
})

describe('Palette §9 с открытым проектом: validate и export как реальные экраны', () => {
  /**
   * Создаёт проект из фикстуры /tmp/rimloc-wizard-mod через UI-мастер
   * (реальный Rust-скан). Палитра затем ведёт на ЖИВЫЕ экраны проверок,
   * глоссария и сборки — это и есть команды validate/build/export мандата.
   */
  before(async () => {
    await ensureClosed('project-pre')
    await browser.execute(() => { window.location.hash = '#/projects' })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue('/tmp/rimloc-wizard-mod')
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]')
    await browser.$('[data-testid="wizard.version"]').selectByVisibleText('1.6')
    await browser.$('[data-testid="wizard.next"]').click()
    await browser.$('[data-testid="wizard.next"]').click()
    // Создание = реальный скан мода; ждём воркспейс.
    await browser.waitUntil(
      async () => (await browser.execute(() => window.location.hash)) === '#/workspace',
      { timeout: 60000, interval: 250 },
    )
    await waitExisting('[data-testid="ws.root"]', 30000)
    await shot('project-created-workspace')
  })

  it('«Проверки» открывает ЖИВОЙ экран валидации (validate)', async () => {
    await openPalette('validate-live')
    await clickItemByLabel('Проверки')
    await waitExisting('[data-testid="checks.rerun"]', 15000)
    // Валидатор прогоняется эффектом при монтировании: findings-блок всегда
    // в DOM, ждём пока исчезнет статус «Запускаю живую валидацию…».
    await browser.waitUntil(
      async () =>
        (await browser.execute(() => {
          const f = document.querySelector('[data-testid="checks.findings"]')
          return Boolean(f) && !f!.textContent!.includes('Запускаю живую валидацию')
        })),
      { timeout: 30000, interval: 250 },
    )
    const body = await browser.execute(() => document.body.innerText.slice(0, 400))
    console.log(`[palette-acc] VALIDATE_LIVE экран: ${body.replace(/\n+/g, ' | ').slice(0, 220)}`)
    await shot('palette-validate-live')
  })

  it('«Глоссарий» открывает живой экран с формой добавления', async () => {
    await openPalette('glossary-live')
    await clickItemByLabel('Глоссарий')
    await waitExisting('[data-testid="gl.table"]', 15000)
    await shot('palette-glossary-live')
  })

  it('«Сборка и экспорт» открывает живой экран сборки (build/export)', async () => {
    await openPalette('export-live')
    await clickItemByLabel('Сборка и экспорт')
    await waitExisting('[data-testid="be.outdir"]', 15000)
    await shot('palette-export-live')
  })
})
