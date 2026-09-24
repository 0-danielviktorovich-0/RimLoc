// Shared MOCK build state (W6, lead 027): the ONE simulated build engine that
// both the Build screen and the product tour use. The tour has no second
// engine and no DOM-click tricks — it starts the same run the real button
// starts and awaits the same completion.
//
// Contract:
//   - start() → { token, done }. `done` resolves true only when THIS run
//     actually completed; stale runs (reset/restart mid-flight) resolve false.
//   - reset() cancels the current run: phase back to idle, no completion.
//   - Everything is in-memory mock; no filesystem, no backend.
export type MockBuildPhase = 'idle' | 'building' | 'done';

export const MOCK_BUILD_MS = 1400;

class MockBuildStore {
  phase = $state<MockBuildPhase>('idle');
  /** Monotonic id of the current/last run; bumped by every start/reset. */
  private token = 0;

  /**
   * Start a build run. Returns a completion promise that resolves TRUE only
   * when this exact run reached `done` — a reset/restart makes older runs
   * resolve false so stale completions can never count.
   */
  start(): { token: number; done: Promise<boolean> } {
    if (this.phase === 'building') {
      // A run is already in flight: it, not the caller, owns completion.
      const current = this.token;
      return { token: current, done: Promise.resolve(false) };
    }
    this.token += 1;
    const token = this.token;
    this.phase = 'building';
    const done = new Promise<boolean>((resolve) => {
      setTimeout(() => {
        if (this.token === token && this.phase === 'building') {
          this.phase = 'done';
          resolve(true);
        } else {
          resolve(false);
        }
      }, MOCK_BUILD_MS);
    });
    return { token, done };
  }

  /** Cancel/supersede the current run (Test again, demo reset). */
  reset() {
    this.token += 1;
    this.phase = 'idle';
  }
}

export const buildState = new MockBuildStore();
