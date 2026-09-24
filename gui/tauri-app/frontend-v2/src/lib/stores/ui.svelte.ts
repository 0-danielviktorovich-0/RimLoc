// UI-level store: per-screen mock states. Screen navigation itself moved to
// router.svelte.ts (hash-based); this store keeps the dev-panel toggles that
// force loading/empty/error on Home and Workspace.
export type SimpleState = 'ready' | 'loading' | 'error';
export type WorkspaceState = 'ready' | 'loading' | 'empty' | 'error';
/** Home renders onboarding (first-run), recent projects (returning) or the
 * no-mods empty state with Try Demo (W6, MOCK_LIVE_ONBOARDING_MANDATE §9). */
export type HomeMode = 'first-run' | 'returning' | 'no-mods';

class UiStore {
  homeState = $state<SimpleState>('ready');
  wsState = $state<WorkspaceState>('ready');
  homeMode = $state<HomeMode>('returning');
}

export const ui = new UiStore();
