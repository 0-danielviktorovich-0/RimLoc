import type { Options } from '@wdio/types'

// Large-mod E2E lane (directive §12): the 12k-row corpus on the REAL
// packaged build.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./large-mod.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': { application: '/tmp/rimloc-int/spawn-wrapper-int.sh' },
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
  mochaOpts: { timeout: 600_000 },
  reporters: ['spec'],
}
