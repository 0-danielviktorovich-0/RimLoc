# Pre-beta security audit — RimLoc

- **Дата:** 2026-10-05
- **HEAD:** `11eafaa1027822913f55dbfefcb3a9ed88beafa6` (ветка `feature/ui-r1-convergence`, коммит `chore: влить codex/pass-b-tails`)
- **Режим аудита:** read-only. Ничего не коммитилось, не пушилось, зависимости не обновлялись.
- **Метод:** фиксируются команды и их фактический вывод; каждое наблюдение классифицировано по severity.

## Сводка находок

| Severity | Кол-во | Что |
|---|---|---|
| Critical | 0 | — |
| High | 0 | — |
| **Medium** | **1** | F-1: output-пути legacy-команд без containment |
| **Low** | **5** | F-2…F-5, F-8, F-10 (см. таблицу ниже) |
| Info | 4 | F-6, F-7, F-9, F-11 |
| Реальных секретов | **0** | все хиты secret-scan — тест-фикстуры и мок-данные |

| ID | Severity | Кратко |
|---|---|---|
| F-1 | Medium | `scan_mod`/`scan_strings_gui`/`validate_mod`/`xml_health`/`diff_xml_cmd` пишут `out_json`/`out_csv`/`out_dir` через `make_absolute()` без проверки формы/containment — `..` и абсолютные пути принимаются |
| F-2 | Low | `withGlobalTauri: true` — глобальный IPC-мост в окне |
| F-3 | Low | `capabilities/default.json`: `windows: ["main", "*"]` — wildcard вместо явного окна |
| F-4 | Low | Yanked-крейт `yoke-derive 0.8.3` в Cargo.lock — `cargo deny check advisories` FAILED |
| F-5 | Low | Устаревший вложенный `gui/tauri-app/src-tauri/Cargo.lock` (tauri 2.8.5, dialog 2.4.0, shell 2.3.1) — Dependabot обновляет мёртвый lockfile |
| F-6 | Info | `permissions/allow-commands.json` — «мёртвое» разрешение: ссылается на несуществующие команды и не подключён ни одной capability |
| F-7 | Info | `devtools: true` в конфиге окна — в release игнорируется (cargo-фича `devtools` не включена), но намерение стоит зафиксировать комментарием |
| F-8 | Low | frontend-v2: 22 high-уязвимости в dev/test-тулинге (WebDriverIO-цепочка, mocha, extract-zip, serialize-javascript); prod-зависимости чистые |
| F-9 | Info | 9 игноров RUSTSEC в `deny.toml` — все с обоснованием, но нужен периодический пересмотр |
| F-10 | Low | Вендорные патчи `wry`/`tao` (`[patch.crates-io]`, noactivate) — секьюрити-фиксы апстрима не подтягиваются автоматически |
| F-11 | Info | Мок-данные с синтетическими «токенами» (`frontend-v2/src/lib/mock/diagnostics.ts`) — не секреты, но при будущих сканах будут шуметь |

---

## 1. cargo deny check

**Команда:** `cargo deny check` (cargo-deny, конфиг `deny.toml` в корне).

**Вывод (итоговая строка):**

```
advisories FAILED, bans ok, licenses ok, sources ok
```

| Секция | Результат | Детали |
|---|---|---|
| advisories | **FAILED** | Единственная ошибка: `error[yanked]: detected yanked crate (try 'cargo update -p yoke-derive')` — `yoke-derive 0.8.3` отозван с crates.io (транзитивно через `yoke → icu_* → idna/url/reqwest/tauri`). Ни одного открытого RUSTSEC-advisory по нашим зависимостям. **F-4.** |
| bans | ok | `multiple-versions = "warn"`, `wildcards = "warn"` — предупреждения (дубли `sha2` 0.10.9+0.11.0, `ico` 0.3.0+0.5.0, `toml` 0.5/0.8/0.9/1.1.6 и др.) не блокируют. |
| licenses | ok | Allow-лист из 16 лицензий (GPL-3.0, MIT, Apache-2.0, BSD, ISC, Unicode-3.0, MPL-2.0, OpenSSL, …) + clarify для `ring`. Нарушений нет. |
| sources | ok | `unknown-registry = "deny"`, `unknown-git = "deny"` — посторонних источников нет. |

**Игноры advisories (deny.toml, все с обоснованием в комментариях) — F-9:**

