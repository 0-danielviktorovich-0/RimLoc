import type { Options } from '@wdio/types'

// React-lane geometry probes (§50): deterministic layout invariants against
// the REAL app (automation bundle, isolated data). Background-only.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./react-geometry.spec.ts'],
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
