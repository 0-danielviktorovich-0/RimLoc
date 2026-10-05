# Repository Guidelines

Переписан аудитом документации 2026-10-05 под фактическое состояние ветки
`feature/ui-r1-convergence` (см. `docs/development/REPOSITORY_STATE_2026-10.md`,
`docs/competitive/RIMLOC_CAPABILITIES_BASELINE.md` — якорь возможностей).

## Project Structure & Module Organization

RimLoc — кроссплатформенный инструмент перевода модов RimWorld (и других игр, через
адаптеры): Rust workspace + Tauri 2 desktop GUI + MkDocs-сайт в `docs/`.

### Rust workspace (`Cargo.toml`, 15 крейтов + GUI)

| Крейт | Ответственность | Инвариант |
|---|---|---|
| `rimloc-domain` | общие типы и JSON-схемы (`RimLocApplication` в canonical.rs) | никакого IO |
| `rimloc-core` | ядро логики перевода | без UI/CLI-специфики |
| `rimloc-parsers-xml` | чтение/парсинг XML RimWorld | только XML |
| `rimloc-export-po` / `export-csv` / `export-xliff` | экспорт форматов | адаптеры форматов + IO |
| `rimloc-import-po` / `import-xliff` | импорт форматов | адаптеры форматов + IO |
| `rimloc-validate` | правила валидации | чистые проверки |
| `rimloc-services` | оркестрация: scan/validate/import/build, project_store v2, ui_catalog, contract, observability | переиспользуется CLI и GUI, файловый IO разрешён |
| `rimloc-config` | конфигурация (`rimloc.toml`) | — |
| `rimloc-llm` | провайдер-шаблоны (UI существует, LLM-вызовы не подключены) | не вызывать платные API без явного разрешения владельца |
| `rimloc-plugin-api` / `rimloc-plugin-jsonftl` | плагины сканирования | — |
| `rimloc-cli` | тонкий командный слой (`src/main.rs`) | никакой бизнес-логики |

Новая логика — в соответствующем крейте (оркестрация — в `rimloc-services`), наружу —
через CLI; не класть ядро в CLI. Контракты вывода (CSV/JSON/PO/XLIFF) стабильны:
ломкое изменение JSON = bump `OUTPUT_SCHEMA_VERSION` + `rimloc-cli schema` + правка docs.
Общие крейты платформо-нейтральны; ОС-специфика — за фичами и вне ядра.

Установки RimWorld и папки модов — **read-only входы**: инструмент читает дерево игры/модов
и пишет только в явно запрошенные пользователем выходные пути (перевод, сохранения проекта,
экспорт). Никогда не добавлять ветку кода, пишущую в исходники игры или модов.

### GUI: Tauri 2 + три фронтенда (`gui/tauri-app/`)

- **`frontend-react/` — продакшен-фронт**: React 19 + Vite 7 + TS strict + Tailwind 4,
  OKLCH-токены Lovable R1. Единый hash-роутер (`src/App.tsx`, маршруты home/projects/checks/
  compare/glossary/export/tools/settings/workspace/existing/selfloc/diagnostics/providers/lm).
- **`frontend-v2/` — замороженный fallback (Svelte 5)**: НЕ удаляется и НЕ редизайнится
  (мандат владельца, `docs/design/CURRENT_SVELTE_BASELINE.md` — «FROZEN LEGACY / FALLBACK /
  REGRESSION ORACLE»). Критические фиксы — только для operability fallback'а; его vitest/svelte-check
  остаются regression-гейтом (`npm run check`, `npm test`), а не дорожкой новых фич.
- **`frontend/` — legacy v1 shell (vanilla JS)**: reference only, не расширять.
- **`src-tauri/` — Rust-бэкенд Tauri** (`rimloc-gui`): тонкий слой над `rimloc-services`.

**Framework-neutral `RimLocClient`** — единственная клиентская поверхность над binding-контрактом
(`gui/tauri-app/frontend-react/src/lib/client/client.ts`; скопирован из замороженного Svelte-клиента,
wire-DTO идентичны): handshake с `ui_contract_version`, мутации с `expectedRevision + sessionEpoch`,
типизированные `stale_revision`/`save_failed` (draft не затирается). UI-фреймворк можно сменить,
не трогая domain/services/backend — канон границы: `docs/architecture/FRONTEND_UI_BOUNDARY.md`.