| RUSTSEC | Крейт | Обоснование |
|---|---|---|
| RUSTSEC-2026-0002, RUSTSEC-2026-0253 | lru | Зпинен `^0.12` tauri 2.x; soundness-фиксы в 0.16+/0.18+; недостижимо из кода RimLoc |
| RUSTSEC-2024-0436 | paste | Unmaintained, транзитив UI/XML-стека; фиксированной версии нет |
| RUSTSEC-2024-0370 | proc-macro-error | Unmaintained, транзитив макро-депов tauri |
| RUSTSEC-2025-0075/0080/0081/0098/0100 | unic-* | Семейство unmaintained (via tauri/wry текст); форк апстримом не принят |

Рекомендация: пересматривать игноры раз в квартал; `yoke-derive` лечится `cargo update -p yoke-derive` (не делалось — режим read-only).

---

## 2. npm audit

**Команды:** `npm audit --json` в `gui/tauri-app/frontend-react` и `gui/tauri-app/frontend-v2`. Оба `package-lock.json` на месте.

### frontend-react

```json
"vulnerabilities": { "info": 0, "low": 0, "moderate": 0, "high": 0, "critical": 0, "total": 0 }
```

175 зависимостей (prod 28, dev 148). **Чисто.**

### frontend-v2

**Итог: 22 high, 0 critical.** Все 22 — в dev/test-тулинге; поле `dependencies` у пакета пустое (prod-зависимостей нет вообще), значит **в прод-бандл ни один уязвимый пакет не попадает** (frontendDist — результат `vite build`, рантайм-зависимостей не имеет). **F-8.**

| Цепочка | Severity | Суть | Fix |
|---|---|---|---|
| `@wdio/cli`, `@wdio/config`, `@wdio/*`, `webdriverio`, `expect-webdriverio`, `@wdio/tauri-service` | high | несколько advisory через транзитивы | webdriverio 10 (semver-major) |
| `extract-zip` (via `@puppeteer/browsers`) | high | symlink path traversal → произвольная запись файлов при распаковке (GHSA-jmr9-qjv8-65gv, GHSA-7pqw-9j4j-h8q3) | webdriverio 10 |
| `serialize-javascript` (via mocha) | high | RCE через RegExp.flags/Date.toISOString + DoS | @wdio/mocha-framework 10 |
| `braces` → chokidar → mocha | high | stack-exhaustion DoS | @wdio/mocha-framework 10 |
| `deepmerge-ts` | high | stack exhaustion на рекурсивных графах | webdriverio 10 |
| `proxy-agent`/`pac-proxy-agent`/`get-uri`/`basic-ftp` | high | транзитивы proxy-цепочки | webdriverio 10 |

Оценка риска: **не влияет на прод-артефакт**; риск локальный — E2E-прогоны WDIO (extract-zip при скачивании браузера). Не запускать E2E в непроверенных окружениях; апгрейд webdriverio 9→10 — после беты.

---

## 3. Secret scan

**Команды (минуя `target/`, `node_modules/`, `.git/`, `dist/`, `test-results/`):**

```bash
rg -n --hidden -g '!target/**' -g '!node_modules/**' -g '!.git/**' -e 'xox[baprs]-' -e 'AKIA[0-9A-Z]{16}' \
   -e 'gho_[A-Za-z0-9]{36}' -e 'ghp_[A-Za-z0-9]{36}' -e 'sk-[A-Za-z0-9]{20,}' -e 'AIza[0-9A-Za-z_-]{35}' \
   -e 'BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY'
rg -n --hidden -i … 'password\s*[:=]…' 'api[_-]?key…' 'Bearer [A-Za-z0-9_\-\.]{20,}' 'hooks\.slack\.com'
rg -n --hidden … 'ghp_|github_pat_' 'HF_[A-Za-z0-9]{30,}' 'glpat-' 'dop_v1_'
git ls-files | grep -iE '\.env|secret|credential|token' ; find . -name .npmrc …
```

**Результат: ни одного настоящего секрета.** Все хиты разобраны:

