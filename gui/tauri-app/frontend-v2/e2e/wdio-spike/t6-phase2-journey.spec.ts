// T6 phase 2 (owner-authorized EXCLUSIVE run): the FULL RimLoc chain on an
// isolated test copy of the real mod — create project → deterministic
// marker edits (Keyed plain / Keyed placeholder / DefInjected) → validate →
// build-mod into an isolated output. Driven in background via embedded WDIO.
const SRC = '/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main/testlab/run/t6/src-copy/1814383360'
const OUT = '/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main/testlab/run/t6/mod-package'


// One edit attempt + bounded retries: an Enter-flush triggers a backend
// apply_intents roundtrip that rebuilds the table — a single click can land
// on a stale row node and the editor never opens.
async function editCell(id: string, value: string): Promise<void> {
  for (let attempt = 1; attempt <= 4; attempt++) {
    await browser.execute((tid) => {
      const row = document.querySelector(`[data-testid="workspace.row.${CSS.escape(tid)}"]`)
      row?.scrollIntoView({ block: 'center' })
      const cell = row?.querySelector('.cell.target') as HTMLElement | null
      cell?.click()
    }, id)
    await browser.pause(400)
    const editor = await browser.$(`[data-testid="workspace.editor.${id}"]`)
    if (await editor.isExisting()) {
      await editor.setValue(value)
      await browser.keys('Enter')
      await browser.pause(800) // backend roundtrip; table rebuild settles
      return
    }
    await browser.pause(1200)
  }
  throw new Error(`editor never opened for ${id}`)
}

const MARKERS: Array<[string, string, string]> = [
  // [key-fragment, marker value, class label]
  ['VWE_WeaponDeteriorationInfo', 'RIMLOC-T6 MARKER оружие деградирует после серии выстрелов', 'Keyed-plain'],
  ['VWE_ShotRemaining', 'RIMLOC-T6 MARKER выстрелов осталось: {0}', 'Keyed-placeholder'],
]

describe('T6 phase 2: RimLoc chain to game-loadable package', () => {
  it('creates project on the test copy, applies markers, validates, builds mod', async () => {
    // --- 1. create project from the test copy ---
    const homeH1 = await browser.$('h1=RimLoc — переводы модов RimWorld')
    await homeH1.waitForExist({ timeout: 20_000 })
    const pathInput = await browser.$('[data-testid="home.contract.path"]')
    await pathInput.waitForExist({ timeout: 10_000 })
    await pathInput.setValue(SRC)
    const createBtn = await browser.$('[data-testid="home.contract.create"]')
    await createBtn.click()
    // creation runs the real scanner (247 entries)
    const editorH1 = await browser.$('h1=Редактор перевода')
    await editorH1.waitForExist({ timeout: 60_000 })
    console.log('[t6p2] project created, workspace open')

    // --- 2. inventory: what classes did the scan produce ---
    const rowIds = await browser.execute(() =>
      Array.from(document.querySelectorAll<HTMLElement>('[data-testid^="workspace.row."]')).map(
        (el) => el.getAttribute('data-testid')!.replace('workspace.row.', ''),
      ),
    )
    console.log('[t6p2] entries:', rowIds.length)
    const classes: Record<string, number> = {}
    for (const id of rowIds) {
      const kind = id.split(':')[0]
      classes[kind] = (classes[kind] ?? 0) + 1
    }
    console.log('[t6p2] classes:', JSON.stringify(classes))

    // --- 3. marker edits (inline editor + Enter flush) ---
    for (const [frag, value] of MARKERS) {
      const id = rowIds.find((r) => r.includes(frag))
      if (!id) {
        console.log(`[t6p2] WARN: no entry matching ${frag}`)
        continue
      }
      // target cell click opens the inline editor
      await editCell(id, value)
      console.log(`[t6p2] marker set on ${id} (${value.slice(0, 30)}...)`)
    }
    // DefInjected research marker (second representative class)
    const researchId = rowIds.find((r) => r.startsWith('ResearchProjectDef') && r.includes('.label'))
    if (researchId) {
      await editCell(researchId, `RIMLOC-T6 MARKER research ${researchId.split(':')[1]?.replace('.label', '')}`)
      const key = researchId.split(':')[1] ?? researchId
      await editor.setValue(`RIMLOC-T6 MARKER research ${key.replace('.label', '')}`)
      await browser.keys('Enter')
      await browser.pause(400)
      console.log('[t6p2] DefInjected research marker set:', researchId)
    } else {
      console.log('[t6p2] NOTE: no ResearchProjectDef entries in scan (class not exercised)')
    }

    // --- 4. validate ---
    await browser.execute(() => { location.hash = '#/build' })
    await browser.pause(900)
    const validateBtn = await browser.$('[data-testid="contractops.validate.run"]')
    if (await validateBtn.isExisting()) {
      await validateBtn.click()
      const vres = await browser.waitUntil(
        async () => {
          const el = await browser.$('[data-testid="contractops.validate.status"]')
          return (await el.isExisting()) ? el.getText() : false
        },
        { timeout: 30_000, interval: 500 },
      )
      console.log('[t6p2] validate:', vres)
    }

    // --- 5. build-mod into the isolated output ---
    const outField = await browser.$('[data-testid="contractops.buildmod.outdir"]')
    await outField.waitForExist({ timeout: 10_000 })
    await outField.setValue(OUT)
    await browser.pause(200)
    const run = await browser.$('[data-testid="contractops.buildmod.run"]')
    await run.click()
    const outcome = await browser.waitUntil(
      async () => {
        const ok = await browser.$('[data-testid="contractops.buildmod.result"]')
        if (await ok.isExisting()) return 'ok'
        const err = await browser.$('[data-testid="contractops.buildmod.error"]')
        if (await err.isExisting()) return 'error: ' + (await err.getText())
        return false
      },
      { timeout: 60_000, interval: 500 },
    )
    console.log('[t6p2] build-mod outcome:', outcome)
    if (!outcome.startsWith('ok')) throw new Error(outcome)
    console.log('[t6p2] CHAIN COMPLETE')
  })
})
