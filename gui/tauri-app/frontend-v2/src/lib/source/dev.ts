// Developer opt-in gate for the W7 scenario controls (MOCK_LIVE_ONBOARDING
// §11: scenario access is a DEV/developer feature, never product UI).
// A scenario picker is visible when the build is a dev build, or when the
// operator opts in explicitly via ?dev=1 or localStorage rimloc.dev=1.
// W6 owns DevPanel; after integration it can render SourceScenarioPicker
// (components/source) inside its own panel using SOURCE_SCENARIOS.
export function isDevScenarios(): boolean {
  if (import.meta.env.DEV) return true;
  try {
    if (new URLSearchParams(window.location.search).has('dev')) return true;
    return window.localStorage.getItem('rimloc.dev') === '1';
  } catch {
    return false;
  }
}
