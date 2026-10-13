import { defineConfig } from 'vitest/config'

// Full-production React coverage is deliberately honest: untested screens
// appear as 0% rather than disappearing from the denominator. State hotspots
// (project.ts/entryUid.ts) are reported individually, not used as a proxy for
// coverage of the entire frontend.
export default defineConfig({
  test: {
    environment: 'happy-dom',
    setupFiles: ['./vitest.setup.ts'],
    include: ['src/**/*.test.ts'],
    // The store module is a singleton with module-level state; tests that
    // need a fresh session call vi.resetModules() in their own setup.
    restoreMocks: true,
    coverage: {
      provider: 'v8',
      include: ['src/**/*.{ts,tsx}'],
      // Keep Vitest defaults excluding tests, .d.ts and generated files.
      reporter: ['text', 'json', 'lcov'],
      reportsDirectory: 'coverage',
    },
  },
})
