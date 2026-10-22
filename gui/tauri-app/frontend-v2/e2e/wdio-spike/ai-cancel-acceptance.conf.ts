import type { Options } from '@wdio/types'

// AI-cancel acceptance native E2E (review 2026-10-14 P2): the REAL packaged
// automation build (React R1 + automation bridge) drives the AI job journey
// over actual Tauri IPC — start, cancel, terminal Cancelled, stale-revision
// refusal — plus the production gating checks (mock mode absent from the
// UI, errors visible in the AI screen). Isolated profile: RIMLOC_DATA_DIR
// points at /tmp/rimloc-ai/data (spawn wrapper); no owner data involved.
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./ai-cancel-acceptance.spec.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'tauri',
      'tauri:options': {
        application: '/tmp/rimloc-ds/spawn-wrapper-ai.sh',
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
        embeddedPort: 4471,
        captureBackendLogs: true,
        captureFrontendLogs: true,
      },
    ],
  ],
  framework: 'mocha',
  mochaOpts: { timeout: 240_000 },
  reporters: ['spec'],
}
