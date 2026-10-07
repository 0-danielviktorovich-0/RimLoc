import type { Options } from '@wdio/types'

// Chat-batch E2E lane (rel22): the FULL user journey through the real UI —
// select → create → export prompt → paste response → strict import → preview
// → apply → translations land in the workspace. Drives the automation build
// over the seeded palette-lm profile (same contract as palette-acceptance).
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./chatbatch-acceptance.spec.ts'],
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
