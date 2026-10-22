import type { Options } from '@wdio/types'

/**
 * Portable critical-path config (REVIEW_180e5344 action 2): the application
 * under test comes from RIMLOC_XML_VERIFY_APP (absolute path to the app
 * binary or a spawn wrapper that execs it). The spec generates its own
 * fixture mod and profile, so no other machine-specific input exists.
 */
const app = process.env.RIMLOC_XML_VERIFY_APP
if (!app) {
  throw new Error(
    'RIMLOC_XML_VERIFY_APP is required: set it to the RimLoc app binary (or spawn wrapper) to test',
  )
}

export const config: Options.Testrunner = {
  runner: 'local', specs: ['./xml-verify.spec.ts'], maxInstances: 1,
  capabilities: [{ browserName: 'tauri', 'tauri:options': { application: app } }],
  logLevel: 'warn', bail: 0,
  services: [['@wdio/tauri-service', { driverProvider: 'embedded', embeddedPort: 4473 }]],
  framework: 'mocha', mochaOpts: { timeout: 300_000 }, reporters: ['spec'],
}
