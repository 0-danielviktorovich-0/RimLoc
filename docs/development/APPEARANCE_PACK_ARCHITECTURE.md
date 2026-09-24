# Appearance Pack Architecture (G6) — RimLoc

Статус: **архитектурная граница зафиксирована**; реализация — поэтапная (см. §6).
Мандаты: `APPEARANCE_PACK_MANDATE.md` (кастомизация) и `UI_SDK_MANDATE.md`
(replaceable frontend / UI SDK) — кумулятивны к GUI_DESIGN_SPEC.md.

## 1. Три уровня кастомизации

| Уровень | Что | Код | Когда |
|---|---|---|---|
| **L1 Theme Pack** | декларативные токены (палитры light/dark, семантика, типографика, density, spacing, радиусы, границы, тени, иконки, motion, звуки, ассеты) | запрещён | после фриза, дешёвый POC возможен |
| **L2 Layout Pack** | размещение/видимость/размер/схлопывание одобренных регионов поверх существующих компонентов | запрещён | deferred |
| **L3 Developer UI Extension** | доверенный исполняемый UI (explicit trusted mode, SDK, capability declarations, sandbox, revocation, safe mode) | изолированная отдельная система | deferred, не через L1/L2 loader |

## 2. Принципы (инварианты)

1. **Паки меняют презентацию, не семантику** — одни команды, валидация, модель проекта,
   правила безопасности во всех вариантах.
2. **Декларативность**: L1/L2 — данные (versioned schema), никакого исполняемого кода.
3. **Ноль привилегий**: пак не имеет доступа к ФС/shell/секретам/провайдерам/данным
   проекта/сети; ассеты — только пассивные, валидация типа/размера/путей (path traversal
   запрещён), лимиты ресурсов.
4. **Fallback**: неуказанный токен → базовые токены RimLoc; сломанный пак → disable →
   встроенный Precision/default → объяснение пользователю; safe-ui recovery.
5. **Light/Dark обязательны** для serious-тем; system = селектор между ними; a11y-валидация
   (контрасты AA, focus, reduced-motion) на import/edit — first-party проходят гейт всегда.
6. **Versioned schema** + миграции + clear errors; устаревший пак не мешает старту.
7. **Форматы**: `.rimloctheme` / `.rimloclayout` / `.rimlocprofile` (или чище) — manifest:
   schema version, name, author, description, compatible versions, capabilities, assets,
   license. Секреты и данные переводов в паках запрещены.

## 3. Связь с текущей реализацией

- `frontend-v2/src/styles/` уже структурно близок: primitive (`--p-*`) → semantic
  (`--color-*` через `data-style/theme/palette/density`) → component. Four directions
  (precision/aurora/workshop/editorial) со временем становятся **first-party pack'ами** —
  это dogfood-доказательство выразительности схемы; до тех пор остаются CSS-файлами.
- Style Lab — авторский инструмент (инспект токенов, preview, a11y-проверка, export pack);
  остаётся за флагом, не прод-навигация.
- Хардкод style-специфичных форков компонентов запрещён: расхождения направлений — только
  через токены/минимальные CSS-дельты.

## 4. Layout Pack (когда дойдёт)

Одобренные регионы (topbar, project toolbar, left nav, editor, target panel, context/
details, suggestions, status, review sidebar, command actions) + декларативные правила
placement/visibility/sizing/collapse/density/responsive. Целевые раскладки: Professional,
Minimal (editor + context drawer), Laptop (collapsible + bottom context), Review split.
Будущий «Customize Workspace» — визуальный режим show/hide/move/resize/save preset без
правки конфигов.

## 5. Профили

Именованные комбинации Theme + Layout + Density + панели + motion (+ звуки):
Default / Compact Translator / Laptop / OLED / My Workspace. Global default + per-project
override (открытие проекта восстанавливает его workspace, глобальную тему не меняя без
явной настройки). Секреты и данные переводов в профилях запрещены.

## 6. Дорожная карта

| Этап | Что |
|---|---|
| сейчас | токены структурны; Style Lab чист; без style-форков компонентов; эта граница задокументирована |
| после FREEZE | (опц.) дешёвый POC: конвертация precision в `.rimloctheme` + loader с fallback |
| deferred | Layout Builder, community registry/gallery, L3 extensions, hot-reload авторский режим |

## 7. Replaceable Frontend (смежное, G6-future)

Бизнес-логика живёт в `rimloc-domain` → `rimloc-services`; фронтенды (сейчас — официальный
Svelte shell) работают через **framework-neutral типизированный UI-контракт** (version +
capabilities) и **единый транспорт** (UI → RimLocClient → TauriTransport; никаких
`invoke()` в компонентах). Mock-клиент и representative mock corpus — постоянные dev-фичи.
Полные альтернативные UI — build-time выбор разработчиком; runtime-загрузка стороннего UI —
отдельная будущая security-задача. Детали: `UI_SDK_MANDATE.md`;
`UI_EXTENSION_ARCHITECTURE.md` — написать после появления реализации, не опережая её.

## 8. Тесты (на этапе L1/L2)

import valid / malformed pack / unsupported schema / path traversal / missing token
fallback / theme switch / layout switch / safe-mode recovery / a11y warning / profile
restore — кросс Light/Dark и ОС.
