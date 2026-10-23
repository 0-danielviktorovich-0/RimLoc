/**
 * Local OpenAI-compatible mock server for AI-acceptance lanes.
 * - POST /v1/chat/completions: parses the records JSON from the user
 *   message and answers {"results":[{"id","translation":"[MOCK] "+source}]}
 * - MOCK_DELAY_MS: delays EVERY completion response (cancel lane)
 * - GET /stats and POST /reset: request counter for the cancel
 *   no-further-calls assertion (out-of-band, never called by the app)
 * Fixed port 18100 (success) / 18101 (slow) via argv.
 */
import { createServer } from 'node:http'
import { writeFileSync } from 'node:fs'

// No caller-supplied configuration at all: the lane profile is a fixed
// constant table (18100 = instant success, 18101 = delayed for cancel),
// so no user-provided path or duration ever reaches the server.
const PORT = parseInt(process.argv[2] ?? '18100', 10)
const PROFILE: Record<number, { counterFile: string; delayMs: number }> = {
  18100: { counterFile: '/tmp/rimloc-ai/counter-success.json', delayMs: 0 },
  18101: { counterFile: '/tmp/rimloc-ai/counter-slow.json', delayMs: 6000 },
}
const profile = PROFILE[PORT]
if (!profile) {
  throw new Error('mock-openai: unsupported port ' + PORT + ' (use 18100 or 18101)')
}
const counterFile = profile.counterFile
const delayMs = profile.delayMs
let requests = 0

function persist(): void {
  try { writeFileSync(counterFile, JSON.stringify({ requests, ts: Date.now() })) } catch { /* ok */ }
}

createServer((req, res) => {
  if (req.method === 'GET' && req.url === '/stats') {
    res.writeHead(200, { 'content-type': 'application/json' })
    res.end(JSON.stringify({ requests }))
    return
  }
  if (req.method === 'POST' && req.url === '/reset') {
    requests = 0; persist()
    res.writeHead(200, { 'content-type': 'application/json' })
    res.end('{"reset":true}')
    return
  }
  if (req.method === 'POST' && req.url?.endsWith('/chat/completions')) {
    requests += 1; persist()
    let body = ''
    req.on('data', (c) => { body += c })
    req.on('end', () => {
      let results: { id: string; translation: string }[] = []
      try {
        const parsed = JSON.parse(body)
        const user = String(parsed?.messages?.[1]?.content ?? '')
        const arrStart = user.indexOf('[')
        const arrEnd = user.lastIndexOf(']')
        const records = JSON.parse(user.slice(arrStart, arrEnd + 1))
        results = records.map((r: { id: string; source: string }) => ({
          id: r.id, translation: '[MOCK] ' + String(r.source ?? ''),
        }))
      } catch { results = [] }
      const answer = () => {
        const content = JSON.stringify({ results })
        try { writeFileSync('/tmp/rimloc-ai/last-exchange.json', JSON.stringify({ requestPreview: body.slice(0, 1200), results, content: content.slice(0, 800) }, null, 1)) } catch {}
        res.writeHead(200, { 'content-type': 'application/json' })
        res.end(JSON.stringify({ choices: [{ message: { content } }] }))
      }
      if (delayMs > 0) setTimeout(answer, delayMs)
      else answer()
    })
    return
  }
  res.writeHead(404); res.end('{"error":"not found"}')
}).listen(port, '127.0.0.1', () => {
  console.log(`mock-openai listening on 127.0.0.1:${port} delay=${delayMs}ms counter=${counterFile}`)
  persist()
})
