// W6 regression: global mock/data-mode badge (MOCK_LIVE_ONBOARDING_MANDATE
// §1). The whole scaffold is a mock, and the badge must say so on every route
// and in every build — its visibility must NOT depend on import.meta.env.DEV
// (a preview build still shows the truth). Also verifies the W5 local
// diagnostics badge was consolidated into the global one.
import { describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import App from '../src/App.svelte';
import { i18n } from '../src/i18n/store.svelte';
import { project } from '../src/lib/stores/project.svelte';
import { cleanupMounted, exists, goto, mountCmp, q } from './helpers';

describe('global mock badge', () => {
  it('is visible on every route of the app shell', () => {
    for (const route of ['home', 'workspace', 'review', 'build', 'diagnostics', 'help', 'settings']) {
      goto(`#/${route}`);
      mountCmp(App);
      expect(exists('mock-badge'), `badge on #/${route}`).toBe(true);
      cleanupMounted();
    }
  });

  it('uses the honest i18n label in both locales', () => {
    goto('#/home');
    mountCmp(App);
    const badge = document.querySelector('[data-testid="mock-badge"]');
    expect(badge?.textContent).toContain('Демо-данные (мок)');
    cleanupMounted();
    // EN pass: switch the live locale (localStorage alone is read at boot).
    i18n.setLocale('en');
    mountCmp(App);
    expect(document.querySelector('[data-testid="mock-badge"]')?.textContent).toContain('Demo data (mock)');
  });

  it('has no env-based gating in the component source (preview builds included)', async () => {
    // The honesty contract is structural: the badge component must not read
    // import.meta.env at all, so a DEV flag can never hide it.
    const { readFile } = await import('node:fs/promises');
    const source = await readFile('src/lib/components/MockBadge.svelte', 'utf8');
    expect(source).not.toContain('import.meta.env');
    expect(source).not.toContain('DEV');

    // And the W5 local badge is gone from Diagnostics — one badge, one truth.
    goto('#/diagnostics');
    mountCmp(App);
    flushSync();
    expect(exists('diagnostics.demoBadge')).toBe(false);
    expect(exists('mock-badge')).toBe(true);
  });
});

// Audit P1-4 regression: the global chip must tell the DATA-MODE truth. On a
// real contract project the mock badge was a lie ("Демо-данные") while real
// edits persisted into the managed project — the chip becomes the honest
// live badge; fixture/demo data keeps the mock badge.
describe('data-mode badge truth', () => {
  it('shows the live badge on a contract project, never the mock one', async () => {
    const ok = await project.createContractProject('/mods/Demo');
    expect(ok).toBe(true);
    expect(project.source).toBe('contract');

    for (const route of ['home', 'workspace', 'build', 'diagnostics']) {
      goto(`#/${route}`);
      mountCmp(App);
      expect(exists('live-badge'), `live badge on #/${route}`).toBe(true);
      expect(exists('mock-badge'), `no mock badge on #/${route}`).toBe(false);
      cleanupMounted();
    }
    goto('#/workspace');
    // The locale state persists across tests in this file (an earlier test
    // switched to EN) — pin RU for the label assertion.
    i18n.setLocale('ru');
    mountCmp(App);
    expect(q('live-badge').textContent).toContain('Живой проект');
    cleanupMounted();
  });

  it('keeps the mock badge on fixture/demo data', () => {
    project.reset(); // back to the explicit fixture dataset
    goto('#/home');
    mountCmp(App);
    expect(exists('mock-badge')).toBe(true);
    expect(exists('live-badge')).toBe(false);
  });
});
