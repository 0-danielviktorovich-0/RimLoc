# Tier B — мульти-игровые/future-адаптеры: архитектурные требования к нашему LocalizationAdapter

**Дата:** 2026-10-05 · **Лейн:** tierB-adapters · **Слой:** research/roadmap (НЕ бета-блокеры)
**Рабочая папка:** `/tmp/w3-tierb/`

## Метод и рамки

- RimLoc-эталон: `/Volumes/Portable-SSD/caches/targets/rimloc-tgbench/release/rimloc-cli` — существует и исполняется (проверено `--version`, 2026-10-05). В задании путь назван `.../release/rimloc`; фактическое имя бинаря — `rimloc-cli`. Диффы в этом лейне не требовались — бинарь не использовался. Наш worktree архитектуры: `_rimloc-worktrees/ba-main`, HEAD `72259e0b98c78ca0f34057d86a5ad32f7deb24dd` (зафиксирован `git rev-parse HEAD`).
- Remis, paradox-localize, paradox-translation-toolkit — клонированы shallow в `/tmp/w3-tierb/` (только чтение), изучены исходники; Remis-тесты запускались (см. ниже).
- Корпус RimWorld-модов не трогался; ничего в `/tmp/rimloc-diff` и в репозитории не писалось.

---

## 1. Remis (Drlinglong/Remis) — главный ориентир

### Identity и статус

| Поле | Значение |
|---|---|
| Репозиторий | github.com/Drlinglong/Remis, клон HEAD `5440918` от **2026-10-05** (v3.2.2) — активен ежедневно |
| Лицензия | AGPL-3.0 (`LICENSE`) |
| Стек | Tauri 2 + React 19 + FastAPI/Python + PydanticAI (README:40-44) |
| Платформа | Windows desktop (бейдж Platform-Windows); игры: Paradox×6 + Surviving Mars — **Stable**, RimWorld + Project Zomboid — **Preview**: «generated packages have not been validated in-game» (README, раздел «Game support in 3.2.1») |
| MT-роутер | 12+ провайдеров-хендлеров: anthropic, deepseek, gemini, grok, local, modelscope, nvidia, openai, openrouter, qwen, siliconflow, copilot (`scripts/core/*_handler.py`); + Ollama/LM Studio как OpenAI-совместимые |
| Прочее | checkpoint_manager.py (500 строк), glossary_health_service.py (188), агенты `fix_agent.py`/`translation_fixer_agent.py`, перевод-кэш |
| **Статус прогона** | **SOURCE_CONFIRMED_ONLY** — исходники прочитаны; собственные юнит-тесты адаптеров исполнены на macOS (ниже); GUI-прогон не выполнялся (Windows-only) |

### Проверка запуском ( executed, не выдумка )

```
cd /tmp/w3-tierb/Remis && python3 -m pytest tests/core/game_adapters/test_project_zomboid.py \
  tests/core/game_adapters/test_rimworld_external_fixture.py -q
→ 15 passed, 1 skipped (0.06s)   # чистые адаптерные тесты, macOS, минимальный venv

… test_package_transaction.py test_shared_workflow.py test_incremental_review.py -q
→ 17 passed, 6 failed — все 6 падений: ModuleNotFoundError: No module named 'openai'
  (интеграционные тесты импортируют провайдерский стек; в минимальном venv его нет)
```

Вывод сам по себе архитектурный: **чистые адаптеры Remis запускаются headless и платформо-независимо**, а глобальный `tests/conftest.py:3` тянет `scripts.shared.task_state` → `ws_manager` → fastapi, т.е. конftest сцеплен с апп-шеллом — тесты адаптеров нельзя выполнить без web-слоя. У нас шов должен быть чище.

### Архитектура адаптеров — швы и контракты

**Минимальный интерфейс — структурный Protocol, 7 методов** (`scripts/core/game_adapters/contracts.py:63-83`):

```python
class GameAdapter(Protocol):
    id: str; game_id: str
    def discover(root, source_lang, game_version=None) -> Discovery   # Resource+Diagnostic
    def parse(path, metadata) -> Document                             # Document = path+source_text+entries+metadata
    def parse_text(text, path, metadata) -> Document
    def render(document, translations, target_lang) -> dict[path, str]
    def package_metadata(root, target_lang, game_version) -> dict     # напр. About.xml / mod.info
    def validate(source, target) -> list[Diagnostic]
    def language_folder(language) -> str                              # locale → игровая папка
```

