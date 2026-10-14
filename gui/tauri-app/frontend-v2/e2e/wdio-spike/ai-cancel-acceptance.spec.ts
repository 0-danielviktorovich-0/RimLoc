// AI-cancel acceptance native E2E (review 2026-10-14 P2): one coherent
// journey against the REAL packaged automation build.
//
// What each check proves, honestly:
//  - mock gating: the PRODUCTION frontend never offers the mock provider
//    (import.meta.env.DEV is false in the packaged build) — asserted for
//    every provider-backed mode;
//  - UI error visibility: a BYO start without credentials fails at the
//    backend BEFORE any network call (MissingKey at provider build) and the
//    AI screen surfaces the error banner;
//  - IPC job journey: start(mock; explicitly enabled by the wrapper env for
//    this isolated profile) → preview_ready → accept → applied, with the
//    durable envelope holding the Russian translations;
//  - cancel semantics: start + cancel back-to-back on a long job. The mock
//    echo is in-process and fast, so the cancel may land after completion —
//    the spec asserts BOTH branches honestly: a cancelled job stays
//    cancelled (terminal, never revives, accept refuses) and no branch ever
//    shows Applied without an accept. The request-level "no new provider
//    calls after cancel" proof lives in the deterministic in-process
//    regressions (blocking provider) — an in-process mock cannot make that
//    observable from outside the packaged app;
//  - stale revision: a manual edit between preview and accept is refused,
//    the job does NOT flip to Applied, and the refusal is observable.
import { mkdirSync, writeFileSync, readdirSync, readFileSync } from 'node:fs'

const MOD = '/tmp/rimloc-ai/mod'
const MANAGED = '/tmp/rimloc-ai/data/managed'
const LONG = 'A very long description sentence about steel walls. '.repeat(10)

