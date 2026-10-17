import type { Options } from '@wdio/types'

// Integration-candidate UI verification lane: the REAL packaged automation
// build of the INTEGRATION branch (React-default cutover + UI fixes),
// isolated profile at /tmp/rimloc-int/data.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./integration-ui.spec.ts'],
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
