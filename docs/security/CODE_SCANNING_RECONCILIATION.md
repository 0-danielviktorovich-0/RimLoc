# Ревизия Code Scanning алертов RimLoc — сверка с деревом `feature/ui-r1-convergence`

- **Дата:** 2026-10-05
- **Алерты:** 130 open-алертов GitHub Code Scanning (CodeQL), привязаны к `refs/heads/main`, коммит анализа `9591f4f10dbd3f43e33811e21fd8f6fab4e47b39` (2026-09-26 23:07 +0900)
- **Сверочное дерево:** worktree `/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main`, ветка `feature/ui-r1-convergence`, HEAD `da2fc777267dfec5c936d06a525b3a6f36d63c2b` — на 280 коммитов впереди `9591f4f` (`git rev-list 9591f4f..HEAD --count` → `280`)
- **Режим:** read-only по коду. Файлы репозитория не менялись (создан только этот документ). Ничего не коммитилось и не пушилось, dismiss в UI GitHub не ставились.

---

## 1. Метод

1. **Инвентарь.** `gh api repos/0-danielviktorovich-0/RimLoc/code-scanning/alerts?state=open&per_page=100 --paginate` → ровно **130** алертов (совпадает с оценкой владельца «~130»). По каждому вытянуты: номер, `rule.id`, `security_severity_level`, CWE-теги, `most_recent_instance.location.path:start_line`, `created_at`, `most_recent_instance.commit_sha`, `most_recent_instance.ref`.
2. **Проверка привязки.** Все 130 алертов: `most_recent_instance.ref = refs/heads/main`, `commit_sha = 9591f4f…` — то есть они описывают код **месячной давности**, а не текущую ветку.
3. **Сверка с деревом.** Для каждой группы «правило+путь» (27 групп) в worktree `ba-main` проверено: существование файла, счётчик `git rev-list 9591f4f..HEAD --count -- <файл>` (менялся ли после анализа) и **фактическое содержимое каждой алертной строки** (sed/grep по текущим файлам; для переехавших строк — `git show 9591f4f:<файл>`). Каждой группе дано конкретное обоснование; bulk-dismiss запрещён и не применялся.
4. **Ограничения метода.** Повторный прогон CodeQL на HEAD не выполнялся (требует CodeQL CLI и сборочной базы; вне рамок этой ревизии). Классификации — результат статической сверки «алерт ↔ текущий код» и анализа модели угроз. После мержа PR #60 GitHub автоматически пересчитает алерты на новом `main`; ожидаемое поведение каждой группы указано в плане (§6).

## 2. Сводка

| Severity | Кол-во | Правило | Разбор |
|---|---|---|---|
| Critical | 1 | `py/command-line-injection` | DEV_ONLY (testlab, путь/бинарь из YAML-конфига dev-инструмента) |
| High | 125 | `rust/path-injection` 85 · `py/path-injection` 40 | 80 FALSE_POSITIVE (прод-крейсы, taint = аргументы локального пользователя) · 41 DEV_ONLY (testlab) · 2 DEV_ONLY (`build.rs`) · 2 TEST_ONLY |
| Medium | 4 | `actions/missing-workflow-permissions` | STALE_CODE — фикс уже в ветке (`permissions: {}` + per-job минимум), закроется мержем PR #60 |

CWE: path-injection — CWE-022/023/036/073/099; command-line-injection — CWE-078/088; workflow-permissions — CWE-275.

**Итог по классификациям (130):** FALSE_POSITIVE **80** · DEV_ONLY **43** · STALE_CODE **4** · TEST_ONLY **3** · PRODUCTION_REACHABLE **0** · DUPLICATE **0** · NEEDS_INVESTIGATION **0**.

**Бета-гейт:** неразобранных exploitable Critical/High — **0** (см. §7).

## 3. Сводная таблица групп (27 групп «правило+путь»)

