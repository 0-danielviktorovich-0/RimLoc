// Shared UI-interaction helpers for the draft-safety native specs.
// ONE definition per interaction — the phase-1/phase-2 oracle drift
// (review P0-1: the restart spec kept the broken synthetic option-click
// while phase 1 was fixed) is exactly what this module prevents.
// Every switch ASSERTS that the select actually changed; a silent no-op
// fails here, never downstream.

function pause(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

/** Switch the workspace target locale and PROVE it switched. React
 *  controlled <select> ignores synthetic WebDriver option clicks — use the
 *  native value setter + a bubbling change event. */
export async function switchTargetLocale(
  browser: WebdriverIO.Browser,
  locale: string,
): Promise<void> {
  await browser.execute((loc: string) => {
    const sel = document.querySelector('[data-testid="ws.target-locale"]') as HTMLSelectElement
    if (!sel) throw new Error('locale select not found')
    const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value')!.set!
    setter.call(sel, loc)
    sel.dispatchEvent(new Event('change', { bubbles: true }))
  }, locale)
  await pause(600)
  const value = await browser
    .$('[data-testid="ws.target-locale"]')
    .getValue()
  if (value !== locale) {
    throw new Error(`locale switch failed: expected ${locale}, select still ${value}`)
  }
}

/** Commit the CURRENTLY selected row's draft through the editor save
 *  button. Settle criterion = THE DURABLE TRUTH: the managed envelope on
 *  disk gains the expected (locale, text) translation. The button's busy
 *  state is reported separately — a stuck busy pill with a persisted
 *  envelope is a UI finding, not a data-safety failure. */
export async function commitCurrentRow(
  browser: WebdriverIO.Browser,
  expect: { locale: string; text: string },
): Promise<void> {
  const fs = await import('node:fs')
  const mgr = '/tmp/rimloc-ds/data/managed'
  const before = fs.readdirSync(mgr).filter((f) => f.endsWith('.rimloc.json')).length
  const save = await browser.$('[data-testid="ws.editor-save"]')
  if (!(await save.isExisting())) throw new Error('editor save button not found')
  const btnBefore = await save.getAttribute('disabled')
  if (btnBefore !== null && btnBefore !== 'false') {
    throw new Error(`save button disabled before the click (dirty state lost?)`)
  }
  await save.click()
  let envelopeOk = false
  let btnStuck: string | null = null
  for (let i = 0; i < 100; i++) {
    await pause(200)
    const files = fs.readdirSync(mgr).filter((f) => f.endsWith('.rimloc.json'))
    if (files.length >= before) {
      for (const f of files) {
        try {
          const env = JSON.parse(fs.readFileSync(`${mgr}/${f}`, 'utf8'))
          const hit = (env.translations ?? []).some(
            (t: { locale: string; text: string }) =>
              t.locale === expect.locale && t.text === expect.text,
          )
          if (hit) envelopeOk = true
        } catch { /* mid-write */ }
      }
    }
    if (envelopeOk) {
      btnStuck = await save.getAttribute('disabled')
      break
    }
  }
  if (!envelopeOk) {
    const err = await browser.$('.ws-error').getText().catch(() => '<no .ws-error>')
    throw new Error(
      `commit never reached the envelope: lastError=${JSON.stringify(err)}`,
    )
  }
  // The envelope holds the translation. If the editor keeps a disabled
  // save button, note it — the evidence file reports this honestly.
  if (btnStuck !== null && btnStuck !== 'false') {
    await browser.execute((v: string) => {
      const w = window as unknown as { __ds_ui_stuck?: string }
      w.__ds_ui_stuck = v
    }, btnStuck)
  }
  await pause(400)
}

export { pause }
