// UI-level store: current screen and per-screen mock states.
// The dev panel drives these toggles; every screen must render loading/empty/error.
export type Screen = 'home' | 'workspace';
export type SimpleState = 'ready' | 'loading' | 'error';
export type WorkspaceState = 'ready' | 'loading' | 'empty' | 'error';

class UiStore {
  screen = $state<Screen>('home');
  homeState = $state<SimpleState>('ready');
  wsState = $state<WorkspaceState>('ready');

  openWorkspace() {
    this.screen = 'workspace';
  }

  closeWorkspace() {
    this.screen = 'home';
  }
}

export const ui = new UiStore();
