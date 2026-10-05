import type { Options } from '@wdio/types'

// perf-v2 (mandate §7): Svelte artifact rel15-automation, isolated RIMLOC_DATA_DIR in /tmp/perf-v2.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./perf-v2.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: '/tmp/perf-v2/wrapper-svelte.sh',
      },
    },
  ],
  logLevel: 'warn',
  bail: 0,
  connectionTimeout: 30000,
  services: [
    [
      '@wdio/tauri-service',
      { driverProvider: 'embedded', embeddedPort: 4457, captureBackendLogs: true, captureFrontendLogs: true },
    ],
  ],
  framework: 'mocha',
  mochaOpts: { timeout: 110_000 },
  reporters: ['spec'],
}
