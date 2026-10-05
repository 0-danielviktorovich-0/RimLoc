# Аудит правды документации RimLoc — 2026-10-05

- **Дата:** 2026-10-05
- **Дерево:** worktree `ba-main`, ветка `feature/ui-r1-convergence`, HEAD `da2fc77` (плавает — активная кампания)
- **Якорь возможностей:** `docs/competitive/RIMLOC_CAPABILITIES_BASELINE.md` (2026-10-05, код на main)
- **Режим:** read-only по коду; правлены только документация, `mkdocs.yml`, `AGENTS.md` и один явно безопасный CI-фикс (см. §7). Коммиты — вне этого лейна.

## 1. Метод

1. Полный обход `docs/` — 168 файлов (без `site/`, без `.DS_Store`), по всем подпапкам:
   `architecture/`, `campaign/`, `competitive/`, `design/`, `development/` (+`development/testing/`),
   `integration/`, `security/`, `readme/ru/`, `en/`, `ru/`, `overrides/`, `assets/`, `screenshots/`.
2. Заголовки и даты каждого внутреннего документа прочитаны; спорные и содержательные — прочитаны целиком.
3. Фича-заявления user-facing доков (EN/RU) сверены с фактическими возможностями по
   `RIMLOC_CAPABILITIES_BASELINE.md`, списком команд `crates/rimloc-cli/src/commands/`,
   состоянием фронтендов (`gui/tauri-app/`) и живым GitHub-состоянием (`gh api`).
4. Сборка сайта: `python3 -m mkdocs --version` → mkdocs 1.6.1 (venv, `requirements-docs.txt`:
   mkdocs-material 9.7.7, mkdocs-static-i18n 1.3.1; установлен в `/tmp/rimloc-venv` по разрешению брифа —
   глобальный `pip3 install --user` заблокирован PEP 668). Гейт:
   `SITE_URL=https://0-danielviktorovich-0.github.io/RimLoc/ mkdocs build --strict`.
