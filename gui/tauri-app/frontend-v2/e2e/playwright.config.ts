// T2a — semantic autonomous acceptance of the frontend (wave 10).
// Architecture: docs/development/testing/AUTONOMOUS_ACCEPTANCE_ARCHITECTURE.md
//   - headless Chromium ONLY (same-host non-interference: the owner's pointer,
//     focus, Spaces and clipboard are never touched — headless:true is a hard
//     contract here, never override to false);
//   - the real frontend codebase on the vite dev server (mock transport is the
//     explicit dev default: import.meta.env.DEV → devMode → client mode 'mock');
//   - semantic-first: ARIA role / accessible name / data-testid / DOM state,
//     geometry only via boundingBox (no screenshots, no pixel diffs — T5).
// Run manually from gui/tauri-app/frontend-v2: npm run test:e2e (CI-independent).
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './invariants',
  // Trace/screenshot are diagnostics for FAILURES only — no visual regression
  // in this wave (T5 owns that). retries>=1 makes 'on-first-retry' traces land.
  retries: 1,
  timeout: 30_000,
  expect: { timeout: 7_000 },
  fullyParallel: true,
  reporter: [['list'], ['html', { open: 'never', outputFolder: '../test-results/e2e-report' }]],
  outputDir: '../test-results/e2e-artifacts',
  use: {
    headless: true, // NEVER headless:false — same-host non-interference (mandate §4)
    // localhost, not 127.0.0.1: vite 7 binds the IPv6 loopback on this host.
    baseURL: 'http://localhost:5199',
    viewport: { width: 1280, height: 720 },
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    video: 'off'
  },
  projects: [{ name: 'chromium', use: { browserName: 'chromium' } }],
  webServer: {
    command: 'npm run dev -- --port 5199 --strictPort',
    url: 'http://localhost:5199',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000
  }
});
