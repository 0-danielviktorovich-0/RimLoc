// React-lane i18n (mandate §57): EN + RU first-class, flat keys mirroring
// the frozen Svelte catalog convention. These keys JOIN the shared
// self-localization catalog pipeline in Phase E — for now the lane carries
// only its own new keys; frozen frontend-v2 dictionaries stay untouched.

export type Locale = 'en' | 'ru'

const ru: Record<string, string> = {
  'nav.projects': 'Проекты',
  'nav.entries': 'Строки перевода',
  'nav.checks': 'Проверки',
  'nav.compare': 'Сравнение',
  'nav.glossary': 'Глоссарий',
  'nav.export': 'Сборка и экспорт',
  'nav.tools': 'Инструменты',
  'nav.settings': 'Настройки',
  'shell.noProject': 'Проект не открыт',
  'shell.localProject': 'Локальный проект',
  'shell.newProject': 'Новый проект',
  'shell.safetyNote': 'Только переводы. Оригинальный мод в безопасности.',
  'shell.placeholderTitle': 'React-лайна R1 живёт',
  'shell.placeholderBody':
    'Представительский экран собирается на реальном контракте: визуальная система Lovable R1 + RimLocClient.',
  'a11y.openNav': 'Открыть навигацию',
  'a11y.closeNav': 'Закрыть навигацию',
  'a11y.lightTheme': 'Светлая тема',
  'a11y.darkTheme': 'Тёмная тема',
}

const en: Record<string, string> = {
  'nav.projects': 'Projects',
  'nav.entries': 'Translation strings',
  'nav.checks': 'Checks',
  'nav.compare': 'Compare',
  'nav.glossary': 'Glossary',
  'nav.export': 'Build & export',
  'nav.tools': 'Tools',
  'nav.settings': 'Settings',
  'shell.noProject': 'No project open',
  'shell.localProject': 'Local project',
  'shell.newProject': 'New project',
  'shell.safetyNote': 'Translations only. The original mod stays safe.',
  'shell.placeholderTitle': 'The React R1 lane is alive',
  'shell.placeholderBody':
    'The representative screen is being built on the real contract: the Lovable R1 visual system + RimLocClient.',
  'a11y.openNav': 'Open navigation',
  'a11y.closeNav': 'Close navigation',
  'a11y.lightTheme': 'Light theme',
  'a11y.darkTheme': 'Dark theme',
}

let locale: Locale = 'ru'
const dicts: Record<Locale, Record<string, string>> = { ru, en }

export function setLocale(l: Locale): void {
  locale = l
}
export function getLocale(): Locale {
  return locale
}
export function t(key: string, params?: Record<string, string | number>): string {
  const raw = dicts[locale][key] ?? dicts.en[key] ?? key
  if (!params) return raw
  return Object.entries(params).reduce((acc, [k, v]) => acc.replaceAll(`{${k}}`, String(v)), raw)
}