Данные-контракты: `Entry(key, value, line_number, metadata, status)` (contracts.py:13-27), `Document(path, source_text, entries, metadata)` (29-34), `Diagnostic(code, message, path, severity)` (44-53), `Discovery(resources, diagnostics, metadata)` (56-60).

**Главный принцип** (docstring contracts.py:1-5): *«Game versions describe discovery context, never the identity of a translation. Adapters only read and render. The workflow owns approval, persistence and writes»*. Адаптер — stateless пара «прочитай/отрендери»; всё состояние (approval, персистентность, запись) — у workflow.

**Реестр и capabilities** (`registry.py`):
- явная фабрика `ADAPTER_FACTORIES = {"project_zomboid": …, "rimworld": …}` (registry.py:6-9), fallback на `LegacyAdapter` для незарегистрированных игр (23-28);
- `adapter_for_path()` — распознавание адаптера по форме пути (`Languages/{Keyed,DefInjected,Strings}`, `Defs/*.xml` → rimworld; `…/Translate/*.txt|json` → PZ) (31-43);
- `game_capabilities(game_id) -> dict` — машиночитаемая проекция возможностей: `resource_adapter` (structured | surviving_mars_csv | paradox), `independent_translation_mod`, `paradox_deployment`, `source_cleanup`, `runtime_verified`, `coverage_kind` (46-56). Это рабочий прецедент нашего §F3/F4 capability discovery.

**RimWorld-адаптер** (`game_adapters/rimworld.py`, 439 строк) — ровно наш домен:
- **versioned format-rules**: `FORMAT_RULES_VERSION = "rimworld-1.6-v1"` (rimworld.py:13) + белый список переводимых def-полей 1.6 (`_V16_DEF_FIELDS`, 14-23, селектируется в `_def_fields_for`, 250-251);
- честная версионная охрана: правила верифицированы только для `^1.6`; иначе диагностический код `game_version_rules_unverified`, «runtime behavior is unverified» (46-47);
- **LoadFolders.xml**: ветки `v1.x` + условия `IfModActive/IfModNotActive/IfModActiveAll`; без списка активных модов условная ветка = error-диагностика `loadfolders_condition_unknown`, а не молчаливый выбор (275-319); версионные папки `1.x` — выбор «максимум ≤ запрошенной версии», при неизвестной версии — **candidate set + диагностика** (327-339) — точный аналог нашего Exact/Potential view;
- **честные границы офлайн-извлечения**: `Patches/` → `patch_runtime_unknown` («PatchOperations can alter translatable Defs; offline extraction cannot resolve their runtime results»), `Assemblies/*.dll` → `assembly_strings_unavailable` (238-247). Remis PatchOperations НЕ вычисляет — у нас это вычисляемое подмножество (`PatchStage::Applied/Partial`), наша сильная сторона;
- **override-семантика**: ключи существующего DefInjected исключают Defs-фолбэк тех же полей, счётчик `source_override_count` (184-207) — аналог нашего Overridden-контекста;
- overlay-коллизии: «later layer takes precedence» + диагностика `overlay_collision` (227-235);
- рендер **отказывается** работать с document'ом, у которого есть error-диагностика (`_reject_document_errors`, 379-382);
- генерация самостоятельного translation-mod: `About.xml` c `packageId.<lang>`, `modDependencies`, `loadAfter` (103-126);
- XXE-гигиена: DOCTYPE/ENTITY в About.xml и LoadFolders.xml → отказ (287, 352-354).