| # | Правило / CWE (сокр.) | Файл | Алертов | Sev | Строки менялись с 9591f4f | Классификация | Корень | Рекомендация |
|---|---|---|---|---|---|---|---|---|
| 1 | rust/path-injection / 022 | `crates/rimloc-services/src/import.rs` | 32 | high | нет | FALSE_POSITIVE | R2 | dismiss (гр. A) |
| 2 | py/path-injection / 022 | `testlab/scripts/generate_manifests.py` | 22 | high | нет | DEV_ONLY | R1 | paths-ignore |
| 3 | rust/path-injection / 022 | `gui/tauri-app/src-tauri/src/main.rs` | 12 | high | да (22 комм.) | FALSE_POSITIVE | R2 | dismiss (гр. A) |
| 4 | rust/path-injection / 022 | `crates/rimloc-services/src/util.rs` | 9 | high | нет | FALSE_POSITIVE | R2 | dismiss (гр. A) |
| 5 | py/path-injection / 022 | `testlab/scripts/blind_benchmark.py` | 5 | high | нет | DEV_ONLY | R1 | paths-ignore |
| 6 | actions/missing-workflow-permissions / 275 | `.github/workflows/ci.yml` | 4 | medium | **да** | STALE_CODE | R3 | мерж PR #60 |
| 7 | py/path-injection / 022 | `testlab/ui_automation/lifecycle.py` | 4 | high | нет | DEV_ONLY | R1 | paths-ignore |
| 8 | rust/path-injection / 022 | `crates/rimloc-cli/src/commands/learn_patches.rs` | 4 | high | нет | FALSE_POSITIVE | R2 | dismiss (гр. A) |
| 9 | rust/path-injection / 022 | `crates/rimloc-services/src/learn/export.rs` | 4 | high | нет | FALSE_POSITIVE | R2 | dismiss (гр. A) |
| 10 | py/path-injection / 022 | `testlab/ui_automation/driver.py` | 3 | high | да (1) | DEV_ONLY | R1 | paths-ignore |
| 11 | py/path-injection / 022 | `testlab/ui_automation/journey.py` | 3 | high | **да (1)** | DEV_ONLY (+STALE: строки сдвинулись) | R1 | paths-ignore |
| 12 | rust/path-injection / 022 | `crates/rimloc-import-po/src/lib.rs` | 3 | high | нет | FALSE_POSITIVE | R2 | dismiss (гр. A) |
| 13 | rust/path-injection / 022 | `crates/rimloc-services/src/learn/mod.rs` | 3 | high | нет | FALSE_POSITIVE | R2 | dismiss (гр. A) |
| 14 | rust/path-injection / 022 | `crates/rimloc-services/src/plugins.rs` | 3 | high | нет | FALSE_POSITIVE (residual risk — см. §4.14) | R2 | dismiss (гр. A) |
| 15 | py/path-injection / 022 | `testlab/scripts/tkey_population_reconcile.py` | 2 | high | нет | DEV_ONLY | R1 | paths-ignore |
| 16 | rust/path-injection / 022 | `crates/rimloc-cli/build.rs` | 2 | high | нет | DEV_ONLY | R1 | paths-ignore (или dismiss) |
| 17 | rust/path-injection / 022 | `crates/rimloc-cli/src/commands/export_po.rs` | 2 | high | нет | FALSE_POSITIVE | R2 | dismiss (гр. A) |
| 18 | rust/path-injection / 022 | `crates/rimloc-cli/tests/i18n_keys.rs` | 2 | high | нет | TEST_ONLY | R1 | paths-ignore `**/tests/**` |
| 19 | rust/path-injection / 022 | `crates/rimloc-parsers-xml/src/lib.rs` | 2 | high | да (1) | FALSE_POSITIVE | R2 | dismiss (гр. A) |
| 20 | rust/path-injection / 022 | `crates/rimloc-services/src/scan.rs` | 2 | high | да (3) | FALSE_POSITIVE | R2 | dismiss (гр. A) |
| 21 | **py/command-line-injection / 078** | `testlab/ui_automation/lifecycle.py:72` | 1 | **critical** | нет | DEV_ONLY | R1 | paths-ignore |
| 22 | py/path-injection / 022 | `testlab/ui_automation/__main__.py` | 1 | high | нет | DEV_ONLY | R1 | paths-ignore |
| 23 | rust/path-injection / 022 | `crates/rimloc-cli/src/main.rs:1907` | 1 | high | да (1) | FALSE_POSITIVE | R2 | dismiss (гр. A) |
| 24 | rust/path-injection / 022 | `crates/rimloc-config/src/lib.rs:91` | 1 | high | нет | FALSE_POSITIVE | R2 | dismiss (гр. A) |
| 25 | rust/path-injection / 022 | `crates/rimloc-services/src/learn/keyed.rs:21` | 1 | high | нет | FALSE_POSITIVE | R2 | dismiss (гр. A) |
| 26 | rust/path-injection / 022 | `crates/rimloc-services/src/observability.rs:1217` | 1 | high | нет | FALSE_POSITIVE | R2 | dismiss (гр. A) |
| 27 | rust/path-injection / 022 | `crates/rimloc-services/src/session.rs:1743` | 1 | high | **да (17 комм.)** | TEST_ONLY (+STALE: строка переехала) | R1/R2 | dismiss «used in test» |

