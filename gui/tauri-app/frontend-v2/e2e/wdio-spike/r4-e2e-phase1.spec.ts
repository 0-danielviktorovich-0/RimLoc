// r4-e2e phase 1 (mandate §4, steps 1-4a): REAL HugsLib corpus.
// §1 create project via wizard (real Rust scan)
// §2 Language Manager catalog + user language + target locale switch (ja)
// §3 import existing translation (dry-run → apply → re-import = 0 new)
// §4a edit 3 strings with [R4-*] markers, commit, nav away/back
// State for phase 2 → /tmp/r4-e2e/state.json. TEMPORARY — not committed.
import fs from 'node:fs'
import assert from 'node:assert'

const HUGSLIB =
  '/Users/danielviktorovich/Developing/rimloc-test-corpus/steamcmd-root/steamapps/workshop/content/294100/818773962'
const JA_DIR = `${HUGSLIB}/Languages/Japanese`
const STATE_PATH = '/tmp/r4-e2e/state.json'

const MARKER_KEYS = [
  'HugsLib_settings_btn',
  'HugsLib_settings_resetAll',
  'HugsLib_setting_showNews_label',
]

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
async function text(sel: string): Promise<string> {
  return browser.execute((s) => document.querySelector(s)?.textContent ?? '', sel)
}
/** Exact-key selection: search, then click the row whose key code matches
 *  exactly (substring searches like resetAll/resetAll_prompt need this). */
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
/** Wait until the editor shows `value` for `key` (commit landed). */
async function waitEdited(key: string, value: string, timeout = 25000): Promise<string> {
  await browser.waitUntil(
    async () => (await selectKeyExact(key)) === key &&
      (await browser.execute(
        () => (document.querySelector('[data-testid="ws.editor-textarea"]') as HTMLTextAreaElement)?.value ?? '',
      )) === value,
    { timeout, interval: 600 },
  )
  return browser.execute(
    () => (document.querySelector('[data-testid="ws.editor-textarea"]') as HTMLTextAreaElement)?.value ?? '',
  )
}
async function badgeCount(): Promise<number> {
  const b = await text('.tree-root span')
  const m = b.match(/(\d+)/)
  return m ? Number(m[1]) : -1
}

