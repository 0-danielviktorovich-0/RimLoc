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
  'home.eyebrow': 'ЛОКАЛЬНАЯ БИБЛИОТЕКА',
  'home.title': 'Ваши проекты',
  'home.subtitle': 'Управляемые проекты RimLoc на этом компьютере.',
  'home.revision': 'Ревизия',
  'home.open': 'Открыть',
  'home.newTranslation': 'Начать новый перевод',
  'home.newTranslationHint': 'Из папки мода или примера',
  'home.emptyNote': 'Управляемых проектов пока нет — создайте первый.',
  'home.openExistingNote': 'Открыть/обновить существующий перевод — через карточку проекта или мастер.',
  'ws.noSnapshot': 'Проект ещё не открыт.',
  'ws.backHome': 'На главную',
  'ws.search': 'Поиск строк',
  'ws.searchPlaceholder': 'Найти строку или ключ…',
  'ws.resetFilters': 'Сбросить фильтры',
  'ws.filter.all': 'Все',
  'ws.filter.empty': 'Без перевода',
  'ws.filter.issues': 'С замечаниями',
  'ws.rows': 'строк',
  'ws.of': 'из',
  'ws.sessionChanges': 'Изменения в текущей сессии',
  'ws.addTarget': 'Добавить перевод…',
  'ws.editorTitle': 'РЕДАКТОР СТРОКИ',
  'ws.prev': 'Предыдущая строка',
  'ws.next': 'Следующая строка',
  'ws.copyKey': 'Копировать ключ',
  'ws.source': 'Исходник',
  'ws.translation': 'Перевод',
  'ws.translationPlaceholder': 'Введите перевод…',
  'ws.unsaved': 'Есть несохранённые изменения',
  'ws.saved': 'Сохранено',
  'ws.chars': 'символов',
  'ws.emptyWarning': 'Пустой перевод не пройдёт валидацию перед сборкой.',
  'ws.saveAndNext': 'Сохранить и дальше',
  'ws.save': 'Сохранить перевод',
  'ws.revert': 'Отменить изменения',
  'ws.tabSource': 'Источник',
  'ws.file': 'Файл',
  'ws.line': 'Строка',
  'ws.whyThisSource': 'Почему этот исходник',
  'ws.dirty': 'есть неподтверждённые правки',
  'ws.resize': 'Изменить ширину панелей',
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
  'home.eyebrow': 'LOCAL LIBRARY',
  'home.title': 'Your projects',
  'home.subtitle': 'RimLoc managed projects on this machine.',
  'home.revision': 'Revision',
  'home.open': 'Open',
  'home.newTranslation': 'Start a new translation',
  'home.newTranslationHint': 'From a mod folder or the sample',
  'home.emptyNote': 'No managed projects yet — create the first one.',
  'home.openExistingNote': 'Open/update an existing translation via a project card or the wizard.',
  'ws.noSnapshot': 'No project open yet.',
  'ws.backHome': 'Back home',
  'ws.search': 'Search strings',
  'ws.searchPlaceholder': 'Find a string or key…',
  'ws.resetFilters': 'Reset filters',
  'ws.filter.all': 'All',
  'ws.filter.empty': 'Untranslated',
  'ws.filter.issues': 'With findings',
  'ws.rows': 'rows',
  'ws.of': 'of',
  'ws.sessionChanges': 'Changes in this session',
  'ws.addTarget': 'Add translation…',
  'ws.editorTitle': 'STRING EDITOR',
  'ws.prev': 'Previous string',
  'ws.next': 'Next string',
  'ws.copyKey': 'Copy key',
  'ws.source': 'Source',
  'ws.translation': 'Translation',
  'ws.translationPlaceholder': 'Type the translation…',
  'ws.unsaved': 'Unsaved changes',
  'ws.saved': 'Saved',
  'ws.chars': 'chars',
  'ws.emptyWarning': 'An empty translation will fail the pre-build validation.',
  'ws.saveAndNext': 'Save & next',
  'ws.save': 'Save translation',
  'ws.revert': 'Revert changes',
  'ws.tabSource': 'Source',
  'ws.file': 'File',
  'ws.line': 'Line',
  'ws.whyThisSource': 'Why this source',
  'ws.dirty': 'unacked edits present',
  'ws.resize': 'Resize panes',
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
