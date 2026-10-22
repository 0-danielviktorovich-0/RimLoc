import type { Options } from '@wdio/types'

// Search & editor verification lane (directive §3): the nine user
// scenarios against the REAL packaged automation build — exact key,
// partial key, source text, translated text, def-type and file filters,
// direct navigation to a match, edit + safe save, clear-filters
// selection recovery, and large-project search timing.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./search-editor.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: '/tmp/rimloc-int/spawn-wrapper-int.sh',
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
        embeddedPort: 4473,
        captureBackendLogs: true,
        captureFrontendLogs: true,
      },
    ],
  ],
  framework: 'mocha',
  mochaOpts: { timeout: 240_000 },
  reporters: ['spec'],
}
