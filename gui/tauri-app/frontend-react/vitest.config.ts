import { defineConfig } from 'vitest/config'

// Unit lane of the React frontend: the data-safety store semantics
// (locale-scoped durable drafts, full-identity key space) run here against
// a mocked contract client — deterministic, no backend, no disk writes
// outside the OS temp. Native E2E lives in the WDIO suites (frontend-v2
// lane drives the packaged app).
export default defineConfig({
  test: {
    environment: 'happy-dom',
    setupFiles: ['./vitest.setup.ts'],
    include: ['src/**/*.test.ts'],
    // The store module is a singleton with module-level state; tests that
    // need a fresh session call vi.resetModules() in their own setup.
    restoreMocks: true,
  },
})