**Project Zomboid-адаптер** (`project_zomboid.py`, 500 строк) — второй proof-of-concept того же Protocol:
- «treats translation resources as data, **never executes Lua**» (1-5): собственный мини-лексер таблиц вместо Lua-интерпретатора (421-467);
- **source-preserving render**: файл не регенерируется — точечные сплайсы по захваченным спанам `value_start/value_end` в исходном тексте (247-273);
- версионные ветки `common/` + `42.x/` — «максимум применимой ≤ runtime», при неизвестной версии — newest + диагностический код `version_selection_inferred` (101-133);
- защита файловой системы: symlink/junction/reparse-point обрезаются везде (`_is_link`, 56-67; `_walk_data_files` с containment-проверкой, 180-203);
- placeholder-валидация: регулярка защищённых токенов + `placeholder_mismatch` severity=error (322-327, 496-500);
- alias-таблица locale→игровая папка (`en→EN`, `ru→RU`, `zh_cn→CH`, `pt_br→PTBR`…) как данные адаптера (21-46).

**Workflow-мост** (`workflow_bridge.py`) — контракты записи, которых у нас в таком виде нет:
- `write_document` (104-142): по-элементная валидация → render → `_verify_rendered` → конфликт-проверки манифеста → атомарная запись;
- **`_verify_rendered` (145-155) — round-trip верификация рендера**: выходной текст re-parse'ится тем же адаптером и множество «ключ→значение» обязано совпасть с ожидаемым, иначе «Rendered resource does not preserve every translation key and value»;
- **манифест выходного пакета** `.remis-localization-manifest.json`, schema_version: 1 (158-179): game_id, source_mod_id, на файл — `source_path`, `language`, `source_hash`, entries с `needs_review`. Повторная запись проверяет принадлежность: «Output package belongs to another game» / «another source Mod» / «Multiple source resources would overwrite» (127-138) — защита от контаминации пакетов между играми/модами;
- `safe_output` (77-85) — запрет выхода записи за корень пакета (absolute/`..`/`:`);
- `_write_package` (182-203) — атомарная запись всех файлов пакета с полным rollback при отказе;
- переиспользование (`translation_reuse.py`): существующие переводы ищутся по путям, которые **предсказывает сам адаптер** через свой render; конфликт (один ключ — разные значения) = ValueError «Conflicting existing translations require review», а не молчаливый выбор (39-43);
- review-state (`review_state.py`): `needs_review` живёт в манифесте и валиден только пока `sources[key] == entry["source"]` — устаревшие флаги не применяются (23-33);
- read-only capability-инспекция (`project_support.py:9-31`): dry-run discovery c диагностикой до любых записей; стабильные file_id через uuid5 от нормализованного пути.

---

## 2. xTranslator (SSTE) — Bethesda, закрытый код: только DOC

### Identity и статус

| Поле | Значение |
|---|---|
| Источник | официальная страница Nexus Mods — nexusmods.com/skyrimspecialedition/mods/134 (загружена WebFetch 2026-10-05) |
| Автор | McGuffin; закрытый код, freeware; разные бранды одного тула: tesvTranslator / sseTranslator / fallout4Translator / falloutNVTranslator |
| Версия | v1.4.5; ~822k просмотров страницы |
| Игры | Skyrim (LE/SE/AE), Fallout 4/NV/76, Starfield — переключение workspace на лету |
| **Статус прогона** | **DOC_ONLY** — закрытый код, прогон невозможен и не проводился |

### Что известно и что перенимать по UX

- **Режимы работы**: Esp (прямой перевод плагина), Strings (deprecated), **Hybrid** — esp как records/fields-layout + строки в .STRINGS файлах (совмещает структуру плагина с внешними строками), MCM/Translate, PapyrusPex со встроенным декомпилятором, где internal-переменные залочены как non-editable.
- **Словарная система**: нативный **SST dictionary** с тегами, collabID-комментариями и обратной совместимостью; XML-импорт/экспорт как обменный формат; community shared dictionaries; **inline dictionary**, строящийся на лету из уже существующих пар строк.
- **espCompare**: построение пар «источник→перевод» напрямую между двумя esp на разных языках — майнинг пар из готовых переводов для стартового словаря.
- **Diff-viewer** оригинал/обновление строк — workflow обновления мода; эвристические подсказки при правке похожих строк с приоритетом словарей (drag-sort).
- **Происхождение черновиков**: строки, автопереведённые от derived/old-data, получают WARNING и **не сохраняются в словари** — явная дисциплина происхождения; у нас это роль `Origin` (Human/Tm/Llm/Imported) в каноне.
- **Валидация**: Alias Tool Check (целостность `<alias>`-плейсхолдеров между исходником и переводом), цветовые статусы (validated/translated/partial/warning), RegEx поиск/замена, spellcheck (Word/Hunspell).
- **MT**: Yandex/MsTranslator/Google/DeepL/Youdao с rate-limiter'ом.
- Кодировки: все известные codepage + per-string fallback encoding, CJK-безопасное редактирование; VMAD-строки — строгий режим, автоперевод выключен по умолчанию («editing a wrong VMAD can break your mod»).
- **Явный non-goal самого вендора**: «not a tool to localize/delocalize esp/esm — use xEdit» — честная граница скоупа в описании продукта.

