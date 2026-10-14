import type { Options } from '@wdio/types'

// Unsaved-drafts restart leg (review iteration 2 P0-1): FOUR uncommitted
// drafts across TWO locales must survive a real app restart. This conf runs
// TWICE: run 1 = drafts-unsaved-setup.spec (creates drafts, ends on uk,
// NO commits), run 2 = drafts-unsaved-verify.spec (fresh process: uk
// persisted, uk drafts raised, switch back to ru, all four verified).
// SPEC=<name> env selects the spec.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: [`./${process.env.SPEC ?? 'drafts-unsaved-setup.spec.ts'}`],
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
  mochaOpts: { timeout: 300_000 },
  reporters: ['spec'],
}