| Где | Что | Вердикт |
|---|---|---|
| `crates/rimloc-services/src/contribution.rs` (тесты `scan_secrets`) | `sk-abcdef…`, `AKIAIOSFODNN7EXAMPLE` (канонический пример из доков AWS), `ghp_abcabc…`, `-----BEGIN RSA PRIVATE KEY-----` | Ложная тревога: фикстуры, проверяющие сам детектор секретов |
| `crates/rimloc-services/src/observability.rs` (тесты маскирования) | `xoxb-0123…`, `ghp_0123…`, `Bearer eyJhbGciOiJIUzI1NiJ9` | Ложная тревога: тесты redaction-механизма |
| `gui/tauri-app/frontend-v2/tests/*` | аналогичные фикстуры валидатора вкладов | Ложная тревога |
| `frontend-v2/src/lib/mock/diagnostics.ts` | «Bearer eyJhbGciOi…mock», `ghp_4tX9…` в мок-данных UI | Ложная тревога (синтетика), но помечено F-11: будущие сканеры будут шуметь — стоит заменить на очевидные плейсхолдеры |
| `docs/…/SELFLOC_BRIDGE.md`, `DEBUGGING_OBSERVABILITY.md` | упоминания паттернов `ghp_…`, `sk-…` как документация маскирования | Ложная тревога |

Дополнительно: `.env`-файлов в git нет, `.npmrc` нет, файлов с `secret|credential` в именах (содержательные) нет. GitHub-токенов/облачных ключей в дереве нет.

---

## 4. Tauri-аудит

### 4.1 Конфигурация (`gui/tauri-app/src-tauri/tauri.conf.json`)

| Параметр | Значение | Оценка |
|---|---|---|
| CSP | `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src ipc: http://ipc.localhost` | Хорошо: никаких внешних origin, script-src без `unsafe-*`. `unsafe-inline` для стилей — обычная плата фреймворков, риск низкий. |
| assetProtocol | `enable: false, scope: []` | Хорошо: протокол ассетов выключен. |
| pattern | `brownfield` | ОК. |
| withGlobalTauri | `true` | **F-2 (Low):** `window.__TAURI__` доступен любому скрипту страницы. Фронтенды используют `__TAURI_INTERNALS__.invoke` как основной путь и `__TAURI__` только как fallback (`frontend-v2/src/lib/client/instance.svelte.ts:32`) — то есть глобал фактически нужен только как запасной. Совместно со строгим CSP риск ограничен, но стоит рассмотреть отключение и удаление fallback. |
| devtools | `true` (окно main) | **F-7 (Info):** в release-сборках девтулзы включаются только cargo-фичей `tauri/devtools`, а в `src-tauri/Cargo.toml` стоит `tauri = { version = "2", features = [] }` — значит, в проде девтулзы выключены. Намерение зафиксировать комментарием. |
| frontendDist | `../frontend-v2/dist` | Прод собирается из frontend-v2 (frontend-react — параллельная лейн через `tauri.react.conf.json`). |

### 4.2 Capabilities и плагины

**capabilities/default.json** (единственная capability прод-сборки): окна `["main", "*"]`, permissions: `core:default`, `dialog:default`, `dialog:allow-open`, `dialog:allow-save`.

- **F-3 (Low):** wildcard `*` шире, чем нужно, — любая будущая окно получит те же права. Сузить до `["main"]`.
- fs-плагин **не подключён** (появляется в графе только транзитивно через `tauri-plugin-dialog`) — ни одного fs-permission не выдано.
- shell-плагин **отсутствует** (удалён из `src-tauri/Cargo.toml`; `open_path` работает через крейт `open` — LaunchServices/ShellExecute/xdg-open напрямую, без шелл-интерпретатора).

**Плагины, зарегистрированные в рантайме** (`main()`):
- `tauri-plugin-dialog` — всегда (нативные диалоги open/save);
- `tauri-plugin-window-state` (`=2.0.0`, пин точный) — только в НЕ-automation сессиях;
- `tauri-plugin-wdio-webdriver` + `tauri-plugin-wdio` — ТОЛЬКО при cargo-фиче `automation-bridge` (см. 4.4).

**F-6 (Info):** `src-tauri/permissions/allow-commands.json` объявляет кастомное разрешение на 36 команд, включая несуществующие (`save_text_file`, `collect_diagnostics`), и не подключено ни одной capability — мёртвый конфиг, ввести в заблуждение может только человека.

### 4.3 IPC-поверхность (invoke_handler)

Два регистрационных входа: **продакшн** (`RIMLOC_LEGACY_COMMANDS` не задан) — 30 команд; **legacy opt-in** (`RIMLOC_LEGACY_COMMANDS=1`) — те же 30 + 28 привилегированных. Константа `LIVE_COMMANDS` в `main.rs` связана тестами (`src/tests.rs`) с реальным `generate_handler!`.

#### Продакшн-вход — контракт (17, `contract_adapter.rs` → `rimloc-services::session`)

