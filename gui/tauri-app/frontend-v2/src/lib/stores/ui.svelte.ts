// UI-level store: per-screen mock states. Screen navigation itself moved to
// router.svelte.ts (hash-based); this store keeps the dev-panel toggles that
// force loading/empty/error on Home and Workspace.
export type SimpleState = 'ready' | 'loading' | 'error';
export type WorkspaceState = 'ready' | 'loading' | 'empty' | 'error';
/** Home renders either the onboarding (first-run) or the recent-projects (returning) view. */
export type HomeMode = 'first-run' | 'returning';

class UiStore {
  homeState = $state<SimpleState>('ready');
  wsState = $state<WorkspaceState>('ready');
  homeMode = $state<HomeMode>('returning');
}

export const ui = new UiStore();
