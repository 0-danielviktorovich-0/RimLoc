import type { Options } from '@wdio/types'

// Real-provider E2E lane (directive §4): Ollama (qwen2.5:3b, local,
// http://localhost:11434/v1) through the FULL graphical workflow —
// provider instance (IPC upsert, no key needed), BYO selection on the
// AI screen, job start, live preview from real model calls, per-row
// selection, accept-selected, durable translation on disk.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./ollama-e2e.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: '/tmp/rimloc-int/spawn-wrapper-int.sh',
      },
    },
  ],
  logLevel: 'warn',
  bail: 0,
  services: [
    [
      '@wdio/tauri-service',
      {
        driverProvider: 'embedded',
        embeddedPort: 4473,
        captureBackendLogs: true,
        captureFrontendLogs: true,
      },
    ],
  ],
  framework: 'mocha',
  mochaOpts: { timeout: 300_000 },
  reporters: ['spec'],
}