| Команда | Что делает / что принимает | Пути |
|---|---|---|
| `contract_handshake` | версия UI-контракта + capability-report | нет |
| `project_create` | канонический проект из read-only исходного мода; принимает абсолютный `mod_root.path`, `target_version`; минтит `proj-<id>` | вход-корень мода (только чтение) |
| `project_open` | открыть проект по `project_id` | id, валидируется (см. 5) |
| `project_list` | список управляемых проектов | нет |
| `project_snapshot` | снапшот проекта по `project_id` | id |
| `project_apply_intents` | применить пачку переводов (persist-before-ack); принимает `ApplyIntentsRequest` | нет клиентских путей |
| `project_refresh` | повторное чтение проекта с диска по `project_id` | id |
| `project_cancel_next` | отмена следующего батча интентов по `project_id` | id |
| `project_validate` | валидация открытого проекта (read-only); `project_id`, `session_epoch`, `locale`-фильтр | нет |
| `project_export` | экспорт переводов в **caller-specified** `out_dir`; guard: absolute + не внутри source-tree/managed-root | **out_dir — guarded** |
| `project_build_mod` | полный дроп-ин мод-пакет в caller-`out_dir`; guard-партиция как у `project_export` | **out_dir — guarded** |
| `project_diagnose` | санитизированный support-bundle по последней операции в `out_dir` | **out_dir — guarded** |
| `project_import_existing` | DRY-RUN анализ существующего пака переводов; guard: absolute, directory, не внутри managed-root, кросс-чек папка↔локаль | pack dir — read-only, guarded |
| `project_apply_existing` | применение переиспользуемого набора пака в открытый проект | нет клиентских путей записи |
| `project_glossary` | термины проекта (read-only); `project_id`, `session_epoch` | id |
| `project_glossary_upsert` | создать/обновить термин (persist-before-ack) | нет |
| `project_glossary_delete` | удалить термин; неизвестный термин — типизированный отказ | нет |

#### Продакшн-вход — безопасные legacy-экстры (13)

| Команда | Что делает / что принимает | Пути |
|---|---|---|
| `get_app_info` | версия приложения | нет |
| `build_identity` | commit/profile/features работающего бинарника (read-only) | нет |
| `scan_mod` | скан единиц перевода; `root` + набор опций; пишет `out_json`/`out_csv` | **F-1:** out-пути через `make_absolute(root, …)` — относительные резолвятся к корню скана, `..`/абсолютные принимаются как есть |
| `scan_strings_gui` | инвентарь строк из `Strings/*.txt` под `Languages/`; пишет `out_json` | **F-1** |
| `validate_mod` | валидация мода; `root`, флаги сравнений; пишет `out_json` | **F-1** |
| `validate_po_gui` | валидация PO-файла (read-only); принимает `po_path` | чтение по клиентскому пути (умеренный риск) |
| `xml_health` | health-скан XML; `root`; пишет `out_json` | **F-1** |
| `coverage_gui` | покрытие перевода source↔target (read-only) | только чтение |
| `diff_xml_cmd` | диф source/target деревьев; пишет `out_json`/`out_dir` | **F-1** |
| `get_cli_i18n` | строки CLI-локализации по `lang` | нет |
| `pick_directory` | нативный диалог выбора папки; `initial` — начальная директория диалога | выбор через диалог |
| `selfloc_catalog_dir` | путь к бандл-каталогу UI-локализации (read-only) | нет |
| `selfloc_build_contribution` | офлайн-бандл вклада самоперевода в caller-`out_dir`; guard-партиция services (`§6 gate`) | **out_dir — guarded** |

#### Legacy-привилегированные (28, ТОЛЬКО `RIMLOC_LEGACY_COMMANDS=1`) — суммарно

`learn_defs` (ML-обучение, принимает `ml_url`), `export_po`, `import_po` (запись в дерево мода), `build_mod`, `lang_update_cmd` (скачивание zip из репозитория и обновление Languages), `annotate_cmd`, `init_lang_cmd`, `get_log_info`, `save_text_via_dialog` (запись только через нативный save-диалог — путь никогда не приходит из WebView), `log_message`, `open_path` (открытие произвольного пути системным обработчиком; `open`-крейт, существование проверяется), `set_debug_options` (RUST_BACKTRACE), `get_diagnostics`, `collect_diagnostics_via_dialog` (save-диалог), `simulate_error`, `simulate_panic`, `morph_cmd` (внешний морфологический API, токен через env), `learn_keyed_cmd`, `learn_patches_cmd`, `dump_schemas`, `get_profile`, `apply_translation` (запись в дерево мода; `lang_dir` проверен `lang_dir_form_ok`, но `file` — снова `make_absolute`), `load_tm`, `load_plugin_cmd` (загрузка нативного .dylib/.so/.dll — только по allowlist `~/.rimloc/plugins-allow.json` с canonicalize-сверкой), `list_plugins_cmd`, `export_xliff_gui`, `import_xliff_gui`, `merge_keyed_gui` (эти три — `ensure_caller_path_absolute`).

