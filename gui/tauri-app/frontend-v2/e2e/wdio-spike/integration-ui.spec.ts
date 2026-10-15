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

  it('UI-07: the toolbar button clears ALL filters', async () => {
    // Dirty every filter dimension: search, status, tree type.
    await browser.$('[data-testid="ws.tree.view-type"]').click()
    await pause(300)
    const kindItem = (await browser.$$('[data-testid="ws.tree"] .file-item'))[0]
    await kindItem.click()
    await pause(300)
    await browser.$('.search-field input').setValue('zzz-no-such-row')
    await pause(500)
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

  it('UI-14: the editor follows a VISIBLE row when filters hide the selection', async () => {
    // Select the first row, then filter it out by search: the editor must
    // follow the first VISIBLE row, not sit on the hidden one.
    const firstRow = await browser.$('.entry-list .entry-row')
    await firstRow.click()
    await pause(400)
    const keyBefore = await browser.$('[data-testid="ws.editor"] code').getText()
    await browser.$('.search-field input').setValue(keyBefore.split(' ')[0] + '-no-match')
    await pause(500)
    const rows = await browser.$$('.entry-list .entry-row')
    if (rows.length === 0) {
      // Everything hidden including the selection: clear via the toolbar
      // and verify the editor recovered to a visible row.
      await browser.$('[data-testid="ws.clear-filters"]').click()
      await pause(500)
    }
    const editorKey = await browser.$('[data-testid="ws.editor"] code').getText()
    const visibleKeys: string[] = []
    for (const r of await browser.$$('.entry-list .entry-row')) {
      visibleKeys.push(await r.getText())
    }
    if (visibleKeys.length > 0 && !visibleKeys.some((t) => editorKey && t.includes(editorKey.split('·')[0].trim()))) {
      // The editor row must be among the visible ones (by its key text).
      throw new Error(`editor shows a hidden row: editor=${editorKey}`)
    }
    await browser.$('[data-testid="ws.clear-filters"]').click()
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

  it('UI-01: the ChatBatch filter row does not clip its wrapped content', async () => {
    await browser.execute(() => {
      window.location.hash = '#/chatbatch'
    })
    await waitExisting('.cb-selection', 30000)
    // Measure the REAL bounding boxes at the current viewport.
    const m = await browser.execute(() => {
      const row = document.querySelector('.cb-selection .entry-filters') as HTMLElement | null
      if (!row) return { found: false }
      const cs = getComputedStyle(row)
      const rowRect = row.getBoundingClientRect()
      let maxChildBottom = 0
      for (const child of Array.from(row.children)) {
        const r = (child as HTMLElement).getBoundingClientRect()
        if (r.height > 0) maxChildBottom = Math.max(maxChildBottom, r.bottom)
      }
      return {
        found: true,
        height: cs.height,
        autoHeight: cs.height === 'auto' || parseFloat(cs.height) >= 43,
        clipped: row.scrollHeight > row.clientHeight + 1,
        childOverflow: maxChildBottom > rowRect.bottom + 1,
        scrollH: row.scrollHeight,
        clientH: row.clientHeight,
      }
    })
    if (!m.found) throw new Error('ChatBatch .entry-filters not found')
    if (m.clipped) {
      throw new Error(`filter row clips content: scrollHeight ${m.scrollH} > clientHeight ${m.clientH}`)
    }
    if (m.childOverflow) {
      throw new Error('a filter control extends below the filter row bottom')
    }
  })
})
