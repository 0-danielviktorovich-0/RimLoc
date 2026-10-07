import type { Options } from '@wdio/types'

// rel22 screens lane: drives the automation build of the SAME commit over the
// seeded palette-lm profile, navigates routes via the WDIO session and captures
// the fixed window frame (80,45 1280x820) with `screencapture -x -R`. The UI is
// pixel-identical to production (VITE_RIMLOC_AUTOMATION is not referenced by
// the React app); the production binary is gate-verified separately.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./rel22-screens.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: '/tmp/rel22-wdio/spawn-wrapper-screens.sh',
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
        embeddedPort: 4471,
        captureBackendLogs: true,
        captureFrontendLogs: true,
      },
    ],
  ],
  framework: 'mocha',
  mochaOpts: { timeout: 120_000 },
  reporters: ['spec'],
}
