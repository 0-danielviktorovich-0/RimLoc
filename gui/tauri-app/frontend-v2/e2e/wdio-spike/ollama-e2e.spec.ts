// Directive §4 — real-provider E2E on the packaged build with the LOCAL
// Ollama server (qwen2.5:3b). Everything goes through the REAL surfaces:
// the provider instance is created through the typed IPC (no key needed
// for Ollama), the AI screen's BYO mode picks it, the job runs REAL
// model calls, the preview is reviewed per-row, accept-selected applies
// through the canonical intents, and the durable envelope carries the
// translation. No invented credentials anywhere.
import { mkdirSync, rmSync, writeFileSync, readFileSync, readdirSync, existsSync, statSync } from 'node:fs'

const MOD = '/tmp/rimloc-int/mod'
const MANAGED = '/tmp/rimloc-int/data/managed'
function pause(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

async function waitExisting(selector: string, timeout = 30000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

describe('directive §4 — Ollama real-provider end to end', () => {
  before(async () => {
    // NOTE: no webview-side liveness fetch here — WKWebView blocks
    // plain-http fetches from the app origin; the PRODUCT talks to
    // Ollama from native Rust (reqwest), which this lane exercises
    // through the real job path. A dead server surfaces as the job's
    // typed error.
    // The app process starts BEFORE this hook and holds its provider
    // settings; wiping the data dir here races it. The durable check
    // below selects THIS run's project by the newest envelope mtime
    // instead.
    rmSync(MOD, { recursive: true, force: true })
    mkdirSync(`${MOD}/1.6/Defs`, { recursive: true })
    mkdirSync(`${MOD}/About`, { recursive: true })
    writeFileSync(
      `${MOD}/About/About.xml`,
      '<?xml version="1.0" encoding="utf-8"?><ModMetaData><name>Ollama Fixture</name><author>rimloc-testlab</author><packageId>rimloc.test.ollama</packageId><supportedVersions><li>1.6</li></supportedVersions></ModMetaData>',
    )
    writeFileSync(
      `${MOD}/1.6/Defs/Things.xml`,
      '<Defs><ThingDef><defName>SunLamp</defName><label>sun lamp</label><description>A lamp that imitates sunlight.</description></ThingDef><ThingDef><defName>Cooler</defName><label>cooler</label><description>A device that cools the air.</description></ThingDef></Defs>',
    )
  })

  it('project created through the wizard', async () => {
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

  it('provider instance created through the typed IPC (no key for Ollama)', async () => {
    const res = await browser.executeAsync((done) => {
      const bridge = (window as unknown as { __TAURI__?: { core?: { invoke?: (c: string, a: unknown) => Promise<unknown> } } }).__TAURI__
      bridge?.core
        ?.invoke?.('contract_provider_instance_upsert', {
          request: {
            preset: 'ollama',
            label: 'Local Ollama (E2E)',
            model: 'qwen2.5:3b',
            base_url: 'http://localhost:11434/v1',
            secret: null,
          },
        })
        .then((r) => done({ ok: JSON.stringify(r) }))
        .catch((e: unknown) => done({ refused: String(e) }))
    })
    const r = res as { ok?: string; refused?: string }
    if (!r.ok) throw new Error(`instance upsert refused: ${r.refused}`)
  })

  it('BYO job runs real model calls to preview, accept-selected applies durably', async () => {
    await browser.execute(() => {
      window.location.hash = '#/aitranslate'
    })
    await waitExisting('[data-testid="ai.modes"]', 30000)
    await browser.$('[data-testid="ai.mode.byo"]').click()
    await pause(600)
    // The instance select must list the Ollama instance.
    await waitExisting('[data-testid="ai.instance"]', 15000)
    const options = await browser.execute(() =>
      Array.from(document.querySelectorAll('option')).map((o) => o.textContent ?? ''),
    )
    if (!options.some((t) => t.includes('Local Ollama'))) {
      throw new Error(`BYO instances missing Ollama: ${JSON.stringify(options)}`)
    }
    const run = await waitExisting('[data-testid="ai.run"]', 15000)
    await browser.waitUntil(async () => run.isEnabled(), { timeout: 20000, interval: 500 })
    await run.click()

    // REAL model round-trips: wait for the preview (bounded patience —
    // two labels through a local 3b model).
    await browser.waitUntil(
      async () => {
        const state = await browser.execute(
          () => document.querySelector('[data-testid="ai.job-state"]')?.textContent ?? '',
        )
        return /preview|ready|проверк/i.test(String(state))
      },
      { timeout: 240000, interval: 2000 },
    )
    // At least one translated row appeared from the REAL model.
    let rows: { identity: string; status: string }[] = []
    for (let i = 0; i < 60; i++) {
      rows = (await browser.execute(() =>
        Array.from(document.querySelectorAll('[data-testid="ai.row"]')).map((r) => ({
          identity: (r.querySelector('code')?.textContent ?? '').trim(),
          status: (r.querySelector('.status-label')?.textContent ?? '').trim(),
        })),
      )) as { identity: string; status: string }[]
      if (rows.some((r) => r.status === 'translated')) break
      await pause(1000)
    }
    const translated = rows.filter((r) => r.status === 'translated')
    if (translated.length === 0) throw new Error(`no translated rows from the real model: ${JSON.stringify(rows)}`)

    // Per-row review: select exactly ONE translated row, apply selected.
    const chosen = translated[0]
    const cb = await browser.$(`[data-testid="ai.select.${chosen.identity}"]`)
    if (!(await cb.isExisting())) throw new Error(`selection checkbox missing for ${chosen.identity}`)
    await cb.click()
    await pause(300)
    const applyBtn = await browser.$('[data-testid="ai.accept-selected"]')
    if (!(await applyBtn.isExisting())) throw new Error('apply-selected button missing')
    await applyBtn.click()

    await browser.waitUntil(
      async () => {
        const state = await browser.execute(
          () => document.querySelector('[data-testid="ai.job-state"]')?.textContent ?? '',
        )
        return /applied|применен/i.test(String(state))
      },
      { timeout: 60000, interval: 1000 },
    )

    // Durable truth (persist-before-ack): in the Ollama project's
    // envelope the ONLY Russian translation among the fixture rows is the
    // chosen identity — string matching is not enough (the inventory part
    // legitimately mentions every key), so the envelope is PARSED.
    let appliedKeys: string[] = []
    for (let i = 0; i < 100 && appliedKeys.length === 0; i++) {
      await pause(200)
      if (existsSync(MANAGED)) {
        // THIS run's project = the newest envelope mentioning the fixture.
        let newest = ''
        let newestMtime = 0
        for (const f of readdirSync(MANAGED)) {
          if (!f.endsWith('.rimloc.json')) continue
          const txt = readFileSync(`${MANAGED}/${f}`, 'utf8')
          if (!txt.includes('SunLamp') && !txt.includes('Cooler')) continue
          const m = statSync(`${MANAGED}/${f}`).mtimeMs
          if (m > newestMtime) {
            newestMtime = m
            newest = txt
          }
        }
        if (newest) {
          const env = JSON.parse(newest) as {
            translations?: { source_id?: { key?: string }; locale?: string; text?: string }[]
          }
          for (const t of env.translations ?? []) {
            if (t.locale === 'Russian' && (t.source_id?.key ?? '').match(/^(SunLamp|Cooler)\./)) {
              appliedKeys.push(`${t.source_id?.key}=${t.text ?? ''}`)
            }
          }
        }
      }
    }
    if (appliedKeys.length === 0) throw new Error('the selected translation never reached the durable envelope')
    if (appliedKeys.length > 1) {
      throw new Error(`accept-selected applied more than the chosen row: ${JSON.stringify(appliedKeys)}`)
    }
    if (!(appliedKeys[0].split('=')[1] ?? '').trim()) {
      throw new Error(`the applied translation is empty: ${appliedKeys[0]}`)
    }
  })
})