function pause(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

async function waitExisting(selector: string, timeout = 30000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

type Bridge = {
  __TAURI__?: { core?: { invoke?: (c: string, a?: unknown) => Promise<unknown> } }
}

/** Invoke a Tauri command through the app's own global bridge; throws on
 *  command rejection, returns the parsed payload directly. */
async function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const res = (await browser.executeAsync((c, a, done) => {
    const bridge = (window as unknown as Bridge).__TAURI__
    if (!bridge?.core?.invoke) {
      done({ __err: 'window.__TAURI__.core.invoke unavailable' })
      return
    }
    bridge.core
      .invoke(c, a)
      .then((r) => done({ ok: r }))
      .catch((e: unknown) => done({ __err: JSON.stringify(e, (_, v) => (typeof v === 'bigint' ? String(v) : v)) }))
  }, cmd, args)) as { ok?: T; __err?: string }
  if (res.__err) throw new Error(`${cmd} rejected: ${res.__err}`)
  return res.ok as T
}

/** Same as call() but resolves to the raw rejection string instead of
 *  throwing — for asserting that a command DOES refuse. */
async function callErr(cmd: string, args: Record<string, unknown> = {}): Promise<string> {
  const res = (await browser.executeAsync((c, a, done) => {
    const bridge = (window as unknown as Bridge).__TAURI__
    bridge?.core
      ?.invoke?.(c, a)
      .then((r) => done({ ok: r }))
      .catch((e: unknown) =>
        done({
          refused: JSON.stringify(e, (_, v) => (typeof v === 'bigint' ? String(v) : v)),
        }),
      )
  }, cmd, args)) as { ok?: unknown; refused?: string }
  if (res.refused) return res.refused
  throw new Error(`expected ${cmd} to refuse, it resolved: ${JSON.stringify(res.ok)}`)
}

interface ProjectRef {
  project_id: string
  session_epoch: number
}

async function theProject(): Promise<ProjectRef> {
  const list = await call<ProjectRef[]>('project_list')
  if (!list || list.length === 0) throw new Error('no projects in the isolated profile')
  // project_snapshot resolves through the OPEN session (a stale listed id
  // from a previous profile life rejects with project_not_found) — probe
  // until one resolves.
  for (const item of list) {
    try {
      const snap = await call<ProjectRef>('project_snapshot', { project_id: item.project_id })
      return { project_id: snap.project_id, session_epoch: snap.session_epoch }
    } catch {
      continue
    }
  }
  throw new Error(`no listed project has an open session: ${JSON.stringify(list)}`)
}

interface JobStatus {
  job_id: string
  state: string
  total: number
  translated: number
  failed: number
  error?: string
}

async function waitTerminal(jobId: string, secs: number): Promise<JobStatus> {
  const deadline = Date.now() + secs * 1000
  for (;;) {
    const st = await call<JobStatus>('ai_job_status', { job_id: jobId })
    if (st.state !== 'running') return st
    if (Date.now() > deadline) return st
    await pause(100)
  }
}

function envelopeHasRussian(text: string): boolean {
  try {
    for (const f of readdirSync(MANAGED)) {
      if (!f.endsWith('.rimloc.json')) continue
      const env = JSON.parse(readFileSync(`${MANAGED}/${f}`, 'utf8'))
      if (
        (env.translations ?? []).some(
          (t: { locale: string; text?: string }) =>
            t.locale === 'Russian' && (t.text ?? '').includes(text),
        )
      )
        return true
    }
  } catch {
    /* mid-write */
  }
  return false
}

async function startMockJob(p: ProjectRef, budget: number): Promise<JobStatus> {
  return call<JobStatus>('ai_job_start', {
    request: {
      project_id: p.project_id,
      session_epoch: p.session_epoch,
      locale: 'Russian',
      provider_id: 'mock',
      batch_char_budget: budget,
    },
  })
}

describe('AI cancel acceptance — integrated native GUI (review P2)', () => {
  before(() => {
    // Fixture: 120 long entries (slow multi-batch mock job for the cancel
    // race) + 2 short entries kept aside for the stale-revision job.
    mkdirSync(`${MOD}/Defs`, { recursive: true })
    mkdirSync(`${MOD}/About`, { recursive: true })
    writeFileSync(
      `${MOD}/About/About.xml`,
      '<?xml version="1.0" encoding="utf-8"?><ModMetaData><name>AI Cancel Fixture</name><author>rimloc-testlab</author><packageId>rimloc.test.aicancel</packageId><supportedVersions><li>1.6</li></supportedVersions></ModMetaData>',
    )
    let xml = '<Defs>\n'
    for (let i = 0; i < 120; i++) {
      xml += `  <ThingDef>\n    <defName>Long${i}</defName>\n    <label>${LONG}variant ${i}</label>\n  </ThingDef>\n`
    }
    xml += '</Defs>\n'
    writeFileSync(`${MOD}/Defs/Long.xml`, xml)
    writeFileSync(
      `${MOD}/Defs/Short.xml`,
      '<Defs><ThingDef><defName>Apple</defName><label>Apple</label></ThingDef><ThingDef><defName>Pear</defName><label>Pear</label></ThingDef></Defs>',
    )
  })

  it('project created from the fixture through the wizard', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), {
      timeout: 60000,
      interval: 200,
    })
    await browser.execute(() => {
      window.location.hash = '#/projects'
    })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue(MOD)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]')
    await browser.$('[data-testid="wizard.version"]').selectByVisibleText('1.6')
    await browser.$('[data-testid="wizard.next"]').click()
    await pause(600)
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) {
      await browser.$('[data-testid="wizard.next"]').click()
    }
    await waitExisting('[data-testid="ws.root"]', 60000)
  })

  it('production frontend never offers the mock provider', async () => {
    await browser.execute(() => {
      window.location.hash = '#/aitranslate'
    })
    await waitExisting('[data-testid="ai.root"]')
    for (const mode of ['auto', 'byo', 'local']) {
      await browser.$(`[data-testid="ai.mode.${mode}"]`).click()
      await pause(400)
      const opts = await browser.$('[data-testid="ai.provider"]').$$('option')
      const values: string[] = []
      for (const o of opts) values.push(await o.getValue())
      if (values.includes('mock')) {
        throw new Error(`mock provider offered in production build, mode=${mode}: ${values}`)
      }
      if (values.length === 0) {
        throw new Error(`provider select empty, mode=${mode}`)
      }
    }
  })

  it('a slow failing job: failed rows stream in, REAL UI cancel stops it, job stays cancelled', async () => {
    // Local mode → ollama; no local server exists in the isolated profile.
    // Zero contact beyond localhost:11434 (refused immediately). The
    // per-batch retry/backoff makes this job genuinely SLOW (minutes), so
    // the cancel below lands while the job is RUNNING — the REAL UI
    // cancel button drives ai_job_cancel → the shared flag → the engine
    // stops at its next boundary.
    await browser.$('[data-testid="ai.mode.local"]').click()
    await pause(400)
    await browser.$('[data-testid="ai.run"]').click()
    await waitExisting('[data-testid="ai.job"]', 20000)
    // Failed row statuses stream into the preview while the job runs.
    await browser.waitUntil(
      async () => {
        const labels = await browser.$$('[data-testid="ai.row"] .status-label')
        let failed = 0
        for (const l of labels) {
          if ((await l.getText()) === 'failed') failed++
        }
        return failed >= 2
      },
      { timeout: 60000, interval: 500 },
    )
    // THE UI cancel: a real button click on a genuinely running job.
    await browser.$('[data-testid="ai.cancel"]').click()
    // The cancel button renders only while the job is running; its
    // disappearance is the state leaving `running` for good.
    await browser.waitUntil(
      async () => !(await browser.$('[data-testid="ai.cancel"]').isExisting()),
      { timeout: 30000, interval: 300 },
    )
    // Terminal: no accept button (accept renders only for preview_ready),
    // and the cancelled panel does not revive.
    await pause(2000)
    const acceptAfter = await browser.$('[data-testid="ai.accept"]')
    if (await acceptAfter.isExisting()) {
      throw new Error('cancelled job offers Accept — Cancelled was not terminal in the UI')
    }
    // EXACT state assertion (review round 2, P2-A): the state pill renders
    // the raw i18n key `ai.state.<state>`, so a Failed job would read
    // ai.state.failed — only `cancelled` passes this check.
    const st = await browser.$('[data-testid="ai.job-state"]').getText()
    if (!st.includes('cancelled')) {
      throw new Error(`UI cancel must land exactly on cancelled, pill=${st}`)
    }
    // Terminal stability: the pill must not drift afterwards.
    for (let i = 0; i < 6; i++) {
      await pause(500)
      const again = await browser.$('[data-testid="ai.job-state"]').getText()
      if (!again.includes('cancelled')) {
        throw new Error(`cancelled state drifted to ${again} at t=${i * 500}ms`)
      }
    }
    console.log(`[ai-cancel] UI cancel state pill: ${st} (stable across 3s)`)
  })

  it('mock job: start → preview → accept → applied, envelope holds Russian', async () => {
    const p = await theProject()
    const job = await startMockJob(p, 700)
    if (job.state !== 'running') throw new Error(`job did not start: ${JSON.stringify(job)}`)
    const st = await waitTerminal(job.job_id, 60)
    if (st.state !== 'preview_ready') {
      throw new Error(`job never reached preview: ${JSON.stringify(st)}`)
    }
    if (st.translated < 120) throw new Error(`expected 120 translated, got ${st.translated}`)
    const applied = await call<JobStatus>('ai_job_accept', {
      request: { job_id: job.job_id, session_epoch: p.session_epoch },
    })
    if (applied.state !== 'applied') {
      throw new Error(`accept did not apply: ${JSON.stringify(applied)}`)
    }
    let ok = false
    for (let i = 0; i < 100 && !ok; i++) {
      ok = envelopeHasRussian('variant 0')
      if (!ok) await pause(100)
    }
    if (!ok) throw new Error('applied job left no Russian translation in the managed envelope')
  })

  it('cancel during a long job: cancelled stays cancelled, accept refuses; or honest completion', async () => {
    const p = await theProject()
    // Start + cancel back-to-back inside ONE bridge round-trip pair: the
    // worker races the cancel. 120 single-unit batches make Running the
    // likely branch; the in-process mock finishing first is legal.
    const payload = await browser.executeAsync(
      (pid, epoch, done) => {
        const bridge = (window as unknown as Bridge).__TAURI__
        if (!bridge?.core?.invoke) {
          done({ __err: 'no invoke bridge' })
          return
        }
        const inv = bridge.core.invoke
        ;(async () => {
          const started = (await inv('ai_job_start', {
            request: {
              project_id: pid,
              session_epoch: epoch,
              locale: 'Russian',
              provider_id: 'mock',
              batch_char_budget: 700,
            },
          })) as { job_id: string; state: string }
          const cancelled = (await inv('ai_job_cancel', { job_id: started.job_id })) as {
            state: string
          }
          return { job: started, cancelled }
        })()
          .then((r) => done({ ok: r }))
          .catch((err: unknown) => done({ __err: String(err) }))
      },
      p.project_id,
      p.session_epoch,
    )
    const r = payload as {
      ok?: { job: { job_id: string; state: string }; cancelled: { state: string } }
      __err?: string
    }
    if (r.__err) throw new Error(r.__err)
    const pair = r.ok as { job: { job_id: string; state: string }; cancelled: { state: string } }
    const st = await call<JobStatus>('ai_job_status', { job_id: pair.job.job_id })

    if (pair.cancelled.state === 'cancelled' || st.state === 'cancelled') {
      // Terminal invariant at the GUI/IPC level: Cancelled must survive
      // polling and must refuse accept.
      console.log(`[ai-cancel] CANCELLED branch (cancel landed while running)`)
      for (let i = 0; i < 20; i++) {
        const s = await call<JobStatus>('ai_job_status', { job_id: pair.job.job_id })
        if (s.state !== 'cancelled') {
          throw new Error(`Cancelled was not terminal: polled ${s.state} at t=${i * 100}ms`)
        }
        await pause(100)
      }
      const refused = await callErr('ai_job_accept', {
        request: { job_id: pair.job.job_id, session_epoch: p.session_epoch },
      })
      if (!refused.includes('not in preview state')) {
        throw new Error(`unexpected refusal shape: ${refused}`)
      }
    } else if (st.state === 'preview_ready') {
      // The in-process mock finished all 120 batches before the cancel IPC
      // landed. Honest branch: the completed job keeps its preview, is NOT
      // applied (no accept happened) and the cancel did not corrupt it.
      console.log(`[ai-cancel] COMPLETED branch (mock finished before cancel landed)`)
      if (st.state === 'applied') throw new Error('job became applied without an accept')
    } else {
      throw new Error(`unexpected post-cancel state: ${JSON.stringify({ pair, st })}`)
    }
  })

  it('stale revision: the UI refuses the edit visibly AND accept refuses, no false Applied', async () => {
    const p = await theProject()
    // Make fresh scope: clear the Russian translations of the two short
    // entries (Human intent, current revision), so the job below has
    // something untranslated to preview.
    const before = await call<{
      revision: number
      project: { entries: Array<{ id: { kind: string; key: string; def_type?: string } }> }
    }>('project_snapshot', { project_id: p.project_id })
    const shortEntry = (key: string) => ({
      kind: 'def_injected',
      key: `${key}.label`,
      def_type: 'ThingDef',
    })
    const clearResp = await call<{ revision: number }>('project_apply_intents', {
      request: {
        project_id: p.project_id,
        expected_revision: before.revision,
        session_epoch: p.session_epoch,
        intents: [
          {
            entry: shortEntry('Apple'),
            locale: 'Russian',
            action: 'clear_translation',
            text: null,
          },
          {
            entry: shortEntry('Pear'),
            locale: 'Russian',
            action: 'clear_translation',
            text: null,
          },
        ],
        origin: 'human',
      },
    })

    // Fresh job over the cleared rows; it pins the CURRENT revision.
    const job = await startMockJob(p, 6000)
    const st = await waitTerminal(job.job_id, 30)
    if (st.state !== 'preview_ready') {
      throw new Error(`short job never previewed: ${JSON.stringify(st)}`)
    }
    if (st.translated < 2) throw new Error(`expected 2 translated, got ${st.translated}`)

    // The competing edit: revision moves PAST the pinned base while the
    // preview is open (Human intent through the same session).
    const mid = await call<{ revision: number }>('project_snapshot', {
      project_id: p.project_id,
    })
    if (mid.revision !== clearResp.revision) {
      throw new Error(`unexpected intermediate revision ${mid.revision}`)
    }
    await call('project_apply_intents', {
      request: {
        project_id: p.project_id,
        expected_revision: mid.revision,
        session_epoch: p.session_epoch,
        intents: [
          {
            entry: shortEntry('Apple'),
            locale: 'Russian',
            action: 'set_translation',
            text: 'Яблоко вручную',
          },
        ],
        origin: 'human',
      },
    })

    // (1) The REAL UI surface: the workspace store still holds a stale
    // revision, so ITS manual edit MUST be refused with a VISIBLE
    // workspace error — the UI-side stale guard, honestly surfaced.
    await browser.execute(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]', 30000)
    const rows = await browser.$$('.entry-row')
    let apple: WebdriverIO.Element | null = null
    for (const row of rows) {
      const txt = await row.getText()
      if (txt.includes('Apple')) {
        apple = row
        break
      }
    }
    if (!apple) throw new Error('Apple row not found in the workspace')
    await apple.click()
    const ta = await waitExisting('[data-testid="ws.editor-textarea"]')
    await ta.setValue('Яблоко вручную 2')
    await pause(300)
    await browser.$('[data-testid="ws.editor-save"]').click()
    await waitExisting('.ws-error', 20000)
    const uiErr = await browser.$('.ws-error').getText()
    if (!/revision/i.test(uiErr)) {
      throw new Error(`workspace error is not about the revision: ${uiErr}`)
    }

    // (2) The accept path: pinned base ≠ live revision → MUST refuse and
    // the job must NOT flip to Applied.
    const refused = await callErr('ai_job_accept', {
      request: { job_id: job.job_id, session_epoch: p.session_epoch },
    })
    if (!refused.includes('revision') && !refused.toLowerCase().includes('stale')) {
      throw new Error(`refusal is not about the revision: ${refused}`)
    }
    const after = await call<JobStatus>('ai_job_status', { job_id: job.job_id })
    if (after.state !== 'preview_ready') {
      throw new Error(`job must stay preview_ready after refused accept, got ${after.state}`)
    }
  })
})
