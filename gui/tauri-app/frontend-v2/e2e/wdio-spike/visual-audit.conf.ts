import type { Options } from '@wdio/types'

// UI-12 deep visual audit lane: the REAL packaged automation build walked
// across every surface in BOTH themes with structural DOM metrics — text
// contrast (WCAG ratios computed in-page, oklch-aware), checkbox
// visibility, table alignment, horizontal overflow. No screenshots: every
// check is a computed fact, and the full report lands in
// /tmp/rimloc-int/visual-audit-report.json as the review artifact.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./visual-audit.spec.ts'],
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