5. Исторические кампан-логи (чекпоинты, handoff'ы, планы завершённых кампаний) классифицированы
   пачками как SUPERSEDED с пометкой «история» — они не переписываются, новое состояние живёт в новых
   датированных документах (см. `STATE_CORRECTION_2026-10-05.md`).

## 2. Сводка матрицы (168 файлов)

| Класс | Файлов | Комментарий |
|---|---|---|
| CURRENT | **127** | в т.ч. все EN/RU user-docs за отмеченными ниже, research-референсы об игре, код-якорные документы, security-аудиты 2026-10-05, assets/schemas (гейт schema-drift CI) |
| PARTIALLY_STALE | **21** | G4/G5/G6-мандаты и Svelte GUI-дизайн-доки (требования действуют, имплементационные детали относятся к замороженному Svelte GUI, прод-фронт теперь React 19); CAMPAIGN_SNAPSHOT и release-доки на 27–29.09 (до UI R1); `guide/gui.md` EN/RU («прод теперь v2 Svelte» — уже не так); `install.md` EN/RU (см. §4.2) |
| STALE | **5** | `docs/screenshots/*.png` — сироты: ни одна страница EN/RU/README на них не ссылается (grep), и сняты они со старого UI; «Coming soon: screenshots» в `en/dev/index.md:215` всё ещё обещает их |
| SUPERSEDED | **9** | история: `design/MORNING_CHECKPOINT_2026-10-05`, `design/UI_R1_CHECKPOINT`, `development/AUTONOMOUS_PLAN`, `AUTONOMOUS_STATUS`, `FINAL_ACCEPTANCE`, `NIGHT_HANDOFF_2026-09-26`, `FRONTEND_BAKEOFF_STATE` (решение принято — React R1), `COMMUNITY_TESTING_HANDOFF` (в `exclude_docs`), `integration/HANDOFF-COMPUTER-FABRIC` |
| BROKEN_LINK | **6** | файлы CURRENT по содержанию, но с битым якорем — см. §5 |
| DUPLICATE | **0** | строгих дублей нет; одно функциональное перекрытие — `development/COMPETITOR_MATRIX.md` ↔ `competitive/COMPETITOR_DEEP_DIVE_2026-10.md` (разные уровни: implementation-design v2 vs source-инспекция) — консолидировать при следующей ревизии competitive |
| MISSING | **0 файлов** | но есть отсутствующие СТРАНИЦЫ — см. §6 |

## 3. Ключевые содержательные находки (фича-заявления ↔ реальность)

3.1. **`docs/development/REPOSITORY_STATE_2026-10.md:37` — «Тегов/GitHub Releases: нет» — неточно.**
Живой API (`gh api repos/0-danielviktorovich-0/RimLoc/releases`): **20 релизов**, все pre-release,
свежайший `v0.1.0-alpha.1-dev.e774d67` от 2025-10-02; теги на месте. Верно другое: новых тегов/релизов
с 2025-10 не создаётся (release workflows parked, `9591f4f`). Рекомендация: переформулировать
«новых тегов/релизов нет; исторические dev-pre-releases 2025-09…10 — устаревшие».

3.2. **crates.io жив:** `rimloc-cli 0.1.0-alpha.1` опубликован (проверено `GET crates.io/api/v1/crates/rimloc-cli`).
Инструкция `cargo install rimloc-cli` в `en/install.md` работает.

3.3. **`en/install.md` / `ru/install.md` вводят в заблуждение выбором релиза:** «choose the latest
release that is NOT marked Pre-release» — **все 20 релизов помечены Pre-release**, стабильного нет;
ветка `dev-latest`-ассетов описана, но свежих dev-релизов год не было. Странице нужна честная пометка
«сейчас публикуются только dev pre-releases» или удаление секции до первого стабильного релиза.

3.4. **`en/guide/gui.md` / `ru/guide/gui.md` отстают дважды.** Баннер честно помечает страницу как
legacy v1 и говорит «поставляемое приложение теперь использует v2 (Svelte)». На 2026-10-05 прод-фронт —
**React 19** (`frontend-react`, `tauri.react.conf.json` → `frontend-react/dist`), Svelte v2 — замороженный
fallback (`docs/design/CURRENT_SVELTE_BASELINE.md`), v1 — reference. Страницу нужно переписать под React GUI
(скриншоты актуального UI там же закроют §STALE по `docs/screenshots/`).

3.5. **CLI-доки покрывают 11 команд из ~19.** Фактические команды (`ls crates/rimloc-cli/src/commands/`):
annotate, build_mod, compare, diff_xml, doctor, export_po, import_po, init, lang_update, learn_defs,
learn_patches, morph, scan, schema, translate, validate, version_diff, wordinfo, xml_health.
`en/cli/index.md` описывает 11; в nav отдельно есть `lang_update`. Без страниц вообще:
`doctor`, `compare`, `translate`, `wordinfo`, `version_diff`, `learn_defs`/`learn_patches`, `schema`,
экспорт/импорт XLIFF/CSV/JSON (XLIFF — LEVEL 4 в baseline, команда есть). Уровень из baseline:
покрытые команды — LEVEL 5, значит доки user-facing отстают от продукта, а не наоборот.

3.6. **Возможности LEVEL 5 из baseline подтверждены доками** (scan/validate/export-po/import-po/build_mod/
diff_xml/annotate/xml_health/morph/init/lang_update) — расхождений «документирует то, чего нет» не найдено.
Заявления о TM — только в design-доках с честными 0 (`RIMLOC_CAPABILITIES_BASELINE.md:49-53`);
`TM_LIVE_BRIEF.md` — бриф на доработку (лейн `wt-tm-live` жив), не user-facing обещание.

3.7. **EN/RU parity:** деревья идентичны, кроме **`en/dev/plugins.md` — русской пары нет**
(diff `find en` ↔ `find ru`). Все остальные 38/39 страниц структурно парны. Плагины — dev-тема;
перевод нужен, либо честная пометка «EN only» в RU `dev/index.md`.

## 4. mkdocs.yml — аудит и правки

Проверка: `mkdocs --version` → 1.6.1 (venv из `requirements-docs.txt`); затем
`SITE_URL=https://0-danielviktorovich-0.github.io/RimLoc/ mkdocs build --strict`.

Было (до правок): `Aborted with 2 warnings in strict mode!` —
1) `Material emoji logic has been officially moved into mkdocs-material`;
2) `A reference to 'ru/guide/gui-preview.md' is included in the 'nav' configuration, which is not found`.

Применённые правки (только ссылки/конфиг, содержание не тронуто):

