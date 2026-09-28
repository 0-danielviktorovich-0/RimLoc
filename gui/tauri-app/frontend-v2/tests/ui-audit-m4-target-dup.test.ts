// M-4 (UI audit 2026-09-29, /tmp/rimloc-ui-bugs.md): the active target was
// shown TWICE side by side — «Русский 0% ▾» in the selector and a pinned
// chip «Русский 0%» right next to it. Fix: the selector is the single source
// for the ACTIVE locale; pinned quick tabs remain only for the OTHER pinned
// targets (multi-target), swapping live with the active switch.
import { beforeEach, describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import TargetSwitcher from '../src/lib/languages/TargetSwitcher.svelte';
import { languages } from '../src/lib/languages/store.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { cleanupMounted, exists, mountCmp } from './helpers';

describe('M-4: the selector is the only place the active language appears', () => {
  beforeEach(() => {
    cleanupMounted();
    project.reset(); // fixture source — multi-target switching is allowed
    // Deterministic pinning regardless of what a previous test persisted.
    languages.pinned = ['ru', 'uk'];
    if (languages.activeLocale !== 'ru') languages.setActive('ru');
  });

  it('the active pinned locale has no chip next to its own selector', () => {
    expect(languages.activeLocale).toBe('ru');
    mountCmp(TargetSwitcher);
    // ru is active → no ru pin; the OTHER pinned target stays.
    expect(exists('languages.pinned.ru')).toBe(false);
    expect(exists('languages.pinned.uk')).toBe(true);
  });

  it('switching the active locale swaps which pin is shown', () => {
    mountCmp(TargetSwitcher);
    languages.setActive('uk');
    flushSync();
    expect(exists('languages.pinned.uk')).toBe(false);
    expect(exists('languages.pinned.ru')).toBe(true);
  });

  it('an unpinned active locale leaves the other pins untouched', () => {
    languages.pinned = ['uk', 'ja'];
    mountCmp(TargetSwitcher);
    expect(exists('languages.pinned.uk')).toBe(true);
    expect(exists('languages.pinned.ja')).toBe(true);
  });
});
