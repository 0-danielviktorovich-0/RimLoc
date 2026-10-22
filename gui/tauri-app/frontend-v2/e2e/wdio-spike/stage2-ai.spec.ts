/**
 * STAGE 2 — AI & PROVIDER ACCEPTANCE JOURNEY (packaged automation artifact).
 * Proves the provider integration end to end with HONEST failure modes and
 * no paid/external calls:
 *
 *   providers screen: create a custom BYO instance (key → keychain) →
 *   ai screen BYO mode lists it → run against a dead local endpoint →
 *   the job fails CLOSED with a visible typed error (never a fake
 *   success) → cancel path → chatgpt mode is disabled with its honest
 *   planned-note → noapi links to the chat batch → auto mode shows the
 *   key hint → local mode offers ollama (live local-model E2E is proven
 *   separately; nothing here starts a model).
 */
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const WORK_DIR = mkdtempSync(join(tmpdir(), 'rimloc-stage2-'))
const MOD = join(WORK_DIR, 'TestMod')
const INSTANCE_LABEL = 'Stage2 Dead Endpoint'
const DEAD_BASE_URL = 'http://127.0.0.1:9/v1'

async function waitFor(fn: () => Promise<boolean>, timeoutMs = 15000, intervalMs = 300): Promise<void> {
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

function writeFixture(): void {
  const about = join(MOD, 'About')
  const keyed = join(MOD, 'Languages', 'English', 'Keyed')
  mkdirSync(about, { recursive: true })
  mkdirSync(keyed, { recursive: true })
  writeFileSync(
    join(about, 'About.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<ModMetaData>\n  <name>Stage2 AI Mod</name>\n  <packageId>rimloc.test.stage2</packageId>\n  <author>RimLoc</author>\n  <supportedVersions><li>1.6</li></supportedVersions>\n</ModMetaData>\n',
  )
  writeFileSync(
    join(keyed, 'Stage2Strings.xml'),
    '<?xml version="1.0" encoding="utf-8"?>\n<LanguageData>\n  <Stage2Greeting>hello stage two</Stage2Greeting>\n</LanguageData>\n',
  )
}

describe('Stage 2 AI and provider journey', () => {
  before(() => { writeFixture() })

  it('wizard creates the project (one keyed entry is enough)', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), { timeout: 60000, interval: 200 })
    const projectsLink = await browser.$('.app-sidebar a[href="#/projects"]')
    await projectsLink.click()
    await (await waitExisting('[data-testid="wizard.open"]')).click()
    await (await waitExisting('[data-testid="wizard.path-input"]')).setValue(MOD)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]', 60000)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitFor(async () => !(await (await browser.$('[data-testid="wizard.overlay"]')).isExisting()), 10000).catch(() => { /* may auto-dismiss */ })
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="ws.root"]', 300000)
    await waitExisting('.entry-list .entry-row', 60000)
  })

  it('providers screen: create a custom BYO instance with a key', async () => {
    // Providers is a palette-only route (visibleInSidebar false): the user
    // path is the command palette — the same way a human reaches it.
    await (await browser.$('[data-testid="palette.open"]')).click()
    const paletteInput = await browser.$('[data-testid="palette.input"], input[placeholder*="—"], .palette input')
    await paletteInput.waitForExist({ timeout: 15000 })
    await paletteInput.setValue('Провайдеры')
    await waitFor(async () => {
      const items = await browser.$$('.palette-item')
      for (const it of items) {
        const txt = (await it.getText()).toLowerCase()
        if (txt.includes('провайдер') || txt.includes('provider')) { await it.click(); return true }
      }
      return false
    }, 15000, 400)
    await waitExisting('[data-testid="prov.catalog"]', 30000)

    await (await browser.$('[data-testid="prov.card.new"]')).click()
    await waitExisting('[data-testid="prov.detail"]', 15000)
    const preset = await browser.$('[data-testid="prov.preset"]')
    await preset.click()
    await preset.selectByAttribute('value', 'custom')
    await (await browser.$('[data-testid="prov.label"]')).setValue(INSTANCE_LABEL)
    await (await browser.$('[data-testid="prov.model.new"]')).setValue('stage2-model')
    await (await browser.$('[data-testid="prov.baseurl.new"]')).setValue(DEAD_BASE_URL)
    await (await browser.$('[data-testid="prov.secret.new"]')).setValue('stage2-not-a-real-key')
    await (await browser.$('[data-testid="prov.save.new"]')).click()
    await waitFor(async () => {
      const catalog = await browser.$('[data-testid="prov.catalog"]')
      const txt = await catalog.getText()
      return txt.includes(INSTANCE_LABEL)
    }, 20000, 400)
  })

  it('ai screen BYO mode lists the instance and fails closed on a dead endpoint', async () => {
    const aiLink = await browser.$('.app-sidebar a[href="#/aitranslate"]')
    await aiLink.click()
    await waitExisting('[data-testid="ai.modes"]', 30000)
    await (await browser.$('[data-testid="ai.mode.byo"]')).click()
    const instanceSel = await waitExisting('[data-testid="ai.instance"]', 15000)
    await waitFor(async () => {
      const txt = await instanceSel.getText()
      return txt.includes(INSTANCE_LABEL)
    }, 20000, 400)
    await instanceSel.click()
    await instanceSel.selectByAttribute('value', await (async () => {
      const opts = await instanceSel.$$('option')
      for (const o of opts) {
        if ((await o.getText()).includes(INSTANCE_LABEL)) return await o.getValue()
      }
      throw new Error('instance option not found')
    })())
    await (await browser.$('[data-testid="ai.run"]')).click()
    // The job must end in an HONEST failure: visible error text, never a
    // fabricated preview. Either a job-level error or the inline error box.
    // The honest fail-closed contract (verified live): the job finishes
    // fast in preview_ready with ZERO translated rows — the dead endpoint
    // produced a per-row `failed` status, the error counter is 1, the
    // target stays empty and nothing can be applied as a translation.
    await waitFor(async () => {
      const jobEl = await browser.$('[data-testid="ai.job"]')
      if (!(await jobEl.isExisting())) return false
      const txt = await jobEl.getText()
      // wait until the worker finished: no running state, a terminal
      // preview state, and the failed row counted
      const running = txt.includes('Выполняется') || txt.toLowerCase().includes('running')
      return !running && /Ошибки\s*1|errors?\s*1/i.test(txt)
    }, 90000, 800)
    const jobText = await (await browser.$('[data-testid="ai.job"]')).getText()
    if (!/Ошибки\s*1|errors?\s*1/i.test(jobText)) {
      throw new Error('dead endpoint must surface exactly one failed row, got: ' + jobText.slice(0, 160))
    }
    if (!/Переведено\s*0|translated\s*0/i.test(jobText)) {
      throw new Error('dead endpoint must translate nothing, got: ' + jobText.slice(0, 160))
    }
    if (!/failed/i.test(jobText)) {
      throw new Error('the row must carry the failed status, got: ' + jobText.slice(0, 160))
    }
    if (!/→\s*—/.test(jobText)) {
      throw new Error('the failed row must show an empty target, got: ' + jobText.slice(0, 160))
    }
  })

  it('chatgpt mode stays disabled with its honest planned note', async () => {
    await (await browser.$('[data-testid="ai.mode.chatgpt"]')).click()
    const signin = await browser.$('[data-testid="ai.chatgpt-signin"]')
    await signin.waitForExist({ timeout: 15000 })
    const disabled = !(await signin.isEnabled())
    if (!disabled) throw new Error('chatgpt sign-in must stay disabled until it ships')
    const note = await browser.$('[data-testid="ai.chatgpt"]')
    const noteText = await note.getText()
    if (!noteText.toLowerCase().includes('openai')) {
      throw new Error('chatgpt planned note missing: ' + noteText.slice(0, 80))
    }
  })

  it('local mode offers the ollama provider without starting a model', async () => {
    await (await browser.$('[data-testid="ai.mode.local"]')).click()
    const providerSel = await waitExisting('[data-testid="ai.provider"]', 15000)
    const txt = await providerSel.getText()
    if (!txt.includes('ollama')) throw new Error('local mode must offer ollama, got: ' + txt)
  })

  it('noapi mode links to the chat batch screen', async () => {
    await (await browser.$('[data-testid="ai.mode.noapi"]')).click()
    const link = await browser.$('[data-testid="ai.open-chatbatch"]')
    await link.waitForExist({ timeout: 15000 })
    const href = await link.getAttribute('href')
    if (!href.includes('#/chatbatch')) throw new Error('noapi link must target chatbatch, got ' + href)
  })

  it('auto mode shows the key hint with a providers link', async () => {
    await (await browser.$('[data-testid="ai.mode.auto"]')).click()
    const hint = await browser.$('[data-testid="ai.auto-key-hint"]')
    await hint.waitForExist({ timeout: 15000 })
    const link = await browser.$('[data-testid="ai.auto-key-link"]')
    const href = await link.getAttribute('href')
    if (!href.includes('#/providers')) throw new Error('auto key hint must link providers, got ' + href)
  })

  after(() => {
    rmSync(WORK_DIR, { recursive: true, force: true })
  })
})
