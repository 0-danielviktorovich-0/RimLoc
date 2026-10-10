import type { Options } from '@wdio/types'

// r4-e2e phase 2: fresh app instance (same wrapper, same data dir) —
// persist-after-restart + checks cycle + build/export + chat-batch.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./r4-e2e-phase2.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: '/tmp/r4-e2e/spawn-wrapper-r4.sh',
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
        embeddedPort: 4469,
        captureBackendLogs: true,
        captureFrontendLogs: true,
      },
    ],
  ],
  framework: 'mocha',
  mochaOpts: { timeout: 180_000 },
  reporters: ['spec'],
}
