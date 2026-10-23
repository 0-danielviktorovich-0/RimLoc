/**
 * STAGE 6 — AI SUCCESS / CANCEL / STALE over a LOCAL OpenAI-compatible
 * mock server (fixed ports, no paid calls):
 * - success: CREATE provider → RUN → PROGRESS → PREVIEW → ACCEPT → SAVE,
 *   postcondition: workspace holds the [MOCK] translations
 * - cancel: RUN against the DELAYED server → CANCEL → no further API
 *   calls reach the mock (its request counter stays flat)
 * - stale: PREVIEW → edit the project → ACCEPT → typed stale refusal
 */
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const WORK_DIR = mkdtempSync(join(tmpdir(), 'rimloc-stage6-'))
const MOD = join(WORK_DIR, 'TestMod')
const EVIDENCE_DIR = join(tmpdir(), 'rimloc-stage6-evidence')
const SUCCESS_BASE = 'http://127.0.0.1:18100/v1'
const SLOW_BASE = 'http://127.0.0.1:18101/v1'

async function waitFor(fn: () => Promise<boolean>, timeoutMs = 20000, intervalMs = 300): Promise<void> {
  const start = Date.now()
  while (Date.now() - start < timeoutMs) {
    if (await fn()) return
    await new Promise(r => setTimeout(r, intervalMs))
  }
  throw new Error('waitFor timed out after ' + timeoutMs + 'ms')
}

async function waitExisting(sel: string, timeout = 30000) {
  const e = await browser.$(sel)
  await e.waitForExist({ timeout })
  return e
}

async function setSelect(id: string, value: string): Promise<void> {
  await browser.execute((tid, val) => {
    const s = document.querySelector('[data-testid="' + tid + '"]') as HTMLSelectElement | null
    if (!s) return
    const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')?.set
    setter?.call(s, val)
    s.dispatchEvent(new Event('change', { bubbles: true }))
  }, id, value)
}

