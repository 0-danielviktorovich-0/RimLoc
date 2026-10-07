import type { Options } from '@wdio/types'

// Seed lane (rel22): creates ONE project from /tmp/rimloc-wizard-mod in the
// isolated palette-lm data dir, so acceptance suites start with a live open
// project (rel19-21 profiles carried accumulated state; virgin dirs show
// no-project stubs on TM/ChatBatch and the route markers rightly fail).
// Same wrapper/port/data dir as palette-acceptance.conf.ts.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./seed-project.spec.ts'],
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
