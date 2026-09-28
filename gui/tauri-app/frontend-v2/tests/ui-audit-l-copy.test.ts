// L-package (UI audit 2026-09-29, /tmp/rimloc-ui-bugs.md) — terminology and
// RU/EN hybrid copy, fixed as dictionaries (en/ru parity kept):
//   L-2  «Translation Memory» inside the RU dictionary → «Память переводов»
//   L-4  «Структура (advanced)» → «Структура (расширенная)»
//   L-9  path placeholder / «Язык папки» — RU labels; a path EXAMPLE may
//        keep latin words as path DATA («/Users/you/…»), that is not copy
//   L-10 «(контракт)» removed from the build-screen card titles (internal term)
//   L-12 «Base URL» → «Адрес сервера», the key row «(keychain)» literal →
//        dictionary value («связка ключей»), «Список с сервера» → «Запрашивать у сервера»
// Plus the M-11 companion keys: the status words no longer embed «(мок)».
import { describe, expect, it } from 'vitest';
import { en } from '../src/i18n/en';
import { ru } from '../src/i18n/ru';

describe('L-package: RU copy has no EN hybrids, en/ru stay in parity', () => {
  it('L-2: one concept — one RU name', () => {
    expect(ru['wizard.w4.existing.tm']).toBe('Память переводов');
    expect(en['wizard.w4.existing.tm']).toBe('Translation Memory');
  });

  it('L-4: «Структура (расширенная)» — no latin word in the RU label', () => {
    expect(ru['workspace.navigator.advanced']).toBe('Структура (расширенная)');
    expect(en['workspace.navigator.advanced']).toBe('Structure (advanced)');
  });

  it('L-10: «(контракт)» / "(contract)" is gone from the build card titles', () => {
    for (const key of [
      'contractops.validate.title',
      'contractops.export.title',
      'contractops.buildmod.title',
      'contractops.diagnose.title'
    ]) {
      expect(ru[key]).not.toContain('(контракт)');
      expect(en[key]).not.toContain('(contract)');
    }
    expect(ru['contractops.validate.title']).toBe('Валидация');
    expect(ru['contractops.export.title']).toBe('Сборка перевода');
    expect(ru['contractops.buildmod.title']).toBe('Сборка мод-пакета');
    expect(ru['contractops.diagnose.title']).toBe('Диагностика');
  });

  it('L-9: the folder-language label and the placeholder LABEL are pure RU (path stays data)', () => {
    expect(ru['contractops.export.desc']).toContain('Язык папки:');
    // The example path is DATA — latin segments allowed; the leading label
    // («например …») must be Russian.
    expect(ru['contractops.abs_path_example']).toMatch(/^например \//);
    expect(ru['contractops.abs_path_example_mod']).toMatch(/^например \//);
    expect(ru['contractops.abs_path_example_bundle']).toMatch(/^например \//);
  });

  it('L-12: «Адрес сервера», «связка ключей», «Запрашивать у сервера»', () => {
    expect(ru['providers.baseUrl']).toBe('Адрес сервера');
    expect(en['providers.baseUrl']).toBe('Base URL');
    expect(ru['providers.inst.key.keychain']).toBe('связка ключей');
    expect(en['providers.inst.key.keychain']).toBe('keychain');
    expect(ru['providers.form.discovery.auto']).toBe('Запрашивать у сервера');
  });

  it('M-11 companion: status words are clean, the mock mark is its own key', () => {
    expect(ru['providers.status.connected']).toBe('Подключён');
    expect(ru['providers.status.offline']).toBe('Офлайн');
    expect(en['providers.status.connected']).toBe('Connected');
    expect(en['providers.status.offline']).toBe('Offline');
    expect(ru['providers.status.mockMark']).toBe('(мок)');
    expect(en['providers.status.mockMark']).toBe('(mock)');
    expect(ru['providers.status.configured']).toBe('Настроен (не проверялся)');
    expect(en['providers.status.configured']).toBe('Configured (not verified)');
  });

  it('M-6 companion: the bridge lines exist in both dictionaries', () => {
    for (const key of [
      'review.overview.queueLink',
      'review.overview.queueExplainer',
      'review.queue.overviewLink'
    ]) {
      expect(ru[key], `ru missing ${key}`).toBeTruthy();
      expect(en[key], `en missing ${key}`).toBeTruthy();
    }
  });
});
