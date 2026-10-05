// D-2 regression (design pass 2026-10-01): a route-scoped coach step must
// NOT render on a foreign route. The entry effect navigates once (never
// yanking), but if the user (or a deep link) leaves mid-step, the card used
// to stay on screen with workspace copy over Settings/Providers/Build —
// covering content with a hint about a table that is not there.
// E2E over the App shell, same pattern as onboarding-replay.
import { beforeEach, describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { onboarding } from '../src/lib/stores/onboarding.svelte';
import { cleanupMounted, click, exists, goto, mountCmp } from './helpers';

describe('D-2: coach steps are route-scoped', () => {
  beforeEach(() => {
    cleanupMounted();
    localStorage.clear();
  });

  it('workspace step pauses on a foreign route, resumes on return', () => {
    mountCmp(App); // boot at home
    goto('#/workspace'); // live navigation mounts Workspace → first-run coach
    expect(exists('onboarding.overlay')).toBe(true); // step 1 = workspace-scoped

    // Owner wanders to Settings mid-step (no yank-back by design).
    goto('#/settings');
    expect(onboarding.open).toBe(true); // the tour is still active...
    expect(exists('onboarding.overlay')).toBe(false); // ...but NOT rendered here

    // Deep-link wandering across more routes — still paused.
    goto('#/providers');
    expect(exists('onboarding.overlay')).toBe(false);

    // Back to the step's route: the coach resumes.
    goto('#/workspace');
    expect(exists('onboarding.overlay')).toBe(true);
  });

  it('a home-scoped step does not leak onto workspace either', () => {
    // The demo script has home-scoped steps (steps.ts route: 'home').
    mountCmp(App);
    goto('#/workspace');
    click('onboarding.skip');
    expect(exists('onboarding.overlay')).toBe(false);
  });
});