---

## 3. Paradox-экосистема — будущие адаптеры: чем отличаются требования

### 3.1. paradox-localize (lintax) — Go, MIT

- Клон `/tmp/w3-tierb/paradox-localize` (обновлён 2026-09-05). **Статус: SOURCE_CONFIRMED_ONLY** (исходники+README; прогон не выполнялся).
- Целевой юзкейс тот же, что у RimLoc для RimWorld: «raw localization keys instead of translated text» при English-only модах; пишет sidecar `.yml` рядом с модом.
- **Ключевая находка — precedence варьируется МЕЖДУ ИГРАМИ одного семейства** (README «How it works», п.4): разные Paradox-игры грузят локали по разным правилам — «FIOS = first-in-order-sorted, others use reverse-alpha». Хак тулзы: пишет ДВА файла — `0_<modid>_consolidated_l_<lang>.yml` (сортируется первым) и `zz_…` (последним), захардкоженные hardlink'ом. Для нас: **модель приоритетов — часть семантики адаптера**, не ядра; у нас это уже так (`AdapterIdentity` селектирует семантику, canonical.rs:266-275).
- **Placeholder-защита**: `$VARIABLE$`, `[GetModifier(...)]`, `@icon!`, `#bold …#!`, `\n` → нейтральные `{0}{1}{2}`, проверка выживания токенов в ответе, при потере — оставить английский («wrong, but safe»).
- MT: локальный Ollama по умолчанию (free/private), облако — BYOK OpenAI-compatible. Кэш на тройку (game, source-lang, target-lang).

### 3.2. Paradox Translation Toolkit (khoeos) — GPL-3.0, TS/Turbo monorepo, 31★

- Клон `/tmp/w3-tierb/paradox-translation-toolkit` (обновлён 2026-10-02, CI). **Статус: SOURCE_CONFIRMED_ONLY**.
- Декомпозиция monorepo: `packages/games` (знание об играх) отделён от `parser`, `translate`, `converter`, `report` — адаптерный шов на уровне пакетов.
- **Декларативный `GameDefinition`** — самый чистый в выборке формат профиля игры (`packages/shared/src/index.ts:56-66`):

```ts
interface GameDefinition {
  id: string; displayName: string; steamAppId?: number
  localisationDirName: 'localisation' | 'localization'   // игры различаются написанием!
  layout: 'flat' | 'nested-by-language' | 'both'
  languageFileToken: Partial<Record<LanguageCode, string>> // pt-BR→braz_por, zh-Hans→simp_chinese
  overrideSubdirs: string[]                                 // ['replace']
  userFolder: string
  domain: string                                            // текстовое описание домена — контекст для LLM-промптов
}
```

- Режимы вывода (`CONVERT_MODES`, index.ts:68-72): `add-to-current` | `extract-to-folder` | `create-translation-mod`. Режимы содержимого (`TARGET_CONTENTS`, 78): `missing-keys` | `complete-file` | `regenerate-file`, с `.bak` перед перезаписью.
- Коллекционное сканирование «key by key», чтобы «отдельный локализационный мод, уже покрывающий ключ, не перезаписывался» (README) — приоритет на уровне коллекции модов.
- Таблица игр с честной зрелостью: Stellaris/CK3 «✅ Tested», EU4/EU5/HoI4/Vic3 «⚠️ lightly tested», Imperator «not tested».

### Отличия Paradox-домена от RimWorld (для будущих адаптеров)

