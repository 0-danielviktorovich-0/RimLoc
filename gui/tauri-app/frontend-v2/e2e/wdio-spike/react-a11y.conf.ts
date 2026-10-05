import type { Options } from '@wdio/types'

// React-lane a11y probes (§56): programmatic evidence, semantic channel.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./react-a11y.spec.ts'],
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
  mochaOpts: { timeout: 180_000 },
  reporters: ['spec'],
}
