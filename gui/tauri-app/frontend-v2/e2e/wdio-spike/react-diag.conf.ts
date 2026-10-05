import type { Options } from '@wdio/types'

// React-lane (R1) smoke: drives the REAL built app (react automation bundle)
// over the embedded WDIO channel. Isolated RIMLOC_DATA_DIR via the wrapper
// env (caller sets it) — owner data untouched. Harness lives here because
// the @wdio deps are installed in frontend-v2; files are additive react-*.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./react-diag.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application:
          '/Users/danielviktorovich/Developing/_rimloc-worktrees/wt-ui-r1/testlab/ui_automation/wdio/spawn-wrapper-react.sh',
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
        embeddedPort: 4457,
        captureBackendLogs: true,
        captureFrontendLogs: true,
      },
    ],
  ],
  framework: 'mocha',
  mochaOpts: { timeout: 120_000 },
  reporters: ['spec'],
}
