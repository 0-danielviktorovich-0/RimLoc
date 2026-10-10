// r4-e2e phase 2 (mandate §4, steps 4b-7): REAL HugsLib corpus, fresh app
// instance over the same RIMLOC_DATA_DIR profile.
// §4b PERSIST: recents reopen → edits/imports/target intact, no dup rows
// §5  checks: break a placeholder → finding; fix → gone; report export
// §6  build to /tmp/r4-e2e/out → disk structure/keys/markers + idempotent
//     rebuild (in-spec fs diff)
// §7  chat-batch mini: 2 strings → prompt → manual answer → apply
// TEMPORARY — not committed.
import fs from 'node:fs'
import path from 'node:path'
import crypto from 'node:crypto'
import assert from 'node:assert'

const STATE_PATH = '/tmp/r4-e2e/state.json'
const OUT_DIR = '/tmp/r4-e2e/out'

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
  const searchInput = await browser.$('.entry-toolbar .search-field input')
  await searchInput.setValue(key)
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
async function setTargetJa(): Promise<void> {
  await browser.execute(() => {
    const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement
    sel.value = 'ja'
    sel.dispatchEvent(new Event('change', { bubbles: true }))
  })
  await pause(800)
}
function shaTree(root: string): Record<string, string> {
  const out: Record<string, string> = {}
  const walk = (dir: string): void => {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name)
      if (e.isDirectory()) walk(p)
      else out[path.relative(root, p)] = crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex')
    }
  }
  walk(root)
  return out
}

