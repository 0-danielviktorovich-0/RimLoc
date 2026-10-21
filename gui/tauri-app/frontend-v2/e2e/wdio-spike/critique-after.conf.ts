import type { Options } from '@wdio/types'

// AFTER-captures for the critique round-2 fixes: compressed workspace hero,
// legible disabled primary button, footer scrim. Same isolated profile and
// wrapper as the visual-audit lane.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./critique-after.spec.ts'],
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
  mochaOpts: { timeout: 240_000 },
  reporters: ['spec'],
}
