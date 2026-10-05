import type { Options } from '@wdio/types'

// Restart leg of the LM persistence check: this run relaunches the artifact
// app (fresh WDIO session → новый процесс), поэтому наличие qa-persist здесь
// доказывает персистентность across restart. Cleanup: удаляет qa-persist.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./lm-restart.spec.ts'],
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
        embeddedPort: 4469,
        captureBackendLogs: true,
        captureFrontendLogs: true,
      },
    ],
  ],
  framework: 'mocha',
  mochaOpts: { timeout: 120_000 },
  reporters: ['spec'],
}