## 4. Детальные обоснования по группам

Модель угроз, общая для всех классификаций FALSE_POSITIVE: **RimLoc — локальный инструмент (CLI + Tauri GUI), исполняющийся с правами запустившего его пользователя; единственный оператор путей — сам пользователь** (аргументы CLI, поля GUI-форм). CodeQL помечает «user-provided value» как taint, но передача пути от оператора инструмента файловому API не пересекает границу доверия — атаковать некого: привилегированного контекста (setuid, сервер, сервис) нет. Это не «код идеален», а «правило не соответствует модели угроз десктопного инструмента». Реальные boundary-механизмы в коде есть и перечислены в §5 (корень R2).

### 4.1 `crates/rimloc-services/src/import.rs` — 32 алерта, FALSE_POSITIVE
Все sink'и — `Path::exists()`-проверки, `std::fs::copy(out_xml, *.xml.bak)` (backup), `std::fs::read`, запись через `write_language_data_xml` (проверено: строки 26, 87–111, 177–188, 243–284, 376–467 совпадают с алертными). Путь `out_xml`/`po` — параметры `import_po_to_file()`, приходящие из CLI-команды `import-po` (clap-аргументы пользователя, точка входа `crates/rimloc-cli/src/main.rs:1624` → `commands::import_po::run_import_po`). Содержимое PO-файла попадает только в **значения** переводов, не в пути. Файл не менялся с 9591f4f (`changed_since_main=0`, последний коммит 4610a31 2026-09-26) — алерты актуальны по строкам, но FP по модели угроз.

### 4.2 `testlab/scripts/generate_manifests.py` — 22 алерта, DEV_ONLY
Sink'и — `os.walk(mod_path)`, `os.listdir`, `open(...)` записи манифестов (строки 29–159 сверены). Источник — `argparse`: `--mods-root` (default `/Applications/RimWorld.app/Mods`), `--out-dir` (default внутри `testlab/`); скрипт декларирован READ-ONLY по отношению к модам (докстринг, строки 2–8). Запускается вручную разработчиком для генерации тест-манифестов. Не production-код, в поставку не входит.

### 4.3 `gui/tauri-app/src-tauri/src/main.rs` — 12 алертов, FALSE_POSITIVE
Sink'и — запись `out_json`/`out_csv` отчётов сканирования (строки 797, 1368, 1370, 1390–1392) и построение ответов (2085–2087, 2822–2847 — сериализация, не fs). Каждая fs-точка прикрыта **двумя** проверками, видимыми в коде: `ensure_caller_path_absolute(field, path)` — отказ для относительных путей (main.rs:1442), и `make_absolute(scan_root, path)` (main.rs:1427). Файл активно менялся после анализа (22 коммита), но все 12 алертных мест существуют в текущем виде. Taint-источник CodeQL — параметры Tauri-команд из локального фронтенда; окно local-only, пользователь и есть оператор.

