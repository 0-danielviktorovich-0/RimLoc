import type { Options } from '@wdio/types'

// Stage-1 core acceptance journey: packaged automation artifact of merged
// main; the launcher (spawn wrapper) owns the isolated profile contract.
const app = process.env.STAGE1_APP
if (!app) {
  throw new Error('STAGE1_APP is required: absolute path to the automation app wrapper')
}
export const config: Options.Testrunner = {
  runner: 'local', specs: ['./stage1-journey.spec.ts'], maxInstances: 1,
  capabilities: [{ browserName: 'tauri', 'tauri:options': { application: app } }],
  logLevel: 'warn', bail: 0,
  services: [['@wdio/tauri-service', { driverProvider: 'embedded', embeddedPort: 4474 }]],
  framework: 'mocha', mochaOpts: { timeout: 420_000 }, reporters: ['spec'],
}
