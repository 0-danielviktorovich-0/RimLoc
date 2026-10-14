import type { Options } from '@wdio/types'

// Draft-safety native E2E (rel26-draft-fixes), phase 1: live-session leg —
// collision rows, draft independence, locale-switch slice survival. The
// restart leg lives in drafts-restart.spec.ts (fresh WDIO session = new app
// process). Artifact: the rel26 production .app via the ds spawn wrapper.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./drafts-safety.spec.ts'],
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
  mochaOpts: { timeout: 180_000 },
  reporters: ['spec'],
}