### 4.4 `crates/rimloc-services/src/util.rs` — 9 алертов, FALSE_POSITIVE
Sink'и — внутри **самих санитайзер-функций**: `write_atomic` (создание tmp-файла `.{name}.tmp.<pid>-…`, `fs::rename`, строки 112–147) и `verify_xml_char_validity` (`read_dir`/`read`, строки 393–408). `write_atomic` документирован как symlink-safe (замена листа вместо записи через symlink, докстринг строки ~100–110), путь приходит уже после boundary-проверок вызывающих. Флаговать общий хелпер — типовая сверх-генерализация taint-анализа; это один корень, а не 9 уязвимостей.

### 4.5 `testlab/scripts/blind_benchmark.py` — 5 алертов, DEV_ONLY
Sink'и — `mkdir -p` и запись бенчмарк-отчётов; путь — `--out-dir` из argparse (строки 135–156 сверены; default `testlab/benchmarks`). Dev-скрипт бенчмаркинга extraction.

### 4.6 `.github/workflows/ci.yml` — 4 алерта, STALE_CODE
CodeQL (строки 15/27/40/56 в момент анализа) не находил ограничения прав GITHUB_TOKEN. В ветке это уже исправлено: `git diff 9591f4f..HEAD -- .github/workflows/ci.yml` показывает добавленные `permissions: {}` на верхнем уровне (текущая строка 21) и per-job `permissions:` минимумы (строки 34, 48, 63, 81, 100, 109, 131); комментарий в шапке файла фиксирует правило. Последний коммит файла 9423208 (2026-09-27) — **после** анализа CodeQL. После мержа PR #60 и нового анализа все 4 алерта закроются автоматически. Единственная группа, где мерж сам по себе уменьшает счётчик.

### 4.7 `testlab/ui_automation/lifecycle.py` — 4 алерта path-injection, DEV_ONLY
Sink'и — чтение/запись/удаление pid-файла (`open(pid_file)` строки 46, 85, 124; `os.unlink` 127). `pid_file` — поле YAML-конфига journey (`journey.py:152` — `cfg.get('pid_file')`), конфиг лежит в репозитории и передаётся разработчиком через `--config`. Dev-инструмент acceptance-прогонов Tauri-приложения.

### 4.8 `crates/rimloc-cli/src/commands/learn_patches.rs` — 4 алерта, FALSE_POSITIVE
Sink'и — `create_dir_all`/`File::create` (строки 19, 21, 56, 58 сверены). Пути строятся от `scan_root` (корень мода, указанного пользователем): `out_dir = scan_root.join("learn_out")` (строка 16), `out_json` — опциональный пользовательский override. Запись — внутри дерева, выбранного оператором.

### 4.9 `crates/rimloc-services/src/learn/export.rs` — 4 алерта, FALSE_POSITIVE
Sink'и — `File::create(path)` отчётов learn (строки 26, 33, 91) и `create_dir_all(parent)` (77). Пути: `base.join("Languages").join(lang_dir).join(rel_path)` (строка ~76) — `base` и `lang_dir` пользовательские, `rel_path` — **реальные** относительные пути из walkdir по мод-дереву (не строки из XML), поэтому `..`-компоненты невозможны по построению.

### 4.10 `testlab/ui_automation/driver.py` — 3 алерта, DEV_ONLY
Sink'и — `os.makedirs(out_dir)` скриншотов (119), `screencapture -x <out_path>` через `subprocess.run` списком аргументов (227), `os.makedirs(work_dir)` (236). `out_dir`/`work_dir` — из того же YAML-конфига (default `/tmp/ui-automation-*`, journey.py:118–119). Файл менялся после анализа (1 коммит, 2a4dfc6), но все три sink-строки существуют как описано.

### 4.11 `testlab/ui_automation/journey.py` — 3 алерта, DEV_ONLY (+STALE-компонент)
Алертные строки 115/120/121 в `9591f4f` — `open(config_path)`, `os.makedirs(shots_dir)`, `os.makedirs(work_dir)` (проверено `git show 9591f4f:…journey.py`); коммит 2a4dfc6 (после анализа) добавил шаги `optional`/`set_text` и **сдвинул строки** — на этих местах сейчас не-fs код (`detail = f'text_rows=…'`, `driver.assert_offscreen(fresh)`). Т.е. алерты указывают на старую разметку; сами sink'и никуда не делись — `os.makedirs` теперь на строках 140–141, источник тот же YAML-конфиг. Классификация по сути: DEV_ONLY; при пересчёте на новом main алерты просто переедут на актуальные строки.

