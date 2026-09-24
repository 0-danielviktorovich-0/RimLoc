<!-- mandate_id: g6-ui-sdk | wave: G6 | scope: replaceable frontend readiness: neutral contract, one transport, mocks -->
# МАНДАТ: Replaceable Frontend / UI SDK Architecture (G6-future, 2026-09-24)

Future-readiness ограничение, НЕ стройка: официальный Svelte UI остаётся основным.

- Бизнес-логика вне фронтенда: rimloc-domain/core → rimloc-services → стабильный типизированный
  UI/service contract → реализации фронтендов. Фронтенд не владеет семантикой проекта,
  идентичностями переводов, правилами валидации, извлечением, персистенсом, eligibility.
- Текущий Svelte = «Official RimLoc UI Shell»; будущие шеллы (React/Vue/Solid/...) возможны
  против того же контракта без дублирования ядра. Сейчас фреймворки не вводить.
- Framework-neutral типизированный контракт (Projects/Languages/Entries/Translations/
  Validation/Jobs/Providers/Build/Diagnostics/Settings+capabilities), request/response/event
  типы; ui_contract_version + capabilities (multiTarget, wordInfo, wholeGameLocalization,
  chatBatchTranslation, knowledgeRules, diagnostics, providerManagement); additive evolution,
  breaking = версия + миграции.
- TS SDK/клиент без Svelte-runtime; официальный фронтенд ест тот же клиент, а не приватный IPC.
- **ОДИН транспорт**: централизованный Tauri IPC (UI → RimLocClient → TauriTransport → backend),
  никакие invoke(...) в компонентах — единый шов безопасности/версионирования/тестов.
- Mock RimLoc backend/client — постоянная dev-фича (детерминированные сценарии без бэкенда);
  representative corpus: first run / normal mod / huge project / multi-target / existing
  translation / validation failures / provider offline / sourceChanged / TKey multi-context /
  diagnostics failure / whole-game.
- UI conformance suite (create→choose content→languages→add target→edit→switch→import→review→
  validate→build→provider→diagnose); a11y-контракт для любых UI (keyboard, contrast,
  reduced motion); производительность (startup, 10k+ editor, search, target switching).
- gui/ граница (contract/ ui-kit/ frontends/ src-tauri/) — при следующем рефакторинге, не
  сейчас (churn). UI-kit ≠ бизнес-компоненты: шарим токены/иконки/схемы/контракт/моки/тесты,
  Svelte-компоненты остаются в Svelte-реализации.
- Уровни расширения: Theme Pack (токены) → Layout Pack (раскладка) → Full UI Shell
  (разработчик). Theme Pack не получает исполняемых привилегий. Альтернативные UI — сначала
  build-time выбор разработчиком; runtime-загрузка стороннего UI JS — отдельная будущая
 安全问题. `cargo xtask create-ui-shell` — потом, не в RC.
- Проектные данные не в фронтенд-форматах (не serde Svelte-сторов); UI-преференции могут
  быть shell-specific, продукт-стейт (UI language, target locale, project data) — нет.
- Доки потом: UI_EXTENSION_ARCHITECTURE.md (не писать опережающих реализацию).

СЕЙЧАС ТРЕБУЕТСЯ ТОЛЬКО: чистая service/UI граница; централизованный транспорт; нейтральные
контракты; структурные токены; переиспользуемые моки; ноль бизнес-правды во фронтенде.

## Решения lead для post-freeze binding

Это требования к реализации и приёмке, не заявление о готовности. Начало binding:
приняты L и provenance/identity, выполнен compact delta review. Новые UI-фреймворки,
универсальная plugin-система и перестройка каталогов по-прежнему вне RC.

- Создание проекта вызывает принятый `build_project`, включая version/patch/source
  resolution. `scan_units_auto` с отдельным bridge не заменяет этот путь.
- `projectId` — сохранённая непрозрачная идентичность; путь — изменяемые метаданные.
  Entry ID содержит полный принятый discriminator, не один отображаемый ключ.
- Изменения проекта сериализованы: подготовить состояние → атомарно сохранить →
  подтвердить и опубликовать revision. Ошибка сохраняет прежнее состояние и dirty draft.
  `expectedRevision` и project/session epoch защищают от старых ответов и записи после
  смены проекта/языка. Внешние изменения проверяются сильнее одного mtime; без silent overwrite.
- Native validation/preview/build используют canonical translations и один план вывода,
  без промежуточного PO. Service guard запрещает вывод во все известные source/game/
  Workshop roots, включая не выбранный сейчас мод. Проверяются symlink и path traversal.
- Отмена — проверяемые checkpoints и безопасный staged output, не только UI callback.
  Отмена/сбой не возвращают успех и не устанавливают частичный результат в источники.
- Диагностика всегда очищена: исходный failed operation ID, причины и affected entries
  сохраняются; preview/copy/write используют один sanitized payload, без `redact=false`.
- Source actions принимают project/entry/context identity. Backend разрешает известный
  путь, проверяет корни, symlink, тип и наличие файла; чтение ограничено по размеру.
  External editor — executable + массив аргументов, без shell interpolation.
- Production Tauri registration не оставляет callable legacy `apply_translation`,
  arbitrary-write/open/plugin routes в обход guard. Наличие permissions-файла само по
  себе не доказывает enforcement — проверяется фактическая поверхность команд.
- Один нейтральный SDK проверяет method/params/response/version. Capabilities отражают
  реальные операции; typed unsupported допустим в промежуточном срезе, не как RC completion.
- Live production entry не содержит MockTransport, fake connected providers или fake
  successful saves. Явный demo/dev flow отделён; глобальный badge зависит от data mode.
- Обязательный путь нового GUI: create → source → targets → edit/switch → save/restart/
  reopen → review/validate → safe build → inspect source → diagnose actual failure.
  Ручной chat-batch workflow с ID, parsing, stale guard и review/apply работает без API;
  existing/TM/glossary сохраняют provenance и read-only scope.
- Сначала небольшой typed seam, затем независимые project-journey и discovery/source/
  diagnostics модули. Один владелец SDK types и Tauri registrations. Conformance fixtures
  дополняются проверкой собранного Tauri-приложения на реальном read-only моде и изолированных
  файлах. Наличие CI-конфига не заменяет запуск на платформе; signing/publishing —
  отдельные внешние гейты, не разрешение на платные действия или выпуск.
