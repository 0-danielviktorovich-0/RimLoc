import type { Options } from '@wdio/types'

// Source Inspector E2E lane (C2): the workspace SOURCE block must show the
// REAL source of each row from the live contract snapshot — project-relative
// file, parser-guaranteed line, winner reason (selected_by), mod root —
// asserted via data-testid="src.*". Drives the automation build over the
// seeded palette-lm profile (same contract as chatbatch-acceptance).
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./source-inspector.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: '/tmp/palette-lm/spawn-wrapper-palette.sh',
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
  mochaOpts: { timeout: 120_000 },
  reporters: ['spec'],
}
