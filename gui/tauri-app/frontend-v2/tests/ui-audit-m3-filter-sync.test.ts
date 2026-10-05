// M-3 (UI audit 2026-09-29, /tmp/rimloc-ui-bugs.md): the workspace had TWO
// diverging filter sets — the left «ЗАПИСИ/СТАТУС» navigator and the chips
// above the table. «Переведено» existed only in the chips, «Осиротевшие»
// only in the navigator. Fix: the chips stay, the navigator becomes
// navigation OVER the same project.filters state — identical status sets on
// both surfaces, every click visible on both, multi-select everywhere.
import { beforeEach, describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import Navigator from '../src/lib/components/workspace/Navigator.svelte';
import FilterBar from '../src/lib/components/workspace/FilterBar.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { i18n } from '../src/i18n/store.svelte';
import { click, cleanupMounted, exists, mountCmp, q } from './helpers';

// The ONE status vocabulary both surfaces must expose (FilterBar QUICK ===
// Navigator STATUS_ITEMS by design).
const STATUSES = [
  'untranslated',
  'pending_review',
  'sourceChanged',
  'translated',
  'todo',
  'orphan'
] as const;

describe('M-3: the navigator and the table chips are one synchronized filter set', () => {
  beforeEach(() => {
    cleanupMounted();
    project.reset();
    i18n.setLocale('ru');
  });

  it('both surfaces expose the SAME status set — «Переведено» and «Осиротевшие» in both', () => {
    mountCmp(Navigator, { kindCounts: { Keyed: 1, DefInjected: 1, TKey: 1 } });
    mountCmp(FilterBar, { counts: project.statusCounts() });

    for (const s of STATUSES) {
      expect(exists(`workspace.navigator.status.${s}`), `navigator missing ${s}`).toBe(true);
      expect(exists(`workspace.filter.${s}`), `chips missing ${s}`).toBe(true);
    }
    // The two former gaps, by their human labels.
    expect(q('workspace.navigator.status.translated').textContent).toContain('Переведено');
    expect(q('workspace.filter.orphan').textContent).toContain('Осиротевшие');
  });

  it('a navigator click toggles the SAME filter the chips show (and back)', () => {
    mountCmp(Navigator, { kindCounts: { Keyed: 1, DefInjected: 1, TKey: 1 } });
    mountCmp(FilterBar, { counts: project.statusCounts() });

    click('workspace.navigator.status.orphan');
    expect(project.filters).toEqual(['orphan']);
    expect(q('workspace.filter.orphan').getAttribute('aria-pressed')).toBe('true');
    expect(q('workspace.navigator.status.orphan').getAttribute('aria-pressed')).toBe('true');

    // The chip click mirrors into the navigator too.
    click('workspace.filter.orphan');
    expect(project.filters).toEqual([]);
    expect(q('workspace.navigator.status.orphan').getAttribute('aria-pressed')).toBe('false');
  });

  it('a chip click lights the matching navigator item; multi-select stays multi', () => {
    mountCmp(Navigator, { kindCounts: { Keyed: 1, DefInjected: 1, TKey: 1 } });
    mountCmp(FilterBar, { counts: project.statusCounts() });

    click('workspace.filter.translated');
    expect(q('workspace.navigator.status.translated').getAttribute('aria-pressed')).toBe('true');
    click('workspace.filter.todo');
    expect(project.filters).toEqual(['translated', 'todo']);
    expect(q('workspace.navigator.status.todo').getAttribute('aria-pressed')).toBe('true');

    // «Все записи» clears the shared state for both surfaces.
    click('workspace.navigator.all');
    expect(project.filters).toEqual([]);
    flushSync();
    expect(q('workspace.filter.translated').getAttribute('aria-pressed')).toBe('false');
    expect(q('workspace.filter.todo').getAttribute('aria-pressed')).toBe('false');
  });

  it('the shared filter actually filters the table (orphan click narrows rows)', () => {
    mountCmp(Navigator, { kindCounts: { Keyed: 1, DefInjected: 1, TKey: 1 } });
    click('workspace.navigator.status.orphan');
    const orphanCount = project.statusCounts().orphan;
    expect(project.filtered().length).toBe(orphanCount);
  });
});