### 4.12 `crates/rimloc-import-po/src/lib.rs` — 3 алерта, FALSE_POSITIVE
Sink'и — `File::open(po_path)` (15), `create_dir_all(parent)` + `File::create(out_path)` в `write_language_data_xml` (183, 186). `po_path`/`out_path` — аргументы CLI пользователя (import/export-po). Содержимое PO → только значения, не пути.

### 4.13 `crates/rimloc-services/src/learn/mod.rs` — 3 алерта, FALSE_POSITIVE
Sink'и — `create_dir_all(&opts.out_dir)` (86), `File::create(learned_path)` (122), `File::create(out_path)` (165). Все пути — комбинации пользовательского `out_dir`/`learned_out` из CLI-опций learn.

### 4.14 `crates/rimloc-services/src/plugins.rs` — 3 алерта, FALSE_POSITIVE (residual risk)
Алертные sink'и — только проверки существования (`is_file` 90, `is_dir` 127, `read_dir` 128) для динамических плагинов. Обращаю внимание отдельно: рядом лежит `load_dynamic_plugin` — `libloading::Library::new(path)` на `.so/.dll/.dylib` (строки 101–124). Path-injection здесь ничего не добавляет (загрузка плагина — это уже явное выполнение кода по выбору пользователя), но сам механизм — точка доверия: **если когда-нибудь путь к плагинам начнёт приходить из данных мода/сети — это станет реальным RCE**. Сейчас путь задаёт пользователь (CLI/конфиг). Классификация FP, с фиксацией residual risk; при изменении источника пути — пересмотреть.

### 4.15 `testlab/scripts/tkey_population_reconcile.py` — 2 алерта, DEV_ONLY
Sink'и — `out.parent.mkdir()` / `out.write_text` (162–163); `out` — `--out-json` из argparse (строка 101, default внутри `testlab/reports/`).

### 4.16 `crates/rimloc-cli/build.rs` — 2 алерта, DEV_ONLY
Sink'и — `fs::read_dir(&i18n_dir)` (20) и `fs::write(&dst)` (59). Пути — `CARGO_MANIFEST_DIR` и `OUT_DIR`, то есть окружение, контролируемое cargo/сборщиком; выполняется на машине сборки, в рантайм продукта не попадает.

### 4.17 `crates/rimloc-cli/src/commands/export_po.rs` — 2 алерта, FALSE_POSITIVE
Sink'и — `dir.exists()` в `has_definj_files` (89) и `suggested.exists()` (120), где пути строятся от пользовательского `scan_root` (`scan_root.join("Languages").join(src_dir).join("DefInjected")`). Чистые проверки существования + warn-сообщения.

### 4.18 `crates/rimloc-cli/tests/i18n_keys.rs` — 2 алерта, TEST_ONLY
Integration-тест (каталог `tests/`), который сам содержит containment-проверки: `assert!(base.starts_with(&repo_root))` и `assert!(canon.starts_with(&base))` (строки 15–33). Ирония: CodeQL флагит тест на path traversal.

### 4.19 `crates/rimloc-parsers-xml/src/lib.rs` — 2 алерта, FALSE_POSITIVE
Sink'и — `fs::read_to_string(path)` в `load_defs_dict_from_file` (726) и `load_type_schema_as_dict` (753). Путь — аргумент пользователя (словарь дефиниций/схема, CLI-опции). Файл менялся после анализа (1 коммит), алертные строки на месте.

### 4.20 `crates/rimloc-services/src/scan.rs` — 2 алерта, FALSE_POSITIVE
Sink'и — `p.is_dir()` / `languages_root.is_dir()` в `autodiscover_learn_dirs` (149, 159): перебор **фиксированных** имён (`_learn`, `learn_out`, `Learn`, `learn`) от пользовательского корня сканирования. Свободного управления путём нет — только конкатенация констант.

