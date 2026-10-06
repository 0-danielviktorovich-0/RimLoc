---
title: Desktop GUI
---

# Графический интерфейс RimLoc

Desktop-приложение построено на **Tauri 2** и использует общий Rust service layer.

## Текущее состояние фронтендов

- **React 19 / frontend-react** — будущий production UI и текущая R1-линия.
- **Svelte / frontend-v2** — замороженный legacy fallback/reference на время миграции.
- **frontend/** — старый исторический shell, не продуктовая цель.

Не считайте любой локальный <code>.app</code> React-сборкой автоматически. Во время разработки уже были stale-bundle коллизии, поэтому owner/test artifact должен иметь явную build identity.

## Для чего нужен React workspace

Текущий product workflow включает:

- Home / recent projects;
- новый перевод;
- открыть/обновить существующий;
- workspace/editor;
- переключение target locale;
- Checks/validation;
- glossary;
- Translation Memory по мере включения в pre-beta;
- build/export;
- diagnostics;
- settings и language management.

Рабоче выглядящий control должен появляться только тогда, когда соответствующая backend/adapter capability действительно доступна.

## Обычный workflow переводчика

1. Создать/открыть проект.
2. Выбрать источник и target locale.
3. Переводить прямо в редакторе.
4. Смотреть контекст/source.
5. Запустить validation.
6. Собрать/экспортировать результат в отдельный output-каталог.

**PO для этого не нужен.** PO остаётся опциональным interchange-форматом для внешних CAT-инструментов.

## Сборка React-candidate

Нужны Rust, Node.js 20+ и системные зависимости Tauri.

~~~bash
cd gui/tauri-app/frontend-react
npm ci
npm run build

cd ../src-tauri
cargo tauri build --config tauri.react.conf.json
~~~

Development-запуск:

~~~bash
cd gui/tauri-app/src-tauri
cargo tauri dev --config tauri.react.conf.json
~~~

## Безопасность

- Game/Workshop/source каталоги считаются read-only.
- Generated output должен идти в отдельный каталог.
- Автотесты используют изолированный RimLoc data/profile, чтобы фикстуры не попадали в Recent Projects владельца.
- Production artifact не должен содержать automation/test bridges.

## Визуальный статус

React R1 следует утверждённому Lovable-derived design direction. Старые screenshots в документации могут показывать Svelte до замены на owner-approved React-набор.

## См. также

- [Начало работы](../getting-started.md)
- [Гайд переводчика](translators.md)
- [Решение проблем](../troubleshooting.md)
- [Frontend boundary](../../architecture/FRONTEND_UI_BOUNDARY.md)
