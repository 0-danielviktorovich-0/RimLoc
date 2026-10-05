// Embedded WDIO spike — background semantic drive of the real app.
// Frontier checklist: session → Home semantically → navigate → text input →
// tab switch → scroll → geometry → backend op → close.
// Non-interference is asserted by the OUTER monitor (frontmost/pointer/
// clipboard sampled during the run), not from inside the spec.
// Note: RimLoc buttons carry labels in aria-label (textContent empty),
// discovered via charCode dump 30.09 — all matching below is aria-first.

describe('embedded WDIO spike — background drive', () => {
  it('drives the real app without activation', async () => {
    // --- 1. Home, semantically (h1 heading text) ---
    const homeH1 = await browser.$('h1=RimLoc — переводы модов RimWorld')
    await homeH1.waitForExist({ timeout: 20_000 })
    console.log('[spike] home h1 ok')

    // --- 2. Navigate: Settings (aria-label) ---
    const settingsBtn = await browser.$('button[aria-label="Настройки"]')
    await settingsBtn.click()
    const settingsH1 = await browser.$('h1=Настройки')
    await settingsH1.waitForExist({ timeout: 10_000 })
    console.log('[spike] navigate → settings ok')

    // --- 3. Back Home (strict button match; hash-route fallback) ---
    const homeBtn = await browser.$('button[aria-label="На главную"]')
    if (await homeBtn.isExisting()) {
      await homeBtn.click()
    } else {
      console.log('[spike] WARN: back-home button not found, hash fallback')
      await browser.execute(() => {
        location.hash = '#/home'
      })
    }
    let backHome = true
    try {
      await homeH1.waitForExist({ timeout: 5_000 })
    } catch {
      backHome = false
      await browser.execute(() => {
        location.hash = '#/home'
      })
      await homeH1.waitForExist({ timeout: 5_000 })
    }
    console.log('[spike] navigate → home ok (direct click:', backHome, ')')

    // --- 4. Text input via native setter + InputEvent (Svelte-visible) ---
    const inputResult = await browser.execute(() => {
      const input = (() => {
        const direct = document.querySelector<HTMLInputElement>('input[aria-label="Папка мода"]')
        if (direct) return direct
        const labels = Array.from(document.querySelectorAll('label'))
        const lbl = labels.find((l) => (l.textContent ?? '').includes('Папка мода'))
        if (lbl?.htmlFor) return document.getElementById(lbl.htmlFor) as HTMLInputElement | null
        return lbl ? (lbl.querySelector('input') as HTMLInputElement | null) : null
      })()
      if (!input) return { found: false }
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!
      setter.call(input, '/tmp/spike-folder-input-probe')
      input.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText' }))
      return { found: true, domValue: input.value }
    })
    console.log('[spike] text input:', JSON.stringify(inputResult))
    if (inputResult.found !== true || inputResult.domValue !== '/tmp/spike-folder-input-probe') {
      console.log('[spike] WARN: folder input not driven (not found or value mismatch)')
    }

    // --- 5. Open the selfloc workspace (first recent project) ---
    let opened = false
    for (const sel of ['button[aria-label="Открыть"]', 'button=Открыть']) {
      const b = await browser.$(sel)
      if (await b.isExisting()) {
        await b.click()
        opened = true
        break
      }
    }
    if (!opened) {
      await browser.execute(() => {
        const b = Array.from(document.querySelectorAll('button')).find((x) =>
          (x.textContent ?? '').trim() === 'Открыть',
        )
        b?.click()
      })
    }
    const editorH1 = await browser.$('h1=Редактор перевода')
    await editorH1.waitForExist({ timeout: 30_000 })
    console.log('[spike] project open ok')

    // --- 6. Tab switch: Review («Проверка») ---
    const reviewTab = await browser.$('button[aria-label="Проверка"], [role="radio"][aria-label="Проверка"]')
    if (await reviewTab.isExisting()) {
      await reviewTab.click()
    } else {
      await browser.execute(() => {
        const el = Array.from(document.querySelectorAll<HTMLElement>('*')).find(
          (x) => (x.getAttribute('role') === 'radio' || x.tagName === 'BUTTON') &&
                 ((x.textContent ?? '').trim() === 'Проверка' || x.getAttribute('aria-label') === 'Проверка'),
        )
        el?.click()
      })
    }
    await browser.pause(600)
    console.log('[spike] tab switch → review attempted')

    // --- 7. Scroll: find the tallest scroll container, drive scrollTop ---
    const scrollProbe = await browser.execute(() => {
      const els = Array.from(document.querySelectorAll<HTMLElement>('*'))
      return els
        .filter((el) => el.scrollHeight > el.clientHeight + 40)
        .map((el) => ({
          tag: el.tagName,
          cls: String(el.className).slice(0, 60),
          scrollHeight: el.scrollHeight,
          clientHeight: el.clientHeight,
        }))
        .sort((a, b) => b.scrollHeight - a.scrollHeight)
        .slice(0, 5)
    })
    console.log('[spike] scrollable containers:', JSON.stringify(scrollProbe))
    if (scrollProbe.length > 0) {
      const scrolled = await browser.execute(() => {
        const el = Array.from(document.querySelectorAll<HTMLElement>('*')).find(
          (e) => e.scrollHeight > e.clientHeight + 40,
        )!
        el.scrollTop = 200
        return { scrollTop: el.scrollTop, scrollHeight: el.scrollHeight, clientHeight: el.clientHeight }
      })
      if (scrolled.scrollTop < 100) {
        throw new Error(`scroll did not apply: ${JSON.stringify(scrolled)}`)
      }
      console.log('[spike] scroll ok:', JSON.stringify(scrolled))
    }

    // --- 8. Geometry: viewport vs document (blank-tail invariant inputs) ---
    const geo = await browser.execute(() => ({
      innerH: window.innerHeight,
      innerW: window.innerWidth,
      docH: document.documentElement.scrollHeight,
      bodyScrollW: document.body.scrollWidth,
      hOverflow: document.documentElement.scrollWidth > document.documentElement.clientWidth + 1,
    }))
    console.log('[spike] geometry:', JSON.stringify(geo))

    // --- 9. DOM sanity + current route ---
    const href = await browser.execute(() => window.location.href)
    console.log('[spike] href:', href)

    // --- 10. Backend op through the live Tauri bridge ---
    try {
      const projects = await browser.tauri.execute(async (tauri) => {
        const list = await tauri.core.invoke('project_list')
        return Array.isArray(list) ? { count: list.length } : { raw: String(list).slice(0, 120) }
      })
      console.log('[spike] backend op project_list ok:', JSON.stringify(projects))
    } catch (e) {
      console.log('[spike] backend op FAILED:', String(e).slice(0, 200))
    }
  })
})
