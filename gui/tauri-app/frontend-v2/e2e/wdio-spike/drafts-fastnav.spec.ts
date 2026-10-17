// Fast navigation + write failure (review P0-3, native):
//   A. type → immediately open another project → return: the draft survived
//      (the debounced flush is QUIESCED before the flip, not cancelled);
//   B. managed root made read-only → type → the UI must show an honest
//      save ERROR, never a saved state; after permissions are restored the
//      retry persists the work.
import * as fs from 'node:fs'
import { pause } from './ds-helpers'

const MOD1 = '/tmp/rimloc-ds/mod'
const MOD2 = '/tmp/rimloc-ds/mod2'
const MGR = '/tmp/rimloc-ds/data/managed'

function writeMod(root: string, name: string, pkg: string, label: string): void {
  fs.mkdirSync(`${root}/Defs`, { recursive: true })
  fs.mkdirSync(`${root}/About`, { recursive: true })
  fs.writeFileSync(
    `${root}/About/About.xml`,
    `<?xml version="1.0" encoding="utf-8"?><ModMetaData><name>${name}</name><author>rimloc-testlab</author><packageId>${pkg}</packageId><supportedVersions><li>1.6</li></supportedVersions></ModMetaData>`,
  )
  fs.writeFileSync(
    `${root}/Defs/A_Thing.xml`,
    `<Defs><ThingDef><defName>Solo</defName><label>${label}</label></ThingDef></Defs>`,
  )
}

async function waitExisting(selector: string, timeout = 20000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

async function openProjectByName(name: string): Promise<void> {
  await browser.execute(() => { window.location.hash = '#/projects' })
  await waitExisting('[data-testid="projects.project-card"]')
  const cards = await browser.$$('[data-testid="projects.project-card"]')
  let hit: WebdriverIO.Element | null = null
  for (const c of cards) {
    const t = await c.getText()
    if (t.includes(name)) { hit = c; break }
  }
  if (!hit) {
    const texts: string[] = []
    for (const c of cards) texts.push(await c.getText())
    throw new Error(`project card "${name}" not found; cards=${JSON.stringify(texts)}`)
  }
  await hit.click()
  await waitExisting('[data-testid="ws.root"]', 45000)
  await pause(500)
}

describe('draft safety, phase 4 — fast navigation and honest write failure', () => {
  before(() => {
    // The runner wipes the data dir; this leg is SELF-CONTAINED — it
    // creates BOTH projects itself (stale cross-run state made the earlier
    // run open a wrong duplicate project).
    writeMod(MOD1, 'Safety Fixture', 'rimloc.test.safety', 'Safety Thing')
    writeMod(MOD2, 'Fast Nav Fixture', 'rimloc.test.fastnav', 'Solo Thing')
  })

  async function createProject(root: string): Promise<void> {
    await browser.execute(() => { window.location.hash = '#/projects' })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue(root)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]')
    await browser.$('[data-testid="wizard.version"]').selectByVisibleText('1.6')
    await browser.$('[data-testid="wizard.next"]').click()
    await pause(600)
    if (await browser.$('[data-testid="wizard.overlay"]').isExisting()) {
      await browser.$('[data-testid="wizard.next"]').click()
    }
    await waitExisting('[data-testid="ws.root"]', 45000)
    await pause(400)
  }

  it('A. a draft younger than the debounce survives an immediate project switch', async () => {
    await browser.waitUntil(
      async () => (await browser.$('.app-sidebar')).isExisting(),
      { timeout: 30000, interval: 100 },
    )
    // Create project 1, then project 2.
    await createProject(MOD1)
    await createProject(MOD2)

    // Type into the second project, then IMMEDIATELY navigate away (<1.5s).
    await browser.$('[data-testid="ws.entry.0"]').click()
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('быстрый черновик')
    // Cards render the mod FOLDER name, not the About name.
    await openProjectByName('modРевизия') // project 1 — inside the debounce window
    await openProjectByName('mod2') // back to project 2
    const v = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    console.log(`[ds-fastnav] RETURNED_DRAFT=${JSON.stringify(v)}`)
    if (v !== 'быстрый черновик')
      throw new Error(`fast-navigation draft lost: ${JSON.stringify(v)}`)
  })

  it('B. a failing durable write shows an honest error and recovers on retry', async () => {
    // Make the managed root read-only: the sidecar write must FAIL.
    fs.chmodSync(MGR, 0o555)
    try {
      await browser.$('[data-testid="ws.entry.0"]').click()
      await browser.$('[data-testid="ws.editor-textarea"]').setValue('черновик при отказе записи')
      // Wait past the debounce; the sync indicator must report the ERROR.
      await browser.waitUntil(
        async () => {
          const ind = await browser.$('[data-testid="ws.drafts-sync"]')
          if (!ind.isExisting()) return false
          const cls = await ind.getAttribute('class')
          return (cls || '').includes('destructive')
        },
        { timeout: 15000, interval: 300 },
      )
      const cls = await browser.$('[data-testid="ws.drafts-sync"]').getAttribute('class')
      console.log(`[ds-fastnav] SYNC_ERROR_VISIBLE=${JSON.stringify(cls)}`)
    } finally {
      fs.chmodSync(MGR, 0o755)
    }
    // Permissions restored: the Retry affordance persists the work.
    const retry = await browser.$('[data-testid="ws.drafts-retry"]')
    if (await retry.isExisting()) await retry.click()
    await browser.waitUntil(
      async () => {
        const ind = await browser.$('[data-testid="ws.drafts-sync"]')
        const cls = (await ind.isExisting()) ? ((await ind.getAttribute('class')) || '') : ''
        return !cls.includes('destructive')
      },
      { timeout: 15000, interval: 300 },
    )
    // Scan ALL sidecars — several projects have their own stores.
    let vals: string[] = []
    for (const f of fs.readdirSync(MGR).filter((x) => x.includes('.drafts.json'))) {
      const store = JSON.parse(fs.readFileSync(`${MGR}/${f}`, 'utf8'))
      vals.push(...Object.values(store.entries).map((e: { text: string }) => e.text))
    }
    console.log(`[ds-fastnav] RECOVERED=${JSON.stringify(vals)}`)
    if (!vals.includes('черновик при отказе записи'))
      throw new Error('retry did not persist the draft after permissions recovered')
  })
})
