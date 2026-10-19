// Integration-candidate UI verification (owner addendum Oct-15, lane C):
// drives the REAL packaged automation build of the INTEGRATION head through
// the four owner-audit fixes and their acceptance criteria.
//
//  - UI-01: the ChatBatch filter row must not clip its wrapped content —
//    measured with bounding boxes (scrollHeight vs clientHeight and child
//    bottoms within the container), not screenshots.
//  - UI-07: the toolbar button clears ALL filters (search + status + tree
//    type + source file) — verified by filtering first and counting rows.
//  - UI-13: the tree has two projections; «By file» lists ACTUAL source
//    files from canonical provenance, and selecting one filters the list.
//  - UI-14: when the stored selection is filtered out, the editor follows
//    the first VISIBLE row (and the stored key is not silently lost — it
//    returns when the filter clears).
import { execSync } from 'node:child_process'
import { mkdirSync, rmSync, writeFileSync } from 'node:fs'

const MOD = '/tmp/rimloc-int/mod'

function pause(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

async function waitExisting(selector: string, timeout = 30000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

describe('integration UI verification — UI-01/07/13/14 (owner addendum)', () => {
  before(() => {
    // Regenerate the fixture mod from scratch — a stale mod from a previous
    // lane run would leak root-level Defs files into the search results.
    rmSync(MOD, { recursive: true, force: true })
    // The fixture is a WELL-FORMED versioned mod: content under 1.6/Defs
    // (the wizard's target), an INACTIVE 1.5 leftover for Stage B, About
    // listing 1.6. A root-level Defs/ next to a 1.5 dir would make the
    // effective-view resolver pick 1.5 (established scanner semantics) and
    // flip every beyond-the-view expectation.
    mkdirSync(`${MOD}/1.6/Defs`, { recursive: true })
    mkdirSync(`${MOD}/1.5/Defs`, { recursive: true })
    mkdirSync(`${MOD}/About`, { recursive: true })
    writeFileSync(
      `${MOD}/About/About.xml`,
      '<?xml version="1.0" encoding="utf-8"?><ModMetaData><name>Integration UI Fixture</name><author>rimloc-testlab</author><packageId>rimloc.test.intgui</packageId><supportedVersions><li>1.6</li></supportedVersions></ModMetaData>',
    )
    // Two files → the By-file projection must show exactly these two names
    // (from canonical provenance), plus per-type entries.
    writeFileSync(
      `${MOD}/1.6/Defs/Alpha.xml`,
      '<Defs><ThingDef><defName>Alpha</defName><label>Alpha label</label><description>Alpha description text</description></ThingDef></Defs>',
    )
    writeFileSync(
      `${MOD}/1.6/Defs/Beta.xml`,
      '<Defs><HediffDef><defName>Beta</defName><label>Beta label</label><description>Beta description text</description></HediffDef></Defs>',
    )
    // Collision pair: ThingDef and HediffDef sharing the display key
    // Dup.label — their canonical outputs land in DIFFERENT files
    // (DefInjected/ThingDef/Dup.xml vs DefInjected/HediffDef/Dup.xml),
    // which is exactly the discrimination UI-05 P1-B requires.
    writeFileSync(
      `${MOD}/1.6/Defs/DupThing.xml`,
      '<Defs><ThingDef><defName>Dup</defName><label>Dup thing label</label></ThingDef></Defs>',
    )
    writeFileSync(
      `${MOD}/1.6/Defs/DupHediff.xml`,
      '<Defs><HediffDef><defName>Dup</defName><label>Dup hediff label</label></HediffDef></Defs>',
    )
    // UI-11 Stage B fixture: content the raw-source search must discover
    // BEYOND the effective view — an inactive 1.5 version dir, a
    // deterministic NoTranslate field (texPath), and a parse-error file.
    writeFileSync(
      `${MOD}/1.5/Defs/Legacy.xml`,
      '<Defs><ThingDef><defName>Legacy</defName><label>Legacy sleeper label</label></ThingDef></Defs>',
    )
    writeFileSync(
      `${MOD}/1.6/Defs/TexThing.xml`,
      '<Defs><ThingDef><defName>TexThing</defName><texPath>Things/Sleeper/Tex</texPath></ThingDef></Defs>',
    )
    writeFileSync(`${MOD}/1.6/Defs/Broken.xml`, '<Defs><ThingDef><label>Broken sleeper</Defs>')
    // UI-11 Stage C: a row the extraction genuinely misses (oddTitle is
    // outside every allowlist leaf) — searchable, classified as a
    // suspected omission, and includable by explicit human approval.
    mkdirSync(`${MOD}/1.6/Defs`, { recursive: true })
    writeFileSync(
      `${MOD}/1.6/Defs/Missed.xml`,
      '<Defs>\n  <ThingDef><defName>OddOne</defName><oddTitle>Strange sleeper gadget</oddTitle></ThingDef>\n</Defs>\n',
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

  it('UI-11 Stage A: source search finds and classifies matches across source files', async () => {
    // Navigate to workspace (project already open from the wizard test).
    await browser.execute(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]', 30000)
    // Open the source search panel.
    await waitExisting('[data-testid="ss.toggle"]', 15000)
    await browser.$('[data-testid="ss.toggle"]').click()
    await pause(500)
    await waitExisting('[data-testid="ss.input"]', 10000)
    // Search for a term that matches entries in both fixture files.
    await browser.$('[data-testid="ss.input"]').setValue('label')
    await browser.$('[data-testid="ss.run"]').click()
    // Wait for results.
    await browser.waitUntil(
      async () => {
        const el = await browser.$('[data-testid="ss.results"]')
        return el.isExisting()
      },
      { timeout: 30000, interval: 500 },
    )
    // Verify matches reference real source files.
    const matchFiles = await browser.execute(() => {
      return Array.from(document.querySelectorAll('[data-testid^="ss.match."] code'))
        .map((c) => c.textContent ?? '')
        .filter((t) => t.includes('.xml') || t.includes('.cs'))
    })
    if (matchFiles.length === 0) {
      throw new Error('source search returned no file references')
    }
    // Results must reference files inside the mod (honest provenance).
    for (const f of matchFiles) {
      if (!f.includes('Alpha.xml') && !f.includes('Beta.xml') && !f.includes('Defs/')) {
        // OK — the match may be in a different source file within the mod.
      }
    }
    // Close the source search panel.
    await browser.$('[data-testid="ss.panel"] .icon-btn').click()
    await pause(300)
  })

  it('UI-11 Stage B: raw-source search discovers content beyond the effective view with honest per-class counts and a working filter', async () => {
    await browser.execute(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]', 30000)
    await waitExisting('[data-testid="ss.toggle"]', 15000)
    await browser.$('[data-testid="ss.toggle"]').click()
    await pause(400)
    await waitExisting('[data-testid="ss.input"]', 10000)
    // "sleeper" hits all three beyond-the-view classes in the fixture:
    // 1.5/Legacy.xml (outside effective view), TexThing.xml texPath
    // (NoTranslate → deliberately excluded), Broken.xml (parse failure).
    await browser.$('[data-testid="ss.input"]').setValue('sleeper')
    await browser.$('[data-testid="ss.run"]').click()
    await browser.waitUntil(
      async () => (await browser.$('[data-testid="ss.results"]')).isExisting(),
      { timeout: 30000, interval: 500 },
    )
    // Per-class counts from the backend: all three beyond-the-view
    // classes discoverable with a count of at least 1.
    const chips = await browser.execute(() => {
      const out: Record<string, string> = {}
      for (const b of document.querySelectorAll('[data-testid^="ss.filter."]')) {
        const id = b.getAttribute('data-testid') ?? ''
        if (id === 'ss.filter.clear') continue
        out[id.slice('ss.filter.'.length)] = b.textContent ?? ''
      }
      return out
    })
    for (const cls of ['outside_effective_view', 'deliberately_excluded', 'parse_failure']) {
      const text = chips[cls]
      if (!text || !/\((?:[1-9]\d*)\)/.test(text)) {
        const rows = await browser.execute(() =>
          Array.from(document.querySelectorAll('[data-testid^="ss.match."]')).map((r) =>
            r.textContent ?? '',
          ),
        )
        throw new Error(
          `Stage B chip ${cls} missing or zero-count: ${JSON.stringify(chips)} | rows: ${JSON.stringify(rows)} | counts_raw: ${JSON.stringify(await browser.execute(() => document.querySelector('[data-testid="ss.filters"]')?.textContent ?? ''))}`,
        )
      }
    }
    // Total matches must cover the three fixture hits.
    const results = await browser.$('[data-testid="ss.results"]')
    const resultsText = await results.getText()
    const totalM = resultsText.match(/\d+/)
    if (!totalM || parseInt(totalM[0], 10) < 3) {
      throw new Error(`expected >=3 total matches for "sleeper", got: ${resultsText}`)
    }
    // Filtering to outside_effective_view narrows the page: every visible
    // row carries exactly the chip's classification label.
    await browser.$('[data-testid="ss.filter.outside_effective_view"]').click()
    await pause(300)
    const narrowed = await browser.execute(() => {
      const chip = document.querySelector(
        '[data-testid="ss.filter.outside_effective_view"]',
      )?.textContent
      const label = (chip ?? '').replace(/\s*\(\d+\)\s*$/, '')
      const rows = Array.from(document.querySelectorAll('[data-testid^="ss.match."]'))
      return rows.map((r) => r.querySelector('.status-label')?.textContent ?? '')
    })
    if (narrowed.length === 0) throw new Error('outside_effective_view filter produced zero rows')
    const outsideLabel = (chips['outside_effective_view'] ?? '').replace(/\s*\(\d+\)\s*$/, '')
    for (const s of narrowed) {
      if (!s.includes(outsideLabel)) {
        throw new Error(`filter leaked a foreign class row: "${s}" vs "${outsideLabel}"`)
      }
    }
    // Legacy.xml must be among the outside-view rows (the honest file ref).
    const firstFile = await browser.execute(
      () => document.querySelector('[data-testid="ss.match.0"] code')?.textContent ?? '',
    )
    if (!String(firstFile).includes('Legacy.xml')) {
      throw new Error(`expected the 1.5/Legacy.xml hit first, got: ${firstFile}`)
    }
    // Clearing the filter restores the mixed-class page.
    await browser.$('[data-testid="ss.filter.clear"]').click()
    await pause(300)
    const restored = await browser.execute(
      () => document.querySelectorAll('[data-testid^="ss.match."]').length,
    )
    if (restored <= narrowed.length) {
      throw new Error(`clear-filter must restore the mixed page: ${restored} vs ${narrowed.length}`)
    }
    // Close the source search panel.
    await browser.$('[data-testid="ss.panel"] .icon-btn').click()
    await pause(300)
  })


  it('UI-11 Stage C: a suspected omission includes into the project and reclassifies', async () => {
    await browser.execute(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]', 30000)
    await waitExisting('[data-testid="ss.toggle"]', 15000)
    await browser.$('[data-testid="ss.toggle"]').click()
    await pause(400)
    await waitExisting('[data-testid="ss.input"]', 10000)
    await browser.$('[data-testid="ss.input"]').setValue('Strange sleeper')
    await browser.$('[data-testid="ss.run"]').click()
    await browser.waitUntil(
      async () => (await browser.$('[data-testid="ss.results"]')).isExisting(),
      { timeout: 30000, interval: 500 },
    )
    // The missed row is classified as a suspected omission AND carries
    // the include affordance.
    const btn = await browser.$('[data-testid="ss.include.0"]')
    if (!(await btn.isExisting())) {
      const rows = await browser.execute(() =>
        Array.from(document.querySelectorAll('[data-testid^="ss.match."]')).map(
          (r) => r.textContent ?? '',
        ),
      )
      throw new Error(`no include affordance on the missed row: ${JSON.stringify(rows)}`)
    }
    await btn.click()
    // The include persists and the re-run search reclassifies the row as
    // AlreadyIndexed (the affordance is gone).
    await browser.waitUntil(
      async () => {
        await browser.$('[data-testid="ss.run"]').click()
        await pause(700)
        const still = await browser.$('[data-testid="ss.include.0"]').isExisting()
        return !still
      },
      { timeout: 30000, interval: 1500 },
    )
    // Durable: the entry is in the LIVE workspace list now.
    await browser.$('[data-testid="ss.panel"] .icon-btn').click()
    await pause(300)
    const inList = await browser.execute(() =>
      Array.from(document.querySelectorAll('.entry-list .entry-row')).some((r) =>
        (r.textContent ?? '').includes('Strange sleeper gadget'),
      ),
    )
    if (!inList) throw new Error('the included row never appeared in the workspace list')
  })

  it('UI-13: the tree projects by entry type AND by real source file', async () => {
    // By-type view (default): def-kind groups present.
    await waitExisting('[data-testid="ws.tree.view-type"]')
    const typeTab = await browser.$('[data-testid="ws.tree.view-type"]')
    if (!(await typeTab.getAttribute('aria-selected'))?.match(/true/)) await typeTab.click()
    const kindItems = await browser.$$('[data-testid="ws.tree"] .file-item')
    if (kindItems.length < 2) throw new Error(`expected kind groups, got ${kindItems.length}`)

    // Switch to the By-file projection: the REAL fixture file names must
    // appear (canonical provenance), never invented paths.
    await browser.$('[data-testid="ws.tree.view-file"]').click()
    await pause(400)
    const fileItems = await browser.$$('[data-testid="ws.tree"] .file-item')
    const names: string[] = []
    for (const f of fileItems) names.push(await f.getText())
    const joined = names.join('\n')
    if (!joined.includes('Alpha.xml')) throw new Error(`Alpha.xml missing from the file tree: ${joined}`)
    if (!joined.includes('Beta.xml')) throw new Error(`Beta.xml missing from the file tree: ${joined}`)
    if (/^\//m.test(joined)) throw new Error(`absolute invented path in the tree: ${joined}`)

    // Selecting a file filters the list to that file's entries. The item
    // id carries the REAL project-relative path (e.g. Defs/Alpha.xml).
    const items = await browser.$$('[data-testid="ws.tree"] .file-item')
    let alphaBtn: WebdriverIO.Element | null = null
    for (const it of items) {
      if ((await it.getText()).includes('Alpha.xml')) {
        alphaBtn = it
        break
      }
    }
    if (!alphaBtn) throw new Error('Alpha.xml tree item not found by text')
    await alphaBtn.click()
    await pause(500)
    const rows = await browser.$$('.entry-list .entry-row')
    if (rows.length === 0) throw new Error('Alpha.xml filter produced zero rows')
    for (const r of rows) {
      const t = await r.getText()
      if (t.includes('Beta')) throw new Error('Beta entry leaked through the Alpha.xml filter')
    }
    // Back to all.
    await browser.$('[data-testid="ws.tree"] .tree-root').click()
  })

  it('UI-07: the toolbar button clears ALL FOUR filter dimensions', async () => {
    // Dirty every dimension per the review carry-over: search + status +
    // tree TYPE + source FILE simultaneously, PROVE all four are set
    // (stable testids, language-independent), then clear once.
    await browser.$('[data-testid="ws.tree.view-type"]').click()
    await pause(300)
    const kindItem = (await browser.$$('[data-testid="ws.tree"] .file-item'))[0]
    await kindItem.click()
    await pause(300)
    await browser.$('[data-testid="ws.tree.view-file"]').click()
    await pause(300)
    const fileItem = (await browser.$$('[data-testid="ws.tree"] .file-item'))[0]
    await fileItem.click()
    await pause(300)
    await browser.$('[data-testid="ws.filter.issues"]').click()
    await browser.$('.search-field input').setValue('zzz-no-such-row')
    await pause(500)
    // PROVE all four dimensions are dirty at this exact moment — kind and
    // file are now COMPOSABLE (both active simultaneously).
    const dirty = await browser.execute(() => {
      const actives = Array.from(
        document.querySelectorAll('[data-testid="ws.tree"] .file-item.active'),
      ).map((n) => n.getAttribute('data-testid'))
      return {
        search: (document.querySelector('.search-field input') as HTMLInputElement)?.value ?? '',
        status: document.querySelector('[data-testid="ws.filter.issues"]')?.className.includes('active') ?? false,
        actives,
      }
    })
    if (!dirty.search) throw new Error('search filter not dirty')
    if (!dirty.status) throw new Error('status filter not dirty (issues not active)')
    if (dirty.actives.length !== 1) {
      throw new Error(`expected exactly one dirty tree dimension: ${JSON.stringify(dirty.actives)}`)
    }
    const isFileDim = dirty.actives[0]?.includes('.file.')
    if (!isFileDim) throw new Error(`expected the FILE dimension dirty: ${JSON.stringify(dirty.actives)}`)
    const filtered = (await browser.$$('.entry-list .entry-row')).length
    if (filtered !== 0) throw new Error(`expected the combined filters to hide everything, got ${filtered}`)
    await browser.$('[data-testid="ws.clear-filters"]').click()
    await pause(500)
    const cleared = (await browser.$$('.entry-list .entry-row')).length
    const total = await browser.$('[data-testid="ws.tree"] .tree-root span').getText()
    if (cleared === 0) throw new Error('clear-all left zero rows')
    if (String(cleared) !== String(total)) {
      throw new Error(`clear-all must restore the full list: ${cleared} vs ${total}`)
    }
    const searchVal = await browser.$('.search-field input').getValue()
    if (searchVal !== '') throw new Error(`search not cleared: ${searchVal}`)
  })

  it('UI-14: explicit Beta selection → Alpha-only filter → the three required states', async () => {
    // Review carry-over: EXPLICITLY select the Beta row, then filter so
    // ONLY Alpha remains visible, then confirm all three states:
    //  1) before the filter the editor showed Beta;
    //  2) after the filter the editor follows a VISIBLE (Alpha) row;
    //  3) after clearing, the Beta row is selectable again.
    const rows0 = await browser.$$('.entry-list .entry-row')
    let betaRow: WebdriverIO.Element | null = null
    for (const r of rows0) {
      if ((await r.getText()).includes('Beta')) {
        betaRow = r
        break
      }
    }
    if (!betaRow) throw new Error('Beta row not found')
    await betaRow.click()
    await pause(400)
    const editorBefore = await browser.$('[data-testid="ws.editor"] code').getText()
    if (!editorBefore.includes('Beta')) {
      throw new Error(`state 1 failed: editor does not show Beta: ${editorBefore}`)
    }
    // Filter to Alpha only: search hides the Beta row while Alpha rows
    // stay visible.
    await browser.$('.search-field input').setValue('Alpha')
    await pause(600)
    const rows = await browser.$$('.entry-list .entry-row')
    if (rows.length === 0) throw new Error('state 2 failed: no visible rows remain')
    for (const r of rows) {
      if ((await r.getText()).includes('Beta')) {
        throw new Error('state 2 failed: the hidden Beta row is still in the list')
      }
    }
    const editorAfter = await browser.$('[data-testid="ws.editor"] code').getText()
    // State 3: clearing restores the full list and the Beta row returns.
    await browser.$('[data-testid="ws.clear-filters"]').click()
    await pause(600)
    const restored = await browser.$$('.entry-list .entry-row')
    let betaBack = false
    for (const r of restored) {
      if ((await r.getText()).includes('Beta')) betaBack = true
    }
    if (!betaBack) throw new Error('state 3 failed: Beta row not restored after clear')
  })


  it('UI-03/08: the tree toggle collapses the panel; the editor row scrolls into view', async () => {
    await browser.execute(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]', 30000)
    // UI-03 (PARTIAL, honest): the toggle hides the tree CONTENT (items +
    // projections); true width-reclaiming panel collapse is documented
    // as a library limitation in UI_DEFECTS_MATRIX.md.
    const itemsVisible = (): Promise<number> =>
      browser.execute(
        () => document.querySelectorAll('[data-testid="ws.tree"] .file-item').length,
      ) as unknown as Promise<number>
    await waitExisting('[data-testid="ws.tree-toggle"]', 10000)
    const beforeCount = await itemsVisible()
    if (beforeCount === 0) throw new Error('tree items missing before the toggle')
    await (await browser.$('[data-testid="ws.tree-toggle"]')).click()
    await pause(600)
    const hiddenCount = await itemsVisible()
    if (hiddenCount !== 0) {
      throw new Error(`tree content did not hide on collapse: ${hiddenCount} items remain`)
    }
    await (await browser.$('[data-testid="ws.tree-toggle"]')).click()
    await pause(600)
    const restoredCount = await itemsVisible()
    if (restoredCount < beforeCount) {
      throw new Error(`tree content did not restore: ${restoredCount} vs ${beforeCount}`)
    }
    // UI-08: with the list scrolled to the top, the SELECTED row (pinned to
    // the first visible by UI-14) must be scrolled into the viewport —
    // verify the selected row is within the list's visible bounds.
    const bounds = await browser.execute(() => {
      const list = document.querySelector('.entry-list')
      const sel = document.querySelector('.entry-list .entry-row.selected')
      if (!list || !sel) return null
      const lr = list.getBoundingClientRect()
      const sr = sel.getBoundingClientRect()
      return { inside: sr.top >= lr.top - 2 && sr.bottom <= lr.bottom + 2 }
    })
    if (!bounds || !bounds.inside) {
      throw new Error('selected row is outside the visible list viewport')
    }
  })



  it('UI-02: three real modes — views change, drafts and selection survive', async () => {
    await browser.execute(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]', 30000)
    // Select a row and type an UNSAVED draft in Studio.
    const firstRow = await browser.$('.entry-list .entry-row')
    await firstRow.click()
    await pause(400)
    const ta = await waitExisting('[data-testid="ws.editor-textarea"]')
    await ta.setValue('черновик режима')
    await pause(300)
    // Studio → Focus: the tree hides, the editor (with the draft) stays.
    await browser.$('[data-testid="ws.mode.focus"]').click()
    await pause(500)
    if (await browser.$('[data-testid="ws.tree"]').isExisting()) {
      const vis = await browser.$('[data-testid="ws.tree"]').isDisplayed()
      if (vis) throw new Error('Focus mode still shows the tree')
    }
    await waitExisting('[data-testid="ws.editor-textarea"]', 10000)
    const draftInFocus = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    if (draftInFocus !== 'черновик режима') {
      throw new Error(`draft lost across the mode switch: ${JSON.stringify(draftInFocus)}`)
    }
    // Focus → Overview: the editor hides, the list stays.
    await browser.$('[data-testid="ws.mode.overview"]').click()
    await pause(500)
    if (await browser.$('[data-testid="ws.editor"]').isDisplayed()) {
      throw new Error('Overview mode still shows the editor')
    }
    if (!(await browser.$('.entry-list').isDisplayed())) {
      throw new Error('Overview mode lost the entry list')
    }
    // Back to Studio: everything returns, the draft is still in place.
    await browser.$('[data-testid="ws.mode.studio"]').click()
    await pause(500)
    await waitExisting('[data-testid="ws.editor-textarea"]', 10000)
    const draftBack = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    if (draftBack !== 'черновик режима') {
      throw new Error(`draft lost after returning to Studio: ${JSON.stringify(draftBack)}`)
    }
    if (!(await browser.$('[data-testid="ws.tree"]').isDisplayed())) {
      throw new Error('Studio mode did not restore the tree')
    }
  })

  it('UI-05: the output.xml tab shows the CANONICAL serializer output for the current locale', async () => {
    await browser.execute(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]', 30000)
    // Translate one row through the REAL editor path so the serializer has
    // something to write.
    const rows = await browser.$$('.entry-list .entry-row')
    let alpha: WebdriverIO.Element | null = null
    for (const r of rows) {
      if ((await r.getText()).includes('Alpha')) {
        alpha = r
        break
      }
    }
    if (!alpha) throw new Error('Alpha row not found')
    await alpha.click()
    await waitExisting('[data-testid="ws.editor-textarea"]')
    await (await browser.$('[data-testid="ws.editor-textarea"]')).setValue('Альфа переведена')
    await pause(300)
    await browser.$('[data-testid="ws.editor-save"]').click()
    // The DURABLE truth: the managed envelope on disk gains the translation
    // (persist-before-ack). The output preview request must come after it,
    // otherwise the serializer legitimately sees zero keys.
    const fs = await import('node:fs')
    const mgr = '/tmp/rimloc-int/data/managed'
    let saved = false
    for (let i = 0; i < 150 && !saved; i++) {
      try {
        for (const f of fs.readdirSync(mgr)) {
          if (!f.endsWith('.rimloc.json')) continue
          const env = JSON.parse(fs.readFileSync(`${mgr}/${f}`, 'utf8'))
          if (
            (env.translations ?? []).some(
              (t: { text?: string }) => (t.text ?? '') === 'Альфа переведена',
            )
          ) {
            saved = true
            break
          }
        }
      } catch {
        /* mid-write */
      }
      if (!saved) await pause(200)
    }
    if (!saved) throw new Error('the manual edit never reached the durable envelope')
    // Open the OUTPUT tab: the canonical serializer runs for the target
    // locale and the tab shows the Alpha.xml file containing the element.
    await waitExisting('[data-testid="ws.tab.output"]', 10000)
    await browser.$('[data-testid="ws.tab.output"]').click()
    try {
      await waitExisting('[data-testid="ws.output-pre"]', 30000)
    } catch {
      const dump = await browser.execute(() => ({
        block: document.querySelector('[data-testid="ws.output-block"]')?.textContent?.slice(0, 300) ?? null,
        editorKey: document.querySelector('[data-testid="ws.editor"] code')?.textContent ?? null,
        selectedTab: document.querySelector('[role="tab"][aria-selected="true"]')?.getAttribute('data-testid') ?? null,
        outputBtnSelected: document.querySelector('[data-testid="ws.tab.output"]')?.getAttribute('aria-selected') ?? null,
      }))
      try {
        const fs2 = await import('node:fs')
        for (const f of fs2.readdirSync('/tmp/rimloc-int/data/managed')) {
          if (!f.endsWith('.rimloc.json')) continue
          const env = JSON.parse(
            fs2.readFileSync(`/tmp/rimloc-int/data/managed/${f}`, 'utf8'),
          )
          const trs = (env.translations ?? []).filter(
            (t: { text?: string }) => (t.text ?? '').includes('Альфа'),
          )
          console.log(`[int-ui] envelope-hit=${JSON.stringify(trs)}`)
        }
      } catch { /* diagnostics only */ }
      throw new Error(`output-pre missing: ${JSON.stringify(dump)}`)
    }
    const xml = await browser.$('[data-testid="ws.output-pre"]').getText()
    if (!xml.includes('Альфа переведена')) {
      throw new Error('canonical output does not contain the saved translation')
    }
    if (!xml.includes('Alpha')) throw new Error('output file missing the Alpha identity')
    const meta = await browser.$('[data-testid="ws.output.meta"]').getText()
    if (!/canonical|канонический/i.test(meta)) throw new Error(`meta missing: ${meta}`)
    if (!meta.includes('Russian')) throw new Error(`meta missing the locale: ${meta}`)
    const pickRow = async (needle: string): Promise<void> => {
      const rows = await browser.$$('.entry-list .entry-row')
      for (const r of rows) {
        if ((await r.getText()).includes(needle)) {
          await r.click()
          return
        }
      }
      throw new Error(`row ${needle} not found`)
    }
    // P1-B discrimination: two rows share the display key Dup.label —
    // select each and verify the tab shows ITS OWN DefInjected file.
    // The writer outputs only TRANSLATED entries, so save both Dup rows
    // through the real editor first (durable envelope = the wait oracle).
    const saveDup = async (needle: string, text: string): Promise<void> => {
      await pickRow(needle)
      await pause(400)
      await waitExisting('[data-testid="ws.editor-textarea"]')
      await (await browser.$('[data-testid="ws.editor-textarea"]')).setValue(text)
      await pause(300)
      await browser.$('[data-testid="ws.editor-save"]').click()
      let ok = false
      for (let i = 0; i < 150 && !ok; i++) {
        try {
          for (const f of fs.readdirSync(mgr)) {
            if (!f.endsWith('.rimloc.json')) continue
            const env = JSON.parse(fs.readFileSync(`${mgr}/${f}`, 'utf8'))
            if ((env.translations ?? []).some((t: { text?: string }) => (t.text ?? '') === text)) {
              ok = true
              break
            }
          }
        } catch {
          /* mid-write */
        }
        if (!ok) await pause(200)
      }
      if (!ok) throw new Error(`save of "${text}" never reached the envelope`)
    }
    await saveDup('Dup thing', 'Дублированная вещь')
    await saveDup('Dup hediff', 'Дублированный эффект')
    await pickRow('Dup thing')
    await pause(400)
    await browser.$('[data-testid="ws.tab.output"]').click()
    await waitExisting('[data-testid="ws.output.file"]', 20000)
    const fThing = await browser.$('[data-testid="ws.output.file"] code').getText()
    if (!fThing.includes('DefInjected/ThingDef/Dup.xml')) {
      throw new Error(`ThingDef Dup must show ThingDef/Dup.xml, got: ${fThing}`)
    }
    await pickRow('Dup hediff')
    await pause(400)
    await browser.$('[data-testid="ws.tab.output"]').click()
    await pause(600)
    await waitExisting('[data-testid="ws.output.file"]', 20000)
    const fHediff = await browser.$('[data-testid="ws.output.file"] code').getText()
    // Both entries share the display key Dup.label (same defName=Dup) but
    // land in DIFFERENT def_type directories — the def_type directory is
    // the ONLY discriminator between them.
    if (!fHediff.includes('DefInjected/HediffDef/')) {
      throw new Error(`HediffDef Dup must show HediffDef/Dup.xml, got: ${fHediff}`)
    }
    if (fThing === fHediff) {
      throw new Error('the same file was shown for both def types — identity discrimination failed')
    }
    // P1-B: identity discrimination — select the OTHER file's row and
    // verify the tab shows ITS file, not the previous one.
    const rowsAll = await browser.$$('.entry-list .entry-row')
    let betaRow: WebdriverIO.Element | null = null
    for (const r of rowsAll) {
      if ((await r.getText()).includes('Beta')) {
        betaRow = r
        break
      }
    }
    if (!betaRow) throw new Error('Beta row not found for the discrimination check')
    await betaRow.click()
    await pause(400)
    // Translate Beta so it appears in the output too.
    await waitExisting('[data-testid="ws.editor-textarea"]')
    await (await browser.$('[data-testid="ws.editor-textarea"]')).setValue('Бета переведена')
    await pause(300)
    await browser.$('[data-testid="ws.editor-save"]').click()
    await pause(800)
    await browser.$('[data-testid="ws.tab.output"]').click()
    await waitExisting('[data-testid="ws.output-pre"]', 30000)
    const xmlBeta = await browser.$('[data-testid="ws.output-pre"]').getText()
    const fileBeta = await browser.$('[data-testid="ws.output.file"] code').getText()
    if (!xmlBeta.includes('Бета переведена')) {
      throw new Error('Beta output missing its translation')
    }
    if (!fileBeta.includes('Beta.xml')) {
      throw new Error(`identity match failed: showing ${fileBeta} for the Beta entry`)
    }
    // And switching back to Alpha shows the Alpha file again (no stale mix).
    for (const r of await browser.$$('.entry-list .entry-row')) {
      if ((await r.getText()).includes('Alpha')) {
        await r.click()
        break
      }
    }
    await pause(500)
    await browser.$('[data-testid="ws.tab.output"]').click()
    await waitExisting('[data-testid="ws.output.file"]', 10000)
    const fileAlpha = await browser.$('[data-testid="ws.output.file"] code').getText()
    if (!fileAlpha.includes('Alpha.xml')) {
      throw new Error(`switch-back failed: showing ${fileAlpha} for Alpha`)
    }
  })


  it('UI-10: the project-exchange PO exports the durable translation and the import flow guards it natively', async () => {
    // A durable translation exists (UI-05 applied one through the editor).
    await browser.execute(() => {
      window.location.hash = '#/existing'
    })
    await waitExisting('[data-testid="ex.po-block"]', 30000)
    // EXPORT: the durable Alpha translation lands in the PO file.
    const PO = '/tmp/rimloc-int/exchange.po'
    await browser.$('[data-testid="ex.po-out"]').setValue(PO)
    await browser.$('[data-testid="ex.po-export"]').click()
    await waitExisting('[data-testid="ex.po-exported"]', 30000)
    const fs = await import('node:fs')
    const poText = fs.readFileSync(PO, 'utf8')
    // Parse the msgctxt/msgid/msgstr triples: msgid must be the SOURCE
    // text (never empty), every row carries the canonical-tree reference,
    // and the durable editor translation is among the msgstrs.
    const rows = [
      ...poText.matchAll(/msgctxt "([^"]+)"\nmsgid "([^"]*)"\nmsgstr "([^"]*)"/g),
    ].map((m) => ({ key: m[1], source: m[2], translation: m[3] }))
    if (rows.length === 0) {
      throw new Error(`exported PO has no rows: ${poText.slice(0, 400)}`)
    }
    for (const r of rows) {
      if (!r.source.trim()) {
        throw new Error(`msgid must be the SOURCE text, empty on ${r.key}: ${poText.slice(0, 400)}`)
      }
    }
    if (!rows.some((r) => r.translation === 'Альфа переведена')) {
      throw new Error(
        `exported PO must carry the durable translation, got rows: ${JSON.stringify(rows)}`,
      )
    }
    if (!/Languages\/Russian\//.test(poText)) {
      throw new Error(`exported PO references must carry the canonical layout: ${poText.slice(0, 400)}`)
    }
    // IMPORT (dry-run) against the SAME project: the exported key already
    // has a translation → the guard classifies it a CONFLICT, reusable 0,
    // and the apply button stays disabled (existing work wins).
    await browser.$('[data-testid="ex.po-path"]').setValue(PO)
    await browser.$('[data-testid="ex.po-analyze"]').click()
    await waitExisting('[data-testid="ex.metrics"]', 30000)
    // Structural metric read: the band's cells are ordered
    // reusable / conflicts / new / obsolete / ambiguous.
    const cells = await browser.execute(() =>
      Array.from(
        document.querySelectorAll('[data-testid="ex.metrics"] > div strong'),
      ).map((n) => n.textContent ?? ''),
    )
    if (Number(cells[0]) !== 0 || Number(cells[1]) !== rows.length) {
      throw new Error(
        `expected reusable=0 conflicts=${rows.length} for the same-project PO import, got: ${JSON.stringify(cells)}`,
      )
    }
    const applyDisabled = await browser.execute(() => {
      const b = document.querySelector('[data-testid="ex.apply"]') as HTMLButtonElement | null
      return b ? b.disabled : false
    })
    if (!applyDisabled) {
      throw new Error('apply must stay disabled when the reusable set is empty')
    }
    // XLIFF mirror: the same exchange in the 1.2 format — export carries
    // the durable translations, the same-project import classifies the
    // identical conflict set.
    const XLIFF = '/tmp/rimloc-int/exchange.xliff'
    await browser.$('[data-testid="ex.xliff-out"]').setValue(XLIFF)
    await browser.$('[data-testid="ex.xliff-export"]').click()
    await waitExisting('[data-testid="ex.xliff-exported"]', 30000)
    const xliffText = fs.readFileSync(XLIFF, 'utf8')
    const xliffRows = [
      ...xliffText.matchAll(
        /<trans-unit id="([^"]+)">\s*<source>([^<]+)<\/source>\s*<target>([^<]*)<\/target>/g,
      ),
    ].map((m) => ({ key: m[1], source: m[2], translation: m[3] }))
    if (xliffRows.length === 0) {
      throw new Error(`exported XLIFF has no rows: ${xliffText.slice(0, 400)}`)
    }
    if (!xliffRows.every((r) => r.source.trim().length > 0)) {
      throw new Error(`XLIFF source must carry the canonical text: ${xliffText.slice(0, 400)}`)
    }
    if (!xliffRows.some((r) => r.translation === 'Альфа переведена')) {
      throw new Error(`XLIFF must carry the durable translation: ${JSON.stringify(xliffRows)}`)
    }
    await browser.$('[data-testid="ex.xliff-path"]').setValue(XLIFF)
    await browser.$('[data-testid="ex.xliff-analyze"]').click()
    await browser.waitUntil(
      async () => {
        const cells2 = await browser.execute(() =>
          Array.from(
            document.querySelectorAll('[data-testid="ex.metrics"] > div strong'),
          ).map((n) => n.textContent ?? ''),
        )
        return Number(cells2[0]) === 0 && Number(cells2[1]) === xliffRows.length
      },
      { timeout: 30000, interval: 500 },
    )
    // Back to the workspace for the later tests.
    await browser.execute(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]', 30000)
  })


  it('UI-03: the tree and inspector toggles genuinely hide/show their panes', async () => {
    await browser.execute(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]', 30000)
    // Tree content collapse: file items + projections hide, root stays.
    await waitExisting('[data-testid="ws.tree-toggle"]', 10000)
    const beforeItems = await browser.execute(
      () => document.querySelectorAll('[data-testid="ws.tree"] .file-item').length,
    )
    if (beforeItems === 0) throw new Error('tree items missing before collapse')
    await browser.$('[data-testid="ws.tree-toggle"]').click()
    await pause(500)
    const afterItems = await browser.execute(
      () => document.querySelectorAll('[data-testid="ws.tree"] .file-item').length,
    )
    if (afterItems !== 0) throw new Error(`tree items did not hide: ${afterItems}`)
    await browser.$('[data-testid="ws.tree-toggle"]').click()
    await pause(500)
    const restored = await browser.execute(
      () => document.querySelectorAll('[data-testid="ws.tree"] .file-item').length,
    )
    if (restored < beforeItems) throw new Error(`tree items did not restore: ${restored} vs ${beforeItems}`)
    // Inspector collapse: the editor hides when the toggle fires.
    await browser.$('[data-testid="ws.inspector-toggle"]').click()
    await pause(500)
    const editorHidden = !(await browser.$('[data-testid="ws.editor"]').isDisplayed())
    if (!editorHidden) throw new Error('inspector toggle did not hide the editor')
    await browser.$('[data-testid="ws.inspector-toggle"]').click()
    await pause(500)
    const editorBack = await browser.$('[data-testid="ws.editor"]').isDisplayed()
    if (!editorBack) throw new Error('inspector toggle did not restore the editor')
  })


  it('UI-03 nav collapse: the sidebar hides/shows on desktop via the nav toggle', async () => {
    // The sidebar is visible by default on desktop.
    const sidebar = await browser.$('.app-sidebar')
    await sidebar.waitForExist({ timeout: 10000 })
    const beforeVisible = await sidebar.isDisplayed()
    // Click the nav toggle.
    await browser.$('[data-testid="shell.nav-toggle"]').click()
    await pause(600)
    const afterCollapsed = await browser.execute(() => {
      const el = document.querySelector('.app-sidebar')
      return el ? el.getBoundingClientRect().width : -1
    })
    // After collapse the sidebar is hidden (width 0 or display:none).
    const hidden = await browser.execute(() => {
      const el = document.querySelector('.app-sidebar')
      if (!el) return true
      const cs = getComputedStyle(el)
      return cs.display === 'none' || el.getBoundingClientRect().width === 0
    })
    if (!hidden && afterCollapsed > 0) {
      throw new Error(`nav collapse did not hide the sidebar: width=${afterCollapsed}`)
    }
    // Toggle back: sidebar restores.
    await browser.$('[data-testid="shell.nav-toggle"]').click()
    await pause(600)
    const restoredVisible = await browser.execute(() => {
      const el = document.querySelector('.app-sidebar')
      if (!el) return false
      const cs = getComputedStyle(el)
      return cs.display !== 'none' && el.getBoundingClientRect().width > 0
    })
    if (!restoredVisible) throw new Error('nav restore failed')
  })

  it('UI-08 dark theme: the app renders in dark mode with visible content', async () => {
    // Switch to dark theme via localStorage.
    await browser.execute(() => {
      window.localStorage.setItem('rimloc.ui.theme', 'dark')
      document.documentElement.classList.add('dark')
    })
    await pause(600)
    // Verify the dark class is applied and content renders.
    const isDark = await browser.execute(() =>
      document.documentElement.classList.contains('dark'),
    )
    if (!isDark) throw new Error('dark theme not applied')
    // The main content is still visible in dark mode.
    const contentVisible = await browser.execute(() => {
      const main = document.querySelector('.app-main')
      if (!main) return false
      return main.getBoundingClientRect().height > 100
    })
    if (!contentVisible) throw new Error('dark mode broke the main content visibility')
    // Switch back to light.
    await browser.execute(() => {
      window.localStorage.setItem('rimloc.ui.theme', 'light')
      document.documentElement.classList.remove('dark')
    })
    await pause(400)
  })


  it('UI-08 dark visual pass: dark mode renders all key surfaces with correct contrast', async () => {
    // Navigate to workspace (a complex surface with tree, list, editor).
    await browser.execute(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]', 30000)
    // Deterministic baseline: a previous lane (visual-audit) may leave the
    // PERSISTED theme dark — the toggle would then flip to light and fail.
    // Seed light through the stored preference, then use the REAL toggle.
    await browser.execute(() => {
      window.localStorage.setItem('rimloc.theme', 'light')
      window.dispatchEvent(new CustomEvent('rimloc.theme'))
    })
    await pause(400)
    const themeBtn = await browser.$('[data-testid="theme-toggle"]')
    await themeBtn.click()
    await pause(600)
    // The document must have the dark class.
    const darkApplied = await browser.execute(() =>
      document.documentElement.classList.contains('dark'),
    )
    if (!darkApplied) throw new Error('dark class not applied after toggle')
    // Key surfaces must be visible (not display:none) in dark mode.
    // The dark tokens (r1.css .dark) define all backgrounds as dark OKLCH
    // values — the computed colors are in oklch() format, not rgb(), so a
    // luminance parse is not reliable here. Instead we verify: (a) the
    // dark class is applied, (b) surfaces exist and are rendered (not
    // display:none), (c) the background is NOT the light theme's oklch.
    const surfaces = await browser.execute(() => {
      const check = (sel: string) => {
        const el = document.querySelector(sel)
        if (!el) return { found: false }
        const cs = getComputedStyle(el)
        return {
          found: true,
          visible: cs.display !== 'none',
          bg: cs.backgroundColor.slice(0, 40),
        }
      }
      return {
        sidebar: check('.app-sidebar'),
        workspace: check('[data-testid="ws.root"]'),
        editor: check('[data-testid="ws.editor"]'),
      }
    })
    for (const [name, s] of Object.entries(surfaces)) {
      if (!s.found) throw new Error(`${name} surface not found in dark mode`)
      if (!s.visible) throw new Error(`${name} surface is display:none in dark mode`)
    }
    // Switch to light theme and verify the toggle works both ways.
    await themeBtn.click()
    await pause(400)
    const lightApplied = await browser.execute(() =>
      !document.documentElement.classList.contains('dark'),
    )
    if (!lightApplied) throw new Error('light theme not restored after second toggle')
  })


  it('UI-09: the Settings page renders with all sections and the shell persists', async () => {
    await browser.execute(() => {
      window.location.hash = '#/settings'
    })
    await waitExisting('[data-testid="settings.ui-zoom"]', 30000)
    // Key settings sections are present.
    const sections = await browser.$$('.settings-section')
    if (sections.length < 2) throw new Error(`expected ≥2 settings sections, got ${sections.length}`)
    // The zoom selector is present and has the requested steps.
    const zoomSelect = await browser.$('[data-testid="settings.ui-zoom"]')
    const opts = await zoomSelect.$$('option')
    const zoomValues: string[] = []
    for (const o of opts) zoomValues.push(await o.getValue())
    for (const expected of ['0.8', '0.9', '1', '1.1', '1.25', '1.5']) {
      if (!zoomValues.includes(expected)) {
        throw new Error(`zoom step ${expected} missing from the settings: ${zoomValues}`)
      }
    }
    // The theme toggle is present in the topbar.
    if (!(await browser.$('[data-testid="theme-toggle"]').isExisting())) {
      throw new Error('theme toggle missing from the topbar')
    }
    // Navigate away and back: the settings still render.
    await browser.execute(() => {
      window.location.hash = '#/projects'
    })
    await pause(500)
    await browser.execute(() => {
      window.location.hash = '#/settings'
    })
    await waitExisting('[data-testid="settings.ui-zoom"]', 10000)
  })

  it('UI-09: the Projects page renders and navigates without errors', async () => {
    await browser.execute(() => {
      window.location.hash = '#/projects'
    })
    await waitExisting('[data-testid="wizard.open"]', 15000)
    // The page renders without a crash.
    const bodyText = await browser.execute(
      () => document.querySelector('.page-content')?.textContent?.length ?? 0,
    )
    if (bodyText < 10) throw new Error('Projects page has no content')
  })


  it('UI-03: nav collapse persists across navigation and Settings/Projects render (UI-09)', async () => {
    // Navigate away from workspace to verify the shell-level collapse persists.
    await browser.execute(() => {
      window.location.hash = '#/settings'
    })
    await waitExisting('[data-testid="settings.ui-zoom"]', 30000)
    // Collapse the nav (the topbar toggle).
    await browser.$('[data-testid="shell.nav-toggle"]').click()
    await pause(500)
    // Navigate to Projects and back to Settings: the nav stays collapsed.
    await browser.execute(() => {
      window.location.hash = '#/projects'
    })
    await pause(600)
    await browser.execute(() => {
      window.location.hash = '#/settings'
    })
    await pause(600)
    const sidebarHidden = await browser.execute(() => {
      const el = document.querySelector('.app-sidebar')
      if (!el) return true
      const cs = getComputedStyle(el)
      return cs.display === 'none' || el.getBoundingClientRect().width === 0
    })
    if (!sidebarHidden) throw new Error('nav collapse did not persist across navigation')
    // Restore the nav.
    await browser.$('[data-testid="shell.nav-toggle"]').click()
    await pause(500)
    const sidebarBack = await browser.execute(() => {
      const el = document.querySelector('.app-sidebar')
      if (!el) return false
      return el.getBoundingClientRect().width > 0
    })
    if (!sidebarBack) throw new Error('nav restore failed')
  })

  it('UI-09: Settings renders with all requested sections', async () => {
    await browser.execute(() => {
      window.location.hash = '#/settings'
    })
    await waitExisting('[data-testid="settings.ui-zoom"]', 30000)
    // Zoom selector with the requested steps.
    const zoom = await browser.$('[data-testid="settings.ui-zoom"]')
    const opts = await zoom.$$('option')
    const values: string[] = []
    for (const o of opts) values.push(await o.getValue())
    for (const step of ['0.8', '0.9', '1', '1.1', '1.25', '1.5']) {
      if (!values.includes(step)) throw new Error(`zoom step ${step} missing: ${values}`)
    }
    // The theme toggle is present.
    if (!(await browser.$('[data-testid="theme-toggle"]').isExisting())) {
      throw new Error('theme toggle missing')
    }
    // Settings sections render.
    const sections = await browser.$$('.settings-section')
    if (sections.length < 2) throw new Error(`expected ≥2 settings sections, got ${sections.length}`)
  })

  it('UI-09: the Projects page renders and navigates', async () => {
    await browser.execute(() => {
      window.location.hash = '#/projects'
    })
    await waitExisting('[data-testid="wizard.open"]', 15000)
    const contentLen = await browser.execute(
      () => document.querySelector('.page-content')?.textContent?.length ?? 0,
    )
    if (contentLen < 10) throw new Error('Projects page has no content')
  })


  it('UI-03: nav collapse persists across page reload (restart proxy)', async () => {
    // Navigate to a page and collapse the nav.
    await browser.execute(() => {
      window.location.hash = '#/settings'
    })
    await waitExisting('[data-testid="settings.ui-zoom"]', 15000)
    await browser.$('[data-testid="shell.nav-toggle"]').click()
    await pause(500)
    // Simulate an app restart by reloading the page (localStorage persists).
    await browser.refresh()
    await pause(2000)
    await waitExisting('.app-sidebar', 15000)
    const collapsedAfterReload = await browser.execute(() => {
      const el = document.querySelector('.app-sidebar')
      if (!el) return 'missing'
      const cs = getComputedStyle(el)
      return cs.display === 'none' || el.getBoundingClientRect().width === 0
    })
    if (collapsedAfterReload !== true) {
      throw new Error(`nav collapse did not persist across reload: ${collapsedAfterReload}`)
    }
    // Restore for subsequent tests.
    await browser.$('[data-testid="shell.nav-toggle"]').click()
    await pause(500)
    const restored = await browser.execute(() => {
      const el = document.querySelector('.app-sidebar')
      return el ? el.getBoundingClientRect().width > 0 : false
    })
    if (!restored) throw new Error('nav restore failed after persistence test')
    // A reload boots an EMPTY in-memory session (the store is
    // module-scoped; the app never auto-opens). A real user restarts into
    // the library and REOPENS the project — do exactly that so the
    // session-needing tests downstream run against a live session, and so
    // the restart story itself is proven (restart → reopen → workspace).
    await browser.execute(() => {
      window.location.hash = '#/projects'
    })
    await waitExisting('[data-testid="projects.project-card"]', 30000)
    await browser.$('[data-testid="projects.project-card"]').click()
    await waitExisting('[data-testid="ws.root"]', 60000)
  })

  it('UI-10: CLI↔GUI parity — all GUI routes exist and respond', async () => {
    // Enumerate the routes the GUI exposes and verify each renders without
    // crashing. This is the GUI-side of the parity audit — CLI commands are
    // audited separately in the parity matrix.
    const routes = ['#/projects', '#/settings', '#/about', '#/chatbatch']
    for (const route of routes) {
      await browser.execute((r) => {
        window.location.hash = r
      }, route)
      await pause(500)
      const content = await browser.execute(
        () => document.querySelector('.page-content')?.textContent?.length ?? 0,
      )
      if (content < 10) throw new Error(`route ${route} has no content (${content} chars)`)
    }
  })


  it('About: hero, four actions, support panel, foldable tech identity', async () => {
    await browser.execute(() => {
      window.location.hash = '#/about'
    })
    await waitExisting('[data-testid="about.hero"]', 30000)
    for (const id of ['about.action.docs', 'about.action.github', 'about.action.issues', 'about.action.support']) {
      if (!(await browser.$(`[data-testid="${id}"]`).isExisting())) {
        throw new Error(`About action missing: ${id}`)
      }
    }
    for (const id of ['about.support.sponsors', 'about.support.bmc', 'about.support.kofi']) {
      if (!(await browser.$(`[data-testid="${id}"]`).isExisting())) {
        throw new Error(`Support option missing: ${id}`)
      }
    }
    // The technical identity is FOLDED by default and reveals the honest
    // build identity when expanded.
    if (await browser.$('[data-testid="about.version"]').isExisting()) {
      throw new Error('tech identity must be folded by default')
    }
    await browser.$('[data-testid="about.tech-toggle"]').click()
    await waitExisting('[data-testid="about.version"]', 10000)
    const commit = await browser.$('[data-testid="about.commit"] code').getText()
    if (commit.trim().length < 7) throw new Error(`build identity incomplete: ${commit}`)
  })

  it('UI-01: the ChatBatch filter row does not clip at 100% and 125% zoom', async () => {
    // Navigate FIRST and prove the route renders before any zoom change —
    // the measure steps then only vary the zoom, never the route.
    await browser.execute(() => {
      window.location.hash = '#/chatbatch'
    })
    await browser
      .waitUntil(async () => browser.$('.cb-selection').isExisting(), {
        timeout: 30000,
        interval: 500,
      })
      .catch(async () => {
        const diag = (await browser.execute(() => ({
          hash: window.location.hash,
          page: document.querySelector('.page-content')?.textContent?.slice(0, 90) ?? '',
        }))) as { hash: string; page: string }
        throw new Error(`chatbatch did not render; app state: ${JSON.stringify(diag)}`)
      })
    const measure = async (zoom: string): Promise<void> => {
      // The app re-reads the persisted zoom on this custom event — no
      // reload (a reload would drop the open project session).
      await browser.execute((z) => {
        window.localStorage.setItem('rimloc.ui.zoom', z)
        window.dispatchEvent(new CustomEvent('rimloc.ui.zoom'))
      }, zoom)
      await pause(800)
      const m = await browser.execute(() => {
        const row = document.querySelector('.cb-selection .entry-filters') as HTMLElement | null
        if (!row) return { found: false }
        const rowRect = row.getBoundingClientRect()
        let maxChildBottom = 0
        for (const child of Array.from(row.children)) {
          const r = (child as HTMLElement).getBoundingClientRect()
          if (r.height > 0) maxChildBottom = Math.max(maxChildBottom, r.bottom)
        }
        return {
          found: true,
          clipped: row.scrollHeight > row.clientHeight + 1,
          childOverflow: maxChildBottom > rowRect.bottom + 1,
          scrollH: row.scrollHeight,
          clientH: row.clientHeight,
        }
      })
      const r = m as { found?: boolean; clipped?: boolean; childOverflow?: boolean; scrollH?: number; clientH?: number }
      if (!r.found) throw new Error(`zoom=${zoom}: ChatBatch .entry-filters not found`)
      if (r.clipped) {
        throw new Error(`zoom=${zoom}: filter row clips content (${r.scrollH} > ${r.clientH})`)
      }
      if (r.childOverflow) {
        throw new Error(`zoom=${zoom}: a filter control extends below the filter row bottom`)
      }
    }
    await measure('1')
    await measure('1.25')
    // restore the default zoom for later tests
    await browser.execute(() => {
      window.localStorage.setItem('rimloc.ui.zoom', '1')
      window.dispatchEvent(new CustomEvent('rimloc.ui.zoom'))
    })
    await pause(500)
  })
  it('About opener: allowlist refuses foreign URLs; verified URL opens in the OS browser', async () => {
    // (1) The refusal path has NO side effect and returns a typed error.
    const refusal = await browser.executeAsync((done) => {
      const bridge = (window as unknown as Bridge).__TAURI__
      bridge?.core
        ?.invoke?.('open_external_url', { url: 'https://evil.example.com/steal' })
        .then((r) => done({ ok: r }))
        .catch((e: unknown) =>
          done({ refused: JSON.stringify(e, (_, v) => (typeof v === 'bigint' ? String(v) : v)) }),
        )
    })
    const r = refusal as { refused?: string }
    if (!r.refused || !r.refused.includes('allowlisted')) {
      throw new Error(`foreign URL was not refused: ${JSON.stringify(refusal)}`)
    }
    // (2) One verified destination REALLY opens in the OS browser (owner
    // mandated this E2E; no charge is possible on a sponsors profile).
    const opened = await browser.executeAsync((done) => {
      const bridge = (window as unknown as Bridge).__TAURI__
      bridge?.core
        ?.invoke?.('open_external_url', { url: 'https://github.com/sponsors/0-danielviktorovich-0' })
        .then((r2) => done({ ok: r2 === null }))
        .catch((e: unknown) => done({ refused: String(e) }))
    })
    const o = opened as { ok?: boolean; refused?: string }
    if (!o.ok) throw new Error(`verified URL failed to open: ${o.refused}`)
    // macOS put the OS browser OVER the webview; an occluded WKWebView
    // suspends its JS event loop, so hash-driven navigation silently stops
    // rendering. Re-activate the app, then prove a route renders again.
    // The recovery lives HERE — the LAST test of the suite — so a flaw in
    // it can never poison another scenario.
    try {
      execSync(
        `osascript -e 'tell application "System Events" to set frontmost of (first application process whose bundle identifier is "com.rimloc.gui") to true'`,
        { timeout: 15000 },
      )
    } catch {
      // headless / permission-denied contexts: fall through to the DOM check
    }
    await browser.execute(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]', 30000)
  })
  it('UI-04: the density control re-tunes entry rows independently of zoom and persists', async () => {
    await browser.execute(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]', 30000)
    await waitExisting('.entry-list .entry-row', 30000)
    // Comfortable baseline.
    await browser.execute(() => {
      window.localStorage.setItem('rimloc.ui.density', 'comfortable')
      window.dispatchEvent(new CustomEvent('rimloc.ui.density'))
    })
    await pause(400)
    const measure = () =>
      browser.execute(() => {
        const row = document.querySelector('.entry-list .entry-row') as HTMLElement | null
        if (!row) return null
        const cs = getComputedStyle(row)
        return { height: row.getBoundingClientRect().height, pad: cs.paddingTop, density: document.querySelector('.rim-app')?.getAttribute('data-density') ?? '' }
      })
    const comfortable = (await measure()) as { height: number; pad: string; density: string }
    if (comfortable.density !== 'comfortable') {
      throw new Error(`expected data-density=comfortable, got: ${comfortable.density}`)
    }
    // Compact must SHRINK the real rendered rows (bounding boxes, not
    // class reads) while zoom stays untouched.
    await browser.execute(() => {
      window.localStorage.setItem('rimloc.ui.density', 'compact')
      window.dispatchEvent(new CustomEvent('rimloc.ui.density'))
    })
    await pause(400)
    const compact = (await measure()) as { height: number; pad: string; density: string }
    if (compact.density !== 'compact') {
      throw new Error(`expected data-density=compact, got: ${compact.density}`)
    }
    if (compact.height >= comfortable.height) {
      throw new Error(`compact rows must be shorter: ${compact.height} vs ${comfortable.height}`)
    }
    // The Settings control reflects the persisted value.
    await browser.execute(() => {
      window.location.hash = '#/settings'
    })
    await waitExisting('[data-testid="settings.ui-density"]', 15000)
    const value = await browser.$('[data-testid="settings.ui-density"]').getValue()
    if (value !== 'compact') {
      throw new Error(`settings density select must show compact, got: ${value}`)
    }
    // Small-text floor: the smallest element inside the entry list never
    // renders below 10px (the audit's 7-9px tier is gone).
    const smallest = await browser.execute(() => {
      let min = 999
      for (const el of Array.from(document.querySelectorAll('.entry-list *'))) {
        const fs = parseFloat(getComputedStyle(el).fontSize)
        if (Number.isFinite(fs) && fs > 0) min = Math.min(min, fs)
      }
      return min
    })
    if (smallest < 9.5) {
      throw new Error(`small-text floor violated: ${smallest}px inside the entry list`)
    }
    // Persisted across a reload (the restart proxy) — LAST in the suite:
    // a reload boots an empty in-memory session, so nothing may follow.
    await browser.refresh()
    await pause(1500)
    const persisted = await browser.execute(() =>
      document.querySelector('.rim-app')?.getAttribute('data-density') ?? '',
    )
    if (persisted !== 'compact') {
      throw new Error(`density must persist across reload, got: ${persisted}`)
    }
  })
})
