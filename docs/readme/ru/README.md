# RimLoc

<p align="center">
  <img src="../../assets/RIMLOC-baner.png" alt="RimLoc" />
</p>

<p align="center">
  <strong>RimWorld-first рабочая станция локализации — local-first, open-source и с архитектурой адаптеров.</strong>
</p>

<p align="center">
  <a href="../../../README.md">English</a> · <strong>Русский</strong>
</p>

<p align="center">
  <a href="https://github.com/sponsors/0-danielviktorovich-0"><img src="https://img.shields.io/badge/Sponsor-GitHub-%23ea4aaa?logo=github-sponsors" alt="GitHub Sponsors" /></a>
  <a href="https://buymeacoffee.com/danielviktorovich"><img src="https://img.shields.io/badge/Buy%20Me%20a%20Coffee-donate-FFDD00?logo=buymeacoffee&logoColor=black" alt="Buy Me a Coffee" /></a>
  <a href="https://ko-fi.com/danielviktorovich"><img src="https://img.shields.io/badge/Ko--fi-support-FF5E5B?logo=ko-fi&logoColor=white" alt="Ko-fi" /></a>
</p>

> **Статус: pre-beta.** Проект активно развивается. Первый production-фокус — RimWorld; новый десктопный интерфейс переводится на React R1. Стабильный публичный десктопный релиз пока не заявляется.