### 4.21 `testlab/ui_automation/lifecycle.py:72` — 1 алерт critical `py/command-line-injection`, DEV_ONLY
`subprocess.Popen(cmd, env=env, …)`; единственные вызовы — `inst.spawn([binary], env)` (journey.py:160, 196, 213), где `binary = app['binary']` из YAML-конфига (journey.py:144), конфиг — файл репозитория, передаваемый `--config` разработчиком. `cmd` — список аргументов, без shell. CodeQL ставит critical, потому что командная строка «зависит от user-provided value» (файл конфига = taint), но источником управляет тот же разработчик, что запускает скрипт. Не exploitable вне dev-машины. **Единственный critical в списке — не продуктовый.**

### 4.22 `testlab/ui_automation/__main__.py:24` — 1 алерт, DEV_ONLY
`open(args.config)` — `--config` из argparse того же dev-инструмента.

### 4.23 `crates/rimloc-cli/src/main.rs:1907` — 1 алерт, FALSE_POSITIVE
Строка — декларация `fn init_tracing()`; sink внутри: `fs::create_dir_all(&log_dir)` + rolling appender (1911–1915). `log_dir` = `resolve_log_dir()`, у которого есть собственный валидатор `is_safe_relative` (отклоняет абсолютные пути и любые компоненты кроме `Normal`/`CurDir` — проверено в коде, main.rs ~1907+). Путь ведёт в пользовательский лог-каталог; taint — env/ОС, не внешний ввод.

### 4.24 `crates/rimloc-config/src/lib.rs:91` — 1 алерт, FALSE_POSITIVE
`std::fs::read_to_string(&path)` для конфига: поиск строго в `CWD/rimloc.toml` и `$HOME/.config/rimloc/rimloc.toml` (комментарий в коде, строки 88–100). Пути конструируются из `current_dir()`/`dirs::config_dir()`, свободного taint нет.

### 4.25 `crates/rimloc-services/src/learn/keyed.rs:21` — 1 алерт, FALSE_POSITIVE
`fs::read_to_string(path)` в `load_keyed_dict_from_file` — путь словаря передаёт пользователь CLI-опцией; чтение, не запись.

### 4.26 `crates/rimloc-services/src/observability.rs:1217` — 1 алерт, FALSE_POSITIVE
`fs::read(&path)` по списку `[report_path, diag_path, env_path]` — файлы, **созданные этим же вызовом** в `out_dir` пользователя несколькими строками выше (`diag_path = out_dir.join("diagnostics.json")`, 1205–1207). Чтение только что записанного.

### 4.27 `crates/rimloc-services/src/session.rs:1743` — 1 алерт, TEST_ONLY (+STALE-компонент)
В `9591f4f` строка 1743 — код тестового модуля (`#[cfg(test)]`; рядом `tempfile::tempdir`, ассерты «no traversal side effects» — проверено `git show 9591f4f:…session.rs`). Файл переписан (17 коммитов, последний 970a309 2026-10-05); на 1743 сейчас докстринг другого теста. Sink находился внутри теста на path-traversal — то есть сканер флагит **тест, проверяющий отсутствие traversal**. Классификация: TEST_ONLY; при пересчёте алерт переедет на актуальные строки тестов либо закроется dismiss «used in test».

## 5. Корневые причины («фиксить корни, не 130 симптомов»)

**R1 — CodeQL Default Setup работает без подключённого конфига, и конфиг не покрывает dev/test зоны.**
Факт: `gh api repos/…/code-scanning/default-setup` → `state: configured`, `config_file: null`, updated 2026-09-20; при этом в репо есть `.github/codeql/codeql-config.yml` с `paths-ignore`, который **никем не подключён**. Одно действие — подключить конфиг и дополнить его — убирает **45** алертов из 130 (41 testlab-Python + 2 `tests/i18n_keys.rs` + 2 `build.rs`).
⚠️ Перед подключением конфиг надо править осторожно: текущий `paths: - RimLoc` не матчится с реальной структурой репо (в корне нет папки `RimLoc/`; алерты приходят из `crates/…`, `gui/…`, `testlab/…`) — если подключить как есть, включающие фильтры отсекут весь анализ. Актуальный список `paths`/`paths-ignore` надо писать от корня репозитория.

