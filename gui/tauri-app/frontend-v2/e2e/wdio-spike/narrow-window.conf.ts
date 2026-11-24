import type { Options } from '@wdio/types'

// T6 narrow-window lane (#161): the REAL packaged automation build driven
// at the minimum window 980x640 (RIMLOC_WINDOW_FRAME env hook in main.rs,
// feature automation-bridge — GATE_B: browser.setViewportSize is a browser
// API and does NOT resize a native Tauri window). Background-only, zero
// activations; every check is a measured DOM fact (getBoundingClientRect,
// precedent react-geometry UI-04 25/25) plus native CGWindowList captures.
// The report lands in $T6_EVIDENCE/narrow-window-report.json.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./narrow-window.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application:
          '/Users/danielviktorovich/Developing/_rimloc-worktrees/t6-window/testlab/ui_automation/wdio/spawn-wrapper-t6.sh',
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
        embeddedPort: 4475,
        captureBackendLogs: true,
        captureFrontendLogs: true,
      },
    ],
  ],
  framework: 'mocha',
  mochaOpts: { timeout: 300_000 },
  reporters: ['spec'],
}
