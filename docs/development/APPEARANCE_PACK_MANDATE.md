# МАНДАТ: Customizable UI / Theme / Layout Pack Architecture (G6, 2026-09-24)

Кумулятивный мандат владельца. Три уровня кастомизации, НЕ смешивать:
- **L1 Theme Pack** — декларативные токены (палитры light/dark, семантика, типографика,
  density, spacing, радиусы, границы, тени, поверхности, иконки, motion, звуки, ассеты).
  Никакого исполняемого кода (JS/Svelte/shell/Python/Rust).
- **L2 Layout Pack** — декларативная раскладка одобренных регионов (topbar, тулбар, левая
  навигация, editor, target-панель, context/details, suggestions, status, review sidebar,
  command actions): placement/visibility/sizing/collapse/density/responsive в безопасных
  рамках. Примерыcapability: Professional / Minimal / Laptop / Review раскладки.
- **L3 Developer UI Extension** — отдельная будущая система доверенных исполняемых
  расширений (explicit trusted mode, SDK, capability declarations, sandbox, revocation,
  safe mode). НЕ через theme-pack loader; НЕ приоритет до зрелости L1/L2.

Ключевые требования: встроенные направления (Precision/Aurora/Workshop/Editorial) со
временем переезжают на то же pack-представление (dogfooding); light/dark обязательны,
system = селектор; не указанный токен → fallback к базовым токенам; визуальный Theme
Editor (Save/Duplicate/Reset/Export/Import; raw-манифест опционально); профили
Appearance/Workspace (Default/Compact Translator/Laptop/OLED...) без секретов и данных
переводов; global vs project override; пакеты `.rimloctheme`/`.rimloclayout`/`.rimlocprofile`
с schema version/name/author/license/compat/manifest; versioned schema + миграции; safe
fallback (сломанный пак → disable → built-in Precision → объяснение; safe-ui recovery);
a11y-валидация на import/edit (контрасты, focus, reduced-motion); ассеты — только пассивные
(валидация типа/размера/путей/traversal); ноль привилегий у паков (нет ФС/shell/секретов/
провайдеров/данных проекта/сети — только презентация); hot reload токенов для авторов через
Style Lab; Style Lab = авторский инструмент, не прод-навигация; community sharing — потом
(сначала файлы/GitHub); лицензии/авторы в метаданных; **паки не форкают бизнес-логику** —
одни команды/валидация/модель/безопасность. Тесты: import valid/malformed/unsupported
schema/path traversal/missing token fallback/switch/safe-mode/a11y warning/profile restore,
кросс Light/Dark/Win/mac/Linux.

СЕЙЧАС: токены структурированы (уже есть), Style Lab чист архитектурно (уже есть), не
создавать style-специфичные форки компонентов, задокументировать границу паков (этот файл).
ОПЦИОНАЛЬНО дешёвый POC first-party Theme Pack. ОТЛОЖЕНО: Layout Builder, публичный
реестр, исполняемые extensions. Не задерживать backend freeze.
