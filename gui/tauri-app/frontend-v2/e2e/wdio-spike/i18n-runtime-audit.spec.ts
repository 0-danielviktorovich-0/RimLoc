/** Runtime i18n audit: walk every screen, look for raw i18n keys
 *  (patterns like "namespace.key" visible as text) and English text
 *  in Russian mode. */
const SCREENS = [
  { name: 'projects', hash: '#/projects', wait: '.app-sidebar' },
  { name: 'workspace-empty', hash: '#/workspace', wait: '.app-sidebar' },
  { name: 'checks', hash: '#/checks', wait: '.app-sidebar' },
  { name: 'existing', hash: '#/existing', wait: '.app-sidebar' },
  { name: 'compare', hash: '#/compare', wait: '.app-sidebar' },
  { name: 'glossary', hash: '#/glossary', wait: '.app-sidebar' },
  { name: 'tm', hash: '#/tm', wait: '.app-sidebar' },
  { name: 'export', hash: '#/export', wait: '.app-sidebar' },
  { name: 'aitranslate', hash: '#/aitranslate', wait: '.app-sidebar' },
  { name: 'chatbatch', hash: '#/chatbatch', wait: '.app-sidebar' },
  { name: 'settings', hash: '#/settings', wait: '.app-sidebar' },
  { name: 'about', hash: '#/about', wait: '.app-sidebar' },
  { name: 'diagnostics', hash: '#/diagnostics', wait: '.app-sidebar' },
  { name: 'selfloc', hash: '#/selfloc', wait: '.app-sidebar' },
  { name: 'lm', hash: '#/lm', wait: '.app-sidebar' },
  { name: 'providers', hash: '#/providers', wait: '.app-sidebar' },
]

describe('runtime i18n audit', () => {
  it('walks all screens checking for raw keys and untranslated text', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), {
      timeout: 60000, interval: 200,
    })
    const results: Array<{screen: string, rawKeys: string[], englishButtons: string[]}> = []
    for (const s of SCREENS) {
      await browser.execute((h: string) => { window.location.hash = h }, s.hash)
      await browser.pause(1200)
      const audit = await browser.execute(() => {
        const rawKeys: string[] = []
        const englishButtons: string[] = []
        // Check all text nodes for i18n key patterns (word.word or word-word.word)
        const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT)
        let node: Node | null
        while ((node = walker.nextNode())) {
          const text = (node.textContent || '').trim()
          if (/^[a-z][a-z0-9]*\.[a-z][a-z0-9.]*$/i.test(text) && text.length < 60) {
            rawKeys.push(text)
          }
        }
        // Check buttons for English text (in Russian mode)
        document.querySelectorAll('button, a, [role="button"]').forEach(el => {
          const text = (el.textContent || '').trim()
          // Common English UI words that shouldn't appear in Russian mode
          if (/\b(Save|Cancel|Delete|Edit|Close|Open|New|Next|Back|Settings|Export|Import|Run|Start|Stop|Retry|Apply|Confirm|Search|Filter|Clear|Add|Remove|Update|Download|Upload|Copy|Cut|Paste)\b/.test(text) && text.length < 30) {
            englishButtons.push(text)
          }
        })
        return { rawKeys: [...new Set(rawKeys)].slice(0, 10), englishButtons: [...new Set(englishButtons)].slice(0, 10) }
      })
      results.push({ screen: s.name, ...audit })
      if (audit.rawKeys.length > 0 || audit.englishButtons.length > 0) {
        console.log(`⚠️ ${s.name}: rawKeys=${JSON.stringify(audit.rawKeys)} english=${JSON.stringify(audit.englishButtons)}`)
      }
    }
    const clean = results.filter(r => r.rawKeys.length === 0 && r.englishButtons.length === 0)
    console.log(`\n=== I18N RUNTIME AUDIT: ${clean.length}/${results.length} screens CLEAN ===`)
    for (const r of results) {
      if (r.rawKeys.length > 0 || r.englishButtons.length > 0) {
        console.log(`  ${r.screen}: ${r.rawKeys.length} rawKeys, ${r.englishButtons.length} English buttons`)
      }
    }
  })
})
