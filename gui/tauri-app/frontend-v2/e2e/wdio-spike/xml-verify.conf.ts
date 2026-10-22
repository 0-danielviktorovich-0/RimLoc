import type { Options } from '@wdio/types'
export const config: Options.Testrunner = {
  runner: 'local', specs: ['./xml-verify.spec.ts'], maxInstances: 1,
  capabilities: [{ browserName: 'tauri', 'tauri:options': { application: '/tmp/rimloc-int/spawn-wrapper-int.sh' } }],
  logLevel: 'warn', bail: 0,
  services: [['@wdio/tauri-service', { driverProvider: 'embedded', embeddedPort: 4473 }]],
  framework: 'mocha', mochaOpts: { timeout: 300_000 }, reporters: ['spec'],
}
