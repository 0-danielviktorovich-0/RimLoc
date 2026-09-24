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