| Правка | Обоснование |
|---|---|
| `copyright: "© 2025 …"` → `"© 2025–2026 RimLoc Project"` | задание: 2025 → 2026 |
| nav: удалён пункт «Предпросмотр и редактор (RU): ru/guide/gui-preview.md» | mkdocs-static-i18n (`docs_structure: folder`) нормализует `ru/**` как переводы страниц; отдельной страницы `ru/guide/gui-preview.md` в nav-пространстве нет — RU-версия доступна переключателем языка. Сняло warning №2 |
| `pymdownx.emoji`: `materialx.emoji.*` → `material.extensions.emoji.*` | миграция эмодзи-логики в mkdocs-material (сняло warning №1) |

Стало: `INFO - Documentation built in 1.95 seconds`, **0 warnings, strict зелёный** (повторный прогон, exit 0).

Оценка остальных полей: `site_url` из ENV — корректно (CI задаёт); `edit_uri: edit/main/docs/` —
актуально; `repo_url` — верный; мёртвых страниц в nav нет (strict это теперь и гарантирует);
старых Svelte GUI-упоминаний в user-docs EN/RU нет (grep по `en/ ru/` — 0 вхождений вне gui-страниц,
в самих gui.md слово Svelte употреблено честно в баннере). Внутренние INFO-находки сборки — в §5.

## 5. Broken links (точный список)

Заголовочный фикс сделан; перечисленное ниже **не чинилось** сознательно — см. рекомендацию.

| Файл | Битая ссылка | Причина |
|---|---|---|
| `ru/faq.md:19` | `glossary.md#плейсхолдер` | якорь не существует |
| `ru/getting-started.md:31,51` | `glossary.md#плейсхолдер` | то же |
| `ru/tips.md:17` | `glossary.md#плейсхолдер` | то же |
| `ru/tutorials/export_po.md` | `../glossary.md#плейсхолдер` | то же |
| `ru/tutorials/translate_mod.md` | `../glossary.md#плейсхолдер` | то же |
| `ru/tutorials/update_translations.md` | `../glossary.md#плейсхолдер` | то же |
| `development/DISCOVERABILITY.md:30` | `#9-чеклист-внедрения` | **исправлено**: раздела 9 в документе нет — пункт 9 удалён из «Содержания» с комментарием |

Механика: в `ru/glossary.md` заголовок `## Плейсхолдер` реально есть, но дефолтный slugify
выпиливает кириллицу — собранный HTML даёт `id="_1"` (проверено `grep '<h2' site/ru/glossary/index.html`).
Ссылки ведут на правильную страницу (не 404), якорь просто не срабатывает; mkdocs строит их как INFO, strict не падает.

**Рекомендация (отдельное решение, не применено):** включить unicode-slugify
(`markdown_extensions.toc.slugify: !!python/name:pymdownx.slugs.uslugify`) — починит все кириллические
якоря, но **изменит остальные якоря RU-страниц** (например, «Dry‑run» c `#dryrun` станет другим из-за
U+2011), т.е. сломает уже расползающиеся внешние ссылки на эти якоря. Безопасная альтернатива на сейчас:
в семи ссылках убрать якорь (вести на `glossary.md` целиком) — допустимо сделать при следующей правке
этих страниц.

## 6. Отсутствующие страницы (класс MISSING — для плана доков)

- `ru/dev/plugins.md` — пары нет (единственный провал EN/RU parity).
- CLI-страницы EN+RU для существующих команд: `doctor`, `compare`, `translate`, `wordinfo`,
  `version_diff`, `learn-defs`/`learn-patches` (плагины сканирования), `schema`,
  а также экспорт/импорт XLIFF/CSV/JSON (у `export-po`/`import-po` страницы есть, у форматов — нет).
- Актуальные скриншоты React GUI (см. STALE по `docs/screenshots/` и обещание в `en/dev/index.md:215`).

## 7. CI — диагноз красных чеков PR #60 (все по логам прогонов 2026-10-05)

Источник: `gh pr checks 60` + скачанные логи job'ов (`gh api …/actions/jobs/<id>/logs`).
Смэпинг на стек: workspace включает `gui/tauri-app/src-tauri` (`rimloc-gui`) → `cargo test --workspace`
и `cargo build` в GUI-крейте требуют то, что в CI не готовится.