describe('r4-e2e phase2: restart-persist → checks → build → chat-batch', () => {
  it('§4b fresh instance does NOT auto-reopen; project reopens from recents', async () => {
    const st = JSON.parse(fs.readFileSync(STATE_PATH, 'utf8'))
    await waitExisting('.app-sidebar', 40000)
    await js(() => {
      window.location.hash = '#/workspace'
    })
    await pause(800)
    const autoOpened = await browser.execute(() =>
      Boolean(document.querySelector('[data-testid="ws.root"]')),
    )
    console.log(`[r4] §4b fresh instance auto-reopened project: ${autoOpened}`)
    assert(!autoOpened, 'fresh instance unexpectedly reopened a project')
    await js(() => {
      window.location.hash = '#/projects'
    })
    await waitExisting('[data-testid="home.project-card"]')
    const cards = await js(() =>
      Array.from(document.querySelectorAll('[data-testid="home.project-card"]')).map(
        (e) => e.querySelector('h3')?.textContent ?? '',
      ),
    )
    console.log(`[r4] §4b recents cards: ${JSON.stringify(cards)}`)
    assert.strictEqual(cards.length, 1, 'expected exactly the HugsLib project in recents')
    await browser.$('[data-testid="home.project-card"]').click()
    await browser.waitUntil(
      async () => (await browser.execute(() => window.location.hash)) === '#/workspace',
      { timeout: 60000, interval: 250 },
    )
    await waitExisting('[data-testid="ws.root"]', 30000)
    await pause(800)
    const reopened = await js(() => ({
      badge: document.querySelector('.tree-root span')?.textContent ?? '',
      selectValue: (document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement)?.value ?? '',
      folder: document.querySelector('.tree-bottom code')?.textContent ?? '',
      untranslated: document.querySelectorAll('.row-target.untranslated').length,
    }))
    console.log(
      `[r4] §4b REOPENED badge=${reopened.badge} select=${reopened.selectValue} folder=${JSON.stringify(reopened.folder)} untranslated=${reopened.untranslated}`,
    )
    assert.strictEqual(
      Number((reopened.badge.match(/(\d+)/) ?? [])[1]),
      st.sourceRows,
      'row count differs after restart — data loss or duplication',
    )
    // OBSERVATION: target locale persistence across restart
    if (reopened.selectValue !== 'ja') {
      console.log(
        `[r4] §4b FINDING-CANDIDATE: target locale reset to '${reopened.selectValue}' after restart (expected 'ja' to persist)`,
      )
      await setTargetJa()
      const after = await js(() => ({
        folder: document.querySelector('.tree-bottom code')?.textContent ?? '',
        untranslated: document.querySelectorAll('.row-target.untranslated').length,
      }))
      console.log(`[r4] §4b after re-switch: folder=${JSON.stringify(after.folder)} untranslated=${after.untranslated}`)
    }
    // markers + imported data intact
    for (const e of st.edited) {
      const shown = await selectKeyExact(e.key)
      assert.strictEqual(shown, e.key, `key ${e.key} missing after restart`)
      const val = await editorValue()
      assert.strictEqual(val, e.value, `edit lost after restart: ${e.key} = ${JSON.stringify(val)}`)
      console.log(`[r4] §4b PERSIST ${e.key} = ${JSON.stringify(val)}`)
    }
    await browser.$('.entry-toolbar .search-field input').setValue('')
    const spotKey = 'HugsLib_settings_resetValue'
    await selectKeyExact(spotKey)
    const spotVal = await editorValue()
    console.log(`[r4] §4b SPOTCHECK ${spotKey} = ${JSON.stringify(spotVal)} (imported)`)
    assert.strictEqual(spotVal, '設定を初期化する', 'imported translation lost after restart')
    const fin = await js(() => ({
      badge: document.querySelector('.tree-root span')?.textContent ?? '',
      untranslated: document.querySelectorAll('.row-target.untranslated').length,
    }))
    assert.strictEqual(Number((fin.badge.match(/(\d+)/) ?? [])[1]), st.sourceRows, 'dup rows after restart')
    assert.strictEqual(fin.untranslated, 0, 'imported translations lost after restart')
    console.log('[r4] §4b persist: edits + imports intact, no duplicates')
  })

  it('§5 checks: broken placeholder → finding → fix → clean; report export', async () => {
    const st = JSON.parse(fs.readFileSync(STATE_PATH, 'utf8'))
    await js(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]')
    await pause(500)
    const shown = await selectKeyExact(PH_KEY)
    assert.strictEqual(shown, PH_KEY, 'placeholder key not found')
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('R4BROKEN Mod設定')
    await browser.$('[data-testid="ws.editor-save-next"]').click()
    await browser.waitUntil(async () => (await editorValue()) === 'R4BROKEN Mod設定', {
      timeout: 30000,
      interval: 400,
    })
    console.log(`[r4] §5 broke ${PH_KEY} → "R4BROKEN Mod設定" ({0} dropped)`)
    await js(() => {
      window.location.hash = '#/checks'
    })
    await waitExisting('[data-testid="checks.findings"]', 60000)
    await browser.waitUntil(
      async () =>
        (await browser.execute(
          () => document.querySelectorAll('[data-testid="checks.findings"] .finding-row').length,
        )) > 0,
      { timeout: 60000, interval: 300 },
    )
    await pause(400)
    const broken = await js(() => ({
      band: document.querySelector('.metrics-band')?.textContent ?? '',
      findings: Array.from(document.querySelectorAll('[data-testid="checks.findings"] .finding-row')).map(
        (e) => e.textContent?.slice(0, 160) ?? '',
      ),
    }))
    console.log(`[r4] §5 BROKEN band=${JSON.stringify(broken.band)}`)
    for (const f of broken.findings) console.log(`[r4] §5 finding: ${JSON.stringify(f)}`)
    const onKey = broken.findings.filter((f) => f.includes(PH_KEY))
    assert(onKey.length > 0, `no finding references the broken key ${PH_KEY}`)
    st.brokenBand = broken.band
    fs.writeFileSync(STATE_PATH, JSON.stringify(st, null, 1))

    // fix back to the exact imported value
    await js(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]')
    await pause(500)
    const shown2 = await selectKeyExact(PH_KEY)
    assert.strictEqual(shown2, PH_KEY)
    await browser.$('[data-testid="ws.editor-textarea"]').setValue(PH_JA)
    await browser.$('[data-testid="ws.editor-save-next"]').click()
    await browser.waitUntil(async () => (await editorValue()) === PH_JA, {
      timeout: 30000,
      interval: 400,
    })
    console.log(`[r4] §5 fixed ${PH_KEY} → ${JSON.stringify(PH_JA)}`)
    await js(() => {
      window.location.hash = '#/checks'
    })
    await waitExisting('[data-testid="checks.findings"]', 60000)
    // the screen auto-validates on mount; wait until errors are 0 again
    await browser.waitUntil(
      async () =>
        (await browser.execute(() => document.querySelector('.metrics-band')?.textContent ?? ''))
          .length > 0 &&
        (await browser.execute(
          () => !document.querySelector('.metrics-band')?.textContent?.match(/[1-9]\d*ошибки/),
        )),
      { timeout: 60000, interval: 300 },
    )
    await pause(400)
    const fixed = await js(() => ({
      band: document.querySelector('.metrics-band')?.textContent ?? '',
      hasKeyFinding: document.body.innerText.includes(PH_KEY),
    }))
    console.log(`[r4] §5 FIXED band=${JSON.stringify(fixed.band)} keyStillMentioned=${fixed.hasKeyFinding}`)
    assert(!/[1-9]\d*ошибки/.test(fixed.band), `errors remain after fix: ${fixed.band}`)
    // support: export the report (download button)
    const hadReport = await browser.execute(() => {
      const btn = document.querySelector('.section-tabs button.ml-auto') as HTMLButtonElement | null
      if (!btn) return false
      btn.click()
      return true
    })
    await pause(1500)
    const candidates = ['/tmp/rimloc-validation.json', `${(process.env.HOME ?? '')}/Downloads/rimloc-validation.json`, '/tmp/r4-e2e/rimloc-validation.json']
    const landed = candidates.filter((p) => fs.existsSync(p))
    console.log(`[r4] §5 report export: clicked=${hadReport} landedOnDisk=${JSON.stringify(landed)}`)
    st.fixedBand = fixed.band
    fs.writeFileSync(STATE_PATH, JSON.stringify(st, null, 1))
  })

  it('§6 build to /tmp/r4-e2e/out → disk structure, keys, markers, idempotent rebuild', async () => {
    const st = JSON.parse(fs.readFileSync(STATE_PATH, 'utf8'))
    fs.rmSync(OUT_DIR, { recursive: true, force: true })
    fs.mkdirSync(OUT_DIR, { recursive: true })
    await js(() => {
      window.location.hash = '#/export'
    })
    await waitExisting('[data-testid="be.outdir"]')
    await browser.$('[data-testid="be.outdir"]').setValue(OUT_DIR)
    await browser.$('[data-testid="be.build"]').click()
    await browser.waitUntil(
      async () =>
        (await browser.execute(
          () =>
            Boolean(document.querySelector('[data-testid="be.result"] .context-content')) ||
            Boolean(document.querySelector('[data-testid="be.error"]')),
        )) as boolean,
      { timeout: 120000, interval: 400 },
    )
    const res = await js(() => ({
      result: document.querySelector('[data-testid="be.result"]')?.textContent ?? '',
      error: document.querySelector('[data-testid="be.error"]')?.textContent ?? '',
    }))
    console.log(`[r4] §6 BUILD result=${JSON.stringify(res.result)} error=${JSON.stringify(res.error)}`)
    assert(!res.error, `build errored: ${res.error}`)
    assert(fs.existsSync(OUT_DIR), 'out dir missing after build')
    const files1 = shaTree(OUT_DIR)
    const relFiles = Object.keys(files1)
    console.log(`[r4] §6 files(${relFiles.length}): ${JSON.stringify(relFiles.slice(0, 20))}`)
    assert(relFiles.length > 0, 'build wrote no files')
    // structure: Languages/Japanese somewhere under out
    const jaFiles = relFiles.filter((f) => f.includes('Languages/Japanese'))
    assert(jaFiles.length > 0, `no Languages/Japanese in build: ${JSON.stringify(relFiles)}`)
    console.log(`[r4] §6 JA files: ${JSON.stringify(jaFiles)}`)
    // keys: parse XML leaf counts; markers present; no source files exported
    let markerHits = 0
    for (const rel of jaFiles) {
      const content = fs.readFileSync(path.join(OUT_DIR, rel), 'utf8')
      for (let i = 1; i <= 3; i++) if (content.includes(`[R4-${i}]`)) markerHits++
    }
    console.log(`[r4] §6 marker hits in build: ${markerHits}/3`)
    assert.strictEqual(markerHits, 3, 'not all [R4-*] markers present in build')
    const sourceLeak = relFiles.filter((f) => /About|\.csproj|Defs\//.test(f) && !f.includes('Languages'))
    console.log(`[r4] §6 source-leak check: ${JSON.stringify(sourceLeak)}`)
    // idempotent rebuild — detect completion by mtime change (content may be
    // identical), then compare content trees
    const mtime1: Record<string, number> = {}
    for (const rel of relFiles) mtime1[rel] = fs.statSync(path.join(OUT_DIR, rel)).mtimeMs
    await browser.$('[data-testid="be.build"]').click()
    let rewriteObserved = true
    try {
      await browser.waitUntil(
        async () => {
          for (const rel of Object.keys(mtime1)) {
            const p = path.join(OUT_DIR, rel)
            if (fs.existsSync(p) && fs.statSync(p).mtimeMs > mtime1[rel]) return true
          }
          return Object.keys(shaTree(OUT_DIR)).some((f) => !(f in files1))
        },
        { timeout: 120000, interval: 1000 },
      )
    } catch {
      rewriteObserved = false
      console.log('[r4] §6 no mtime change observed (build may skip identical files)')
    }
    const files2 = shaTree(OUT_DIR)
    const added = Object.keys(files2).filter((f) => !(f in files1))
    const removed = Object.keys(files1).filter((f) => !(f in files2))
    const changed = Object.keys(files1).filter((f) => f in files2 && files1[f] !== files2[f])
    console.log(
      `[r4] §6 REBUILD rewriteObserved=${rewriteObserved} diff: added=${JSON.stringify(added)} removed=${JSON.stringify(removed)} changed=${JSON.stringify(changed)}`,
    )
    assert.strictEqual(added.length + removed.length + changed.length, 0, 'rebuild is not idempotent')
    st.build = { files: relFiles.length, jaFiles, changedOnRebuild: 0 }
    fs.writeFileSync(STATE_PATH, JSON.stringify(st, null, 1))
  })

  it('§7 chat-batch mini: 2 strings → prompt → manual answer → apply', async () => {
    const st = JSON.parse(fs.readFileSync(STATE_PATH, 'utf8'))
    await js(() => {
      window.location.hash = '#/chatbatch'
    })
    await waitExisting('[data-testid="cb.entries"]')
    await pause(500)
    const pool0 = await js(() => ({
      entries: document.querySelectorAll('[data-testid^="cb.entry."]').length,
      badge: document.body.innerText.match(/(\d+)\s*(?:строк|кандидат)/i)?.[0] ?? '',
    }))
    console.log(`[r4] §7 untranslated-only pool: ${pool0.entries} rows (${pool0.badge})`)
    // all strings are translated after §3 — widen the pool
    await browser.$('[data-testid="cb.filter-untranslated"]').click()
    await pause(500)
    const pool = await js(() => document.querySelectorAll('[data-testid^="cb.entry."]').length)
    console.log(`[r4] §7 full pool: ${pool} candidates`)
    assert(pool >= st.sourceRows, `pool ${pool} < project rows ${st.sourceRows}`)
    for (const key of CB_KEYS) {
      const clicked = await browser.execute((k) => {
        const input = Array.from(
          document.querySelectorAll<HTMLInputElement>('[data-testid^="cb.entry."]').values(),
        ).find((i) => (i.getAttribute('data-testid') ?? '').endsWith(`·${k}`))
        if (!input) return false
        if (!input.checked) input.click()
        return true
      }, key)
      assert(clicked, `checkbox for ${key} not found`)
    }
    await pause(400)
    const selCount = await js(
      () => document.querySelector('[data-testid="cb.selection-count"]')?.textContent ?? '',
    )
    console.log(`[r4] §7 selected: ${JSON.stringify(selCount)}`)
    await browser.$('[data-testid="cb.create"]').click()
    await waitExisting('[data-testid="cb.export-section"]')
    await browser.$('[data-testid="cb.export"]').click()
    await waitExisting('[data-testid="cb.prompt"]')
    const prompt = (await browser.$('[data-testid="cb.prompt"]').getValue()) ?? ''
    assert(prompt.includes('Respond with ONLY one line per string'), 'prompt marker missing')
    for (const k of CB_KEYS) assert(prompt.includes(`${k}: `), `prompt lacks key ${k}`)
    console.log(`[r4] §7 prompt head: ${JSON.stringify(prompt.slice(0, 200))}`)
    // manual answer (human-in-the-loop emulation): key: translation lines
    const answer = CB_KEYS.map((k, i) => `${k}: Ручной перевод ${i + 1}`).join('\n')
    await browser.$('[data-testid="cb.paste"]').setValue(answer)
    await browser.$('[data-testid="cb.import"]').click()
    await waitExisting('[data-testid="cb.preview"]', 60000)
    const preview = await js(() => ({
      rows: document.querySelectorAll('[data-testid="cb.preview-row"]').length,
      text: document.querySelector('[data-testid="cb.preview"]')?.textContent?.slice(0, 200) ?? '',
    }))
    console.log(`[r4] §7 preview rows=${preview.rows} head=${JSON.stringify(preview.text)}`)
    assert.strictEqual(preview.rows, 2, 'preview must show exactly 2 rows')
    await browser.$('[data-testid="cb.apply"]').click()
    await waitExisting('[data-testid="cb.done-section"]', 60000)
    const done = await js(
      () => document.querySelector('[data-testid="cb.done-section"]')?.textContent ?? '',
    )
    console.log(`[r4] §7 DONE=${JSON.stringify(done.slice(0, 160))}`)
    // applied strings live in workspace under ja target; no dup rows
    await js(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]')
    await pause(800)
    const ws = await js(() => ({
      badge: document.querySelector('.tree-root span')?.textContent ?? '',
      untranslated: document.querySelectorAll('.row-target.untranslated').length,
    }))
    assert.strictEqual(Number((ws.badge.match(/(\d+)/) ?? [])[1]), st.sourceRows, 'dup rows after chatbatch apply')
    for (let i = 0; i < CB_KEYS.length; i++) {
      const shown = await selectKeyExact(CB_KEYS[i])
      assert.strictEqual(shown, CB_KEYS[i])
      const val = await editorValue()
      console.log(`[r4] §7 applied ${CB_KEYS[i]} = ${JSON.stringify(val)}`)
      assert.strictEqual(val, `Ручной перевод ${i + 1}`, 'chatbatch value not in workspace')
    }
    await browser.$('.entry-toolbar .search-field input').setValue('')
    console.log('[r4] §7 chat-batch cycle complete on real mod')
  })
})
