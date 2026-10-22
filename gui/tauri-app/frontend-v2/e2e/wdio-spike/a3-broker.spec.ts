/**
 * A3 — GUI↔MCP concurrency through the live broker (issue #10).
 *
 * Preconditions (orchestrator): A3_PROFILE env pointing at a FRESH
 * per-run directory (mkdtemp — no shared predictable /tmp path); the
 * wrapper starts the packaged GUI on RIMLOC_DATA_DIR=$A3_PROFILE/data →
 * the GUI hosts the broker socket. A SECOND rimloc-mcp process connects
 * in broker-client mode (A3_MCP_BIN env) while a human-like WDIO session
 * works in the GUI.
 *
 * The scenario uses only real product paths: the user creates the project
 * through the wizard and edits like a human; the external agent applies
 * through the broker; the UI must reflect the external change WITHOUT user
 * action, the user's unsaved draft must survive, and a stale external apply
 * must be refused typed.
 */
import { spawn, type ChildProcess } from 'node:child_process'
import { existsSync, accessSync, constants } from 'node:fs'
import { isAbsolute, join } from 'node:path'
import { pause } from './search-editor.util'

const PROFILE = process.env.A3_PROFILE ?? ''
if (!PROFILE || !isAbsolute(PROFILE)) {
  throw new Error(
    'A3_PROFILE must name a fresh absolute per-run profile directory (mkdtemp); refusing the shared predictable path',
  )
}
const MOD = join(PROFILE, 'mod')
const MCP_BIN = process.env.A3_MCP_BIN ?? ''

class McpClient {
  private proc: ChildProcess
  private pending = new Map<number, (v: any) => void>()
  private nextId = 0
  private buffer = ''

  constructor() {
    if (!MCP_BIN || !isAbsolute(MCP_BIN) || !existsSync(MCP_BIN)) {
      throw new Error(`A3_MCP_BIN not set, not absolute, or missing: ${MCP_BIN}`)
    }
    try {
      accessSync(MCP_BIN, constants.X_OK)
    } catch {
      throw new Error(`A3_MCP_BIN is not executable: ${MCP_BIN}`)
    }
    this.proc = spawn(MCP_BIN, [], {
      env: { ...process.env, RIMLOC_DATA_DIR: join(PROFILE, 'data') },
      stdio: ['pipe', 'pipe', 'pipe'],
    })
    this.proc.stdout!.setEncoding('utf8')
    this.proc.stdout!.on('data', (chunk: string) => {
      this.buffer += chunk
      let idx: number
      while ((idx = this.buffer.indexOf('\n')) >= 0) {
        const line = this.buffer.slice(0, idx)
        this.buffer = this.buffer.slice(idx + 1)
        if (!line.trim()) continue
        const msg = JSON.parse(line)
        const resolve = this.pending.get(msg.id)
        if (resolve) {
          this.pending.delete(msg.id)
          resolve(msg)
        }
      }
    })
    this.rpc('initialize', {
      protocolVersion: '2026-07-28',
      capabilities: {},
      clientInfo: { name: 'a3-wdio', version: '0.0.1' },
    })
    this.proc.stdin.write(
      JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n',
    )
  }

  callTool(name: string, args: any): Promise<any> {
    return this.rpc('tools/call', { name, arguments: args })
  }

  private rpc(method: string, params: any): Promise<any> {
    const id = ++this.nextId
    return new Promise((resolve, reject) => {
      this.pending.set(id, resolve)
      this.proc.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, params }) + '\n')
      setTimeout(() => {
        if (this.pending.has(id)) {
          this.pending.delete(id)
          reject(new Error(`rpc ${method} timed out`))
        }
      }, 60_000)
    })
  }

  kill() {
    this.proc.kill()
  }
}

function structured(result: any): any {
  return result.result?.structuredContent ?? null
}

async function waitExisting(selector: string, timeout = 30000): Promise<WebdriverIO.Element> {
  const el = await browser.$(selector)
  await el.waitForExist({ timeout })
  return el
}

