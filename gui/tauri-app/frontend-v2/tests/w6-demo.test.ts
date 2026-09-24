// W6 regression: bundled demo project (MOCK_LIVE_ONBOARDING_MANDATE §6/§8)
// and the no-mods empty state (§9).
//   - seeding is deterministic and in-memory only (no fs, no real paths);
//   - the demo is clearly marked and cannot be confused with real projects;
//   - reset returns to the exact starting state; leaving restores the plain
//     pristine dataset;
//   - the no-mods Home offers Try Demo / Choose folder / Configure
//     installation, where only the honest ones do anything.
import { describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { ui } from '../src/lib/stores/ui.svelte';
import { i18n } from '../src/i18n/store.svelte';
import { demoProject, DEMO_PROJECT_NAME } from '../src/lib/demo/demoProject.svelte';
import { mockEntries } from '../src/lib/mock/data';
import { click, cleanupMounted, exists, goto, mountCmp, q } from './helpers';

function snapshot(): string {
  return JSON.stringify(
    project.entries.map((e) => ({ id: e.id, target: e.target, status: e.status, origin: e.origin ?? null }))
  );
}

describe('demo project determinism and isolation', () => {
  it('seeds the demo identity with the pristine dataset', () => {
    demoProject.seed();
    expect(project.projectName).toBe(DEMO_PROJECT_NAME);
    expect(project.isDemo).toBe(true);
    expect(demoProject.active).toBe(true);
    // Multi-target pristine layout restored (ru wizard + uk import + ja pack).
    expect(targetLocales()).toEqual(['ru', 'uk', 'ja']);
  });

  it('resets deterministically: seeded state is always identical', () => {
    demoProject.seed();
    const pristine = snapshot();
    // Mutate aggressively: direct writes, status flips, selection, filters.
    const first = project.byId('keyed-01');
    if (first) {
      first.target = 'изменение из демо';
      project.setStatus('keyed-01', 'translated');
    }
    project.setStatus('keyed-06', 'translated');
    project.select('keyed-04');
    project.setStatusFilter('translated');
    demoProject.resetDemo();
    expect(snapshot()).toBe(pristine);
    expect(project.selectedId).toBeNull();
    expect(project.search).toBe('');
    expect(project.filters).toEqual([]);
  });

  it('leaves no trace in other projects: leave() restores plain pristine', () => {
    const before = snapshot();
    demoProject.seed();
    const first = project.byId('keyed-01');
    if (first) first.target = 'изменение из демо';
    demoProject.leave();
    expect(snapshot()).toBe(before);
    expect(project.projectName).toBe('TestMod');
    expect(project.isDemo).toBe(false);
    expect(demoProject.active).toBe(false);
  });

  it('the demo dataset shape covers all record kinds and a review queue', () => {
    demoProject.seed();
    const kinds = new Set(project.entries.map((e) => e.kind));
    expect([...kinds]).toEqual(expect.arrayContaining(['Keyed', 'DefInjected', 'TKey']));
    const counts = project.statusCounts();
    // Intentional validation material: review queue + sourceChanged + orphans.
    expect(counts.pending_review).toBeGreaterThan(0);
    expect(counts.sourceChanged).toBeGreaterThan(0);
    expect(counts.todo).toBeGreaterThan(0);
    // Placeholder issue present (the reviewer-visible mismatch).
    const withPlaceholder = project.entries.filter((e) => e.issues?.some((i) => i.kind === 'placeholder_mismatch'));
    expect(withPlaceholder.length).toBeGreaterThan(0);
    // TKey multi-context entries exist.
    expect(project.entries.filter((e) => (e.contexts?.length ?? 0) > 1).length).toBeGreaterThan(0);
    // Pristine inventory untouched by identity: same entry count.
    expect(project.entries).toHaveLength(mockEntries.length);
  });
});

function targetLocales(): string[] {
  // The persisted multi-target shape: { active, pinned, targets: [{locale, addedVia}] }.
  try {
    const raw = window.localStorage.getItem('rimloc.project.testmod.multitarget.v1');
    const shape = raw ? (JSON.parse(raw) as { targets: Array<{ locale: string }> }) : { targets: [] };
    return shape.targets.map((t) => t.locale);
  } catch {
    return [];
  }
}

describe('Home: demo card and no-mods state', () => {
  it('shows the marked demo project card and opens it isolated', () => {
    ui.homeMode = 'returning';
    goto('#/home');
    mountCmp(App);
    expect(exists('home.demo')).toBe(true);
    expect(q('home.demo').textContent).toContain('синтетический демо-проект');
    click('home.demo.open');
    flushSync();
    expect(window.location.hash).toBe('#/workspace');
    expect(project.projectName).toBe(DEMO_PROJECT_NAME);
    // The workspace announces the demo identity in the toolbar meta.
    expect(exists('workspace.meta')).toBe(true);
  });

  it('no-mods state offers Try Demo / Choose folder / Configure installation', () => {
    ui.homeMode = 'no-mods';
    goto('#/home');
    mountCmp(App);
    expect(exists('home.nomods')).toBe(true);

    // Choose folder is an honest mock: a note, no navigation, no fake dialog.
    click('home.nomods.folder');
    expect(exists('home.nomods.note')).toBe(true);
    expect(window.location.hash).toBe('#/home');

    // Configure installation routes to the real settings route.
    click('home.nomods.configure');
    flushSync();
    expect(window.location.hash).toBe('#/settings');
  });

  it('no-mods Try Demo seeds the demo and opens the workspace', () => {
    ui.homeMode = 'no-mods';
    goto('#/home');
    mountCmp(App);
    click('home.nomods.demo');
    flushSync();
    expect(window.location.hash).toBe('#/workspace');
    expect(project.projectName).toBe(DEMO_PROJECT_NAME);
    expect(project.isDemo).toBe(true);
  });

  it('demo labels exist in both locales', () => {
    i18n.setLocale('en');
    demoProject.leave(); // fresh label: not "reopen"
    ui.homeMode = 'returning';
    goto('#/home');
    mountCmp(App);
    expect(q('home.demo').textContent).toContain('synthetic demo project');
    expect(q('home.demo.open')?.textContent).toContain('Open demo project');
    i18n.setLocale('ru');
  });
});
