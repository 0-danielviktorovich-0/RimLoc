import type { Options } from '@wdio/types'

// A3 concurrency journey: the REAL packaged automation build hosts the GUI
// broker; an EXTERNAL rimloc-mcp process connects in broker-client mode and
// applies translations while a human-like WDIO session watches the UI.
// The profile is a FRESH per-run directory: the launcher exports A3_PROFILE
// (mkdtemp), the spawn wrapper serves RIMLOC_DATA_DIR=$A3_PROFILE/data from
// it, and the spec reads the same variable — no shared predictable path.
// The wrapper script path itself comes from A3_WRAPPER (owner-controlled
// tooling location); owner data untouched.
const wrapper = process.env.A3_WRAPPER
if (!wrapper || !wrapper.startsWith('/')) {
  throw new Error(
    'A3_WRAPPER must name the spawn wrapper (absolute path); A3_PROFILE must be a fresh per-run mkdtemp directory',
  )
}
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./a3-broker.spec.ts'],
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
