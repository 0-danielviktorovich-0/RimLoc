import type { Options } from '@wdio/types'

// Stage-2 AI & provider acceptance journey: packaged automation artifact;
// the launcher owns the isolated profile (RIMLOC_DATA_DIR in the wrapper).
const app = process.env.GATE_B_APP
if (!app) {
  throw new Error('GATE_B_APP is required: absolute path to the automation app wrapper')
}
export const config: Options.Testrunner = {
  runner: 'local', specs: ['./stage7-gate-b.spec.ts'], maxInstances: 1,
  capabilities: [{ browserName: 'tauri', 'tauri:options': { application: app } }],
  logLevel: 'warn', bail: 0,
  services: [['@wdio/tauri-service', { driverProvider: 'embedded', embeddedPort: 4482 }]],
  framework: 'mocha', mochaOpts: { timeout: 420_000 }, reporters: ['spec'],
}
