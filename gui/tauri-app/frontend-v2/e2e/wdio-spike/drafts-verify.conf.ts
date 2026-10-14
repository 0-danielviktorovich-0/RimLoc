import type { Options } from '@wdio/types'

// Draft-safety verification leg: the reopened project, the EXPORT path, and
// the final XML — both structural identities must land in their own Def
// structures of the built mod.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./drafts-verify.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: '/tmp/rimloc-ds/spawn-wrapper-ds.sh',
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
  mochaOpts: { timeout: 240_000 },
  reporters: ['spec'],
}