**Dual-config сборки Tauri** (identity артефактов: automation-сборка ≠ owner-артефакт):
- `tauri.conf.json` — дефолт (Svelte fallback, `frontend-v2/dist`);
- `tauri.react.conf.json` — прод-лайн React (`frontend-react/dist`);
- `tauri.automation.conf.json` — automation-поверхность (+capability `automation-bridge`,
  Runtime Bridge); прод-сборка от неё свободна, `release-guard` сканирует артефакт.
  Cargo-фича `automation-bridge` — optional.

### Адаптеры локализации (мультиигровая граница)

Канон: `docs/architecture/LOCALIZATION_ADAPTERS.md`. Модель адаптера — seam (design +
`RimLocApplication`); **RimWorld — первичная живая реализация**; **Selfloc — вторая живая**
(«RimLoc переводит RimLoc»: каталог приложения как обычный инвентарь, `selfloc_catalog.rs`,
`docs/development/SELFLOC_BRIDGE.md`). Спекулятивные адаптеры (Minecraft/Terraria/Paradox/Unity)
**не реализуются** — документ проверяет лишь, что граница не потребует их знания сегодня;
будущие адаптеры — capability-driven, добавляются решением владельца.

## Build, Test, and Development Commands

Дисковое ограничение рабочей машины владельца: внутренний диск мал, **любые `cargo build/test/check/clippy`
запускать с `CARGO_TARGET_DIR=/Volumes/Portable-SSD/caches/targets/rimloc` в строке команды**.
На btrfs-SSD — дополнительно `CARGO_INCREMENTAL=0` (локи не поддерживаются).

- `CARGO_TARGET_DIR=… cargo build --workspace` — все крейты и межкрейтные интерфейсы.
- `CARGO_TARGET_DIR=… cargo run -p rimloc-cli -- scan --root test/TestMod` — сквозная проверка CLI.
- `CARGO_TARGET_DIR=… cargo test --workspace` — юнит- и интеграционные сьюты (`-- --nocapture` для stdout).
  Известный подводный камень: workspace включает `rimloc-gui`, чей `generate_context!` требует
  существования `frontendDist` (см. диагноз CI в `docs/development/DOCUMENTATION_AUDIT_2026-10.md`).
- `cargo fmt --all --check` и `CARGO_TARGET_DIR=… cargo clippy --workspace --all-targets --all-features -- -D warnings` — перед ревью.
- `mkdocs serve` (из venv, `requirements-docs.txt`) — превью сайта; строгий гейт:
  `SITE_URL=https://0-danielviktorovich-0.github.io/RimLoc/ mkdocs build --strict`.
- React-лайн: `cd gui/tauri-app/frontend-react && npm ci && npm run build` (tsc strict + Vite).

## Coding Style & Naming Conventions

Rust — дефолтный rustfmt (4 пробела); модули/функции `snake_case`, типы `PascalCase`,
константы `SCREAMING_SNAKE_CASE`; CLI-флаги kebab-case. FTL-ключи в `crates/rimloc-cli/i18n` —
строчные с дефисами, EN первым. TypeScript — strict; компоненты React-функции; стиль — OKLCH-токены,
никаких сырых хексов цвета в новых экранах.

## Testing Guidelines

Юнит-тесты — рядом с кодом; интеграционные CLI — `crates/rimloc-cli/tests` (хелперы `helpers.rs`);
фикстуры XML/PO — в `test/`; временные файлы — через `tempfile`. Гейт-функциональность —
`cargo test --features <feature>`. Новый флаг/подкоманда → интеграционный тест + docs.

### Testing policy (mandatory)
- После любого изменения (код или docs) — локальные проверки перед коммитом:
  build, test, fmt, clippy (см. команды выше); отчёт о результатах — в итоговом ответе.
- Тронут `docs/` — `mkdocs build --strict` (с `SITE_URL=…`) локально.
- Тронут `gui/tauri-app/frontend-v2/` — `npm run check` (svelte-check) + `npm test` (Vitest) в нём;
  пересборка `npm run build` перед `cargo tauri dev` (Tauri обслуживает `dist`, не dev-сервер).
- Тронут `gui/tauri-app/frontend-react/` — `npm run build` зелёный; изменения wire-контракта
  согласованы с `RimLocClient` и `frontend-v2`-оракулом (или осознанно разошлись с пометкой).
