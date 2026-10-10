import type { Options } from '@wdio/types'

// r4-e2e phase 3: §5 re-run (persist-aware) + §7 re-run (error capture) —
// fresh app instance, same data dir. TEMPORARY — not committed.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./r4-e2e-phase3.spec.ts'],
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
