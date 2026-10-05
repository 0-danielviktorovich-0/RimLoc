import type { Options } from '@wdio/types'

// Embedded WDIO spike (frontier directive 2026-09-30): zero-activation
// semantic drive of the real Tauri app via the in-app WebDriver server
// (tauri-plugin-wdio-webdriver, gated by RIMLOC_AUTOMATION=1 at runtime).
// Binary: debug build on the warm workspace target (embedded server linked;
// browser.tauri.execute needs the second plugin — deferred until rebuild).
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./soak.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: '/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main/testlab/ui_automation/wdio/spawn-wrapper-release.sh',
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
  mochaOpts: {
    ui: 'bdd',
    timeout: 70 * 60_000,
  },
  reporters: ['spec'],
}