- Изменил i18n-ключи CLI — `cargo test --package rimloc-cli -- tests_i18n`.
- Изменил CLI-флаги/поведение — обнови интеграционные тесты и прогони всю сьюту.

## Documentation Workflow

- Сайт: `docs/en/` (канон) + `docs/ru/` — структурно парные; EN/RU правки — одним коммитом.
- Внутренние рабочие документы (мандаты, кампан-логи, аудиты) живут в `docs/development`,
  `docs/design`, `docs/competitive`, `docs/campaign`, `docs/security`, `docs/architecture` —
  они НЕ в MkDocs-навигации; датированные логи не переписывают, новое состояние — новым файлом
  с датой (см. `STATE_CORRECTION_2026-10-05.md` как образец).
- Драфты вне навигации — `exclude_docs` в `mkdocs.yml` либо вне `docs/`.
- Якоря на кириллические заголовки не работают (дефолтный slugify их выпиливает) —
  в ссылках на RU-страницы использовать заголовки без кириллицы или без якоря.

## Release Workflow

- Версии руками не бампать; тулза — `release-plz`/`cargo-release` + GitHub Actions
  (`release.toml`, `release-plz.toml`). Workflows публикации — **parked по решению владельца**
  (origin/main `9591f4f`); новые теги/релизы — только по явному одобрению владельца.
- Перед тегом: `CHANGELOG.md` (`Unreleased`), build/test/lint зелёные, docs EN/RU синхронны.
- Артефакты/подписи/SBOM — в CI (`docs/en/dev/index.md`).

## Automation Rules (agents)

- Минимальные диффы строго в скоупе задачи; никаких drive-by рефакторов и массовых переименований.
- Историю не переписывать; no-revert policy ниже. Push/теги/релизы — никогда без явного разрешения.
- Всегда прогонять и отчётить build/test/fmt/clippy после изменений.
- Язык: если владелец пишет по-русски — отвечать по-русски.
- Деструктивные действия (удаления/переносы/формат-свипы) — только спросив.
- Финальный шаг — коммит через `scripts/agent-commit.sh`; если харнесс не даёт коммитить —
  точное сообщение и список файлов в финальном ответе, владелец коммитит сам.

### Agent workflow (безопасный конвейер, 2026-10)

Проверенная практика кампаний UI R1 / TM live / competitive audit — держать её в этом виде:

- **Субагенты и worktrees.** Параллельные лейны — по git-worktree на лейн
  (`ba-main` — интеграция, `wt-ui-r1`, `wt-tm-live`; полный список — `git worktree list`).
  Субагент получает узкий мандат и возвращает артефакт+отчёт; главный контекст не засоряется.
- **Durable checkpoints.** Состояние сессии фиксируется на диске по ходу работы
  (`docs/design/UI_R1_CHECKPOINT.md`, `MORNING_CHECKPOINT_*`, `docs/campaign/STATE_CORRECTION_*`):
  HEAD, ветка, что сделано, что дальше — чтобы любой свежий контекст поднялся с чекпоинта.
- **Context compaction.** При заполнении контекста — чекпоинт-документ ДО компакции;
  в новые контексты грузить чекпоинт, а не всю историю (датированные логи читать по требованию).
- **Запрет STOP при готовой безопасной работе.** Если задача выполняется локально, безопасно
  и мандат выдан — не останавливаться на полупути «спросить разрешения» на следующий
  безопасный шаг; эскалация — только для деструктивного, публичного или противоречий в инструкциях.
- **Semantic/background UI-автоматизация.** GUI-верификация — семантическая (accessibility tree,
  WDIO embedded spike — `docs/development/testing/WDIO_EMBEDDED_SPIKE.md`, T2A Playwright), а не
  пиксельная; запуски — фоновые, без кражи фокуса (`docs/development/FRONTIER_QUERY_FOCUS_FREE_MACOS_AUTOMATION.md`).
- **Правило владельца «не кради фокус».** Окно/приложение, с которым работает владелец, —
  неприкосновенно: UI-автоматизация не должна перехватывать фокус (vendored wry без
  NSApplication::activate, `focus: false` в окне, background-запуски). Нарушение = красный флаг кампании.
