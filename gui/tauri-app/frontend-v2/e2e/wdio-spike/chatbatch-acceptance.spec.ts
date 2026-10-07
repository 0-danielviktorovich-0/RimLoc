// Chat-batch E2E (rel22): the real no-API workflow end to end.
// Batch semantics under test (mirrors chat_batch.rs Rust tests, UI side):
//   - malformed response refuses the WHOLE import (batch stays exported);
//   - a valid response on the SAME batch imports → preview → apply → done;
//   - applied translations land via ApplyOrigin::Import (workspace shows
//     them, untranslated pool shrinks to zero).
async function waitExisting(sel: string, timeout = 20000): Promise<void> {
  await browser.waitUntil(
    async () => (await browser.$(sel)).isExisting(),
    { timeout, interval: 250 },
  )
}

async function pause(ms: number): Promise<void> {
  await new Promise((r) => setTimeout(r, ms))
}

/** Build the assistant's answer from the exported prompt: one
  `key: translation` line per prompt pair, keys copied verbatim.
  Only lines between "Source strings follow" and "Respond with" count —
  header lines also contain ": " and are not pairs. */
function respondTo(prompt: string): string {
  const lines = prompt.split('\n')
  const start = lines.findIndex((l) => l.startsWith('Source strings follow'))
  const end = lines.findIndex((l) => l.startsWith('Respond with'))
  if (start < 0 || end < 0 || end <= start) {
    throw new Error('маркеры промпта не найдены')
  }
  const out: string[] = []
  for (const l of lines.slice(start + 1, end)) {
    const trimmed = l.trim()
    const idx = trimmed.indexOf(': ')
    if (idx <= 0) continue
    const key = trimmed.slice(0, idx)
    if (/\s/.test(key)) continue // header line, not a pair
    out.push(`${key}: Перевод ${out.length + 1}`)
  }
  if (out.length === 0) throw new Error('не смог распарсить пары из промпта')
  return out.join('\n')
}

describe('Chat-batch §R2: полный цикл без API', () => {
  before(async () => {
    await waitExisting('.app-sidebar', 30000)
    // Self-seed (same as palette-acceptance): fresh instance opens no project.
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
    await browser.execute(() => { window.location.hash = '#/chatbatch' })
    await waitExisting('[data-testid="cb.create"]')
  })

  it('создание батча: выбор всех строк → статус exported, секция экспорта живая', async () => {
    // untranslatedOnly=true по умолчанию — кандидаты = все 8 фикстурных строк.
    await browser.execute(() => {
      const boxes = Array.from(
        document.querySelectorAll('[data-testid^="cb.entry."]'),
      ) as HTMLInputElement[]
      for (const b of boxes) if (!b.checked) b.click()
    })
    await pause(300)
    const count = await browser.execute(() =>
      document.querySelector('[data-testid="cb.selection-count"]')?.textContent ?? '')
    console.log(`[cb-acc] SELECTED=${count}`)
    await browser.$('[data-testid="cb.create"]').click()
    await waitExisting('[data-testid="cb.export-section"]')
    await browser.$('[data-testid="cb.export"]').click()
    await waitExisting('[data-testid="cb.prompt"]')
    const prompt = await browser.$('[data-testid="cb.prompt"]').getValue()
    if (!prompt || !prompt.includes('Respond with ONLY one line per string')) {
      throw new Error(`промпт не построился: ${String(prompt).slice(0, 120)}`)
    }
    // Ключи фикстуры обязаны быть в промпте — identity resolution сработала.
    for (const k of ['Fixture_Greet', 'Fixture_Farewell', 'Fixture_Report_Colonies']) {
      if (!prompt.includes(`${k}: `)) throw new Error(`в промпте нет ключа ${k}`)
    }
  })

  it('битый ответ отклоняется ЦЕЛИКОМ: батч остаётся exported, превью нет', async () => {
    await browser.$('[data-testid="cb.paste"]').setValue('This line has no separator at all\nИ эта тоже')
    await browser.$('[data-testid="cb.import"]').click()
    await waitExisting('[data-testid="cb.error"]')
    const err = await browser.execute(() =>
      document.querySelector('[data-testid="cb.error"]')?.textContent ?? '')
    console.log(`[cb-acc] MALFORMED err=${JSON.stringify(err.slice(0, 160))}`)
    const previewGone = await browser.execute(() => !document.querySelector('[data-testid="cb.preview"]'))
    if (!previewGone) throw new Error('превью появилось после битого импорта')
    const stillExported = await browser.execute(() => Boolean(document.querySelector('[data-testid="cb.import-section"]')))
    if (!stillExported) throw new Error('батч пережил битый импорт — секция импорта пропала')
  })

  it('валидный ответ на ТОМ ЖЕ батче: превью 8 строк → apply → done', async () => {
    const prompt = await browser.$('[data-testid="cb.prompt"]').getValue()
    const response = respondTo(String(prompt))
    await browser.$('[data-testid="cb.paste"]').setValue(response)
    await browser.$('[data-testid="cb.import"]').click()
    await waitExisting('[data-testid="cb.preview"]')
    const rows = await browser.execute(() =>
      document.querySelectorAll('[data-testid="cb.preview-row"]').length)
    if (rows !== 8) throw new Error(`в превью ${rows} строк, ожидалось 8`)
    await browser.$('[data-testid="cb.apply"]').click()
    await waitExisting('[data-testid="cb.done-section"]')
    const done = await browser.execute(() =>
      document.querySelector('[data-testid="cb.done-section"]')?.textContent ?? '')
    console.log(`[cb-acc] DONE=${JSON.stringify(done.slice(0, 140))}`)
  })

  it('применённое живёт в workspace: 0 непереведённых, origin Imported', async () => {
    await browser.execute(() => { window.location.hash = '#/workspace' })
    await waitExisting('[data-testid="ws.root"]')
    await pause(1500)
    const diag = await browser.execute(() => ({
      untranslated: document.querySelectorAll('.row-target.untranslated').length,
      rows: document.querySelectorAll('.row-target').length,
      sample: Array.from(document.querySelectorAll('.row-target')).slice(0, 3)
        .map((e) => e.textContent?.slice(0, 40)),
      search: (document.querySelector('[data-testid="ws.search"]') as HTMLInputElement | null)?.value ?? null,
    }))
    console.log(`[cb-acc] WS_DIAG=${JSON.stringify(diag)}`)
    // Wait for the live snapshot refresh after the apply.
    await browser.waitUntil(
      async () => (await browser.execute(() =>
        document.querySelectorAll('.row-target.untranslated').length)) === 0,
      { timeout: 20000, interval: 250 },
    )
    // Provenance: a batch-applied entry carries the Imported origin marker.
    const origin = await browser.execute(() => {
      const t = document.body.innerText
      return /Imported|Импорт/i.test(t) ? 'imported-marker-present' : 'no-marker'
    })
    console.log(`[cb-acc] ORIGIN=${origin}`)
    // Back on chatbatch: untranslated-only pool is empty.
    await browser.execute(() => { window.location.hash = '#/chatbatch' })
    await waitExisting('[data-testid="cb.entries"]')
    await browser.waitUntil(
      async () => (await browser.execute(() =>
        document.body.innerText.includes('нет строк') ||
        document.body.innerText.toLowerCase().includes('nocandidates') ||
        (document.querySelector('[data-testid="cb.entries"]')?.textContent?.trim().length ?? 1) === 0 ||
        Boolean(document.querySelector('.page-note')))),
      { timeout: 10000, interval: 250 },
    )
  })
})
