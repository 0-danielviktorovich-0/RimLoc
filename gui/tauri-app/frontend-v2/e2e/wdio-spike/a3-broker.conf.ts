import type { Options } from '@wdio/types'

// A3 concurrency journey: the REAL packaged automation build hosts the GUI
// broker; an EXTERNAL rimloc-mcp process connects in broker-client mode and
// applies translations while a human-like WDIO session watches the UI.
// Isolated profile /tmp/rimloc-a3 (wrapper env); owner data untouched.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./a3-broker.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: '/tmp/rimloc-a3/spawn-wrapper-a3.sh',
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
        embeddedPort: 4479,
        captureBackendLogs: true,
        captureFrontendLogs: true,
      },
    ],
  ],
  framework: 'mocha',
  mochaOpts: { timeout: 240_000 },
  reporters: ['spec'],
}
