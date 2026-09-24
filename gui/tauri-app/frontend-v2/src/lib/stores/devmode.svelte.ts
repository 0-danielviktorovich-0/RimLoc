// Dev/testing mode (W6, MOCK_LIVE_ONBOARDING_MANDATE §11): ONE explicit gate
// for developer tooling — the dev panel, the scenario browser and the
// scenario deep-link runner.
//
// Enabled by any of:
//   - import.meta.env.DEV  (vite dev server — the existing dev-only practice),
//   - localStorage 'rimloc.dev' === '1'  (persisted opt-in for QA on
//     production-style builds),
//   - a boot flag in the URL: ?dev=1 (search or hash query; session-only).
//
// The gate NEVER touches the global Demo-data badge: honesty about the data
// mode is unconditional in every build. Disabled dev mode makes the scenario
// runner ignore ANY ?scenario= value (known, unknown or malicious) without
// touching project stores — the deep-link simply does nothing. The final
// MockTransport production-exclusion stays a post-freeze CI/xtask concern;
// this gate is the pre-freeze mock boundary, not that exclusion.
const STORAGE_KEY = 'rimloc.dev';

function bootFlag(): boolean {
  if (typeof window === 'undefined') return false;
  try {
    const search = new URLSearchParams(window.location.search);
    if (search.get('dev') === '1') return true;
    const hashQuery = window.location.hash.split('?')[1] ?? '';
    return new URLSearchParams(hashQuery).get('dev') === '1';
  } catch {
    return false;
  }
}

function persisted(): boolean {
  try {
    return window.localStorage.getItem(STORAGE_KEY) === '1';
  } catch {
    return false;
  }
}

class DevModeStore {
  enabled = $state(false);

  constructor() {
    this.enabled = bootFlag() || persisted() || import.meta.env.DEV;
  }

  /** Persisted opt-in (QA on production-style builds). */
  enable() {
    this.enabled = true;
    try {
      window.localStorage.setItem(STORAGE_KEY, '1');
    } catch {
      /* ignore */
    }
  }

  disable() {
    this.enabled = false;
    try {
      window.localStorage.setItem(STORAGE_KEY, '0');
    } catch {
      /* ignore */
    }
  }
}

export const devMode = new DevModeStore();
