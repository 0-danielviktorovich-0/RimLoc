/**
 * AFTER-captures for the critique round-2 fixes.
 * Dark theme is forced via the settings screen so the disabled-primary
 * legibility fix is exercised on the theme where it failed before.
 */
async function waitExisting(selector: string, timeout = 30000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

describe('critique round-2 AFTER captures', () => {
  it('dark theme + workspace hero + aitranslate disabled button + settings footer', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), {
      timeout: 60000,
      interval: 200,
    })
    // Open the project through the wizard first (the workspace needs an
    // open project; same fixture the visual-audit lane seeds).
    await browser.execute(() => {
      window.location.hash = '#/projects'
    })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue('/tmp/rimloc-int/mod')
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]', 60000)
    await browser.$('[data-testid="wizard.version"]').selectByVisibleText('1.6')
    await browser.$('[data-testid="wizard.next"]').click()
    await browser.pause(800)
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) {
      await browser.$('[data-testid="wizard.next"]').click()
    }
    await waitExisting('[data-testid="ws.root"]', 300000)

    // Force dark via the settings screen (persisted for the session).
    await browser.execute(() => {
      window.location.hash = '#/settings'
    })
    await waitExisting('[data-testid="settings.theme"]')
    await browser.execute((sel) => {
      const el = document.querySelector('[data-testid="settings.theme"]') as HTMLSelectElement
      const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')!.set!
      setter.call(el, sel)
      el.dispatchEvent(new Event('change', { bubbles: true }))
    }, 'dark')
    await browser.pause(800)

    // Workspace: the compressed hero.
    await browser.execute(() => {
      window.location.hash = '#/workspace'
    })
    await waitExisting('[data-testid="ws.root"]', 300000)
    await browser.pause(1500)
    await browser.saveScreenshot('/tmp/rimloc-int/shots/workspace-hero-after.png')

    // Aitranslate in dark: the disabled run button must be legible.
    await browser.execute(() => {
      window.location.hash = '#/aitranslate'
    })
    await waitExisting('[data-testid="ai.form"]', 60000)
    await browser.pause(1200)
    const m = await browser.execute(() => {
      const btn = document.querySelector('[data-testid="ai.run"]') as HTMLButtonElement | null
      if (!btn) return { found: false }
      btn.scrollIntoView({ block: 'center' })
      const cs = getComputedStyle(btn)
      return {
        found: true,
        disabled: btn.disabled,
        opacity: cs.opacity,
        label: btn.textContent?.trim(),
        color: cs.color,
        bg: cs.backgroundColor,
      }
    })
    console.log('W3 RUN BTN:', JSON.stringify(m))
    await browser.pause(600)
    await browser.saveScreenshot('/tmp/rimloc-int/shots/aitranslate-dark-after.png')

    // Settings long page: footer scrim over scrolled content.
    await browser.execute(() => {
      window.location.hash = '#/settings'
    })
    await browser.pause(1200)
    await browser.saveScreenshot('/tmp/rimloc-int/shots/settings-dark-after.png')
  })
})