- **Identity артефактов: automation ≠ owner artifact.** Артефакты, собранные автоматизацией,
  помечены identity (source SHA, flavor, automation flag) и не выдаются за owner-сборку;
  прод-сборка без automation-поверхности (`release-guard` сканирует артефакт).
- **Изоляция тест-профилей.** UI-автоматизация и soak-прогоны — в отдельных профилях/песочницах
  (`testlab/`), никогда в рабочем профиле владельца и никогда в прод-установке RimWorld.
- **Read-only исходники модов.** Повтор канона: игра/моды — только чтение; запись — только
  в выходные пути проекта.

### Secrets & External Services (GH_TOKEN)
- Разрешено: когда владелец явно передал `GH_TOKEN`/`GITHUB_TOKEN` и попросил — GitHub-операции
  (API, приватные клоны, fetching releases). Передача — только через переменную окружения
  (`export GH_TOKEN=…`); не хардкодить, не писать в файлы под git, не печатать в логи, не эхать env.
- Скоуп: токен строго на запрошенную операцию; не публиковать, не тегать, не менять состояние
  GitHub (releases, labels, settings) без явной просьбы. После — `unset GH_TOKEN`.
- Платные API (LLM-провайдеры) не вызывать без явного разрешения владельца; `rimloc-llm`
  остаётся шаблонным слоем.

### GUI Dependencies (exception for gui/)
- Для качественного долгосрочного Tauri GUI зависимости в `gui/` можно добавлять по необходимости
  (фронт-либы, Tauri-плагины, ZIP/HTTP). Исключение НЕ распространяется на крейты `crates/` — там держим
  компактно. Бизнес-логику — в `rimloc-services`, чтобы не дублировать.

### Auto-commit workflow (mandatory for agents)
- Открыть сессию задачи: `scripts/agent-begin.sh --session <chat-id> [--type chore --scope cli --subject "…" -b "…"]`.
- По ходу: `scripts/agent-context.sh --session <chat-id> --add-file <path>` (повторяемо);
  уточнение сообщения — `--subject "…" -b "…"`.
- Точные ханки в общих файлах: `scripts/agent-mark-change.sh --session <chat-id> begin|end --file <path>`
  — на коммит уйдут только эти ханки, чужие правки в том же файле не попадут.
- Финиш (обязательный шаг): `scripts/agent-commit.sh --session <chat-id>` — stage только файлов
  сессии, пересечение с allowlist, без случайных захватов. `--dry-run` — превью.
- Без `--session` — один глобальный baseline (`.git/agent-baseline.txt`); предпочтительны сессии.
- Хуки включены: `scripts/setup-git-hooks.sh` один раз на клон.
- Финальный гард: `scripts/agent-ensure-commit.sh` (с `--session <id> --auto` при необходимости).

```
export AGENT_SESSION=<chat-id>
scripts/agent-begin.sh --subject "…" --type fix --scope core
# …work…
scripts/agent-commit.sh  # Mandatory finish step
```

## For agents: Changelog & Versioning
- Один кураторский `CHANGELOG.md` (Keep a Changelog + SemVer). Пользовательские изменения —
  под `Unreleased`, секции `Added/Changed/Fixed/Docs/Internal`; формат `- [scope] short (#PR)`,
  без точки в конце. Прошлые записи не переписывать. Релиз = перенос `Unreleased` в `## [X.Y.Z] - YYYY-MM-DD`
  + compare-ссылки внизу.
- Внутренние-only изменения — лейбл PR `internal-only` (обходит changelog-CI).
- SemVer: библиотеки строго; CLI — возможны pre-releases (`-alpha.N`, `-beta.N`). Версии крейтов
  независимы. Агенты не бампают версии, не тегают и не публикуют без явной просьбы.
- Политика коммит-скоупа: рекомендованные scope — `repo, cli, core, parsers-xml, export-csv,
  export-po, import-po, validate, docs, ci, release, tests`.

### MSRV and SemVer checks
- MSRV: Rust `1.89` по workspace (`rust-version` в манифестах); **тестированный тулчейн — `1.96.0`**.
  MSRV поднимать только в мажоре и после свежего аудита фич/зависимостей. Если инструмент
  показывает `1.70` — он читал устаревшие метаданные (до 2026-09-27 так было, это ложь).