describe('r4-e2e phase1: HugsLib create → languages → import → edits', () => {
  it('§1 wizard: create project from real HugsLib (Rust scan)', async () => {
    await waitExisting('.app-sidebar', 40000)
    await js(() => {
      window.location.hash = '#/projects'
    })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue(HUGSLIB)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]')
    await browser.$('[data-testid="wizard.version"]').selectByVisibleText('1.6')
    await browser.$('[data-testid="wizard.next"]').click()
    await browser.$('[data-testid="wizard.next"]').click() // step3 → create
    await browser.waitUntil(
      async () => (await browser.execute(() => window.location.hash)) === '#/workspace',
      { timeout: 170_000, interval: 300 },
    )
    await waitExisting('[data-testid="ws.root"]', 30000)
    await pause(800)
    const info = await js(() => ({
      badge: document.querySelector('.tree-root span')?.textContent ?? '',
      kinds: Array.from(document.querySelectorAll('[data-testid^="ws.tree."]')).map(
        (e) => e.textContent?.slice(0, 60) ?? '',
      ),
      eyebrow: document.querySelector('.heading-eyebrow')?.textContent ?? '',
    }))
    console.log(`[r4] §1 BADGE=${info.badge} EYEBROW=${JSON.stringify(info.eyebrow)}`)
    console.log(`[r4] §1 KINDS=${JSON.stringify(info.kinds)}`)
    const total = await badgeCount()
    assert(total > 50, `scan too small for HugsLib: ${total}`)
    assert(
      /English/i.test(info.eyebrow) && /Russian/i.test(info.eyebrow),
      `default target should be Russian, got ${info.eyebrow}`,
    )
    fs.writeFileSync(
      STATE_PATH,
      JSON.stringify({ sourceRows: total, kinds: info.kinds }, null, 1),
    )
  })

  it('§2a Language Manager: catalog check + add Turkish user language', async () => {
    await js(() => {
      window.location.hash = '#/lm'
    })
    await waitExisting('[data-testid="lm.table"]')
    await pause(400)
    const catalog = await js(() => ({
      rows: Array.from(document.querySelectorAll('[data-testid^="lm.row."]')).map(
        (e) => e.getAttribute('data-testid')?.replace('lm.row.', '') ?? '',
      ),
      h2: document.querySelector('.section-heading h2')?.textContent ?? '',
    }))
    console.log(`[r4] §2a LM_H2=${JSON.stringify(catalog.h2)} ROWS=${JSON.stringify(catalog.rows)}`)
    assert(/[а-яА-Я]/.test(catalog.h2), `LM h2 is not RU text: ${catalog.h2}`)
    for (const id of ['en', 'ru', 'uk', 'ja', 'de', 'pl', 'es', 'zh-Hans']) {
      assert(catalog.rows.includes(id), `builtin ${id} missing from catalog`)
    }
    await browser.$('[data-testid="lm.search"]').setValue('Japan')
    await pause(300)
    const filtered = await js(() =>
      Array.from(document.querySelectorAll('[data-testid^="lm.row."]')).map(
        (e) => e.getAttribute('data-testid')?.replace('lm.row.', '') ?? '',
      ),
    )
    console.log(`[r4] §2a SEARCH Japan → ${JSON.stringify(filtered)}`)
    assert.deepStrictEqual(filtered, ['ja'])
    await browser.$('[data-testid="lm.search"]').setValue('')
    await pause(300)
    await browser.$('[data-testid="lm.add-id"]').setValue('tr')
    await browser.$('[data-testid="lm.add-display"]').setValue('Turkish')
    await browser.$('[data-testid="lm.add-native"]').setValue('Türkçe')
    await browser.$('[data-testid="lm.add-submit"]').click()
    await pause(400)
    const after = await js(() =>
      Array.from(document.querySelectorAll('[data-testid^="lm.row."]')).map(
        (e) => e.getAttribute('data-testid')?.replace('lm.row.', '') ?? '',
      ),
    )
    console.log(`[r4] §2a AFTER ADD: ${JSON.stringify(after)}`)
    assert(after.includes('tr'), 'Turkish user language not added')
    assert.strictEqual(after.filter((r) => r === 'tr').length, 1, 'duplicate tr rows in LM')
  })

  it('§2b target locale → Japanese: entries re-map, same unique count', async () => {
    await js(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]')
    await pause(500)
    const before = await js(() => ({
      badge: document.querySelector('.tree-root span')?.textContent ?? '',
      folder: document.querySelector('.tree-bottom code')?.textContent ?? '',
      opts: Array.from(
        document.querySelectorAll('[data-testid="ws.target-locale"] option'),
      ).map((o) => `${o.getAttribute('value')}:${o.textContent}`),
    }))
    console.log(`[r4] §2b BEFORE badge=${before.badge} folder=${JSON.stringify(before.folder)}`)
    console.log(`[r4] §2b LOCALE_OPTS=${JSON.stringify(before.opts)}`)
    // The canonical lane mechanic (react-multitarget): WebDriver option clicks
    // don't drive React selects in WKWebView — set value + change event.
    await browser.execute(() => {
      const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement
      sel.value = 'ja'
      sel.dispatchEvent(new Event('change', { bubbles: true }))
    })
    await pause(800)
    const selNow = await js(() =>
      (document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement)?.value,
    )
    console.log(`[r4] §2b select.value after switch=${JSON.stringify(selNow)}`)
    assert.strictEqual(selNow, 'ja', 'target-locale select did not switch to ja')
    const after = await js(() => ({
      badge: document.querySelector('.tree-root span')?.textContent ?? '',
      folder: document.querySelector('.tree-bottom code')?.textContent ?? '',
      eyebrow: document.querySelector('.heading-eyebrow')?.textContent ?? '',
      untranslated: document.querySelectorAll('.row-target.untranslated').length,
    }))
    console.log(
      `[r4] §2b AFTER badge=${after.badge} folder=${JSON.stringify(after.folder)} eyebrow=${JSON.stringify(after.eyebrow)} untranslated=${after.untranslated}`,
    )
    assert(
      after.folder.includes('Languages/Japanese'),
      `folder label should be Languages/Japanese, got ${after.folder}`,
    )
    const n1 = await badgeCount()
    assert(n1 > 0)
    const st = JSON.parse(fs.readFileSync(STATE_PATH, 'utf8'))
    assert.strictEqual(n1, st.sourceRows, `row count changed on switch: ${n1} vs ${st.sourceRows}`)
    assert.strictEqual(after.untranslated, n1, 'fresh project must be fully untranslated')
    st.targetLocale = 'ja'
    st.rowsAfterSwitch = n1
    fs.writeFileSync(STATE_PATH, JSON.stringify(st, null, 1))
  })

  it('§3a import existing Japanese: dry-run → apply', async () => {
    await js(() => {
      window.location.hash = '#/existing'
    })
    await waitExisting('[data-testid="ex.dir"]')
    await browser.$('[data-testid="ex.dir"]').setValue(JA_DIR)
    await browser.$('[data-testid="ex.analyze"]').click()
    try {
      await waitExisting('[data-testid="ex.metrics"]', 60000)
    } catch (e) {
      const errState = await js(() => ({
        error: document.querySelector('[data-testid="ex.error"]')?.textContent ?? '(no ex.error)',
        body: document.body.innerText.slice(0, 400),
      }))
      console.log(`[r4] §3a ANALYZE FAILED: ${JSON.stringify(errState.error)} BODY=${JSON.stringify(errState.body)}`)
      throw e
    }
    await pause(400)
    const metrics = await js(() => ({
      metrics: document.querySelector('[data-testid="ex.metrics"]')?.textContent ?? '',
      reusableRows: document.querySelectorAll('[data-testid="ex.reusable"] .diff-row').length,
      applyLabel:
        (document.querySelector('[data-testid="ex.apply"]') as HTMLButtonElement)?.textContent ?? '',
      applyDisabled:
        (document.querySelector('[data-testid="ex.apply"]') as HTMLButtonElement)?.disabled,
    }))
    console.log(
      `[r4] §3a METRICS=${JSON.stringify(metrics.metrics)} rowsShown=${metrics.reusableRows} apply=${JSON.stringify(metrics.applyLabel)} disabled=${metrics.applyDisabled}`,
    )
    const reusable = Number(metrics.metrics.match(/(\d+)/)?.[1] ?? 0)
    assert(reusable > 0, 'dry-run reusable should be > 0 for Languages/Japanese')
    assert(!metrics.applyDisabled, 'apply disabled despite reusable > 0')
    await browser.$('[data-testid="ex.apply"]').click()
    await browser.waitUntil(
      async () =>
        (await browser.execute(() => Boolean(document.querySelector('.inline-success')))) as boolean,
      { timeout: 60000, interval: 250 },
    )
    const applied = await text('.inline-success')
    console.log(`[r4] §3a APPLIED=${JSON.stringify(applied)}`)
    const st = JSON.parse(fs.readFileSync(STATE_PATH, 'utf8'))
    st.importMetrics1 = metrics.metrics
    st.applied1 = applied
    fs.writeFileSync(STATE_PATH, JSON.stringify(st, null, 1))
  })

  it('§3b applied visible in workspace; re-import applies 0, no dup rows', async () => {
    await js(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]')
    await pause(1000)
    const ws = await js(() => ({
      badge: document.querySelector('.tree-root span')?.textContent ?? '',
      untranslated: document.querySelectorAll('.row-target.untranslated').length,
    }))
    const st = JSON.parse(fs.readFileSync(STATE_PATH, 'utf8'))
    console.log(
      `[r4] §3b AFTER IMPORT rows=${ws.badge} untranslated=${ws.untranslated} (was ${st.rowsAfterSwitch})`,
    )
    assert(
      Number(ws.untranslated) < st.rowsAfterSwitch,
      'import should translate a part of rows',
    )
    assert.strictEqual(
      Number((ws.badge.match(/(\d+)/) ?? [])[1]),
      st.rowsAfterSwitch,
      'row count grew after import — DUPLICATE',
    )
    st.untranslatedAfterImport = ws.untranslated
    fs.writeFileSync(STATE_PATH, JSON.stringify(st, null, 1))

    // RE-IMPORT the same folder
    await js(() => {
      window.location.hash = '#/existing'
    })
    await waitExisting('[data-testid="ex.dir"]')
    await browser.$('[data-testid="ex.dir"]').setValue(JA_DIR)
    await browser.$('[data-testid="ex.analyze"]').click()
    await waitExisting('[data-testid="ex.metrics"]', 60000)
    await pause(400)
    const metrics2 = await js(() => ({
      metrics: document.querySelector('[data-testid="ex.metrics"]')?.textContent ?? '',
      applyDisabled:
        (document.querySelector('[data-testid="ex.apply"]') as HTMLButtonElement)?.disabled,
    }))
    console.log(`[r4] §3b RE-DRYRUN=${JSON.stringify(metrics2.metrics)} applyDisabled=${metrics2.applyDisabled}`)
    if (metrics2.applyDisabled) {
      console.log('[r4] §3b apply disabled on re-import (0 reusable) — treated as 0 new')
      st.applied2 = 'apply-disabled(0 reusable)'
    } else {
      await browser.$('[data-testid="ex.apply"]').click()
      await browser.waitUntil(
        async () =>
          (await browser.execute(() => Boolean(document.querySelector('.inline-success')))) as boolean,
        { timeout: 60000, interval: 250 },
      )
      const applied2 = await text('.inline-success')
      console.log(`[r4] §3b RE-APPLIED=${JSON.stringify(applied2)}`)
      assert(/0/.test(applied2), `re-import must apply 0 new, got: ${applied2}`)
      st.applied2 = applied2
    }
    st.importMetrics2 = metrics2.metrics
    await js(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]')
    await pause(800)
    const ws2 = await js(() => ({
      badge: document.querySelector('.tree-root span')?.textContent ?? '',
      untranslated: document.querySelectorAll('.row-target.untranslated').length,
    }))
    assert.strictEqual(
      Number((ws2.badge.match(/(\d+)/) ?? [])[1]),
      st.rowsAfterSwitch,
      'rows grew after re-import — DUPLICATE',
    )
    assert.strictEqual(
      ws2.untranslated,
      ws.untranslated,
      `untranslated changed after no-op import: ${ws.untranslated} → ${ws2.untranslated}`,
    )
    fs.writeFileSync(STATE_PATH, JSON.stringify(st, null, 1))
  })

  it('§4a edit 3 strings with markers, commit, nav away/back', async () => {
    const st = JSON.parse(fs.readFileSync(STATE_PATH, 'utf8'))
    const edited: { key: string; value: string }[] = []
    await js(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]')
    await pause(500)
    for (let i = 0; i < MARKER_KEYS.length; i++) {
      const key = MARKER_KEYS[i]
      const marker = `[R4-${i + 1}]`
      const shown = await selectKeyExact(key)
      assert.strictEqual(shown, key, `editor shows ${JSON.stringify(shown)}, expected ${key}`)
      const prevTarget = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
      const next = `${String(prevTarget ?? '')} ${marker}`.trim()
      await browser.$('[data-testid="ws.editor-textarea"]').setValue(next)
      await browser.$('[data-testid="ws.editor-save-next"]').click()
      const landed = await waitEdited(key, next)
      console.log(`[r4] §4a ${key} → ${JSON.stringify(landed)}`)
      assert.strictEqual(landed, next, 'committed value mismatch')
      edited.push({ key, value: next })
    }
    await browser.$('.entry-toolbar .search-field input').setValue('')
    await pause(300)
    // nav away (checks auto-validates) and back
    await js(() => {
      window.location.hash = '#/checks'
    })
    await waitExisting('[data-testid="checks.findings"]', 60000)
    await pause(600)
    const checks1 = await js(() => ({
      band: document.querySelector('.metrics-band')?.textContent ?? '',
    }))
    console.log(`[r4] §4a CHECKS(base)=${JSON.stringify(checks1.band)}`)
    await js(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]')
    await pause(600)
    for (const e of edited) {
      const shown = await selectKeyExact(e.key)
      assert.strictEqual(shown, e.key, `key ${e.key} not selectable after nav`)
      const val = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
      assert.strictEqual(val, e.value, `edit lost after nav: ${e.key} = ${JSON.stringify(val)}`)
    }
    console.log('[r4] §4a nav away/back: all 3 marker edits in place')
    await browser.$('.entry-toolbar .search-field input').setValue('')
    st.edited = edited
    st.checksBase = checks1.band
    fs.writeFileSync(STATE_PATH, JSON.stringify(st, null, 1))
  })
})
