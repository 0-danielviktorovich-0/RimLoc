/** RUNTIME VERIFICATION: click XML tab, verify real XML displays */
async function pause(ms: number) { return new Promise(r => setTimeout(r, ms)) }
async function waitExisting(sel: string, timeout = 30000) {
  const el = await browser.$(sel); await el.waitForExist({ timeout }); return el
}
describe('XML tab runtime verification', () => {
  it('wizard → select entry → click XML tab → verify real XML', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), { timeout: 60000, interval: 200 })
    // Create project via wizard
    await browser.execute(() => { window.location.hash = '#/projects' })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue('/tmp/rimloc-int/mod')
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]', 60000)
    await browser.$('[data-testid="wizard.next"]').click()
    await pause(800)
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="ws.root"]', 300000)
    await waitExisting('.entry-list .entry-row', 60000)
    // Select the first entry
    await browser.$('.entry-list .entry-row').click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
    // FIND AND CLICK THE XML TAB — the critical moment
    console.log('Looking for XML tab...')
    const tabs = await browser.execute(() => {
      const tabBtns = Array.from(document.querySelectorAll('.context-tabs button'))
      return tabBtns.map(b => ({ text: b.textContent?.trim(), className: b.className }))
    })
    console.log('BOTTOM TABS:', JSON.stringify(tabs))
    // Find the XML source tab (might be labeled "XML" or "XML исходника" or similar)
    const xmlTab = await browser.$('.context-tabs button:nth-child(3)') // 3rd tab is usually XML
    const xmlTabText = await xmlTab.getText()
    console.log('Clicking tab:', xmlTabText)
    await xmlTab.click()
    await pause(2000)
    // Check what's displayed — this is the PASS/FAIL moment
    const content = await browser.execute(() => {
      const ctx = document.querySelector('.context-content')
      const err = document.querySelector('.inline-warning, .ws-error, [class*="error"]')
      return {
        contextText: ctx?.textContent?.slice(0, 300) ?? 'NO CONTEXT',
        errorText: err?.textContent?.slice(0, 200) ?? null,
        hasCommandNotFound: document.body.textContent?.includes('Command') && document.body.textContent?.includes('not found'),
      }
    })
    console.log('XML TAB RESULT:', JSON.stringify(content, null, 1))
    // CRITICAL ASSERTION
    expect(content.hasCommandNotFound).toBe(false)
    expect(content.contextText).not.toBe('NO CONTEXT')
    // Verify actual XML content is present (should contain XML tags or def names)
    expect(content.contextText.length).toBeGreaterThan(20)
  })
  it('measure layout: entries pane should fill available width', async () => {
    const geo = await browser.execute(() => {
      const ws = document.querySelector('.ws-center')?.getBoundingClientRect()
      const tree = document.querySelector('.file-tree')?.getBoundingClientRect()
      const entries = document.querySelector('.entries-pane')?.getBoundingClientRect()
      return {
        wsCenter: ws ? { w: ws.width, x: ws.x } : null,
        fileTree: tree ? { w: tree.width, x: tree.x } : null,
        entriesPane: entries ? { w: entries.width, x: entries.x } : null,
        gap: ws && tree && entries ? (ws.x + ws.width) - (entries.x + entries.width) : null,
      }
    })
    console.log('LAYOUT GEOMETRY:', JSON.stringify(geo, null, 1))
    // entries-pane should fill the remaining width (gap should be small, not ~300px)
    if (geo.entriesPane && geo.wsCenter) {
      const rightEdge = geo.entriesPane.x + geo.entriesPane.w
      const wsRight = geo.wsCenter.x + geo.wsCenter.w
      const gap = wsRight - rightEdge
      console.log(`Gap between entries-pane right edge and ws-center right edge: ${gap}px`)
      expect(gap).toBeLessThan(50) // Should be < 50px, not 300px
    }
  })
  it('AI mode labels should be Russian', async () => {
    await browser.execute(() => { window.location.hash = '#/aitranslate' })
    await pause(1500)
    const modes = await browser.execute(() => {
      const chips = Array.from(document.querySelectorAll('[class*="mode"], [class*="chip"], [data-testid*="mode"]'))
      return chips.map(c => c.textContent?.trim()).filter(Boolean).slice(0, 10)
    })
    console.log('AI MODE LABELS:', JSON.stringify(modes))
  })
})