- Библиотеки: CI гоняет `cargo-semver-checks` для опубликованных крейтов; ломкое API — `major`.
- CLI-вывод — контракт: добавление поля minor; удаление/переименование major. JSON несёт
  `schema_version`; PO-заголовок — `X-RimLoc-Schema`.

## CLI Conventions
- Подкоманды/флаги kebab-case; хелпы в FTL, локализованы.
- Никаких inline пользовательских строк: `tr!`/FTL; логи — только `tracing`.
- JSON-вывод стабилен; при изменении схем/флагов обновляй интеграционные тесты.

## Localization Workflow Notes
- Переводы CLI — `i18n/<lang>/rimloc.ftl`, встроены на этапе сборки. EN — источник правды;
  остальные локали зеркалируют ключи; проверка — `cargo test --package rimloc-cli -- tests_i18n`.
- Новая локаль: `crates/rimloc-cli/i18n/<lang>/` — `build.rs` находит автоматически.

## Commit & Pull Request Guidelines
Шаблон — `.gitmessage.txt`: `type(scope): summary` ≤72 символов (`feat`, `fix`, `docs`, `chore`, …).
Для нетривиальных изменений — тело буллетами `- ` (что/почему/влияние). Без голых subjects вида
`tests: update snapshot`. Коммит-сообщения на английском; русский референс —
`docs/readme/ru/gitmessage.txt`. PR: краткое summary, скоуп, linked issues, инструкция проверки,
CLI-вывод/скриншоты при изменении поведения; CI зелёный до ревью.

### Git hooks
- `scripts/setup-git-hooks.sh` раз на клон (ставит `core.hooksPath` = `.githooks`).
- `commit-msg` проверяет паттерн subject, пустую строку, минимум один `- ` буллет в теле.

### Commit scope policy (mandatory)
- Коммитить только файлы, правленные сознательно в рамках задачи; никаких `git add -A`/`git add .` —
  в рабочем дереве живут правки других сессий.
- Без drive-by рефакторов/ренеймов/массового форматирования; `cargo fmt` гонять, но коммитить
  только свои файлы. Репо-вайд ретабуляция — отдельный PR.
- Не бампать версии, не двигать модули, не трогать генераты вне задачи. Правило — и для людей,
  и для агентов.

### No-revert policy (mandatory)
- Не ревертить и не отбрасывать изменения без явного согласия мейнтейнера/автора.
- Исключения: спасение сломанной сборки/теста либо необходимость для текущего фикса — с
  обоснованием в теле коммита.
- Чужие незакоммиченные правки: спросить (keep/commit/drop), молча не трогать.
- Нужен реверт — отдельный коммит со ссылкой на оригинал (`revert: <hash> <subject>`), без смешивания.

## GUI/CLI Parity and i18n

- CLI и GUI в локстепе: каждая CLI-команда/флаг экспонируется в GUI с той же семантикой.
- Никаких хардкод-строк UI: ключи в `frontend-react/src/lib/i18n/` (EN+RU), RU — дефолтная локаль,
  fallback locale → `en` → key. Для Svelte-fallback — свой `frontend-v2/src/i18n/` (заморожен,
  меняем только при operability-фиксах).
- Новый бэкенд-команд — регистрация в Tauri `invoke_handler`, permissions
  (`src-tauri/permissions/allow-commands.json`), проводка контрола в GUI.
- Mock/demo данные (`frontend-react` mock-транспорт / `frontend-v2/src/lib/mock/`) питают demo-проект
  и onboarding: экран, выглядящий как живой, либо говорит с реальным бэкендом, либо явно помечен
  (capability report, «not wired yet»/demo). Никогда не выдавать mock за живую функциональность.
- API проектируется эргономично для UI: структурная группа опций CLI — один request-struct в Tauri,
  GUI пробрасывает без трансформаций.

Review checklist for contributors:

- [ ] CLI: команда + аргументы реализованы и задокументированы
- [ ] Backend: Tauri-команда зеркалит CLI-типы и поля
- [ ] Permissions обновлены; capability-модель не тронута без необходимости
- [ ] GUI: компоненты обновлены, i18n-ключи EN+RU
- [ ] Mock-экраны capability-gated или явно помечены как demo
- [ ] Логи/прогресс-события подключены к progress panel
- [ ] Frontend-проверки: React — `npm run build`; при трогании fallback — `npm run check` + `npm test` в `frontend-v2`; затем `npm run build` + `cargo tauri dev` чисто