[Документация](https://0-danielviktorovich-0.github.io/RimLoc/ru/) ·
[Начало работы](../../ru/getting-started.md) ·
[Безопасность](../../../SECURITY.md) ·
[Как помочь проекту](../../../CONTRIBUTING.md) ·
[Поддержать разработку](../../ru/community/support.md)

---

## Что такое RimLoc

RimLoc помогает переводить и сопровождать RimWorld-контент без ручной беготни по десяткам XML и без записи поверх оригинальных файлов игры/Workshop.

У проекта два основных интерфейса:

- **десктопная рабочая станция (Tauri 2 + React 19)** — создать/открыть проект, редактировать строки, менять целевой язык, валидировать, пользоваться глоссарием, смотреть контекст, собирать/экспортировать результат;
- **Rust CLI и service layer** — детерминированная автоматизация, CI, пакетные операции, обмен форматами и диагностика.

Архитектурно RimWorld — первый production-адаптер, а не вечная граница продукта. Общие сущности проекта, переводы, TM, глоссарий, ревизии и review не должны зависеть от конкретной игры.

## PO — опциональный формат, а не внутренний формат проекта

RimLoc **не хранит проект как PO** и не требует обязательного PO-round-trip.

Основной GUI-сценарий:

~~~text
найти источник
→ создать/открыть проект
→ перевести прямо в RimLoc
→ проверить/review
→ собрать или экспортировать результат
~~~

PO нужен как **формат обмена**, когда он полезен:

- отдать работу в Poedit или другой CAT;
- вернуть готовый перевод из существующего процесса команды;
- использовать форматные CLI-команды <code>export-po</code> / <code>import-po</code>.

CLI сохраняет эти команды потому, что они полезны сами по себе. Кроме того, <code>build-mod --from-root</code> умеет собирать мод-перевод из уже готового дерева <code>Languages/&lt;язык&gt;</code> без PO.

Долгосрочная модель: PO/CSV/XLIFF/XML — это адаптеры ввода/вывода вокруг канонического проекта, а не обязательная сердцевина RimLoc.

## Что уже есть

| Область | Текущее состояние |
| --- | --- |
| RimWorld scan / канонический inventory | реализовано в Rust-конвейере |
| Ручное редактирование проекта | реализовано в desktop workflow |
| Validation / findings | реализовано |
| Multi-target | реализовано |
| Глоссарий проекта | реализован с persistence |
| PO interchange | реализован как опциональный CLI/service workflow |
| Build/export translation-only мода | реализован в Rust-инструментах |
| Обновление существующего перевода | pre-beta hardening |
| Translation Memory | pre-beta: автопереиспользование + импорт + ручное управление |
| AI/providers | архитектура/UI есть, production-интеграция ещё укрепляется |
| Runtime Bridge | test-lab, не обычная production-функция |
| Другие игры/приложения | пока только архитектура адаптеров |

## Текущий фокус RimWorld

Основная цель — качественно закрыть реальные сценарии RimWorld 1.6:

- моды;
- Core/DLC как источники;
- language packs;
- обновление существующих переводов;
- несколько целевых языков;
- безопасная сборка результата.

Поддержка более старых раскладок сохраняется там, где это практично.

## Новый GUI

Будущий production-фронтенд:

<code>gui/tauri-app/frontend-react/</code>

Старый Svelte-фронтенд временно остаётся:

<code>gui/tauri-app/frontend-v2/</code>

как замороженный fallback/reference до завершения миграции.

React-candidate собирается явно:

~~~bash
cd gui/tauri-app/frontend-react
npm install
npm run build

cd ../src-tauri
cargo tauri build --config tauri.react.conf.json
~~~

## Быстрый CLI-пример

~~~bash
cargo build -p rimloc-cli

cargo run -p rimloc-cli -- scan --root ./test/TestMod --format json
cargo run -p rimloc-cli -- validate --root ./test/TestMod
~~~

PO, если нужен внешний CAT:

~~~bash
cargo run -p rimloc-cli -- export-po \
  --root ./test/TestMod \
  --out-po ./logs/TestMod.po \
  --lang ru
~~~

Сборка напрямую из готовой структуры Languages — без PO:

~~~bash
cargo run -p rimloc-cli -- build-mod \
  --from-root ./Mods/MyTranslatedMod \
  --out-mod ./dist/MyTranslatedMod-RU \
  --lang ru \
  --dry-run
~~~

## Local-first и безопасность

Исходники игры и модов считаются read-only. RimLoc пишет в project storage и явно выбранные выходные каталоги.

До beta отдельно проверяются:

- containment путей и symlink/path traversal;
- Tauri capabilities / IPC;
- секреты провайдеров;
- sanitization diagnostics;
- отсутствие automation/test hooks в production-артефакте;
- зависимости и CodeQL.

О проблемах безопасности сообщайте по [SECURITY.md](../../../SECURITY.md).

## Архитектура

~~~text
Localization source
      ↓
LocalizationAdapter
      ↓
canonical SourceEntry inventory
      ↓
Project / translations / revisions
      ↓
TM / glossary / validation / AI
      ↓
RimLocClient
      ↓
CLI / desktop GUI / future integrations
~~~

Идея проста: новый адаптер для другой игры/приложения должен добавляться как ограниченный модуль, а не требовать переписывания редактора, TM или проекта.

Подробнее: [Localization adapters](../../architecture/LOCALIZATION_ADAPTERS.md).

## До первой beta

Приоритет:

- React R1;
- Existing/update workflow;
- TM + glossary;
- практические сравнения с существующими RimWorld-инструментами;
- security/dependency/code-scanning;
- macOS/Windows acceptance;
- актуальная документация.

Цель — сначала выпустить сильную RimWorld beta и получить реальные отзывы.

## Долгосрочная цель

Если проект окажется полезным, ориентир шире:

- контекст и game-localization UX уровня Gridly;
- глубина профессионального CAT workflow уровня Trados;
- community/continuous-localization идеи уровня Crowdin;
- при этом local-first, open-source и adapter-oriented.

## Документация и сообщество

- [Документация](https://0-danielviktorovich-0.github.io/RimLoc/ru/)
- [Начало работы](../../ru/getting-started.md)
- [Для переводчиков](../../ru/guide/translators.md)
- [GUI](../../ru/guide/gui.md)
- [CLI](../../ru/cli/)
- [Contributing](../../../CONTRIBUTING.md)
- [Security](../../../SECURITY.md)
- [Support](../../../SUPPORT.md)
- [Поддержать проект](../../ru/community/support.md)

## Лицензия

GNU GPL v3 — см. [LICENSE](../../../LICENSE).
