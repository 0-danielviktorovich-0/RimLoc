// rel22 screens: 9 frames x light+dark. Navigation through the live WDIO
// session; capture via browser.takeScreenshot() — the WebDriver PAGE image.
// The app window lives on a hidden Space (noactivate workflow), so any
// desktop-level capture (region or window id) either photographs the OWNER's
// windows or returns black. The page screenshot is exactly the app UI with
// zero desktop leakage and zero activations.
import { writeFileSync, mkdirSync } from 'node:fs'

const SHOTS = '/Users/danielviktorovich/Developing/RimLoc-evidence/artifact-rel22-rc/screens'

async function capture(file: string): Promise<void> {
  const b64 = await browser.takeScreenshot()
  writeFileSync(file, Buffer.from(b64, 'base64'))
}

type Screen = { name: string; hash: string; marker: string; prep?: 'wizard' | null }

const SCREENS: Screen[] = [
  { name: '01-home', hash: '#/home', marker: '[data-testid="wizard.open"]' },
  { name: '02-new-project', hash: '#/home', marker: '[data-testid="wizard.overlay"]', prep: 'wizard' },
  { name: '03-workspace', hash: '#/workspace', marker: '[data-testid="ws.root"]' },
  { name: '04-checks', hash: '#/checks', marker: '[data-testid="checks.rerun"], [data-testid="checks.findings"]' },
  { name: '05-glossary', hash: '#/glossary', marker: '[data-testid="gl.table"]' },
  { name: '06-tm', hash: '#/tm', marker: '[data-testid="tm.count"]' },
  { name: '07-chatbatch', hash: '#/chatbatch', marker: '[data-testid="cb.create"]' },
  { name: '08-build', hash: '#/export', marker: '[data-testid="be.outdir"]' },
  { name: '09-settings', hash: '#/settings', marker: '[data-testid="settings.theme"]' },
]

async function waitExisting(sel: string, timeout = 20000): Promise<void> {
  await browser.waitUntil(
    async () => (await browser.$(sel)).isExisting(),
    { timeout, interval: 250 },
  )
}

async function goTo(s: Screen): Promise<void> {
  if (s.prep === 'wizard') {
    await browser.execute(() => { window.location.hash = '#/home' })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.overlay"]')
    return
  }
  await browser.execute((h) => { window.location.hash = h as string }, s.hash)
  await waitExisting(s.marker)
  await new Promise((r) => setTimeout(r, 700))
}

describe('rel22 screens: 9 frames x light+dark', () => {
  before(async () => {
    mkdirSync(SHOTS, { recursive: true })
    await waitExisting('.app-sidebar', 30000)
    // Same self-seed as palette-acceptance: a fresh app instance does not
    // auto-reopen the persisted project; workspace/TM/chatbatch frames need
    // one open. Idempotent — skips when ws.root is already there.
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
      console.log('[rel22-screens] project seeded')
    }
  })

  it('dark pass', async () => {
    await browser.execute(() => { window.location.hash = '#/settings' })
    await waitExisting('[data-testid="settings.theme"]')
    // Опции селекта локализованы; выбираем по значению (dark/light), не по тексту.
    await browser.execute(() => {
      const sel = document.querySelector('[data-testid="settings.theme"]') as HTMLSelectElement | null
      if (sel) {
        sel.value = 'dark'
        sel.dispatchEvent(new Event('change', { bubbles: true }))
      }
    })
    await new Promise((r) => setTimeout(r, 800))
    for (const s of SCREENS) {
      await goTo(s)
      capture(`${SHOTS}/${s.name}-dark.png`)
    }
  })

  it('light pass', async () => {
    await browser.execute(() => { window.location.hash = '#/settings' })
    await waitExisting('[data-testid="settings.theme"]')
    await browser.execute(() => {
      const sel = document.querySelector('[data-testid="settings.theme"]') as HTMLSelectElement | null
      if (sel) {
        sel.value = 'light'
        sel.dispatchEvent(new Event('change', { bubbles: true }))
      }
    })
    await new Promise((r) => setTimeout(r, 800))
    for (const s of SCREENS) {
      await goTo(s)
      capture(`${SHOTS}/${s.name}-light.png`)
    }
  })
})
