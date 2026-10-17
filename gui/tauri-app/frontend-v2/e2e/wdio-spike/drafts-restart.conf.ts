import type { Options } from '@wdio/types'

// Draft-safety restart leg: this run relaunches the artifact app (fresh WDIO
// session = new process); the drafts sidecar in the managed root and the
// webview localStorage (per-project target locale) must restore the work.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./drafts-restart.spec.ts'],
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
