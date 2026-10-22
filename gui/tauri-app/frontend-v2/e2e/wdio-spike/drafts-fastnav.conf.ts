import type { Options } from '@wdio/types'

// Fast-navigation + write-failure leg (review P0-3): a debounced draft must
// survive an IMMEDIATE project switch, and a failing durable write must
// show an honest error instead of a saved state.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./drafts-fastnav.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: '/tmp/rimloc-ds/spawn-wrapper-ds.sh',
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
  mochaOpts: { timeout: 300_000 },
  reporters: ['spec'],
}
