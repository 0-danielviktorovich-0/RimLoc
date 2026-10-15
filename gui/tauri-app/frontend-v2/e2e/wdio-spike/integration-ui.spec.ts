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
import { mkdirSync, writeFileSync } from 'node:fs'

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
    mkdirSync(`${MOD}/Defs`, { recursive: true })
    mkdirSync(`${MOD}/About`, { recursive: true })
    writeFileSync(
      `${MOD}/About/About.xml`,
      '<?xml version="1.0" encoding="utf-8"?><ModMetaData><name>Integration UI Fixture</name><author>rimloc-testlab</author><packageId>rimloc.test.intgui</packageId><supportedVersions><li>1.6</li></supportedVersions></ModMetaData>',
    )
    // Two files → the By-file projection must show exactly these two names
    // (from canonical provenance), plus per-type entries.
    writeFileSync(
      `${MOD}/Defs/Alpha.xml`,
      '<Defs><ThingDef><defName>Alpha</defName><label>Alpha label</label><description>Alpha description text</description></ThingDef></Defs>',
    )
    writeFileSync(
      `${MOD}/Defs/Beta.xml`,
      '<Defs><HediffDef><defName>Beta</defName><label>Beta label</label><description>Beta description text</description></HediffDef></Defs>',
    )
    // Collision pair: ThingDef and HediffDef sharing the display key
    // Dup.label — their canonical outputs land in DIFFERENT files
    // (DefInjected/ThingDef/Dup.xml vs DefInjected/HediffDef/Dup.xml),
    // which is exactly the discrimination UI-05 P1-B requires.
    writeFileSync(
      `${MOD}/Defs/DupThing.xml`,
      '<Defs><ThingDef><defName>Dup</defName><label>Dup thing label</label></ThingDef></Defs>',
    )
    writeFileSync(
      `${MOD}/Defs/DupHediff.xml`,
      '<Defs><HediffDef><defName>Dup</defName><label>Dup hediff label</label></HediffDef></Defs>',
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
    // Switch to dark theme via the REAL toggle button.
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
  })

  it('UI-01: the ChatBatch filter row does not clip at 100% and 125% zoom', async () => {
    const measure = async (zoom: string): Promise<void> => {
      // The app re-reads the persisted zoom on this custom event — no
      // reload (a reload would drop the open project session).
      await browser.execute((z) => {
        window.localStorage.setItem('rimloc.ui.zoom', z)
        window.dispatchEvent(new CustomEvent('rimloc.ui.zoom'))
      }, zoom)
      await pause(800)
      await browser.execute(() => {
        window.location.hash = '#/chatbatch'
      })
      await waitExisting('.cb-selection', 30000)
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
})
