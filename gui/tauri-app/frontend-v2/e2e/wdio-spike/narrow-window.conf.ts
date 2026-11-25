import type { Options } from '@wdio/types'

// T6 narrow-window lane (#161): the REAL packaged automation build driven
// at the minimum window 980x640 (RIMLOC_WINDOW_FRAME env hook in main.rs,
// feature automation-bridge — GATE_B: browser.setViewportSize is a browser
// API and does NOT resize a native Tauri window). Background-only, zero
// activations; every check is a measured DOM fact (getBoundingClientRect,
// precedent react-geometry UI-04 25/25) plus native CGWindowList captures.
// The report lands in $T6_EVIDENCE/narrow-window-report.json.
// The spawn wrapper resolves from the CURRENT checkout (codex review #174):
// narrow-window-run.sh exports T6_SPAWN_WRAPPER from its own $REPO, so the
// documented one-command reproduction works on any machine. Same contract as
// a3-broker.conf.ts (A3_WRAPPER): no machine-specific defaults in-tree.
const wrapper = process.env.T6_SPAWN_WRAPPER
if (!wrapper || !wrapper.startsWith('/')) {
  throw new Error(
    'T6_SPAWN_WRAPPER must name the spawn wrapper (absolute path); run the lane via narrow-window-run.sh, which exports it from the checkout',
  )
}
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./narrow-window.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: wrapper,
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
