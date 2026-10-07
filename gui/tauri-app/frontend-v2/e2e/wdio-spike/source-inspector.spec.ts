// Source Inspector E2E (lane C2): the workspace SOURCE block shows the REAL
// source of the selected row from the live contract snapshot — asserted via
// data-testid="src.*". No synthetic fixtures ride the production path: the
// project is self-seeded from /tmp/rimloc-wizard-mod (same before() contract
// as chatbatch-acceptance), whose Fixture.xml carries known keys at known
// 1-based lines (Fixture_Greet = line 2, verified by `rimloc-cli scan
// --format json` on this exact tree).
async function waitExisting(sel: string, timeout = 30000): Promise<void> {
  await browser.waitUntil(
    async () => (await browser.$(sel)).isExisting(),
    { timeout, interval: 250 },
  )
}

async function pause(ms: number): Promise<void> {
  await new Promise((r) => setTimeout(r, ms))
}

/** Full textContent of a node (block-level checks). */
async function textOf(testid: string): Promise<string> {
  const raw = await browser.execute((tid) => {
    const el = document.querySelector(`[data-testid="${tid}"]`)
    return (el?.textContent ?? '').trim()
  }, testid)
  return String(raw)
}

/** VALUE text of a src.* row (its `code`/`strong` child — the label span
 *  must not leak into assertions). */
async function valueOf(testid: string): Promise<string> {
  const raw = await browser.execute((tid) => {
    const el = document.querySelector(
      `[data-testid="${tid}"] code, [data-testid="${tid}"] strong`,
    )
    return (el?.textContent ?? '').trim()
  }, testid)
  return String(raw)
}

async function exists(testid: string): Promise<boolean> {
  return Boolean(
    await browser.execute(
      (tid) => Boolean(document.querySelector(`[data-testid="${tid}"]`)),
      testid,
    ),
  )
}

describe('Source Inspector §C2: SOURCE-блок показывает реальный источник', () => {
  before(async () => {
    await waitExisting('.app-sidebar', 30000)
    // Self-seed (same as palette/chatbatch acceptance): a fresh instance
    // opens no project — create one from the wizard fixture mod.
    const needProject = await browser.execute(async () => {
      window.location.hash = '#/workspace'
      await new Promise((r) => setTimeout(r, 800))
      return !document.querySelector('[data-testid="ws.root"]')
    })
    if (needProject) {
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
      await browser.waitUntil(
        async () => (await browser.execute(() => window.location.hash)) === '#/workspace',
        { timeout: 60000, interval: 250 },
      )
      await waitExisting('[data-testid="ws.root"]', 30000)
    }
    // Select the row whose key is Fixture_Greet (rows render `<code>{key}</code>`)
    // and wait for the live editor + SOURCE block.
    await browser.waitUntil(
      async () => {
        const clicked = await browser.execute(() => {
          const rows = Array.from(
            document.querySelectorAll('[data-testid^="ws.entry."]'),
          ) as HTMLElement[]
          const hit = rows.find((r) => r.querySelector('code')?.textContent === 'Fixture_Greet')
          if (!hit) return false
          hit.click()
          return true
        })
        return clicked
      },
      { timeout: 30000, interval: 250 },
    )
    await waitExisting('[data-testid="src.block"]')
    await pause(300)
  })

  it('src.file = реальный относительный путь Languages/English/Keyed/Fixture.xml', async () => {
    const file = await valueOf('src.file')
    console.log(`[src-inspector] FILE=${JSON.stringify(file)}`)
    if (file !== 'Languages/English/Keyed/Fixture.xml') {
      throw new Error(`src.file=${JSON.stringify(file)} — ожидался реальный относительный путь`)
    }
  })

  it('src.line = parser-guaranteed строка 2 (никогда не выдуманная)', async () => {
    const line = await valueOf('src.line')
    console.log(`[src-inspector] LINE=${JSON.stringify(line)}`)
    if (!/^\d+$/.test(line)) throw new Error(`src.line=${JSON.stringify(line)} — не число`)
    if (line !== '2') {
      throw new Error(`src.line=${line} — Fixture_Greet записан парсером на строке 2`)
    }
  })

  it('src.selected-by = причина победителя (winner_reason, локализована)', async () => {
    const why = await valueOf('src.selected-by')
    console.log(`[src-inspector] SELECTED_BY=${JSON.stringify(why)}`)
    if (!why || why === '—') throw new Error('src.selected-by пуст — winner reason не дошёл')
    // tEnum renders the localized vocabulary (ru default); the raw wire token
    // must never leak as user text.
    const known = [
      'Победило первое вхождение в файле', // ru: keyed-first-in-file
      'First in-file occurrence wins', // en fallback
    ]
    if (!known.includes(why)) {
      throw new Error(`src.selected-by=${JSON.stringify(why)} — не словарная причина победителя`)
    }
  })

  it('src.root = живой read-only корень проекта', async () => {
    const root = await valueOf('src.root')
    console.log(`[src-inspector] ROOT=${JSON.stringify(root)}`)
    if (root !== '/tmp/rimloc-wizard-mod') {
      throw new Error(`src.root=${JSON.stringify(root)} — ожидался корень /tmp/rimloc-wizard-mod`)
    }
  })

  it('честное отсутствие: для Keyed-строки нет TKey/версии/патча/условной ветки', async () => {
    // The fixture has none of these facts — the block must NOT fabricate
    // rows for them (honest-absence contract, finding M-7 lineage).
    for (const tid of ['src.tkey', 'src.version', 'src.patch', 'src.conditional']) {
      if (await exists(tid)) throw new Error(`${tid} отрисовался без данных в snapshot — синтетика`)
    }
    const block = await textOf('src.block')
    if (!block.includes('Fixture')) {
      // The file row is contract data; the block must carry it.
      throw new Error('src.block не содержит файл из snapshot')
    }
  })
})