**R2 — Taint-модель CodeQL противоречит модели угроз локального инструмента.**
«user-provided value» для десктопного CLI/GUI — это сам оператор. Никакой код-фикс это не изменит: нельзя «запатчить» то, что пользователь передаёт путь собственному инструменту. Решение — **групповой dismiss с обоснованием** (15 групп по формулировкам §4, ссылка на этот документ в каждом dismiss), плюс сохранение того, что уже сделано правильно и подтверждает осознанную границу: `canonical_view`/`is_within` (`util.rs`, symlink-resolving containment), `write_atomic` (atomic + symlink-safe leaf), `ensure_caller_path_absolute` (gui/main.rs:1442), `is_safe_relative` (cli/main.rs, resolve_log_dir), тесты на traversal (`session.rs`, `i18n_keys.rs`). Закрывает **80** алертов. Остаточный риск, за которым следить: динамические плагины (§4.14) и отсутствие containment-проверки (не только абсолютности) в GUI-путях — это F-1 из `docs/security/PRE_BETA_SECURITY_AUDIT.md`, согласуется с данной ревизией.

**R3 — GITHUB_TOKEN без ограничения прав в CI.**
4 алерта medium. Уже исправлено в ветке (`permissions: {}` + per-job минимум, коммит 9423208 от 2026-09-27 — после анализа CodeQL). Мерж PR #60 + новый анализ — и алерты закроются сами. Код-фиксы не требуются.

## 6. План before/after

| Этап | Действие | Алертов open (прогноз) | Комментарий |
|---|---|---|---|
| **Before** | текущее состояние main @ 9591f4f | **130** (1C / 125H / 4M) | инвентарь этой ревизии |
| Шаг 1 | мерж PR #60 (`feature/ui-r1-convergence` → `main`, state OPEN, MERGEABLE) + CodeQL пересчитает main | **~126** (1C / 122H / 0M) | −4 medium (R3, STALE). Алерты journey.py/session.rs не закроются — переедут на актуальные строки тех же sink'ов |
| Шаг 2 | подключить `.github/codeql/codeql-config.yml` в Default Setup, предварительно переписав `paths` от корня репо и добавив `paths-ignore`: `testlab/**`, `**/tests/**`, `**/build.rs` | **~81** (0C / 81H / 0M) | −45 (R1): уходят 41 testlab-Python **включая единственный critical**, 2 `tests/i18n_keys.rs`, 2 `build.rs`. Остаются 80 High-FP прод-крейсов + 1 High в тест-модуле `session.rs` (внутри `src/`, paths-ignore его не берёт) |
| Шаг 3 | dismiss 15 групп FP + 1 session.rs по §4 (не bulk: каждая с формулировкой и ссылкой сюда) | **0** | −81 (R2 + TEST_ONLY) |

Уточнение к таблице: после шага 2 остаются 81 = 80 FP прод-крейсов + 1 session.rs (тест внутри `src/`, paths-ignore его не берёт). После шага 3 — 0.

Если dismiss в UI по каким-то причинам откладывается, допустим эквивалент: держать конфиг шага 2 и принять остаток как задокументированный FP-фон — бета-гейт (§7) это покрывает, т.к. все оставшиеся разобраны.

## 7. Бета-гейт

**Критерий: 0 неразобранных exploitable Critical/High.**

