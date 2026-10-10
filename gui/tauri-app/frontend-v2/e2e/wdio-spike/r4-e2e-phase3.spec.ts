// r4-e2e phase 3 (mandate §4): §5 checks cycle (persist-aware) + §7 chat-batch
// mini with honest error capture. TEMPORARY — not committed.
import fs from 'node:fs'
import assert from 'node:assert'

const STATE_PATH = '/tmp/r4-e2e/state.json'
const PH_KEY = 'HugsLib_setting_mod_name_title'
const PH_JA = '{0}のMod設定'
const CB_KEYS = ['HugsLib_settings_resetValue', 'HugsLib_settings_windowTitle']

async function waitExisting(sel: string, timeout = 30000): Promise<void> {
  await browser.waitUntil(async () => (await browser.$(sel)).isExisting(), {
    timeout,
    interval: 250,
  })
}
async function pause(ms: number): Promise<void> {
  await new Promise((r) => setTimeout(r, ms))
}
async function js<T>(fn: () => T): Promise<T> {
  return browser.execute(fn) as unknown as T
}
async function selectKeyExact(key: string): Promise<string> {
  await browser.$('.entry-toolbar .search-field input').setValue(key)
  await pause(500)
  const clicked = await browser.execute((k) => {
    const rows = Array.from(document.querySelectorAll('.entry-row'))
    const hit = rows.find(
      (r) => r.querySelector('.row-source code')?.textContent === k,
    ) as HTMLElement | undefined
    if (hit) {
      hit.click()
      return true
    }
    return false
  }, key)
  if (!clicked) return ''
  await pause(400)
  return browser.execute(() => document.querySelector('.detail-key code')?.textContent ?? '')
}
async function editorValue(): Promise<string> {
  return browser.execute(
    () => (document.querySelector('[data-testid="ws.editor-textarea"]') as HTMLTextAreaElement)?.value ?? '',
  )
}
/** Persist detector: the ROW target only changes when the fresh snapshot is
 *  adopted — a draft never touches it. */
async function rowTarget(key: string): Promise<string> {
  await browser.$('.entry-toolbar .search-field input').setValue(key)
  await pause(450)
  return browser.execute((k) => {
    const rows = Array.from(document.querySelectorAll('.entry-row'))
    const hit = rows.find((r) => r.querySelector('.row-source code')?.textContent === k)
    return hit?.querySelector('.row-target')?.textContent ?? ''
  }, key)
}
async function setTargetJa(): Promise<void> {
  await browser.execute(() => {
    const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement
    sel.value = 'ja'
    sel.dispatchEvent(new Event('change', { bubbles: true }))
  })
  await pause(800)
}
async function checksBand(): Promise<string> {
  return browser.execute(() => document.querySelector('.metrics-band')?.textContent ?? '')
}

