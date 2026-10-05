import type { Options } from '@wdio/types'

// Palette acceptance lane (mandate §9): drives the rel17 react ARTIFACT app
// over its embedded WDIO channel. Own port 4469, own isolated data dir —
// the spawn wrapper (see /tmp/palette-lm/spawn-wrapper-palette.sh) sets
// TAURI_WEBDRIVER_PORT=4469 and RIMLOC_DATA_DIR=/tmp/palette-lm/data.
// Zero focus stealing: automation env + noactivate vendor wry/tao.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./palette-acceptance.spec.ts'],
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
