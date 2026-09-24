// Deterministic mock translation corpus per target locale (W2).
//
// Canonical-model semantics in mock form: one source inventory, translations
// keyed by (entry id, locale). The base RU dataset IS the mock corpus in
// ../mock/data.ts (rich statuses/issues); this file supplies ADDITIONAL
// locales so the workspace can prove multi-target behaviour:
//   uk ≈ 40%, ja ≈ 15% (mandated coverage), de/pl/es/zh-Hans — label samples.
//
// Everything here is deterministic: same entry ids → same text → same progress
// on every load, so "From pack"/"Import" flows and regressions are stable.
// Texts are hand-authored samples for mock/demonstration purposes only — they
// never claim to be real community translations.

export type LocaleCorpus = Record<string, string>;

/** entry id → Ukrainian text (~40% of the mock inventory, labels/short UI first). */
export const CORPUS_UK: LocaleCorpus = {
  'keyed-01': '{0}: Надійшов лист.',
  'keyed-03': 'Дослідження',
  'keyed-05': 'Сонячний спалах вивів енергосистему з ладу.',
  'keyed-07': 'Орбітальний торговий маяк',
  'keyed-09': 'Поблизу приземляється кластер механоїдів.',
  'keyed-10': 'Почати психічний ритуал',
  'keyed-11': 'Переохолодження',
  'keyed-15': 'Срібло — стандартна торгова валюта узбеччя.',
  'keyed-16': 'Кількість повторень: {0}',
  'keyed-18': 'Видалити {0}?',
  'keyed-19': 'Ліворуч',
  'keyed-20': 'Дослідження завершено: {0}',
  'keyed-21': 'Ласкаво просимо до модуля вдосконалень. Нехай колоніальне життя буде в радість.',
  'keyed-22': 'Ласкаво просимо до мода транспорту. Гарної їзди узбеччям!',
  'di-01': 'штурмова гвинтівка',
  'di-03': 'довгий меч',
  'di-04': 'протиуламковий жилет',
  'di-07': 'рис',
  'di-11': 'пісок',
  'di-12': 'сонячний генератор',
  'di-14': 'вовк',
  'di-19': 'біонічне око',
  'di-20': 'мармурова колона',
  'di-22': 'парка',
  'di-24': 'синець',
  'tk-01': 'Ваші троє колоністів приходять до тями після довгої мандрівки.',
  'tk-03': 'Угода з дияволом',
  'tk-07': 'Місія на Місяць'
};

/** entry id → Japanese text (~15%, placeholder order follows Japanese syntax). */
export const CORPUS_JA: LocaleCorpus = {
  'keyed-03': '研究',
  'keyed-05': '太陽フレアが電力網を停止させました。',
  'keyed-07': '軌道交易ビーコン',
  'keyed-11': '低体温症',
  'keyed-16': '繰り返し回数: {0}',
  'keyed-18': '{0} を削除しますか？',
  'keyed-20': '研究完了: {0}',
  'di-01': 'アサルトライフル',
  'di-11': '砂',
  'di-14': 'オオカミ',
  'tk-03': '悪魔との取引'
};

/** German sample (labels + a few short UI strings). */
export const CORPUS_DE: LocaleCorpus = {
  'keyed-03': 'Forschung',
  'keyed-05': 'Ein Sonnenflare hat deine Stromversorgung lahmgelegt.',
  'keyed-07': 'Orbitaler Handelsbeacon',
  'keyed-11': 'Unterkühlung',
  'keyed-16': 'Wiederholungsanzahl: {0}',
  'keyed-18': '{0} löschen?',
  'di-01': 'Sturmgewehr',
  'di-04': 'Splitterschutzweste',
  'di-11': 'Sand',
  'di-14': 'Wolf'
};

/** Polish sample. */
export const CORPUS_PL: LocaleCorpus = {
  'keyed-03': 'Badania',
  'keyed-05': 'Flara słoneczna wyłączyła zasilanie.',
  'keyed-07': 'Orbitalny beacon handlowy',
  'keyed-11': 'Hipotermia',
  'keyed-16': 'Liczba powtórzeń: {0}',
  'keyed-18': 'Usunąć {0}?',
  'di-01': 'karabin szturmowy',
  'di-11': 'piasek',
  'di-14': 'wilk'
};

/** Spanish sample. */
export const CORPUS_ES: LocaleCorpus = {
  'keyed-03': 'Investigación',
  'keyed-05': 'Una fulguración solar ha dejado sin energía tu red eléctrica.',
  'keyed-07': 'Baliza comercial orbital',
  'keyed-11': 'Hipotermia',
  'keyed-16': 'Número de repeticiones: {0}',
  'keyed-18': '¿Eliminar {0}?',
  'di-01': 'fusil de asalto',
  'di-11': 'arena',
  'di-14': 'lobo'
};

/** Simplified Chinese sample. */
export const CORPUS_ZH: LocaleCorpus = {
  'keyed-03': '研究',
  'keyed-05': '太阳耀斑摧毁了你的电网。',
  'keyed-07': '轨道交易信标',
  'keyed-11': '失温症',
  'keyed-16': '重复次数：{0}',
  'keyed-18': '删除{0}？',
  'di-01': '突击步枪',
  'di-11': '沙子',
  'di-14': '狼'
};

export const LOCALE_CORPUS: Record<string, LocaleCorpus> = {
  uk: CORPUS_UK,
  ja: CORPUS_JA,
  de: CORPUS_DE,
  pl: CORPUS_PL,
  es: CORPUS_ES,
  'zh-Hans': CORPUS_ZH
};

/** Deterministic last-modified stamp for a corpus locale (stable across loads). */
export function corpusEditedAt(localeId: string, index: number): string {
  const minute = ((index * 7 + localeId.length * 3) % 50) + 10;
  return `2026-09-22T12:${String(minute).padStart(2, '0')}:00Z`;
}

/**
 * How a corpus entry was filled, by the flow that added the target:
 * - pack / import: every corpus entry, origin "imported" (community/pack text)
 * - tm: every corpus entry as a translation-memory match (origin "TM")
 * - empty: nothing (the translator starts from zero)
 */
export type AddFlow = 'empty' | 'pack' | 'tm' | 'import';

export function corpusOrigin(flow: Exclude<AddFlow, 'empty'>): 'imported' | 'TM' {
  return flow === 'tm' ? 'TM' : 'imported';
}

/**
 * The corpus a given locale contributes for a given add-flow:
 * - pack / import → the locale's whole authored corpus;
 * - tm → only the first half (deterministic by entry id): a translation
 *   memory rarely covers everything;
 * - empty → null (no data at all).
 * Locales without an authored corpus (custom languages) contribute nothing in
 * every flow — the workspace shows a genuinely empty target.
 */
export function corpusForLocale(localeId: string, flow: AddFlow): LocaleCorpus | null {
  if (flow === 'empty') return null;
  const own = LOCALE_CORPUS[localeId];
  if (!own) return null;
  if (flow === 'tm') {
    const ids = Object.keys(own).sort();
    const half: LocaleCorpus = {};
    for (const id of ids.slice(0, Math.ceil(ids.length / 2))) half[id] = own[id];
    return half;
  }
  return own;
}
