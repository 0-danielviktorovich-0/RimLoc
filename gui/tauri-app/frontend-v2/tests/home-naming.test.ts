// W1 regression: Home card naming (QA mandate §14 / hands-on §1).
// "Translate a new mod" → "Create translation" / «Новый перевод» with the
// multi-content subtitle, from i18n in both locales. Also guards ru/en key
// parity so no branch ships a missing translation.
import { describe, expect, it } from 'vitest';
import Home from '../src/lib/components/screens/Home.svelte';
import { en } from '../src/i18n/en';
import { ru } from '../src/i18n/ru';
import { i18n } from '../src/i18n/store.svelte';
import { click, exists, mountCmp, q } from './helpers';

describe('home: create-translation card naming', () => {
  it('ru: «Новый перевод» + подзаголовок про мод/игру/DLC/языковой пакет', () => {
    i18n.setLocale('ru');
    mountCmp(Home);
    const card = q('home.entry-new');
    expect(card.textContent).toContain('Новый перевод');
    expect(card.textContent).toContain('Мод, базовая игра, DLC или языковой пакет');
    expect(card.textContent).not.toContain('Перевести новый мод');
  });

  it('en: "Create translation" + multi-content subtitle', () => {
    i18n.setLocale('en');
    mountCmp(Home);
    const card = q('home.entry-new');
    expect(card.textContent).toContain('Create translation');
    expect(card.textContent).toContain('Mod, base game, DLC or language pack');
    expect(card.textContent).not.toContain('Translate a new mod');
  });

  it('card still routes to the wizard', () => {
    i18n.setLocale('en');
    mountCmp(Home);
    click('home.entry-new');
    expect(window.location.hash).toBe('#/wizard');
  });
});

describe('i18n: ru/en key parity for the W1 additions', () => {
  const KEYS = [
    'home.entryNew.title',
    'home.entryNew.description',
    'wizard.w2.subtitle.mod',
    'wizard.w2.subtitle.base',
    'wizard.w2.subtitle.dlc',
    'wizard.w2.subtitle.pack',
    'wizard.w2.installation',
    'wizard.w2.dlc.selected',
    'wizard.w2.dlc.needOne',
    'wizard.w2.dlc.notInstalled',
    'wizard.w2.pack.mapping',
    'wizard.w2.pack.origin.official',
    'wizard.w2.pack.origin.community',
    'wizard.w2.pack.entries',
    'wizard.w2.pack.from',
    'wizard.w2.pack.to',
    'wizard.w2.selected.summary',
    'onboarding.title',
    'onboarding.step',
    'onboarding.skip',
    'onboarding.next',
    'onboarding.finish',
    'onboarding.s1.title',
    'onboarding.s1.text',
    'onboarding.s2.title',
    'onboarding.s2.text',
    'onboarding.s3.title',
    'onboarding.s3.text',
    'onboarding.s4.title',
    'onboarding.s4.text'
  ];

  it('every W1 key exists in both dictionaries', () => {
    for (const key of KEYS) {
      expect(en[key], `en missing: ${key}`).toBeTruthy();
      expect(ru[key], `ru missing: ${key}`).toBeTruthy();
    }
  });

  it('dictionaries match on every W1-area prefix (home/wizard/onboarding/help)', () => {
    // Whole-dict equality is owned by each wave; this guards the areas W1
    // touches. (Known pre-existing drift: languages.*/palette keys — other
    // waves, out of scope here.)
    const PREFIXES = ['home.', 'wizard.', 'onboarding.', 'help.'];
    for (const prefix of PREFIXES) {
      const enKeys = Object.keys(en).filter((k) => k.startsWith(prefix)).sort();
      const ruKeys = Object.keys(ru).filter((k) => k.startsWith(prefix)).sort();
      expect(ruKeys, `prefix ${prefix}`).toEqual(enKeys);
    }
  });
});
