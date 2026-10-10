import type { Options } from '@wdio/types'

// r4-e2e (mandate §4): independent real-mods E2E of the rel22 build.
// SAME contract as chatbatch-acceptance.conf.ts: embedded WDIO port 4469,
// own virgin profile RIMLOC_DATA_DIR=/tmp/r4-e2e/data via the r4 wrapper.
// TEMPORARY spec lane — not committed.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./r4-e2e-phase1.spec.ts'],
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
        embeddedPort: 4469, // the wrapper pins TAURI_WEBDRIVER_PORT=4469
        captureBackendLogs: true,
        captureFrontendLogs: true,
      },
    ],
  ],
  framework: 'mocha',
  mochaOpts: { timeout: 180_000 },
  reporters: ['spec'],
}