- Разобраны все 130 (§3–§4, каждая группа с обоснованием и ссылкой на код).
- Exploitable Critical: **0** — единственный critical (alert #219, `testlab/ui_automation/lifecycle.py:72`) исполняет бинарь из YAML-конфига dev-инструмента на машине разработчика; внешнего источника у taint нет.
- Exploitable High: **0** — из 125 high ни один не даёт недоверенному источнику (XML/PO-данные модов) контроль над путём за пределами выбора оператора: пути записи строятся из CLI/GUI-аргументов и реальных путей файловой системы; taint из содержимого файлов модов попадает только в значения переводов.

## 8. Что не сделано / ограничения

- Повторный прогон CodeQL на `feature/ui-r1-convergence` не запускался — классификации верифицированы статической сверкой строк и diff-анализом, а не новым сканированием. Прогнозы §6 проверить после мержа PR #60 по факту нового анализа.
- Расхождение с брифом: в задаче указано «279 коммитов позади», фактический `git rev-list 9591f4f..HEAD --count` = **280**.
- dismiss'ы в GitHub UI не ставились (вне полномочий ревизии: код и настройки не трогаем, коммитит главный поток).

## 9. R2-волна (2026-10-07, SHA af06a4e): 6 новых High, все DISMISSED_WITH_EVIDENCE

Первый полный CodeQL-анализ, включивший R2-код (chat-batch 273a79c, IfModActive
42fbfc0, defs d712ba8). Все шесть — `rust/path-injection`, high, источник —
значения LoadFolders.xml / файловой записи мода; правило и класс совпадают с
разобранными в §4 (FP-фон прод-крейсов). Прогресс-гейт §7 сохраняется: 0 open
после dismiss'ов (CS и DA).

| # | Место | Класс | Обоснование |
|---|-------|-------|-------------|
| 312 | modview.rs:381 | FP | `root.join("LoadFolders.xml")` — константный join доверенного root (выбор оператора CLI/GUI); идентично группе `src-tauri/commands/*` из §4 |
| 313 | modview.rs:214 | FP | `classic_version_dirs`: имя из `read_dir` самого root, однокомпонентный join (сепараторы в имени невозможны); внешний resolution отсутствует |
| 314 | modview.rs:426 | FP | ветка PLAIN LoadFolders: join уже пропущен через `is_within(&dir, root)` с typed bail (H2-гейт, строки 417–424); sink — read-only скан содержимого |
| 315 | modview.rs:462 | FP | ветка CONDITIONAL IfModActive: тот же H2-гейт `is_within` + typed bail (строки 452–460), срабатывает только при резолвленном true; без контекста путь не строится вовсе (unresolved → POTENTIAL) |
| 316 | modview.rs:511 | FP | `root.join("Common")` — константный join классического лейаута; идентично #312 |
| 317 | tests/ifmodactive_corpus.rs:35 | used_in_tests | корпусный путь из `RIMLOC_IFMOD_CORPUS_MOD` (env), тест skipped без env; класс TEST_ONLY из §4 |

Сопутствующее: Write-guard кампания (canonical_view fail-closed,
ensure_writable_output_path, PO-# refusal) покрывает синки; H2-гейты
верифицированы adversarial-тестами (`write_guard_adversarial`,
`ifmodactive_corpus`). Дисми́ссы по одному через API с сылкой на этот раздел,
не bulk.

## 10. R3-интеграция (2026-10-07, SHA 41c63e0): 10 новых High, все DISMISSED_WITH_EVIDENCE

Контекст: default setup CodeQL работает БЕЗ привязанного репо-конфига
(default-setup API: `config_file_path: null`; привязка требует скоупа
security_events, которого у токена нет) — поэтому paths-ignore
`gui/tauri-app/vendor/**` не отсекают вендорный код, и в скан попал новый
vendor-форк tauri-utils (C6) и dev-тулинг.

| # | Место | Класс | Обоснование |
|---|-------|-------|-------------|
| 321-327 | vendor/tauri-utils-2.10.1-nfs-appledouble/{acl/build.rs:143,199,427, config/parse.rs:281,394, resources.rs:369,370} | VENDOR | Скопированный upstream tauri-utils 2.10.1 (один функциональный фильтр `._` в define_permissions, файлы байт-в-байт upstream); пути строятся из конфига приложения, выбираемого оператором; идентично классу «patched upstream — не RimLoc-owned» (решение #83). Отсюда же: при привязке конфига к default setup эти пути уходят в paths-ignore автоматически |
| 318-320 | testlab/reviewer/sanitize-packet.py:87-93 | DEV_TOOL | `sys.argv[1]`/rglob → read_text: путь выбирает оператор на своей машине, недоверенного источника нет; testlab вне shipping (paths-ignore testlab/** в конфиге — там же причина) |

Санитайзер-фейл-клозд и вендорный фильтр верифицированы живьём (пакет
2305d56: 16 файлов PASS; сборка C6: exit 0 без DMG-стадии). Дисми́ссы по
одному, не bulk.
