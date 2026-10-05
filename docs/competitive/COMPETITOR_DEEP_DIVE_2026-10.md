# RimLoc: глубокий аудит конкурентов (Tier A) — source-инспекция

Дата: 2026-10-05 · Метод: только веб/GitHub-инспекция (`gh api` метаданные + деревья + чтение исходников через contents API), ничего не запускалось и не клонировалось. Evidence level по шкале RimLoc (0 NOT_IMPLEMENTED … 7 IN_GAME_PROVEN). Уровень 2 (SOURCE_CONFIRMED) поставлен только там, где файлы исходников реально прочитаны через gh api — файлы перечислены в разделе. Уровень выше 2 никому не ставился: тесты конкурентов не запускались (наличие тестов отмечено в notes).

## Сводная таблица

| # | Имя | Repo | Язык | Лицензия | Посл. коммит | Статус | Evidence | Угроза — почему |
|---|-----|------|------|----------|--------------|--------|----------|-----------------|
| 1 | Remis | github.com/Drlinglong/Remis | Python (PySide/desktop) | AGPL-3.0 | 2026-09-26 | живой | 2 | **HIGH** — единственный полноценный «суперсет»: мультиигровая платформа с проектами, MT-роутером (7+ провайдеров), глоссарием, переиспользованием переводов, агент-интеграцией; RimWorld уже в preview |
| 2 | RimWorldModTranslator (NicoriciN89) | github.com/NicoriciN89/RimWorldModTranslator | Python | Apache-2.0 | 2026-07-22 | живой (молодой) | 2 | **MED** — функционально ближайший клон суперсета RimLoc: оффлайн Argos+Ollama, Defs с наследованием, экстракция текста из PatchOperations, LoadFolders, глоссарий, инкрементальность, 12 тест-файлов; но 0 звёзд, Windows-GUI |
| 3 | Translation Forge (Momaomao8787) | github.com/Momaomao8787/Translation-Forge | Python | MIT | 2026-10-05 (сегодня) | живой, активный | 2 | **MED** — качественный human-in-the-loop CLI+GUI (CSV-экспорт/импорт, качество-чеки, stale keys, scaffolds), свежий и быстроразвивающийся, но без MT и без проектной модели |
| 4 | Mod Translation Toolkit (DrizztGaming) | github.com/DrizztGaming/Mod-Translation-Toolkit | PowerShell (GUI) | MIT | 2026-09-11 | живой | 2 | **MED** — Windows GUI с Google/DeepL/LibreTranslate, слоистым глоссарием EN→PL, update-detection против обновившегося мода и Workshop-интеграцией; PL-центричный монолит 13.8k строк |
| 5 | RimWorldAiTranslator (chance496) | github.com/chance496/RimWorldAiTranslator | C# (WinForms/.NET) | MIT | 2026-07-16 | живой | 2 | **MED** — сильная инженерия (реальная TM, review-workspace, atomic storage, 46 тест-файлов, LoadFolders-парсер с лимитами), но жёстко корейская ниша (RMK-интеграция, ko-глоссарий) |
| 6 | rimworld-mod-translator (laskinss27-cmyk) | github.com/laskinss27-cmyk/rimworld-mod-translator | Python | MIT | 2026-08-13 | живой | 2 | **LOW** — zero-install Google Sheets workflow с плейсхолдерами и юнит-тестами, RU/EN UI (русскоязычный автор), но ручной цикл и нет встроенного MT |
| 7 | RimTranslate (winterheart) | github.com/winterheart/RimTranslate | Python | GPL-3.0 | 2024-05-03 | спит ~1.5 года | 2 | **LOW** — одиночный PO-скрипт (polib/lxml) с compendium-TM; предвестник PO-подхода RimLoc, не развивается |
| 8 | RimWorldModTranslator (etejasdgjjjj532) | github.com/etejasdgjjjj532/RimWorldModTranslator | Python | MIT | 2026-08-17 | живой (микро) | 2 | **NONE** — 180-строчный XLSX-экстрактор (Defs/Keyed/Patches → таблица) для японского |
| 9 | RimworldModTranslator (TokcDK) | github.com/TokcDK/RimworldModTranslator | C# (WPF) | GPL-3.0 | 2025-04-27 | спит ~5 мес | 2 | **LOW** — WPF-редактор таблицы переводов с автосейвом; не развивается |
| 10 | rimworld-mod-llm-auto-translator (JalapenoLabs) | github.com/JalapenoLabs/rimworld-mod-llm-auto-translator | TypeScript | MIT | 2025-08-05 | заброшен | 2 | **NONE** — выходной-дневной OpenAI-скрипт; собственный llm.ts помечен «TO BE DEPRECATED in favor of LangGraph» |
| 11 | Rimworld-Mod-Translator (kelvinauta) | github.com/kelvinauta/Rimworld-Mod-Translator | JavaScript (Node) | нет | 2024-06-25 | мёртв (1 день жизни) | 2 | **NONE** — 60-строчный DeepL-скрипт, переводит только 3 тега Defs |
| 12 | RimTrans (RimWorld-zh) + исторические форки | github.com/RimWorld-zh/RimTrans | C# WPF (master) + TypeScript/Electron (4.0-alpha) | MIT | 2022-12-10 | мёртв ~4 года | 2 | **NONE** — исторический лид жанра, навсегда alpha; 19+ мёртвых форков; легаси-бренд в CN-сообществе |
| 13 | RimTrans (Aironsoft) | github.com/Aironsoft/RimTrans | C# | MIT | 2021-09-13 | мёртв (1 пуш) | 1 | **NONE** — переиздание дерева RimWorld-zh/RimTrans master тем же составом проектов (Builder/Lite/Test), создано и брошено в один день |
| 14 | rimworld-autonomous-translator (AutonomoAI) | github.com/AutonomoAI/rimworld-autonomous-translator | — (кода нет) | нет | 2026-01-14 | маркетинг-репо | 1 | **NONE** как инструмент — в репо только README/INVESTORS/картинки; но кейс «124 996 слов → играбельный арабский за $8.86» — доказательство feasibility LLM-пайплайна, код закрыт |
| 15 | rwmt (ложное срабатывание) | github.com/rwmt/* | C# | MIT | 2026-08-03 | живой | 2 | **NONE** — rwmt = RimWorld Multiplayer (Zetrith), не локализация; ближайшее — Multiplayer-Locale, переводы самого мода, не инструмент |
| 16 | Ludeon official workflow (контекст) | github.com/Ludeon/RimWorld-&lt;Language&gt; | — (данные+PR) | — | активны | официальный канал | 1 | **NONE** — официальные переводы идут через GitHub-репы Ludeon/RimWorld-&lt;язык&gt; DefInjected + PR; отдельного «официального тулза» нет — RimLoc может стать таким тулом |

Итого: 14 реальных инструментов (из них 1 маркетинг-пустышка), 2 контекстных записи. Level 2 (source-confirmed) — 13 конкурентов, level 1 — 2 (Aironsoft, AutonomoAI; + Ludeon context).

---

## 1. Remis — Drlinglong/Remis

- **Repo**: https://github.com/Drlinglong/Remis (создан 2025-07-13, 2295 файлов, ~180 МБ, 26 звёзд)
- **Стек/лицензия**: Python (PySide-десктоп + SQLite + FastAPI-подобный слой агентов), AGPL-3.0
- **Последний коммит**: 2026-09-26 · **живой**, активная разработка, релизы v2.x→v3.x (release notes в репо)
- **Установка/запуск**: десктоп-приложение (сборка скриптами), локальные проекты (`.remis_project.json`), встроенные агенты (Codex/Copilot, `.agents/skills/remis-agent/SKILL.md`, MCP-adapter в docs)
- **Фичи по исходникам** (читал: `scripts/core/game_adapters/rimworld.py`, `scripts/core/game_adapters/translation_reuse.py`, `scripts/core/api_handler.py`, дерево `main`):
  - Скан: `Languages/<lang>/**` (Keyed/DefInjected/Strings, .xml+.txt), `Defs/**/*.xml`, About.xml (packageId), LoadFolders.xml **с выбором версии-ветки** (`_effective_roots`, `v1.6`-теги, диагностика `loadfolders_version_unknown`/`loadfolders_branch_missing`/`loadfolders_condition_unknown`), инферс версии игры из папок
  - Defs: парсинг полей по фиксированному whitelist `_V16_DEF_FIELDS` (ThingDef::labelShort и т.д.), fallback на Defs при отсутствии DefInjected, language overrides, дедуп/overlay-коллизии, диагностика дублей ключей
  - **Patches: НЕ извлекаются** — честная диагностика `patch_runtime_unknown` («PatchOperations can alter translatable Defs; offline extraction cannot resolve their runtime results»); Assemblies — `assembly_strings_unavailable`
  - MT: роутер провайдеров — OpenAI-совместимые (несколько id), Anthropic, Gemini, DeepSeek, Grok, ModelScope, локальные (Ollama/LM Studio; Gemini CLI удалён); плюс «model arena» (сравнение моделей, tests/test_model_arena_*)
  - TM: `translation_reuse.py` — консервативное переиспользование переводов с диска и из `translation_dirs` проекта; checkpoint_manager (чекпойнты `.remis_checkpoint_*.json`)
  - Глоссарий: glossary_manager + glossary_health_service/health_reviewer (ревью здоровья глоссария), легаси per-game JSON-глоссарии в archive/
  - Валидация: per-game валидаторы (`scripts/config/validators/{ck3,eu4,eu5,hoi4,stellaris,vic3}_rules.py`), rimworld: validate_tokens, дубли, коллизии
  - Экспорт: рендер в игровые форматы (DefInjected/Keyed XML, Strings .txt, Paradox .yml), сборка **отдельного перевод-мода** (About.xml с `packageId.<lang>`, supportedVersions, modDependencies), proofreading-CSV
  - Версионирование: `.metadata/metadata.json`, чекпойнты, FORMAT_RULES_VERSION (`rimworld-1.6-v1`), publication/delivery identity (mars_pipeline)
- **Ограничения**: RimWorld — в статусе **preview** (основной фокус Paradox: Stellaris/Vic3/EU5/CK3/HoI4 + Surviving Mars, Project Zomboid в preview); патчи и C#-строки вне досягаемости; Def-поля — только фиксированный список 1.6; репо перегружен архивом и дизайн-доками; AGPL-3.0 (не для встраивания)
- **Evidence**: 2 — `rimworld.py` (439 строк, полный), `api_handler.py`, `translation_reuse.py` + дерево; есть `tests/core/test_rimworld_adapter.py`, `tests/core/game_adapters/test_rimworld_external_fixture.py`, `tests/test_mod_discovery.py` (не запускал)

## 2. RimWorldModTranslator (NicoriciN89)

- **Repo**: https://github.com/NicoriciN89/RimWorldModTranslator (создан 2026-07-21, 0 звёзд)
- **Стек/лицензия**: Python (PyInstaller GUI, `run_gui.py` + `.spec`), Apache-2.0
- **Последний коммит**: 2026-07-22 · молодой, 2.5 месяца без пуша (но и возраст такой же)
- **Установка/запуск**: Windows GUI (exe через PyInstaller), полностью оффлайн
- **Фичи по исходникам** (читал: `src/scanner.py`, `src/translator.py`, `src/patches.py`, `src/glossary.py` (обзор), `src/llm_polish.py`, `src/rimworld_rules.py`, `src/generator.py`, `src/incremental.py`; дерево тестов):
  - Скан: `Languages/English/{Keyed,DefInjected,Strings}` + `Defs/*.xml` **с разрешением наследования Name/ParentName**; `Patches/*.xml` — **извлечение переводимого текста из PatchOperationAdd/Replace/Insert через разбор типовых xpath** (`Defs/ThingDef[defName="X"]/...`, li[N]-индексы; сложные xpath намеренно пропускаются «лучше пропустить редкий сложный патч, чем сгенерировать неверный ключ»)
  - LoadFolders.xml — «источник истины», выбор версии 1.0–1.6, fallback на папки-версии
  - MT: **Argos Translate (офлайн NMT)** с бандл-пакетами (ru/de/fr), защита плейсхолдеров, анти-обрезание (Argos молча режет хвост с `</color>` — воспроизведено и залатано); опциональная «полировка» локальной LLM через **Ollama** (дефолт qwen2.5:7b, батчи, таймауты, проверка версии модели)
  - Глоссарий: 28 КБ `glossary.py` + пер-языковые модули терминов (de/fr; ru)
  - TM: `incremental.py` — `.translation_cache.json`-снимок английских строк; повторный запуск переводит только новое/изменённое
  - Валидация: `rimworld_rules.py` — плейсхолдеры `{0}`/`{PAWN_nameDef}`/`[species]`, rich-text `<color>`, never-translatable теги/идентификаторы, RuleStrings `key->value`
  - Экспорт: сборка полноценного мода-русификатора (Languages/<Lang> + About.xml с зависимостью от оригинала, экранирование чужих значений)
- **Ограничения**: Windows-центризм (диагностика через PowerShell); качество Argos ниже облачного MT; xpath-парсер патчей покрывает только типовые формы; 0 adopters/звёзд
- **Evidence**: 2 — перечисленные выше файлы; 12 тест-файлов (test_scanner, test_patches, test_glossary, test_rimworld_rules, test_llm_polish, …) — не запускал

## 3. Translation Forge (Momaomao8787)

- **Repo**: https://github.com/Momaomao8787/Translation-Forge (создан 2026-07-01, 2 звезды)
- **Стек/лицензия**: Python (CLI `forge` + GUI, PyInstaller-спеки), MIT; локализованные ошибки (zh_fallback), UI-локали de/es/fr/ja/ko/ru
- **Последний коммит**: 2026-10-05 (день аудита) · **живой, самый активный пушист в выборке**
- **Установка/запуск**: CLI (argparse: check/export/import/scaffold/fix-src/default-prefix) + десктоп GUI
- **Фичи по исходникам** (читал: `core/scan.py`, `core/cli.py`, `core/export.py`, `core/paths.py`, `core/check_quality.py` + обзор `core/def_inherit.py`, `core/stale_keys.py` по импортам):
  - Скан: обнаружение корней Defs (`discover_defs_roots`), DefInjected, выбор версии игры из About.xml `supportedVersions` (max), мод = наличие About.xml **или** LoadFolders.xml (маркер; ветвление LoadFolders не обнаружено)
  - Defs: `DefIndex` с построением индекса наследования; `har_field_rules` — правила пропуска полей; генерация DefInjected-ключей с вложенностью и li-индексами
  - Stale keys: сравнение существующего перевода с исходником (`find_stale_keys` + `read_patch_text` — патчи читаются только для stale-анализа, не как источник задач)
  - Валидация: `check_quality` — дубликаты тегов, смешение write-стратегий, смешение форматов pending
  - Экспорт: **CSV** (human workflow: выгрузка pending → перевод в таблице → импорт обратно `load_entries_csv`/`load_entries_xml`), запись DefInjected (`definjected_write`), scaffolding языковой структуры, meta-файлы рядом с экспортом
  - MT: **нет** (чистый human-in-the-loop)
- **Ограничения**: нет MT/TM-механики кроме повторного импорта; нет версионирования проектов; китайскоязычное происхождение (ошибки с zh-fallback) — для RU-аудитории интерфейс есть (ru-локаль CLI)
- **Evidence**: 2 — перечисленные файлы; обширные тесты (test_check_quality, test_cli_scaffold, test_definjected_key, test_export_merge, test_field_collect, …) — не запускал

## 4. Mod Translation Toolkit (DrizztGaming)

- **Repo**: https://github.com/DrizztGaming/Mod-Translation-Toolkit (создан 2026-08-01, 3 звезды; zips релизов прямо в репо)
- **Стек/лицензия**: PowerShell WPF-GUI (один файл `src/RimWorld/ModTranslationToolkit.ps1` — 13 858 строк), MIT
- **Последний коммит**: 2026-09-11 · живой, версии 0.1→0.10.26
- **Установка/запуск**: Windows-only (VBS-лаунчер без консоли), поддержка профилей: RimWorld Game/Mod, Kenshi, Project Zomboid (экспериментально)
- **Фичи по исходникам** (читал: README, `src/RimWorld/ModTranslationToolkit.ps1` — grep по 13.8k строк):
  - Скан: Languages/English (Keyed + существующий DefInjected приоритетнее fallback-экстракции из Defs), **версионные папки (1.6/Languages/...)**, следование LoadFolders.xml при поиске активных языковых папок; поиск Workshop-модов Steam-библиотек
  - MT: **Google Translate API, DeepL (Free/Plan), LibreTranslate** (self-hosted, проверка доступности языка на сервере); ключи шифруются DPAPI (`Protect-ToolkitSecret`)
  - Плейсхолдеры: маскирование до перевода (`Protect-TranslationPlaceholders`) → реставрация → `Test-TranslationPlaceholderIntegrity`, throw при потере/порче токена
  - Глоссарий: **слоистый EN→PL** (Mod > RimWorld > General) с предпочитаемыми переводами, синонимами, **запрещёнными вариантами** и диагностикой несогласованности; оффлайн-панель словаря; Pester-тесты терминологии (`tests/Terminology.Tests.ps1`)
  - TM/обновления: «Update existing translation» — дифф обновившегося исходного мода против существующего перевод-мода, сохранение совпавших строк, показ только нового/пропавшего
  - Экспорт: CSV import/export, сборка перевод-мода с сохранением packageId/About.xml, генерация BBCode-описания Workshop, копирование Preview.png с оверлеем флага языка; **принудительная атрибуция тулза в каждом сгенерированном моде**
  - Прочее: DLL/UI-диагностика (эвристический скан сборок на KeyBindingDef/Tooltip/Gizmo), Keybind-диагностика
- **Ограничения**: словари захардкожены под EN→PL (MT-языки шире, но глоссарий нет); PowerShell-монолит — тестируемость и портируемость низкие; виндо-зависимость
- **Evidence**: 2 — README + grep-инспекция основного PS1 (провайдеры, плейсхолдеры, валидация); Pester-тесты только по терминологии

## 5. RimWorldAiTranslator (chance496)

- **Repo**: https://github.com/chance496/RimWorldAiTranslator (создан 2026-07-08, 0 звёзд)
- **Стек/лицензия**: C# / .NET (WinForms-App + Core-библиотека), MIT
- **Последний коммит**: 2026-07-16 · живой (2.5 мес)
- **Установка/запуск**: Windows десктоп (single instance, recovery-запуск), CI (dotnet)
- **Идентичность (п.12 разрешён)**: корейский «작업실» — AI/Google черновой перевод + ручная вычитка + трекинг обновлений модов + интеграция с RMK + безопасное применение корейского
- **Фичи по исходникам** (читал: `Translation/TranslationApiClient.cs`, `Discovery/RimWorldModDiscoveryService.cs`, `Review/TranslationMemoryService.cs`; полное дерево Core/Tests):
  - Discovery: безопасный скан Defs/Languages, **полноценный парсер LoadFolders.xml с жёсткими лимитами** (4 МБ, глубина 32, 50k узлов, 256 версий) — защита от XML-бомб
  - MT: OpenAI-совместимый chat completions (`TranslateOpenAiAsync`: батчи по токенам, ретраи, ProviderRequestException-диагностика, запрет insecure loopback); Google — по описанию (в коде прочитан OpenAI-путь)
  - **TM: настоящая TranslationMemoryService** — записи (Identity/Source/Translation/Origin/Status/SourceChanged/SafeToApply), ранжированные подсказки
  - Глоссарий: GlossaryService + сгенерированный ko-глоссарий в ассетах + rimworld-def-field-rules.txt
  - Валидация/качество: TranslationValidator, ProviderValidator, QualityService, DiagnosticBundleService
  - Безопасность записи: AtomicFile/AtomicJsonStore/FileTransaction + recovery-журнал (FileSnapshotJournal) — транзакционная запись переводов
  - Review-механика: ReviewWorkspace, сравнение снапшотов, ReviewApplyService; RMK-экспорт (RmkExportService)
- **Ограничения**: жёстко корейская ниша (ko-глоссарий, «Korean 적용»-безопасность, RMK); Windows; 0 звёзд
- **Evidence**: 2 — три файла Core + дерево; **46 тест-файлов** (concurrency, data compatibility, artifacts) — не запускал

## 6. rimworld-mod-translator (laskinss27-cmyk)

- **Repo**: https://github.com/laskinss27-cmyk/rimworld-mod-translator (создан 2026-04-10, 1 звезда)
- **Стек/лицензия**: Python (один файл 1796 строк + тест), MIT
- **Последний коммит**: 2026-08-13 · живой
- **Установка/запуск**: CLI-скрипт; цикл «экспорт CSV → Google Sheets (можно с GOOGLETRANSLATE) → импорт»; RU/EN UI
- **Фичи по исходникам** (читал: `rimworld_translator.py` полностью (grep-карта + ключевые блоки), имена тестов `test_translator.py`):
  - Скан: English Keyed (тест: `test_scanner_includes_english_keyed_but_not_other_languages`) + Defs с вложенными полями и li-индексами (`test_def_keys_include_nested_fields_and_list_indexes`); **версионные папки**: исключение чужих версий (`test_excludes_other_game_versions`), переопределение root-дефов версионными (`test_versioned_definition_overrides_root_definition`); эвристики «переводимое vs техническое» (is_definitely_technical/is_likely_text, сбор идентификаторов)
  - Плейсхолдеры: `{...}`/`[Tag]` маскирование/размаскирование, round-trip тест, **импорт отбрасывает перевод с битыми плейсхолдерами** (`test_skips_translation_with_broken_placeholders`)
  - TM: `load_existing_translations` из предыдущего CSV; safe_join против path-escape (`test_safe_join_rejects_parent_escape`)
  - Экспорт: CSV (Sheets-совместимый) + сборка **отдельного мода** без изменения исходника (`test_builds_separate_mod_and_leaves_source_unchanged`)
  - MT: нет встроенного — делегируется Google Sheets
- **Ограничения**: ручной цикл, зависимость от Sheets; нет LoadFolders-ветвления (только версионные папки); нет глоссария
- **Evidence**: 2 — исходник + имена тестов (юнит-тесты покрывают ядро — по шкале репо это фактический UNIT_TESTED, но я тесты не запускал)

## 7. RimTranslate (winterheart)

- **Repo**: https://github.com/winterheart/RimTranslate (создан 2016-09-28, 7 звёзд)
- **Стек/лицензия**: Python 3, один файл `RimTranslate.py` (372 строки), polib + lxml; GPL-3.0
- **Последний коммит**: 2024-05-03 · спит ~1.5 года
- **Установка/запуск**: CLI (argparse): source-dir мода → PO/POT-каталог → DefInjected XML + LanguageData.xml
- **Фичи по исходникам** (читал файл целиком):
  - Скан: Defs/ и Keyed/ мода (включая игровой Core); генерация POT из Keyed и Defs, PO-обновление с сохранением переведённых msgstr
  - TM: **compendium** — существующие переведённые DefInjected/Keyed подмешиваются как память (`create_pot_file_from_keyed(..., compendium=True)`, fuzzy-обработка)
  - Экспорт: Gettext PO/POT (совместимость с CAT-инструментами), генерация DefInjected XML и LanguageData.xml из PO
  - MT/глоссарий/валидация/LoadFolders/патчи: нет
- **Ограничения**: только Defs+Keyed; нет PatchOperations/Strings/LoadFolders; одиночный скрипт без тестов; не развивается
- **Evidence**: 2 — файл прочитан полностью

## 8. RimWorldModTranslator (etejasdgjjjj532)

- **Repo**: https://github.com/etejasdgjjjj532/RimWorldModTranslator (создан 2026-03-22, 1 звезда)
- **Стек/лицензия**: Python (translator.py 180 строк + gui.py), MIT
- **Последний коммит**: 2026-08-17 · живой (микропроект)
- **Фичи по исходникам** (читал `translator.py`): экстракция Defs/Keyed/**Patches** в рабочую книгу XLSX для японского; merge существующих Japanese-переводов (DefInjected+Keyed) как базы; экспорт `translation_work.xlsx`
- **Ограничения**: только японский, нет MT/валидации/сборки мода; GUI-обёртка
- **Evidence**: 2 — translator.py

## 9. RimworldModTranslator (TokcDK)

- **Repo**: https://github.com/TokcDK/RimworldModTranslator (создан 2025-03-23, 0 звёзд; README RU+EN)
- **Стек/лицензия**: C# WPF, GPL-3.0, 51 .cs-файл
- **Последний коммит**: 2025-04-27 · спит ~5 мес
- **Фичи по исходникам** (читал `Helpers/ModHelper.cs` + дерево): WPF-редактор переводов (таблица, автосейв по таймеру, сообщения MVVM), чтение About.xml (modVersion/supportedVersions), загрузка строк выбранного мода (LoadSelectedModStrings)
- **Ограничения**: редактор, а не конвейер; развитие остановлено
- **Evidence**: 2 — один файл + дерево (фичи по классам: GameHelper/ModHelper/EditorHelper, Behaviors, Messages)

## 10. rimworld-mod-llm-auto-translator (JalapenoLabs)

- **Repo**: https://github.com/JalapenoLabs/rimworld-mod-llm-auto-translator (создан 2025-08-04, 0 звёзд)
- **Стек/лицензия**: TypeScript (Node, openai SDK), MIT
- **Последний коммит**: 2025-08-05 · заброшен через день после создания
- **Фичи по исходникам** (читал `src/index.ts`, `src/llm.ts`):
  - Обход XML-файлов мода, перевод LLM-ом (OpenAI) с few-shot промптом «Def → DefInjected»; **правила конвертации путей (включая `1.6/Defs/… → 1.6/Languages/<lang>/DefInjected/…`) зашиты в промпт** — структуру выходного пути определяет модель по инструкции
  - Кэш промптов (`promptCache`) — повторные запуски дешевле, но это кэш, не TM
  - В `llm.ts` прямым текстом: «THIS FILE IS TO BE DEPRECATED, IN FAVOR OF LANGGRAPH»
- **Ограничения**: нет валидации плейсхолдеров, нет Keyed-скана в прочитанном, хрупкость промпт-путей, нет поддержки
- **Evidence**: 2 — оба файла

## 11. Rimworld-Mod-Translator (kelvinauta)

- **Repo**: https://github.com/kelvinauta/Rimworld-Mod-Translator (создан 2024-06-25, 4 звезды; 6 КБ)
- **Стек/лицензия**: Node.js (axios, xml2js), лицензии нет
- **Последний коммит**: 2024-06-25 · мёртв (репо одного дня)
- **Фичи по исходникам** (читал `index.js` целиком): копия папки мода с переводом DeepL API (free) **только тегов label/description/thoughtStageDescriptions** в Defs XML
- **Ограничения**: нет Keyed/DefInjected/патчей, нет защиты плейсхолдеров, нет ответственности за лимиты API
- **Evidence**: 2 — файл прочитан

## 12. RimTrans (RimWorld-zh) + исторические форки

- **Repos**: https://github.com/RimWorld-zh/RimTrans (2016-10-21, 83 звезды, 23 форка) + 19+ форков (nonnonstop, Samael87, tinygrox, kphrx, … — все мертвы, крайний пуш форка 2023-01)
- **Стек/лицензия**: две эпохи в одном репо: master — C#/WPF (`RimTrans.Lite` GUI + `RimTrans.Builder`: DefTypeCrawler, KeyedData, InjectionData, Wiki-интеграция SetDict); 4.0-alpha — перепис на TypeScript/Electron (lerna-монорепо, `app/src/main/services/translator/extractor-*`), автор duduluu; MIT
- **Последний коммит**: 2022-12-10 · мёртв ~4 года, 4.0 так и не вышел из alpha
- **Фичи**: экстракция Defs/English-keyed из модов и Core, редактор переводов, публикация в языковые файлы; это исторический родоначальник жанра «локализатор как GUI-приложение» (Ludeon forums тред 30949, 2016)
- **Ограничения**: эпоха до LoadFolders/PatchOperations; Windows WPF; альфа-статус навсегда
- **Evidence**: 2 — деревья обеих веток, `package.json` (4.0-alpha); тела Crawler-классов не читал (фичи по структуре классов)

## 13. RimTrans (Aironsoft)

- **Repo**: https://github.com/Aironsoft/RimTrans (создан 2021-09-13, 20 звёзд)
- **Стек/лицензия**: C#, MIT
- **Последний коммит**: 2021-09-13 · мёртв (создан и запушен в один день)
- **Идентичность (п.4 разрешён)**: переиздание дерева RimWorld-zh/RimTrans master тем же составом проектов (RimTrans.Builder/Lite/Test, Crawler, PSD-логотипы) — «extracting language files from RimWorld mods»; самостоятельной разработки нет
- **Evidence**: 1 — дерево совпадает с RimWorld-zh master структурно 1:1; blob-файлы не читал

## 14. rimworld-autonomous-translator (AutonomoAI)

- **Repo**: https://github.com/AutonomoAI/rimworld-autonomous-translator (создан 2026-01-14, 0 звёзд)
- **Стек/лицензия**: не определены — **кода в репо нет** (только README, INVESTORS.md, картинки; ветка trunk)
- **Идентичность (п.13 разрешён)**: закрытая платформа Autonomo AI; публичное репо — маркетинг
- **Заявлено (DOC_ONLY)**: автономный self-correcting пайплайн для XML/keyed/C#: целостность плейсхолдеров, глоссарий-консистентность, UI-лимиты длины, script-enforcement (арабский), zero-trust валидация с reject/retry/fallback
- **Кейс-стади**: Core+DLC 124 996 EN-слов → играбельная арабская локализация, локальный LLM, $8.86, 9.37 часов (против $36k+ команды) — доказывает feasibility полноцикловой LLM-локализации, но проверить нельзя
- **Evidence**: 1 — README/маркетинг; исходников нет
- **Угроза**: NONE как инструмент (недоступен), но ориентир по амбиции качества

## 15. rwmt — ложное срабатывание списка

- **Repos**: https://github.com/rwmt/Multiplayer (Zetrith's Multiplayer, 671 звезда), rwmt/Multiplayer-Locale и др.
- **Идентичность**: rwmt = RimWorld Multiplayer org, к локализации модов отношения не имеет. Multiplayer-Locale — переводы самого мода (Python-скрипты обслуживания контента). В список суперсета не включать.
- **Evidence**: 2 — метаданные org и Multiplayer-Locale

## 16. Ludeon official workflow (контекст)

- Официальные переводы игры идут через GitHub-репы `Ludeon/RimWorld-<Language>` (пример: RimWorld-Dutch, RimWorld-Swedish): DefInjected XML руками + PR. Отдельного официального инструмента-приложения нет. Для RimLoc это ниша «официальный локализатор для перевод-команд» — свободная.
- Вне GitHub: «Mod translation tool by Kkokoros» (Steam Workshop) — in-game переводчик полей в настройках мода, исходников нет (проверке не поддаётся, в таблицу не включён).

---

## Топ-3 находки, меняющие представление о суперсете

1. **Remis — уже платформа-суперсет, а не «ещё один переводчик».** Мультиигровые адаптеры (RimWorld в preview) с LoadFolders-ветвлением по версиям, overlay-коллизиями и language overrides; MT-роутер на 7+ провайдеров + локальные LLM; переиспользование переводов с диска; чекпойнты и publication identity; глоссарий с «health review»; встроенные агенты (Codex/Copilot) и model arena. RimLoc выигрывает нишей Rust/кроссплатформенность/PO-XLIFF и патчами, но по проектной модели (проект, чекпойнты, глоссарий, agents) Remis — прямой ориентир и главный риск диффузации. AGPL-3.0 защищает от прямого заимствования кода, но не идей.
2. **Экстракция текста из PatchOperations через xpath — редчайшая способность, и она есть у NicoriciN89.** Remis честно диагностирует «offline extraction cannot resolve PatchOperations», а N89 парсит типовые xpath (Add/Replace/Insert, li[N], мульти-defName) и порождает DefInjected-ключи — ровно тот класс текста, который «молча терялся» у всех остальных. Для суперсета RimLoc это подтверждение: патч-экстракция — дифференциатор, который стоит докрутить и прогнать по одному корпусу с N89 (SAME_CORPUS_DIFFERENTIAL, level 6).
3. **Рынок раскололся на два лагеря — и обе крайности уязвимы с фланга RimLoc.** «Оффлайн NMT/LLM» (Argos+Ollama у N89, LibreTranslate у MTT, локальные модели у Remis, кейс AutonomoAI: 125k слов за $8.86) и «human-in-the-loop таблицы» (Translation Forge CSV, laskinss27 Google Sheets, winterheart PO). Ни у кого из 14 нет связки «PO/XLIFF для CAT-профессионалов + LLM-черновик + строгая валидация + сборка мода в одном CLI/GUI» — XLIFF есть только у RimLoc (rimloc-export-xliff), PO — только у RimLoc и winterheart (мёртв). Плюс AutonomoAI доказал marketability «полная локализация с нуля за копейки» — это готовый маркетинговый нарратив, который стоит перехватить с честными цифрами.

## Методологические оговорки

- Ничего не запускалось: все фичи подтверждены чтением кода (level 2) либо README (level 1). Наличие юнит-тестов у конкурентов (N89 — 12 файлов, RWAT — 46, laskinss27 — 9, Translation Forge — 15+) отмечено, но тесты не исполнялись — уровень 3+ никому не присвоен.
- Метаданные (даты/лицензии) — из GitHub API на 2026-10-05; «живой» = push в последние ~3 месяца и не archived.
- Исходники, скачанные в ходе инспекции, лежат рядом: `/tmp/rimloc-competitive/src_*.py|js|ts|cs|ps1`.