### 4.4 Automation-мост в прод-артефакте — исключён на уровне компиляции

Проверено по коду (`src-tauri/Cargo.toml`, `main()`, capabilities, `testlab/release-guard.sh`):

1. **Cargo-фича, не env.** `automation-bridge = ["dep:tauri-plugin-wdio-webdriver", "dep:tauri-plugin-wdio"]` — оба плагина `optional`. Default-фичи только `custom-protocol` → прод-бинарник физически не содержит код моста. В `main()`: `#[cfg(not(feature = "automation-bridge"))] let automation = false;` — `RIMLOC_AUTOMATION=1` в проде не открывает ничего («cannot conjure a listener that was never linked»).
2. **Capabilities.** Права моста (`wdio:allow-execute` и др.) живут в `automation-capability.json` → копируются в `capabilities/automation.json` только automation-сборкой (`tauri.automation.conf.json` подключает её); в прод-сборке файл удаляется, причём сборка **упадёт**, если он останется (Tauri резолвит каждый файл capabilities/, а `wdio:*` в проде — unknown permission).
3. **Testlab-драйвер отдельно.** `testlab/runtime_bridge/` — отдельный C#-проект (`RuntimeBridge.csproj`), в артефакт приложения не входит.
4. **Проверка артефакта.** `testlab/release-guard.sh <.app> [--runtime]`: (a) `strings` бинарника на `tauri_plugin_wdio`, `wdio-webdriver`, `TAURI_WEBDRIVER_PORT`, `wdio/eval`, `__wdio_original_core__`; (b) grep ресурсов на guest-JS маркеры (`wdioTauri` и др.); (c) runtime-негатив — `RIMLOC_AUTOMATION=1` с выделенным портом не должен открыть listener.
5. AX-хуки доступности шипят в каждой сборке, но без listener и без IPC (подтверждено guard-скриптом как ожидаемое).

Вывод: **в прод-артефакте моста нет**, механизм тройной (feature + capabilities + release-guard). Условие — собирать прод без `--features automation-bridge` и прогонять guard; в CI это стоит проверить отдельным шагом, если ещё не проверяется.

---

## 5. Filesystem containment (где сессии пишут файлы)

| Что | Где живёт | Чем удержано |
|---|---|---|
| Управляемые проекты (envelope: переводы, глоссарий, метаданные) | `<data_dir>/com.rimloc.gui/managed/proj-<id>.rimloc.json` (macOS: `~/Library/Application Support/…`); корень канонизируется при старте менеджера; `RIMLOC_DATA_DIR` перенаправляет весь universe для automation-инстансов | `managed_path()`: id обязан матчить `^proj-[a-z0-9-]+$` (fail-closed ДО любого доступа к ФС), затем `is_within_allow` (canonical view, несовпадение = отказ; непознаваемый путь = отказ, не «считаем содержащимся») |
| Глоссарий (`project_glossary_*`) | внутрь того же envelope | `persist_glossary`: persist-before-ack, guard `disk_hash` от внешних изменений, revision/epoch-гарды; запись через `write_atomic` (tmp→rename) |
| Переводы (intents, apply_existing) | туда же (envelope) | + typed identity/refusal-гарды |
| Экспорт/билд мода (`project_export`, `project_build_mod`, `project_diagnose`, `selfloc_build_contribution`) | caller-specified `out_dir` | `ensure_out_dir_absolute` (форма до любого I/O); `is_within` deny-проверки: не внутри source-root (fail-closed, если root не установлен), не внутри managed-root; локаль — строгая форма `^[A-Za-z0-9_-]+$` ДО джойна в `Languages/<locale>`; результат перепарсивается до ack |
| Входящий пак переводов (`project_import_existing`/`apply_existing`) | read-only вход | absolute + directory + не внутри managed-root + кросс-чек имени папки с локалью (иначе английский источник молча «переиспользуется» как перевод) |
| CLI-записи в дерево мода (H1) | `Languages/<dir>/…` | `lang_dir_form_ok` + `ensure_lang_write_target` (форма + deny-containment с symlink-алиасами) |