| Чек | Диагноз (evidence) | Фикс | Статус |
|---|---|---|---|
| **actionlint** (exit 127, 20s) | лог: `go install …@v1.7.12` отработал (скачал go1.26.8 toolchain), затем `actionlint: command not found`. `go install` кладёт бинарник в `$(go env GOPATH)/bin` = `~/go/bin`, которого нет в PATH раннера | добавить `echo "$HOME/go/bin" >> "$GITHUB_PATH"` перед install | **ПРИМЕНЁН** (`.github/workflows/ci.yml`, job `workflows-lint`) — единственная правка CI, явно безопасная |
| **rustfmt + clippy** (15s) | rustfmt-дифф в `gui/tauri-app/src-tauri` (build.rs, main.rs, contract_adapter.rs, selfloc_catalog.rs) | `cargo fmt --all` | уже применён локально ранее: `cargo fmt --all --check` → exit 0 (проверено в этой сессии); нужен пуш |
| **cargo test ×3 ОС** | ubuntu: `glib-2.0.pc not found` (pkg-config) — тест-джоба собирает `rimloc-gui` (workspace member), а GTK-стек ставится только в gui-джобе. macos/windows логи: `The frontendDist configuration is set to "../frontend-v2/dist" but this path doesn't exist` → `generate_context!` (`src-tauri/src/main.rs:4092`) паникует: `dist` в `.gitignore` (`gui/tauri-app/frontend-v2/.gitignore:2`), в чекаут его нет. **Ответ на вопрос: НЕТ, фронт перед тестом gui-крейта не собирается** | вариант A (минимальный): `cargo test --workspace --exclude rimloc-gui --all-features` — GUI-крейт уже покрыт gui/frontend джобами; вариант B: собрать `frontend-v2/dist` (npm ci && npm run build) шагом перед тестом + системные GTK-пакеты на ubuntu | рекомендация (меняет политику гейтов — решать владельцу) |
| **Tauri GUI build** (3m2s) | тот же корень: `cargo build` в `src-tauri` без собранного `dist` → panic `generate_context!` (лог идентичен macos-тесту) | добавить шаг `cd gui/tauri-app/frontend-v2 && npm ci && npm run build` перед `cargo build` (или плейсхолдерный `dist/` для CI) | рекомендация |
| **cargo-deny** (46s) | лог: `error[yanked]: detected yanked crate … yoke-derive 0.8.3` в `Cargo.lock:669` | `cargo update -p yoke-derive` | уже применён локально ранее: в `Cargo.lock` сейчас `yoke-derive 0.8.4` (проверено); нужен пуш |
| **public-api diff** (1m25s) | лог: `cargo public-api -p … --diff-git-branch origin/main` → exit 2 + печать Usage — у свежего `cargo-public-api` (ставится без пина: `cargo install cargo-public-api --locked`) флаг `--diff-git-branch` выпилен/переименован | запинить известную рабочую версию (`cargo install cargo-public-api --locked --version <X>`) и/или перейти на новый синтаксис диффа | рекомендация (нужно проверить актуальный синтаксис) |
| **CodeQL** (2s, «108 new alerts incl. 1 critical») | check-run app = `github-advanced-security` — это **default setup**, не workflow-падение: скан выполнен (4 job'а «Analyze (actions/js-ts/python/rust)» — pass), чек красный из-за найденных алертов. При этом `.github/codeql/codeql-config.yml` исключает пути `RimLoc/gui/…`, `icons/`, `compare/`, `mods/` — **префикс `RimLoc/` в этом репо не существует** (репо-корень и есть RimLoc), т.е. конфиг писан под другой лейаут и vendor `wry-0.55.1-noactivate` сканируется как свой код — отсюда массовые алерты | привести `paths-ignore` к реальным путям (`gui/tauri-app/src-tauri/vendor/**`, `**/target/**`, `test/**`, `docs/**`); алерты вне vendor разбирать по `docs/security/CODE_SCANNING_RECONCILIATION.md` (файл существует, 2026-10-05) | рекомендация (правка security-конфига — не «явно безопасная» категория) |
| **deploy (PR preview)** (1–2s) | тело лога: HTTP 404 `BlobNotFound` от Pages-бэкенда, job падает до первого шага (steps:[] в API; старт 10:36:52 после зелёного build 10:36:46). Pages включён (`build_type: workflow`, `has_pages: true`), артефакт `github-pages` загружен. Повторяется от прогона к прогону; в логах шагов нет — первопричина на стороне Pages-сервиса (известный класс гонок preview-деплоев в environment `github-pages` при сериализации через `concurrency: docs-pages`) | пере-прогон для проверки стабильности; если стабильно красный — отключить preview-деплой (артефакт PR-сборки и так доступен) либо развести concurrency-группы preview/prod | рекомендация (нужен контрольный прогон) |
| cargo-semver-checks, mkdocs build, schema drift, frontend (svelte-check+vitest), changelog verify | pass | — | — |

Замечание к скоупу: `ci.yml`/`docs.yml`/`semver.yml` в остальном соответствуют описанию
`docs/development/CI.md` (пиннинги SHA, permissions по job'ам, parked-релизные workflows в
`.github/workflows-disabled/`) — документ CURRENT.

## 8. Wiki — рекомендация

**Клон невозможен: wiki не инициализирована.** `git clone https://github.com/0-danielviktorovich-0/RimLoc.wiki.git`
→ `remote: Repository not found`; `https://github.com/…/RimLoc/wiki` → HTTP 302 на страницу репозитория
(проверено `curl -sL`), при `has_wiki: true` (`gh api repos/…`). Клонировать нечего — это и есть ответ.

**Рекомендация A (deprecate → MkDocs), применяется тривиально:** контента в wiki ноль, весь user-facing
контент уже в MkDocs (EN+RU, strict-сборка зелёная). Когда владелец будет менять публичные настройки:
`has_wiki → false` (сейчас не меняю — публичное состояние по брифу не трогаем). Вариант B («тонкий
навигационный слой») не имеет смысла при пустой wiki — слой было бы не на чем строить.

## 9. Repo meta и `.github/` (только аудит, настройки не менялись)

`gh api repos/0-danielviktorovich-0/RimLoc`:

| Поле | Значение | Рекомендация |
|---|---|---|
| description | «RimLoc — кроссплатформенный инструмент перевода модов для игры RimWorld» | ок; по `DISCOVERABILITY.md §2` EN-канон должен быть английским — рассмотреть EN description (решение владельца; политика языка description там разобрана) |
| homepage | `null` | поставить `https://0-danielviktorovich-0.github.io/RimLoc/` (сайт жив, `has_pages: true`) |
| topics | `[]` (пусто) | заполнить по списку из `DISCOVERABILITY.md §2.3` (там готовый проверенный набор) |
| has_wiki | `true` | после рекомендации A §8 — выключить |
| has_pages / pages | включён, `build_type: workflow`, URL `https://0-danielviktorovich-0.github.io/RimLoc/` | ок |

`.github/`: issue-шаблоны есть (`bug_report.yml`, `feature_request.yml`, `config.yml` — формы),
`PULL_REQUEST_TEMPLATE.md` есть (свод/type/breaking/how-to-test), `CODE_OF_CONDUCT.md` в корне с
русской версией (`docs/readme/ru/CODE_OF_CONDUCT.md`), `SECURITY.md`, `SUPPORT.md`, `FUNDING.yml`,
`dependabot.yml` — комплект полный, пробелов не найдено.

## 10. Сводка применённых правок (всё — коммитит главный поток)

1. `AGENTS.md` — переписан под текущую архитектуру: Rust workspace с инвариантами крейтов,
   framework-neutral `RimLocClient`, React 19 прод-фронт + замороженный Svelte 5 fallback + legacy v1,
   dual-config Tauri 2 (default/react/automation), модель адаптеров (RimWorld первичный, Selfloc живой,
   будущие — capability-driven, не реализуются), безопасный agent-workflow (worktrees, durable
   checkpoints, compaction, запрет STOP при готовой безопасной работе, semantic/background UI-автоматизация,
   «не кради фокус», identity automation ≠ owner artifact, изоляция тест-профилей, read-only исходники).
   Все политики безопасности сохранены и усилены; устаревшее удалено с доказательством
   («Svelte — UI that ships» ← `REPOSITORY_STATE_2026-10.md`, `tauri.react.conf.json`, `CURRENT_SVELTE_BASELINE.md`).
2. `mkdocs.yml` — copyright 2025–2026, emoji-миграция material.extensions, пункт nav `ru/guide/gui-preview.md`
   удалён (страницы в nav-пространстве i18n нет). `mkdocs build --strict` — зелёный.
3. `docs/development/DISCOVERABILITY.md` — убран висячий пункт оглавления на несуществующий §9.
4. `.github/workflows/ci.yml` — actionlint: `~/go/bin` в `GITHUB_PATH` (единственная CI-правка,
   явно безопасная по брифу).

Не делалось (вне лейна/требует владельца): коммиты и пуш, публичные настройки repo, security-конфиг
CodeQL, политика `--exclude rimloc-gui` в cargo test, preview-деплой, EN/RU install/gui-страницы
(рекомендации выше — они меняют содержание user-facing доков).
