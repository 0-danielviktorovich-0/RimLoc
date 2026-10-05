import type { Options } from '@wdio/types'

// Language Manager acceptance (§10/§32): same isolated lane as the palette
// spec — artifact app, own port 4469, RIMLOC_DATA_DIR=/tmp/palette-lm/data.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./lm-acceptance.spec.ts'],
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
