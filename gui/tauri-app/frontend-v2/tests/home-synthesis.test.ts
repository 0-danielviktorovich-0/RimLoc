// Design-synthesis regression (jury 2026-09-29): the returning-user Home is
// the design-system skeleton — the FIRST content block is the "continue
// working" hero card, the recents list is a semantic table, and the featured
// project is EXCLUDED from the table (no duplicate "main" list).
import { beforeEach, describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { ui } from '../src/lib/stores/ui.svelte';
import { i18n } from '../src/i18n/store.svelte';
import { onboarding } from '../src/lib/stores/onboarding.svelte';
import { cleanupMounted, exists, goto, mountCmp, q } from './helpers';

describe('home synthesis: hero «Продолжить работу» + recents table', () => {
  beforeEach(() => {
    cleanupMounted();
    onboarding.open = false;
    ui.homeMode = 'returning';
    i18n.setLocale('ru');
  });

  function mountHome() {
    goto('#/home');
    mountCmp(App);
    flushSync();
  }

  it('the first content block is the hero card with progress, counters and actions', () => {
    mountHome();
    const hero = q('home.recent.hero');
    // Kicker names the block; the featured project is the most recent one.
    expect(hero.textContent).toContain('Продолжить работу');
    expect(hero.textContent).toContain('Vanilla Furniture Expanded — RU');
    // Progress is announced (role=progressbar + value) and carries the line
    // counters from the fixture (1086 of 1248 → 162 left).
    const bar = hero.querySelector('[role="progressbar"]');
    expect(bar?.getAttribute('aria-valuenow')).toBe('87');
    expect(hero.textContent).toContain('строк переведено');
    expect(hero.textContent).toContain('осталось 162');
    // Status pills: lifecycle + counters that name the next step.
    expect(hero.textContent).toContain('В работе');
    expect(hero.textContent).toContain('Источник обновился: 12');
    expect(hero.textContent).toContain('Замечаний: 4');
    // Actions: continue (primary), review with count, build.
    expect(exists('home.recent-continue.p1')).toBe(true);
    expect(exists('home.continue.review')).toBe(true);
    expect(q('home.continue.review').textContent).toContain('4');
    expect(exists('home.continue.build')).toBe(true);
  });

  it('the featured project is not duplicated in the recents table', () => {
    mountHome();
    const table = q('home.recent');
    expect(table.textContent).not.toContain('Vanilla Furniture Expanded — RU');
    expect(table.textContent).toContain('Rimatomics — RU');
    expect(table.textContent).toContain('Hospitality — DE');
  });

  it('the recents list is a semantic table: scope=col headers, human dates, status notes', () => {
    mountHome();
    const table = q('home.recent');
    const cols = [...table.querySelectorAll('th[scope="col"]')];
    expect(cols.length).toBe(5);
    expect(cols[0].textContent).toContain('Проект');
    // sr-only actions header stays out of sight but in the a11y tree.
    expect(cols[4].querySelector('.visually-hidden')?.textContent).toContain('Действия');
    // Human dates: "изменён …" in every row.
    expect(table.textContent).toContain('изменён');
    // Status vocabulary: marker pill + note with the concrete counter.
    expect(table.textContent).toContain('Источник обновился: 3');
    expect(table.textContent).toContain('Замечаний: 9');
    // Row actions keep the per-project continue contract.
    expect(exists('home.recent-continue.p2')).toBe(true);
    expect(exists('home.recent-continue.p3')).toBe(true);
  });

  it('en locale keeps parity on the new surface', () => {
    i18n.setLocale('en');
    mountHome();
    expect(q('home.recent.hero').textContent).toContain('Continue where you left off');
    expect(q('home.recent.hero').textContent).toContain('lines translated');
    const firstCol = document.querySelector('th[scope="col"]');
    expect(firstCol?.textContent).toContain('Project');
  });
});