**Итог по `..\..\` в project_id:** невозможны — форма `proj-<a-z0-9->` отбрасывает `/`, `..`, точки, абсолютные префиксы ещё до joins, плюс независимая canonical-containment проверка (defense in depth, закомментирована как P2-7).

**Итог по symlink escape:** проверки на canonical view (`is_within_allow`/`is_within`), непознаваемый путь трактуется как отказ в allow-направлении — корректная ориентация.

**Слабое место — F-1 (Medium):** пять prod-зарегистрированных legacy-экстров (`scan_mod`, `scan_strings_gui`, `validate_mod`, `xml_health`, `diff_xml_cmd`) и legacy `apply_translation` принимают out-пути через `make_absolute(base, candidate)`: относительный путь с `..` уходит за корень скана, абсолютный принимается как есть; `File::create` перезапишет существующий файл. Эксплуатация требует компрометации WebView (записать произвольный JSON/CSV-контент в произвольный путь), строгий CSP и отсутствие внешних origin этот вектор сильно сужают, но дисбаланс очевиден: helper `ensure_caller_path_absolute()` уже существует и применяется в `export_xliff_gui`/`import_xliff_gui`/`merge_keyed_gui` — те же три строки нужны на каждом out-пути прод-входа, либо canonical-containment по образцу contract-слоя.

---

## 6. Прочее из scope

- **Вендорные патчи (F-10, Low):** `[patch.crates-io]` подменяет `wry 0.55.1` и `tao 0.35.3` локальными копиями без `NSApplication::activate()` (P0 zero-focus-stealing). Секьюрити-патчи wry 0.55.x после форка нужно подтягивать вручную; при апгрейде tauri (PR #57) патч может «осиротеть» — см. DEPENDABOT_RECONCILIATION.md.
- **Логи:** `gui.log` в app-data, ротация 5 МБ, секреты маскируются на уровне observability-слоя (regex-маски `ghp_/xox/AKIA/…` — подтверждено тестами). Profile-лог не содержит клиентских секретов.
- **Ключи LLM-провайдеров:** `rimloc-llm` опционально использует `keyring 3` (ОС-хранилище); секреты не хранятся в файлах проекта. Ключ keyring 3→4 — см. reconciliation.

---

## 7. Вердикт: top риски до beta

1. **F-1 (Medium) — неограниченные out-пути в prod-зарегистрированных legacy-экстрах.** Дешёвый фикс существующим helper-ом (`ensure_caller_path_absolute` или canonical-containment) на 6 call-sites. Рекомендую закрыть ДО беты: это единственное место, где IPC-поверхность прод-входа пишет за пределы guard-партиции.
2. **F-4 + F-5 + F-8 — supply-chain гигиена.** Yanked `yoke-derive` (одна команда `cargo update -p yoke-derive`), удаление мёртвого вложенного `src-tauri/Cargo.lock` (заодно гасит Dependabot-PR #36/#47), план апгрейда webdriverio→10 после беты. Ни одно не блокирует бету, но всё дешёвое.
3. **F-10 + PR #57 — вендорный wry-патч vs обновление tauri 2.12.** Прежде чем мержить tauri 2.11.6→2.12.0, убедиться, что resolution держит `wry 0.55.1-noactivate` (иначе P0-фикс активации окна молча теряется), и прогнать `testlab/release-guard.sh --runtime`.

Явно НЕ риск-факторы для беты: секретов нет; CSP и capabilities минимальны; automation-мост в прод не попадает (тройная защита); contract-слой (основной UI-путь беты) содержит fail-closed гарды на всех путях записи.

---

## Приложение: команды аудита

```bash
git rev-parse HEAD                                            # 11eafaa1027822913f55dbfefcb3a9ed88beafa6
cargo deny check                                              # advisories FAILED (yanked yoke-derive), bans/licenses/sources ok
cargo deny check advisories | grep -E '^(error|warning)'      # 1×error[yanked] yoke-derive 0.8.3
cd gui/tauri-app/frontend-react && npm audit --json           # 0 уязвимостей
cd gui/tauri-app/frontend-v2 && npm audit                     # 22 high (все dev/test), 0 critical
rg -n --hidden -g '!target/**' -g '!node_modules/**' …        # secret-паттерны → все хиты ложные (фикстуры/моки)
gh pr list --state open --limit 100 --json number,title,headRefName,mergeable,updatedAt
```