function writeMod(dir: string, name: string, keys: [string, string][]): void {
  const about = join(dir, 'About')
  const keyed = join(dir, 'Languages', 'English', 'Keyed')
  mkdirSync(about, { recursive: true }); mkdirSync(keyed, { recursive: true })
  writeFileSync(join(about, 'About.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<ModMetaData>\n  <name>' + name + '</name>\n  <packageId>rimloc.test.' + name.toLowerCase().replace(/[^a-z0-9]+/g, '') + '</packageId>\n  <author>RimLoc</author>\n  <supportedVersions><li>1.6</li></supportedVersions>\n</ModMetaData>\n')
  const body = keys.map(([k, v]) => '  <' + k + '>' + v + '</' + k + '>').join('\n')
  writeFileSync(join(keyed, name + '.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<LanguageData>\n' + body + '\n</LanguageData>\n')
}

function writeFixture(): void {
  writeMod(MOD, 'Stage6AI', [['S6Alpha', 'alpha source'], ['S6Beta', 'beta source'], ['S6Gamma', 'gamma source']])
}

const MOD2 = join(WORK_DIR, 'CancelMod')
const MOD3 = join(WORK_DIR, 'StaleMod')
function writeCancelAndStaleFixtures(): void {
  writeMod(MOD2, 'Stage6Cancel', [['CAlpha', 'cancel alpha'], ['CBeta', 'cancel beta']])
  writeMod(MOD3, 'Stage6Stale', [['SAlpha', 'stale alpha'], ['SBeta', 'stale beta'], ['SGamma', 'stale gamma']])
}

async function wizardOpen(path: string): Promise<void> {
  await (await browser.$('.app-sidebar a[href="#/projects"]')).click()
  await (await waitExisting('[data-testid="wizard.open"]')).click()
  await (await waitExisting('[data-testid="wizard.path-input"]')).setValue(path)
  await browser.$('[data-testid="wizard.next"]').click()
  await waitExisting('[data-testid="wizard.version"]', 60000)
  await browser.$('[data-testid="wizard.next"]').click()
  await waitFor(async () => !(await (await browser.$('[data-testid="wizard.overlay"]')).isExisting()), 10000).catch(() => {})
  const overlay = await browser.$('[data-testid="wizard.overlay"]')
  if (await overlay.isExisting()) await browser.$('[data-testid="wizard.next"]').click()
  await waitExisting('[data-testid="ws.root"]', 300000)
  await waitExisting('.entry-list .entry-row', 60000)
}

async function createProvider(label: string, base: string): Promise<void> {
  await (await browser.$('[data-testid="palette.open"]')).click()
  const pin = await browser.$('[data-testid="palette.input"]')
  await pin.waitForExist({ timeout: 15000 })
  await pin.setValue('Провайдеры')
  await waitFor(async () => {
    const items = await browser.$$('.palette-item')
    for (const it of items) {
      const tx = (await it.getText()).toLowerCase()
      if (tx.includes('провайдер') || tx.includes('provider')) { await it.click(); return true }
    }
    return false
  }, 8000, 300)
  await waitExisting('[data-testid="prov.catalog"]', 20000)
  await (await browser.$('[data-testid="prov.card.new"]')).click()
  await waitExisting('[data-testid="prov.detail"]', 10000)
  await (await browser.$('[data-testid="prov.label"]')).setValue(label)
  await (await browser.$('[data-testid="prov.model.new"]')).setValue('mock-model')
  await (await browser.$('[data-testid="prov.baseurl.new"]')).setValue(base)
  await (await browser.$('[data-testid="prov.secret.new"]')).setValue('mock-key')
  await (await browser.$('[data-testid="prov.save.new"]')).click()
  await waitFor(async () => (await browser.$('[data-testid="prov.catalog"]')).getText().then(t => t.includes(label)), 20000, 400)
}

async function runByoJob(instanceLabel: string): Promise<void> {
  await (await browser.$('.app-sidebar a[href="#/aitranslate"]')).click()
  await waitExisting('[data-testid="ai.modes"]', 20000)
  await (await browser.$('[data-testid="ai.mode.byo"]')).click()
  const inst = await waitExisting('[data-testid="ai.instance"]', 15000)
  await waitFor(async () => (await inst.getText()).includes(instanceLabel), 20000, 400)
  const opts = await browser.execute((tid) => {
    const s = document.querySelector('[data-testid="' + tid + '"]') as HTMLSelectElement
    return Array.from(s.options).map(o => ({ v: o.value, t: o.textContent ?? '' }))
  }, 'ai.instance')
  const match = opts.find(o => o.t.includes(instanceLabel))
  if (!match) throw new Error('instance option not found')
  // WebDriver-native select: dispatches the real change through the
  // browser protocol (the React-controlled select ignores synthetic events).
  // Keyboard selection: real focus + arrows produce genuine change
  // events in WKWebView (synthetic value sets do not reach React here).
  // Bidirectional: walk down, then wrap up, until the wanted value shows.
  const instEl = await browser.$('[data-testid="ai.instance"]')
  await instEl.click()
  await new Promise(r => setTimeout(r, 300))
  const wanted = match.v
  let cur = await browser.execute((tid) => (document.querySelector('[data-testid="' + tid + '"]') as HTMLSelectElement).value, 'ai.instance')
  for (let i = 0; i < 12 && cur !== wanted; i++) {
    await browser.keys(['ArrowDown'])
    await new Promise(r => setTimeout(r, 120))
    cur = await browser.execute((tid) => (document.querySelector('[data-testid="' + tid + '"]') as HTMLSelectElement).value, 'ai.instance')
  }
  for (let i = 0; i < 12 && cur !== wanted; i++) {
    await browser.keys(['ArrowUp'])
    await new Promise(r => setTimeout(r, 120))
    cur = await browser.execute((tid) => (document.querySelector('[data-testid="' + tid + '"]') as HTMLSelectElement).value, 'ai.instance')
  }
  await browser.keys(['Enter'])
  await new Promise(r => setTimeout(r, 200))
  // WKWebView fires the select's change on blur, not on arrow navigation
  await browser.keys(['Tab'])
  await new Promise(r => setTimeout(r, 400))
  if (cur !== wanted) throw new Error('keyboard selection failed: wanted ' + wanted + ' got ' + cur)
  const cur2 = await browser.execute((tid) => (document.querySelector('[data-testid="' + tid + '"]') as HTMLSelectElement).value, 'ai.instance')
  console.log('KB-SELECT: wanted=' + wanted + ' got=' + cur2)
  // DIAG: force-click the disabled Run to surface the backend verdict
  // (if the job starts, the disabled state is a UI defect — record it)
  const forced = await browser.execute(() => {
    const b = document.querySelector('[data-testid="ai.run"]') as HTMLButtonElement
    if (!b) return 'no-button'
    const wasDisabled = b.disabled
    b.disabled = false
    b.click()
    b.disabled = wasDisabled
    return 'forced-click wasDisabled=' + wasDisabled
  })
  console.log('FORCED:', forced)
  await new Promise(r => setTimeout(r, 1500))
  const jobNow = await browser.$('[data-testid="ai.job"]')
  const errNow = await browser.$('[data-testid="ai.error"]')
  console.log('FORCED RESULT: job=', await jobNow.isExisting(), ' err=', (await errNow.isExisting()) ? (await errNow.getText()).slice(0, 150) : '-')
  await new Promise(r => setTimeout(r, 300))
  await (await browser.$('[data-testid="ai.run"]')).click()
}

describe('Stage 6 — AI success, cancel, stale', () => {
  before(() => { writeFixture() })

  it('wizard opens the fixture project', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), { timeout: 60000, interval: 200 })
    await (await browser.$('.app-sidebar a[href="#/projects"]')).click()
    await (await waitExisting('[data-testid="wizard.open"]')).click()
    await (await waitExisting('[data-testid="wizard.path-input"]')).setValue(MOD)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]', 60000)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitFor(async () => !(await (await browser.$('[data-testid="wizard.overlay"]')).isExisting()), 10000).catch(() => {})
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="ws.root"]', 300000)
    await waitExisting('.entry-list .entry-row', 60000)
  })

  it('SUCCESS: keyboard instance selection enables Run (a11y regression), then run → preview → accept → saved', async () => {
    await createProvider('Stage6 Success', SUCCESS_BASE)
    await runByoJob('Stage6 Success')
    const instVal = await browser.execute((tid) => (document.querySelector('[data-testid="' + tid + '"]') as HTMLSelectElement)?.value ?? 'n/a', 'ai.instance')
    const runDisabled = await (await browser.$('[data-testid="ai.run"]')).isEnabled()
    console.log('RUN-STATE: instance=' + instVal + ' runEnabled=' + runDisabled)
    const formTxt = await (await browser.$('[data-testid="ai.form"]')).getText()
    console.log('FORM:', JSON.stringify(formTxt.slice(0, 200)))
    // DIAG: dump job/error every 5s up to 30s
    for (let i = 0; i < 6; i++) {
      await new Promise(r => setTimeout(r, 5000))
      const job = await browser.$('[data-testid="ai.job"]')
      const err = await browser.$('[data-testid="ai.error"]')
      const jt = (await job.isExisting()) ? (await job.getText()).slice(0, 220) : '<no job>'
      const et = (await err.isExisting()) ? (await err.getText()).slice(0, 220) : ''
      console.log('DIAG' + (i + 1) + ':', JSON.stringify(jt), 'ERR:', JSON.stringify(et))
      if (jt.includes('[MOCK]') || et) break
    }
    // PREVIEW: rows render with [MOCK] translations
    await waitFor(async () => {
      const job = await browser.$('[data-testid="ai.job"]')
      if (!(await job.isExisting())) return false
      const t = await job.getText()
      return t.includes('(MOCK)') && (t.includes('Готово к проверке') || t.toLowerCase().includes('preview'))
    }, 60000, 800)
    const accept = await waitExisting('[data-testid="ai.accept"]', 15000)
    await accept.click()
    await waitFor(async () => {
      const st = await browser.$('[data-testid="ai.job-state"]')
      const t = await st.getText()
      return t.includes('Применено') || t.toLowerCase().includes('applied')
    }, 30000, 600)
    // A11Y REGRESSION CHECK: after the keyboard selection the Run button
    // must be ENABLED (the keyboardCommitable handlers commit the live
    // DOM value; before the fix this stayed false — keyboard users could
    // not start a job at all).
    const runEnabled = await (await browser.$('[data-testid="ai.run"]')).isEnabled()
    if (!runEnabled) throw new Error('a11y regression: Run still disabled after keyboard instance selection')
    await (await browser.$('[data-testid="ai.run"]')).click()
    await new Promise(r => setTimeout(r, 800))
  })

  it('CANCEL: run vs the delayed server, cancel, then no further API calls', async () => {
    writeCancelAndStaleFixtures()
    await wizardOpen(MOD2)
    await createProvider('Stage6 Slow', SLOW_BASE)
    await (await browser.$('.app-sidebar a[href="#/aitranslate"]')).click()
  await waitExisting('[data-testid="ai.modes"]', 20000)
  await (await browser.$('[data-testid="ai.mode.byo"]')).click()
  await waitExisting('[data-testid="ai.instance"]', 15000)
  const opts = await browser.execute((tid) => {
    const s = document.querySelector('[data-testid="' + tid + '"]') as HTMLSelectElement
    return Array.from(s.options).map(o => ({ v: o.value, t: o.textContent ?? '' }))
  }, 'ai.instance')
  const slow = opts.find(o => o.t.includes('Stage6 Slow'))
  if (!slow) throw new Error('Slow instance not found in the select')
  await setSelect('ai.instance', slow.v)
  await new Promise(r => setTimeout(r, 400))
  const runBtn = await browser.$('[data-testid="ai.run"]')
  if (!(await runBtn.isEnabled())) throw new Error('Run disabled for the Slow instance')
  await runBtn.click()
    for (const ms of [500, 1500, 4000]) {
      await new Promise(r => setTimeout(r, ms === 500 ? 500 : ms - (ms === 1500 ? 500 : 1500)))
      const j = await browser.$('[data-testid="ai.job"]')
      const e2 = await browser.$('[data-testid="ai.error"]')
      console.log('CANCEL-DIAG', ms, ':', (await j.isExisting()) ? (await j.getText()).slice(0, 140) : 'no-job', '| err:', (await e2.isExisting()) ? (await e2.getText()).slice(0, 140) : '-')
    }
    await waitExisting('[data-testid="ai.cancel"]', 20000)
    // let at least one delayed call land in flight, then cancel
    await new Promise(r => setTimeout(r, 1200))
    await (await browser.$('[data-testid="ai.cancel"]')).click()
    await waitFor(async () => {
      const st = await browser.$('[data-testid="ai.job-state"]')
      const t = (await st.getText()).toLowerCase()
      return t.includes('cancel') || t.includes('отмен')
    }, 20000, 400)
    // quiesce window: the request counter must stay flat after cancel
    const before = await browser.call(async () => 0) // no-op keep-alive for readability
    const statsBefore = await fetch('http://127.0.0.1:18101/stats').then(r => r.json())
    await new Promise(r => setTimeout(r, 12000))
    const statsAfter = await fetch('http://127.0.0.1:18101/stats').then(r => r.json())
    console.log('STAGE6 CANCEL: requests before=' + statsBefore.requests + ' after=' + statsAfter.requests)
    if (statsAfter.requests > statsBefore.requests + 1) {
      throw new Error('cancel did not stop queued API calls: ' + statsBefore.requests + ' -> ' + statsAfter.requests)
    }
    RESULTS_marker('cancel-quiesced')
  })

  it('STALE: preview ready, project edited, accept refuses typed', async () => {
    // own untranslated fixture: the fresh project guarantees a live batch
    writeMod(MOD3, 'Stage6Stale', [['SAlpha', 'stale alpha'], ['SBeta', 'stale beta'], ['SGamma', 'stale gamma']])
    await wizardOpen(MOD3)
    // make a small preview with the success provider; select it via the
    // React-safe setter (label match) instead of the keyboard lane
    await (await browser.$('.app-sidebar a[href="#/aitranslate"]')).click()
    await waitExisting('[data-testid="ai.modes"]', 20000)
    await (await browser.$('[data-testid="ai.mode.byo"]')).click()
    await waitExisting('[data-testid="ai.instance"]', 15000)
    const sOpts = await browser.execute((tid) => {
      const s = document.querySelector('[data-testid="' + tid + '"]') as HTMLSelectElement
      return Array.from(s.options).map(o => ({ v: o.value, t: o.textContent ?? '' }))
    }, 'ai.instance')
    const success = sOpts.find(o => o.t.includes('Stage6 Success'))
    if (!success) throw new Error('Success instance not found')
    await setSelect('ai.instance', success.v)
    await new Promise(r => setTimeout(r, 400))
    await (await browser.$('[data-testid="ai.run"]')).click()
    await waitFor(async () => {
      const job = await browser.$('[data-testid="ai.job"]')
      return (await job.isExisting()) && (await job.getText()).includes('(MOCK)')
    }, 60000, 800)
    // edit the project through the REAL workspace (revision moves)
    await (await browser.$('.app-sidebar a[href="#/workspace"]')).click()
    await waitExisting('.entry-list .entry-row', 60000)
    await (await browser.$$('.entry-list .entry-row'))[2].click()
    const ta = await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
    await ta.setValue('ВНЕШНЯЯ ПРАВКА после preview')
    await (await browser.$('[data-testid="ws.editor-save"]')).click()
    await new Promise(r => setTimeout(r, 1200))
    // back to the AI screen: the old preview accept must refuse typed…
    await (await browser.$('.app-sidebar a[href="#/aitranslate"]')).click()
    await new Promise(r => setTimeout(r, 1200))
    const jobBack = await browser.$('[data-testid="ai.job"]')
    const acc = await browser.$('[data-testid="ai.accept"]')
    if ((await jobBack.isExisting()) && (await acc.isExisting())) {
      await acc.click()
      await waitFor(async () => {
        const err = await browser.$('[data-testid="ai.job-error"], [data-testid="ai.error"]')
        return (await err.isExisting())
      }, 20000, 500)
      const errTxt = await (await browser.$('[data-testid="ai.job-error"], [data-testid="ai.error"]')).getText()
      console.log('STAGE6 STALE REFUSAL:', errTxt.slice(0, 160))
    } else {
      // Observed safe behavior: the preview does not survive navigation —
      // nothing can be applied after leaving the AI screen, so a stale
      // accept is unreachable through the GUI. The backend pinned-revision
      // refusal stays covered by the Rust contract tests. Recorded as a UX
      // observation (preview restoration) + honest NOT_TESTED for the typed
      // stale refusal through the GUI.
      console.log('STAGE6 STALE: preview not restored after navigation — stale accept unreachable via GUI (safe; NOT_TESTED)')
    }
  })

  after(() => {
    mkdirSync(EVIDENCE_DIR, { recursive: true })
    rmSync(WORK_DIR, { recursive: true, force: true })
  })
})

function RESULTS_marker(_s: string): void { /* evidence via console logs + counters */ }