| Измерение | RimWorld | Paradox |
|---|---|---|
| Формат | XML (DefInjected/Keyed/Strings) + PatchOperations | YAML-подобный `l_<lang>:` (`*_l_english.yml`), ключ: N строка, `#`-разметка |
| Порядок загрузки | LoadFolders.xml + version dirs, load order модов, «позже перекрывает» | **разный по игре**: FIOS vs reverse-alpha |
| Расхождение имени папки | `Languages/<Lang>` | `localisation` vs `localization`, `replace/`-подпапки |
| Локальные токены | языковая папка = имя | `braz_por`, `simp_chinese` — собственный словарь токенов |
| Переводимые поля | доступны как XML-пути (def-поля) | весь value целиком; токены/разметка внутри строки |
| DLL-риск | Assemblies/*.dll | нет, но скриптовые события |

---

## 4. Прочие замеченные активные мульти-игровые тулзы

- **Translator++ (dreamsavior/TranslatorPlusPlus)** — GPLv3, pushed 2026-10-02, «CAT dedicated to multiple game engines» (RPG Maker, Ren'Py, …),NetCollab — реальные мультипользовательские сессии локализации с локальной машины. **Статус: DOC_ONLY** (прочитан README через `gh api`; адаптерный код не изучался). Урок: коллаборативный слой как опция поверх десктоп-ядра.
- **UnityL10nTool (dmc31a42)** — 75★, обновлён 2026-06-05, генератор локализационных патчей для Unity-игр. Замечен в поиске (`gh search repos`); глубоко не изучался — **NOT_MATERIALLY_RELEVANT** для нашего адаптерного дизайна (Unity-рантайм патчит уже собранные ресурсы, не исходники модов).

---

## 5. СИНТЕЗ — требования к нашему LocalizationAdapter

### 5.1. Что УЖЕ покрыто нашей архитектурой (подтверждено чтением кода)

| Требование | Наше покрытие | Где |
|---|---|---|
| Идентичность адаптера, «чужой адаптер = чистый отказ» | `AdapterIdentity{adapter_id, adapter_api_version, adapter_project_schema_version}`, `adapter_ids::RIMWORLD/RIMLOC_APPLICATION`, legacy-дефолт, `ProjectLoadDiagnostic::UnknownAdapter` | `rimloc-domain/src/canonical.rs:266-294`; `docs/architecture/LOCALIZATION_ADAPTERS.md` (§F9) |
| Locale-независимая идентичность записи | `SourceEntryId{kind,key,def_type}` без локали | canonical.rs:17-57 |
| Семейства механизмов вместо косметических категорий | `EntryKind` (Keyed/DefInjected/TKey/Strings/Backstories/PatchDerived) | canonical.rs:59-75 |
| Происхождение и версия как данные | `SourceProvenance{version_selected, conditional_branch, patch_stage, selected_by}` + словарь winner_reason | canonical.rs:97-119 |
| Честный view: exact vs potential | `InventoryContext.view: ViewLabel{Exact,Potential}` | canonical.rs:236-257 |
| Один write-path для любого адаптера | `Project::update_translation`, TM/глоссарий как generic core state | canonical.rs:298-386 |
| Происхождение черновиков (урок xTranslator) | `Origin{Unknown,Human,Tm,Llm,Imported}` | canonical.rs:226-234 |
| Патч-подмножество вычисляем (Remis не умеет) | `PatchStage::Applied/Partial` | canonical.rs:121-130 |
| Runtime-приёмка опциональна, через адаптер | §F13 RimWorld Runtime Bridge / WDIO | LOCALIZATION_ADAPTERS.md |
| Запрет in-process сторонних нативных адаптеров | процесс-хост + RPC / WASM / FS-grants | LOCALIZATION_ADAPTERS.md (§F10/F11) |
| Форматный шов уже существует (узкий) | `ScanPlugin{name, matches, scan→Vec<TransUnit>}` + FFI `rimloc_plugin_scan_json` | `rimloc-plugin-api/src/lib.rs:5-19` |

### 5.2. Требования к добавлению в roadmap

| # | Требование | Обоснование (источник) | Классификация |
|---|---|---|---|
| R1 | **Поднять шов адаптера до полного контракта**: сегодня публичный trait покрывает только scan; нужен типизированный trait-контракт «discover/parse → Document; render/validate/language-metadata → вывод» по образцу Remis `GameAdapter` Protocol — ядро без RimWorld-типов, адаптер stateless read/render, запись только у ядра | Remis contracts.py:63-83 + docstring 1-5; наш plugin-api покрывает 1 из 3 стадий | **ROADMAP** (продление `rimloc-plugin-api`, не ломая §F10/F11: сначала нативные in-tree адаптеры) |
| R2 | **Conformance-инвариант адаптера: render round-trip** — выход любого адаптера обязан re-parse'иться тем же адаптером в то же множество «ключ→значение»; error-диагностика документа запрещает рендер/запись | Remis `_verify_rendered` (workflow_bridge.py:145-155) и `_reject_document_errors` (rimworld.py:379-382) | **ROADMAP** (тест-гейт, обязательный для каждого будущего адаптера) |
| R3 | **Манифест выходного пакета** с schema_version, game_id, source_mod_id, source_hash и needs_review на запись + отказ при контаминации (другая игра/мод/двойной источник) — устойчивое re-ассоциирование выходов с источниками между запусками и для proofreading | Remis workflow_bridge.py:158-179, 127-138; review_state.py:23-33 | **ROADMAP** (sidecar-файл или tool-side реестр; в игровой папке — опционально и выключаемо, чтобы не пачкать моды) |
| R4 | **Versioned format-rules catalog**: декларативные, версионируемые наборы переводимых полей/правил формата per game-version (`rimworld-1.6-v1` стиль) + честный диагностический код «правила для версии X не верифицированы» | Remis rimworld.py:13-23, 46-47 | **ROADMAP** (сначала как данные в адаптере RimWorld; ядро знает только о наличии rules_version в provenance) |
| R5 | **Машиночитаемый capability-манифест адаптера** (structured flags: independent_translation_mod, runtime_verified, coverage_kind…) вместо вывода возможностей из формы пути; прецеденты: Remis `game_capabilities()` (registry.py:46-56), наш первый живой образец `testlab/runtime_bridge/capability-manifest.json` | наш §F3/F4 называет это «направлением»; Remis показывает рабочую форму | **ROADMAP** (версионируемый JSON рядом с AdapterIdentity) |
| R6 | **Модификационный профиль игры как данные**: layout ('flat'/'nested-by-language'), имя папки локалей, alias-таблица locale→игровой токен, override-подпапки, домен-описание для LLM-контекста | PTT `GameDefinition` (shared/src/index.ts:56-66); Remis `language_folder` alias-таблицы (project_zomboid.py:21-46) | **ROADMAP** (формат профиля буд. адаптеров; для RimWorld уже зашито в services — переносить в данные при появлении второго адаптера) |
| R7 | **Модель приоритетов/перезаписи — часть семантики адаптера**, включая вариантность между играми семейства (FIOS vs reverse-alpha) и режимы вывода missing-keys/complete-file/regenerate + защита «чужой уже покрытый ключ не перезаписывается» | paradox-localize README п.4 (двойной префикс-хак); PTT TARGET_CONTENTS; Remis `overlay_collision` | **Покрыто архитектурно** (AdapterIdentity селектирует семантику, canonical.rs:266-275; winner_reason) + **ROADMAP** (режимы вывода как политики экспорта, если понадобятся) |
| R8 | **Конфликт переиспользования → review, не молчаливый выбор**; переиспользование предсказывается через сам адаптер (его render предсказывает пути выходов) | Remis translation_reuse.py:39-43 | **Покрыто** (`Lifecycle::Ambiguous`, TM в ядре) — при многoadаптерности оставить правило: неоднозначность = Ambiguous, никогда автопик |
| R9 | **Безопасность адаптера как контракт**: не исполнять игровой код (Lua/JS), обрезать symlinks/junctions, отказ DOCTYPE/ENTITY, containment выходных путей | Remis project_zomboid.py:1-5, 56-67, rimworld.py:287; workflow_bridge.py:77-85 | **Частично покрыто**: Rust-стек (quick-xml/roxmltree, память-безопасные парсеры без внешних сущностей — grep `doctype\|ENTITY` по rimloc-parsers-xml не нашёл обработчиков, т.е. структурно безопасно, но явного диагностического кода на DTD в исходнике нет) → **ROADMAP** (мелочь: явный код диагностики; containment — не требуется: мы не пишем в мод-папки) |
| R10 | **Зрелость адаптера как продукт-данные**: метки Stable/Preview с формулировкой «generated packages have not been validated in-game» + runtime-приёмка как основание метки | Remis README «Game support in 3.2.1»; PTT таблица tested/lightly tested; наш §F13 | **INTENTIONAL_NON_GOAL** как новая механика: §F13 уже делает это мостами; метку формулировать в UI при появлении второго адаптера |
| R11 | **CAT-UX поверх ядра**: inline-словарь из существующих пар, cross-language pair mining, diff-view обновления источника, эвристические подсказки, пер-строковый encoding-fallback, batch MT rate-limiter, «finalize»-экспорт в фиксированную папку | xTranslator (Nexus, DOC_ONLY); пары-майнинг — уже наш TM | **INTENTIONAL_NON_GOAL в Tier B** (это product-слой редактора, не адаптерная граница; закрепить в roadmap продукта, не адаптера) |
| R12 | **Коллаборативные сессии (NetCollab-стиль)** | Translator++ README | **INTENTIONAL_NON_GOAL** (вне границы адаптера и вне текущего скоупа) |

### 5.3. Минимальный интерфейс адаптера (проект для R1, по мотивам выборки)

Ядро знает: EntryIdentity (у нас — `SourceEntryId`), Entry (текст+позиция+метаданные), Document (файл+исходный текст+записи), Diagnostic (code/message/path/severity), Discovery (ресурсы+диагностика+метаданные). Адаптер: `discover`, `parse/parse_text`, `render`, `package_metadata`, `validate`, `language_metadata`. Отличие от Remis, которое стоит перенять у нас самих: identity-объект (`SourceEntryId` + provenance + view) у Remis размазан по `metadata: dict` — **неструктурированный словарь вместо типизированных полей**; наш канон уже строже (`Adapter-specific метаданные: типизированные, не bag of random JSON` — LOCALIZATION_ADAPTERS.md). Future-адаптеры обязаны расширять канон своими типизированными структурами за своим identity — это требование зафиксировать в контракте R1 как прямое ограничение.

### 5.4. Топ-3 (кратко, для LaneResult)

1. **Stateless read/render контракт адаптера при ядре-владельце состояния и записи** — расширить `rimloc-plugin-api` от scan-only до полного контракта (R1), с типизированными (не dict) adapter-метаданными.
2. **Render round-trip + отказ при error-диагностике как conformance-гейт каждого адаптера** (R2) — верифицируемость рендера важнее богатства рендера.
3. **Происхождение/зрелость как данные сквозь весь конвейер**: versioned format-rules (R4) + манифест выходного пакета с source_hash/needs_review и отказом от контаминации (R3) + машиночитаемый capability-манифест (R5) — у нас ядро это уже умеет для проекта, не хватает внешних манифестов.

---

## 6. Честные ограничения

- **Remis GUI не запускался** (Windows-only сборка); статус честно SOURCE_CONFIRMED_ONLY, а не PRACTICALLY_RUN. Прогон собственных юнит-тестов адаптеров Remis: 15 passed/1 skipped; интеграционные 17 passed/6 failed — все 6 из-за отсутствия `openai` SDK в минимальном venv (инфраструктура окружения, не дефект продукта).
- **xTranslator** — закрытый код: только официальная страница Nexus (WebFetch); бинарных проверок нет. Веб-поиск в этот заход был rate-limited (429), факты взяты из загруженной первоисточника-страницы.
- **Paradox-тулзы** — исходники прочитаны, прогоны не выполнялись (Go/Node-инструменты вне рамок этого лейна).
- **Translator++/UnityL10nTool** — README/описание уровня; адаптерный код не изучался.
- Эталонный `rimloc-cli` (HEAD da2fc77 по условию задачи) существует и исполняется, но SHA бинаря из самого бинаря не верифицировался — диффы в лейне не требовались.
- Корпус RimWorld-модов в этом лейне не использовался (research-слой).
