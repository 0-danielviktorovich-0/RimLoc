import type { Options } from '@wdio/types'

// React-lane stress (§27): 10k+ entries through the REAL create+list.
// Isolated data dir; measured scroll/selection latencies recorded in the log.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./react-stress.spec.ts'],
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
      { driverProvider: 'embedded', embeddedPort: 4457, captureBackendLogs: true, captureFrontendLogs: true },
    ],
  ],
  framework: 'mocha',
  mochaOpts: { timeout: 300_000 },
  reporters: ['spec'],
}