describe('A3 — GUI↔MCP concurrency through the live broker', () => {
  let client: McpClient
  let projectId = ''

  it('external client detects the GUI broker (single writer)', async () => {
    await browser.waitUntil(async () => (await browser.$('.app-sidebar')).isExisting(), {
      timeout: 60000,
      interval: 200,
    })
    client = new McpClient()
    const status = await client.callTool('rimloc.status', {})
    const sc = structured(status)
    console.log('A3 STATUS:', JSON.stringify(sc?.app))
    expect(sc?.app).toBe('broker_gui')
  })

  it('user creates the project through the wizard', async () => {
    await browser.execute(() => {
      window.location.hash = '#/projects'
    })
    await waitExisting('[data-testid="wizard.open"]')
    await browser.$('[data-testid="wizard.open"]').click()
    await waitExisting('[data-testid="wizard.path-input"]')
    await browser.$('[data-testid="wizard.path-input"]').setValue(MOD)
    await browser.$('[data-testid="wizard.next"]').click()
    await waitExisting('[data-testid="wizard.version"]', 60000)
    await browser.$('[data-testid="wizard.version"]').selectByVisibleText('1.6')
    await browser.$('[data-testid="wizard.next"]').click()
    await pause(800)
    const overlay = await browser.$('[data-testid="wizard.overlay"]')
    if (await overlay.isExisting()) {
      await browser.$('[data-testid="wizard.next"]').click()
    }
    await waitExisting('[data-testid="ws.root"]', 300000)
    await waitExisting('.entry-list .entry-row', 60000)
  })

  it('user translates Hello world like a human', async () => {
    const input = await browser.$('.search-field input')
    await input.setValue('Hello world')
    await pause(900)
    await browser.$('.entry-list .entry-row').click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('Привет мир')
    await browser.$('[data-testid="ws.editor-save"]').click()
    let saved = false
    for (let i = 0; i < 150 && !saved; i++) {
      await pause(200)
      saved = await browser.execute(() =>
        Array.from(document.querySelectorAll('.entry-list .entry-row')).some((r) =>
          (r.textContent ?? '').includes('Привет мир'),
        ),
      )
    }
    expect(saved).toBe(true)
  })

  it('user leaves an UNSAVED draft in another row', async () => {
    const input = await browser.$('.search-field input')
    await input.setValue('Goodbye friend')
    await pause(900)
    await browser.$('.entry-list .entry-row').click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
    await browser.$('[data-testid="ws.editor-textarea"]').setValue('ЧЕРНОВИК-ПОЛЬЗОВАТЕЛЯ')
    await pause(2000) // the autosave debounce flushes the DRAFT, not a commit
  })

  it('external agent applies through the broker; the UI refetches without user action', async () => {
    const list = structured(await client.callTool('rimloc.projects.list', {}))
    projectId = list.projects[0].project_id
    const snap = structured(
      await client.callTool('rimloc.project.snapshot', {
        project_id: projectId, session_epoch: 1,
      }),
    )
    expect(snap?.revision).toBeGreaterThan(0)
    console.log('A3 SNAPSHOT rev:', snap.revision, 'epoch:', snap.session_epoch)

    const apply = await client.callTool('rimloc.translation.apply', {
      project_id: projectId,
      session_epoch: snap.session_epoch,
      expected_revision: snap.revision,
      intents: [{
        entry: { kind: 'keyed', key: 'Fixture_Action_Open' },
        locale: 'Russian',
        action: 'set_translation',
        text: 'Внешний открытый люк',
      }],
    })
    console.log('A3 APPLY RAW:', JSON.stringify(apply).slice(0, 300))
    expect(apply.result).toBeDefined()

    // THE A3 PROOF: the external change becomes visible in the GUI without
    // any user interaction (broker-changed → React refetch).
    // React controlled input: clear the filter through the native value
    // setter (the ds-helpers pattern), then search FOR the external text.
    await browser.execute(() => {
      const el = document.querySelector('.search-field input') as HTMLInputElement
      if (!el) throw new Error('search field not found')
      const setter = Object.getOwnPropertyDescriptor(
        HTMLInputElement.prototype, 'value',
      )!.set!
      setter.call(el, 'Внешний')
      el.dispatchEvent(new Event('input', { bubbles: true }))
    })
    await pause(800)
    let visible = false
    for (let i = 0; i < 80 && !visible; i++) {
      visible = await browser.execute(() =>
        Array.from(document.querySelectorAll('.entry-list .entry-row')).some((r) =>
          (r.textContent ?? '').includes('Внешний открытый люк'),
        ),
      )
      if (!visible) await pause(300)
    }
    expect(visible).toBe(true)
  })

  it('the unsaved user draft survived the external change', async () => {
    const input = await browser.$('.search-field input')
    await input.setValue('Goodbye friend')
    await pause(900)
    await browser.$('.entry-list .entry-row').click()
    await waitExisting('[data-testid="ws.editor-textarea"]', 15000)
    const value = await browser.$('[data-testid="ws.editor-textarea"]').getValue()
    console.log('A3 DRAFT AFTER EXTERNAL:', JSON.stringify(value))
    expect(String(value)).toContain('ЧЕРНОВИК-ПОЛЬЗОВАТЕЛЯ')
  })

  it('a stale external apply is refused with a typed error', async () => {
    const stale = await client.callTool('rimloc.translation.apply', {
      project_id: projectId,
      session_epoch: 1,
      expected_revision: 1, // stale: the user save + the external apply bumped it
      intents: [{
        entry: { kind: 'keyed', key: 'Fixture_Action_Open' },
        locale: 'Russian',
        action: 'set_translation',
        text: 'НЕ ДОЛЖНО ПРИМЕНИТЬСЯ',
      }],
    })
    expect(stale.error).toBeDefined()
    const code = String(stale.error?.data?.code ?? '')
    console.log('A3 STALE CODE:', code)
    expect(['stale_revision', 'stale_epoch'].includes(code)).toBe(true)
  })

  it('evidence screenshot', async () => {
    await browser.saveScreenshot(join(PROFILE, 'a3-final-evidence.png'))
  })

  after(() => {
    client?.kill()
  })
})