describe('r4-e2e phase3: §5 re-run + §7 chat-batch', () => {
  it('§5R checks: break placeholder → error finding → fix → clean; report export', async () => {
    const st = JSON.parse(fs.readFileSync(STATE_PATH, 'utf8'))
    await waitExisting('.app-sidebar', 40000)
    await js(() => {
      window.location.hash = '#/projects'
    })
    await waitExisting('[data-testid="home.project-card"]')
    await browser.$('[data-testid="home.project-card"]').click()
    await waitExisting('[data-testid="ws.root"]', 60000)
    await pause(800)
    await setTargetJa()
    await pause(400)

    // break
    assert.strictEqual(await selectKeyExact(PH_KEY), PH_KEY, 'PH key not found')
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('R4BROKEN Mod設定')
    await browser.$('[data-testid="ws.editor-save-next"]').click()
    await browser.waitUntil(async () => (await rowTarget(PH_KEY)) === 'R4BROKEN Mod設定', {
      timeout: 30000,
      interval: 400,
    })
    console.log(`[r4] §5R broke ${PH_KEY} (row shows R4BROKEN — commit persisted)`)

    // finding
    await js(() => {
      window.location.hash = '#/checks'
    })
    await waitExisting('[data-testid="checks.findings"]', 60000)
    await browser.waitUntil(async () => /ошибки/.test(await checksBand()), {
      timeout: 60000,
      interval: 300,
    })
    await pause(300)
    const broken = await js(() => ({
      band: document.querySelector('.metrics-band')?.textContent ?? '',
      onKey: Array.from(
        document.querySelectorAll('[data-testid="checks.findings"] .finding-row'),
      )
        .map((e) => e.textContent ?? '')
        .filter((t) => t.includes('HugsLib_setting_mod_name_title')),
    }))
    console.log(`[r4] §5R BROKEN band=${JSON.stringify(broken.band)} onKey=${JSON.stringify(broken.onKey)}`)
    assert(broken.onKey.length > 0, 'no finding references the broken key')
    assert(/lost-placeholder|placeholder/.test(broken.onKey[0]), 'finding is not about placeholders')

    // fix (persist-aware)
    await js(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]')
    await pause(400)
    assert.strictEqual(await selectKeyExact(PH_KEY), PH_KEY)
    await browser.$('[data-testid="ws.editor-textarea"]').setValue(PH_JA)
    await browser.$('[data-testid="ws.editor-save-next"]').click()
    await browser.waitUntil(async () => (await rowTarget(PH_KEY)) === PH_JA, {
      timeout: 30000,
      interval: 400,
    })
    console.log(`[r4] §5R fixed ${PH_KEY} (row shows the restored value)`)

    // fresh validation via rerun button
    await js(() => {
      window.location.hash = '#/checks'
    })
    await waitExisting('[data-testid="checks.rerun"]', 60000)
    await pause(1200)
    await browser.$('[data-testid="checks.rerun"]').click()
    await pause(2500)
    const fixedBand = await checksBand()
    console.log(`[r4] §5R FIXED band=${JSON.stringify(fixedBand)}`)
    assert(!/[1-9]\d*ошибки/.test(fixedBand), `errors remain after fix: ${fixedBand}`)
    const keyErrorRows = await browser.execute((k) =>
      Array.from(document.querySelectorAll('[data-testid="checks.findings"] .finding-row'))
        .filter((r) => (r.textContent ?? '').includes(k))
        .map((r) => r.textContent?.slice(0, 120) ?? ''), PH_KEY)
    console.log(`[r4] §5R rows mentioning the key after fix: ${JSON.stringify(keyErrorRows)}`)
    assert(
      !keyErrorRows.some((t) => /lost-placeholder/i.test(t)),
      'lost-placeholder error still reported after fix',
    )

    // support: report export
    const hadReport = await browser.execute(() => {
      const btn = document.querySelector('.section-tabs button.ml-auto') as HTMLButtonElement | null
      if (!btn) return false
      btn.click()
      return true
    })
    await pause(1500)
    const candidates = [
      '/tmp/rimloc-validation.json',
      `${process.env.HOME ?? ''}/Downloads/rimloc-validation.json`,
      '/tmp/r4-e2e/rimloc-validation.json',
    ]
    const landed = candidates.filter((p) => fs.existsSync(p))
    console.log(`[r4] §5R report export: clicked=${hadReport} landedOnDisk=${JSON.stringify(landed)}`)
    st.reportExport = { clicked: hadReport, landed }
    fs.writeFileSync(STATE_PATH, JSON.stringify(st, null, 1))
  })

  it('§7R chat-batch mini: 2 strings → prompt → manual answer → apply', async () => {
    const st = JSON.parse(fs.readFileSync(STATE_PATH, 'utf8'))
    await js(() => {
      window.location.hash = '#/chatbatch'
    })
    await waitExisting('[data-testid="cb.entries"]')
    await pause(500)
    // leftover exported batch from the interrupted run → start a new one
    const resetClicked = await browser.execute(() => {
      const btn = document.querySelector('[data-testid="cb.new-batch"]') as HTMLButtonElement | null
      if (btn && !btn.disabled) {
        btn.click()
        return true
      }
      return false
    })
    console.log(`[r4] §7R new-batch clicked: ${resetClicked}`)
    await pause(600)
    const pool0 = await js(
      () => document.querySelectorAll('[data-testid^="cb.entry."]').length,
    )
    console.log(`[r4] §7R untranslated-only pool: ${pool0}`)
    if (pool0 === 0) {
      await browser.$('[data-testid="cb.filter-untranslated"]').click()
      await pause(500)
    }
    const pool = await js(() => document.querySelectorAll('[data-testid^="cb.entry."]').length)
    console.log(`[r4] §7R full pool: ${pool}`)
    assert(pool >= st.sourceRows, `pool ${pool} < rows ${st.sourceRows}`)
    for (const key of CB_KEYS) {
      const clicked = await browser.execute((k) => {
        const input = Array.from(
          document.querySelectorAll<HTMLInputElement>('[data-testid^="cb.entry."]').values(),
        ).find((i) => (i.getAttribute('data-testid') ?? '').endsWith(`·${k}`))
        if (!input) return 'missing'
        if (!input.checked) input.click()
        return 'ok'
      }, key)
      assert.strictEqual(clicked, 'ok', `checkbox for ${key}: ${clicked}`)
    }
    await pause(400)
    const selCount = await js(
      () => document.querySelector('[data-testid="cb.selection-count"]')?.textContent ?? '',
    )
    console.log(`[r4] §7R selected: ${JSON.stringify(selCount)}`)
    assert(/2/.test(selCount), `expected 2 selected, got ${selCount}`)
    await browser.$('[data-testid="cb.create"]').click()
    try {
      await waitExisting('[data-testid="cb.export-section"]', 30000)
    } catch (e) {
      const errState = await js(() => ({
        error: document.querySelector('[data-testid="cb.error"]')?.textContent ?? '(no cb.error)',
        body: document.body.innerText.slice(0, 300),
      }))
      console.log(`[r4] §7R CREATE FAILED: ${JSON.stringify(errState)}`)
      throw e
    }
    await browser.$('[data-testid="cb.export"]').click()
    await waitExisting('[data-testid="cb.prompt"]')
    const prompt = (await browser.$('[data-testid="cb.prompt"]').getValue()) ?? ''
    assert(prompt.includes('Respond with ONLY one line per string'), 'prompt marker missing')
    for (const k of CB_KEYS) assert(prompt.includes(`${k}: `), `prompt lacks key ${k}`)
    console.log(`[r4] §7R prompt locale line: ${JSON.stringify(prompt.split('\n').find((l) => l.includes('Target language')) ?? '')}`)
    // Manual answer: echo the prompt's keys VERBATIM (they are batch
    // identities, e.g. 'keyed·<key>'), one 'key: translation' line each —
    // the human workflow. Prompt lines between the markers are pairs.
    const lines = prompt.split('\n')
    const start = lines.findIndex((l) => l.startsWith('Source strings follow'))
    const end = lines.findIndex((l) => l.startsWith('Respond with'))
    assert(start >= 0 && end > start, 'prompt markers missing')
    let n = 0
    const answer: string[] = []
    for (const l of lines.slice(start + 1, end)) {
      const trimmed = l.trim()
      const idx = trimmed.indexOf(': ')
      if (idx <= 0) continue
      const key = trimmed.slice(0, idx)
      if (/\s/.test(key)) continue // header line, not a pair
      n += 1
      answer.push(`${key}: Ручной перевод ${n}`)
    }
    assert.strictEqual(n, 2, `prompt pairs parsed: ${n}, expected 2`)
    const answerText = answer.join('\n')
    console.log(`[r4] §7R answer keys: ${JSON.stringify(answer.map((a) => a.split(':')[0]))}`)
    await browser.$('[data-testid="cb.paste"]').setValue(answerText)
    const pasted = await browser.$('[data-testid="cb.paste"]').getValue()
    console.log(`[r4] §7R paste area holds ${pasted?.split('\n').length} lines`)
    assert(pasted === answerText, 'paste textarea lost the answer (React onChange missed)')
    await browser.$('[data-testid="cb.import"]').click()
    try {
      await waitExisting('[data-testid="cb.preview"]', 60000)
    } catch (e) {
      const errState = await js(() => ({
        error: document.querySelector('[data-testid="cb.error"]')?.textContent ?? '(no cb.error)',
        badge: document.querySelector('[data-testid="cb.status-badge"]')?.textContent ?? '',
        body: document.body.innerText.slice(0, 500),
      }))
      console.log(`[r4] §7R IMPORT FAILED: err=${JSON.stringify(errState.error)} badge=${JSON.stringify(errState.badge)} body=${JSON.stringify(errState.body)}`)
      throw e
    }
    const preview = await js(() => ({
      rows: document.querySelectorAll('[data-testid="cb.preview-row"]').length,
      text: document.querySelector('[data-testid="cb.preview"]')?.textContent?.slice(0, 220) ?? '',
    }))
    console.log(`[r4] §7R preview rows=${preview.rows} head=${JSON.stringify(preview.text)}`)
    assert.strictEqual(preview.rows, 2, 'preview must show exactly 2 rows')
    await browser.$('[data-testid="cb.apply"]').click()
    await waitExisting('[data-testid="cb.done-section"]', 60000)
    const done = await js(
      () => document.querySelector('[data-testid="cb.done-section"]')?.textContent ?? '',
    )
    console.log(`[r4] §7R DONE=${JSON.stringify(done.slice(0, 180))}`)
    await js(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]')
    await pause(800)
    const ws = await js(() => ({
      badge: document.querySelector('.tree-root span')?.textContent ?? '',
      untranslated: document.querySelectorAll('.row-target.untranslated').length,
    }))
    assert.strictEqual(
      Number((ws.badge.match(/(\d+)/) ?? [])[1]),
      st.sourceRows,
      'dup rows after chatbatch apply',
    )
    for (let i = 0; i < CB_KEYS.length; i++) {
      assert.strictEqual(await selectKeyExact(CB_KEYS[i]), CB_KEYS[i])
      const val = await editorValue()
      console.log(`[r4] §7R applied ${CB_KEYS[i]} = ${JSON.stringify(val)}`)
      assert.strictEqual(val, `Ручной перевод ${i + 1}`, 'chatbatch value not in workspace')
    }
    await browser.$('.entry-toolbar .search-field input').setValue('')
    console.log('[r4] §7R chat-batch cycle complete on real mod')
  })
})
