import type { Options } from '@wdio/types'
export const config: Options.Testrunner = {
  runner: 'local',
  specs: ['./canary-export.spec.ts'],
  maxInstances: 1,
  capabilities: [{ browserName: 'tauri', 'tauri:options': { application: '/tmp/rimloc-rw-lab/canary-wrapper.sh' } }],
  logLevel: 'warn', bail: 0,
  services: [['@wdio/tauri-service', { driverProvider: 'embedded', embeddedPort: 4481, captureBackendLogs: false, captureFrontendLogs: false }]],
  framework: 'mocha', mochaOpts: { timeout: 300_000 }, reporters: ['spec'],
}
