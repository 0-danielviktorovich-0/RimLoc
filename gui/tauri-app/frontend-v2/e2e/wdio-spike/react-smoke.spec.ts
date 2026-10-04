// React R1 lane smoke — REAL backend, ISOLATED data copy. Sections run in
// order: wizard J1 creates the KNOWN fixture project; later sections open
// it deterministically (openWizardProject); J2 creates a FRESH project so
// the existing-pack flow has reusable lines (an already-translated copy
// classifies them as conflicts). wdio v9: element.waitFor is REMOVED —
// use waitExisting().
async function waitExisting(sel: string, timeout = 30000): Promise<void> {
  await browser.waitUntil(async () => (await browser.$(sel)).isExisting(), {
    timeout,
    interval: 100,
  })
}

async function pause(ms: number): Promise<void> {
  await new Promise((r) => setTimeout(r, ms))
}

/** Open the wizard-created project deterministically (its card label
 *  carries the fixture mod folder name). Includes ONE documented recovery:
 *  if the virtualizer measured a zero-height pane during mount, rows are
 *  absent until re-layout — a single re-open, never a retry loop. */
async function openWizardProject(): Promise<void> {
  await browser.$('a[href="#/home"]').click()
  await waitExisting('[data-testid="home.project-card"]')
  await pause(1200) // let listProjects settle → card labels carry names
  for (let attempt = 0; attempt < 2; attempt++) {
    await browser.waitUntil(
      async () => {
        const cards = await browser.$$('[data-testid="home.project-card"]')
        for (const c of cards) {
          if ((await c.getText()).includes('rimloc-wizard-mod')) {
            await c.click()
            return true
          }
        }
        return false
      },
      { timeout: 30000, interval: 250 },
    )
    await waitExisting('[data-testid="ws.root"]')
    await pause(800)
    const rows = await browser.$$('[data-testid^="ws.entry."]')
    if (rows.length > 0) return
    console.log('[smoke] zero rows after open — single re-open (recovery)')
    await browser.$('a[href="#/home"]').click()
    await waitExisting('[data-testid="home.project-card"]')
    await pause(1200)
  }
  throw new Error('wizard project workspace rendered no rows after one recovery')
}

/** Create a FRESH managed project from the fixture mod via the wizard UI.
 *  Fresh = untranslated inventory, so the existing-pack flow has reusable
 *  lines to apply (a previously used copy classifies them as conflicts). */
async function createViaWizard(): Promise<void> {
  await browser.$('a[href="#/home"]').click()
  await waitExisting('[data-testid="wizard.open"]')
  await browser.$('[data-testid="wizard.open"]').click()
  await waitExisting('[data-testid="wizard.path-input"]')
  await browser.$('[data-testid="wizard.path-input"]').setValue('/tmp/rimloc-wizard-mod')
  await browser.$('[data-testid="wizard.next"]').click()
  await waitExisting('[data-testid="wizard.version"]')
  await browser.$('[data-testid="wizard.version"]').selectByVisibleText('1.6')
  await browser.$('[data-testid="wizard.next"]').click()
  await browser.$('[data-testid="wizard.next"]').click()
  await waitExisting('[data-testid="ws.root"]')
  await browser.waitUntil(
    async () => (await browser.$$('[data-testid^="ws.entry."]').length) >= 1,
    { timeout: 60000, interval: 250 },
  )
  const body = await browser.$('[data-testid="ws.root"]').getText()
  if (!body.includes('R1 smoke rifle')) throw new Error('inventory lacks fixture strings')
}

describe('J1 wizard: real create → live inventory', () => {
  it('путь → версия → create → workspace с инвентарём фикстуры', async () => {
    await waitExisting('.app-sidebar')
    const brand = await browser.$('.brand').getText()
    if (!brand.includes('RimLoc')) throw new Error(`brand wrong: ${brand}`)
    await createViaWizard()
  })
})

describe('J3 editor: edit → commit → durable revision', () => {
  it('открывает проект визарда, правит, коммитит, ревизия растёт', async () => {
    await openWizardProject()
    const footerBefore = await browser.$('.ws-footer').getText()
    const revBefore = Number(/rev (\d+)/.exec(footerBefore)?.[1] ?? '-1')
    const rows = await browser.$$('[data-testid^="ws.entry."]')
    await rows[0]!.click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 10000)
    const ta = await browser.$('[data-testid="ws.editor-textarea"]')
    const original = await ta.getValue()
    await ta.setValue(`${original} [R1-smoke]`)
    await browser.$('[data-testid="ws.editor-save-next"]').click()
    await browser.waitUntil(
      async () => {
        const footer = await browser.$('.ws-footer').getText()
        const rev = Number(/rev (\d+)/.exec(footer)?.[1] ?? '-1')
        return rev > revBefore
      },
      { timeout: 30000, interval: 250 },
    )
    await browser.waitUntil(
      async () => (await browser.$('[data-testid="ws.root"]').getText()).includes('[R1-smoke]'),
      { timeout: 15000, interval: 250 },
    )
  })
})

describe('checks: live validator', () => {
  it('отчёт по открытому проекту', async () => {
    await openWizardProject()
    await browser.$('a[href="#/checks"]').click()
    await waitExisting('[data-testid="checks.findings"]')
    await browser.waitUntil(
      async () =>
        (await browser.$('[data-testid="checks.findings"]').getText()).length > 0 ||
        (await browser.$('.passed-state').isExisting()),
      { timeout: 30000, interval: 250 },
    )
    const metrics = await browser.$('.metrics-band').getText()
    if (!/\d/.test(metrics)) throw new Error('metrics band empty')
  })
})

describe('glossary: live CRUD', () => {
  it('добавление термина через project_glossary', async () => {
    await openWizardProject()
    await browser.$('a[href="#/glossary"]').click()
    await waitExisting('[data-testid="gl.add-term"]')
    const stamp = `r1-${Date.now()}`
    await browser.$('[data-testid="gl.add-term"]').setValue(stamp)
    await browser.$('[data-testid="gl.add-translation"]').setValue('проверка')
    await browser.$('[data-testid="gl.add-submit"]').click()
    await browser
      .waitUntil(async () => (await browser.$(`[data-testid="gl.row.${stamp}"]`)).isExisting(), {
        timeout: 30000,
        interval: 250,
      })
      .catch(() => {
        throw new Error('glossary row did not appear after live upsert')
      })
  })
})

describe('J2 existing: dry-run → apply reusable (fresh project)', () => {
  it('разбор пакета → классификация → применение переиспользуемого', async () => {
    await createViaWizard() // fresh untranslated inventory → reusable ≥ 1
    await browser.$('a[href="#/existing"]').click()
    await waitExisting('[data-testid="ex.dir"]')
    await browser.$('[data-testid="ex.dir"]').setValue('/tmp/rimloc-existing-pack/Languages/Russian')
    await browser.$('[data-testid="ex.analyze"]').click()
    await waitExisting('[data-testid="ex.metrics"]')
    const metrics = await browser.$('[data-testid="ex.metrics"]').getText()
    if (!/\d/.test(metrics)) throw new Error('existing metrics empty')
    await browser.waitUntil(
      async () => {
        const b = await browser.$('[data-testid="ex.apply"]')
        return (await b.isExisting()) && (await b.isEnabled())
      },
      { timeout: 30000, interval: 250 },
    )
    await (await browser.$('[data-testid="ex.apply"]')).click()
    await browser.waitUntil(async () => (await browser.$('.inline-success')).isExisting(), {
      timeout: 30000,
      interval: 250,
    })
  })
})
