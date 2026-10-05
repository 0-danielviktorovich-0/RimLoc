# Contributing to RimLoc

Обновлено: 2026-10-05. Спасибо за интерес к проекту — здесь описано, как
настроить окружение, следовать конвенциям и прислать изменения, которые легко
отревьюить.

## С чего начать

1. Сделайте форк и заведите feature-ветку от `main` (например,
   `feat/po-import-dedupe`).
2. Поставьте стабильный Rust через [rustup](https://rustup.rs).
3. Соберите и прогоните тесты:
   ```bash
   cargo build --workspace
   cargo test --workspace
   ```
4. Перед каждым коммитом — форматирование и линт (как в CI):
   ```bash
   cargo fmt
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   ```
5. Откройте **Draft PR** рано, дозаполняйте по мере готовности; когда всё
   зелёное — переводите в ready for review.

## Структура репозитория

- `crates/` — Cargo-workspace:
  - `rimloc-domain` — общие типы и JSON-схемы; **без IO**;
  - `rimloc-core` — ядро логики перевода; без специфики UI/CLI;
  - `rimloc-parsers-xml` — только чтение и разбор XML модов;
  - `rimloc-export-csv|po|xliff`, `rimloc-import-po|xliff` — форматные
    адаптеры;
  - `rimloc-validate` — правила проверки; `rimloc-services` — оркестрация,
    общая для CLI и GUI (здесь живёт файловый IO);
  - `rimloc-config` — конфигурация; `rimloc-llm` — провайдеры LLM;
    `rimloc-plugin-api` / `rimloc-plugin-jsonftl` — контракты плагинов;
  - `rimloc-cli` — тонкий командный слой, без бизнес-логики.
- `gui/tauri-app/src-tauri/` — Rust-бэкенд десктоп-GUI (Tauri 2).
- `gui/tauri-app/frontend-react/` — **актуальный фронтенд** (React 19 + Vite +
  TypeScript strict + Tailwind).
- `gui/tauri-app/frontend-v2/` — Svelte-фронтенд: **заморожен**, держится как
  fallback/regression-оракул; багфиксы по согласованию, новые фичи — не сюда.
- `gui/tauri-app/frontend/` — legacy-оболочка v1, оставлена как референс, не
  расширять.
- `test/` — фикстуры для интеграционных тестов; `testlab/` — dev-лаборатория
  (синтетические фикстуры, UI-автоматизация), это не пользовательская
  документация.
- `docs/` — исходники сайта (MkDocs).

## Границы сервисов и фронтенд-контракт

- Логика живёт в крейтах (`rimloc-services` для оркестрации), CLI и GUI — тонкие
  потребители. Не тащите бизнес-логику в `rimloc-cli` или в компоненты
  фронтенда.
- Фронтенды говорят с Rust-бэкендом через **`RimLocClient`** —
  framework-neutral слой контракта над Tauri IPC
  (`gui/tauri-app/frontend-react/src/lib/client/`). Правила: handshake
  проверяет версию контракта; каждая мутирующая операция несёт
  `expectedRevision` + `sessionEpoch`; неудачное сохранение никогда не
  затирает черновик пользователя (typed `save_failed`/`stale_revision`).
  Новый вызов IPC сначала попадает в контракт и `RimLocClient`, потом в UI.
- Римворк-специфика — только в адаптерном слое; общий редактор/валидатор
  работают по канонической модели из `rimloc-domain`.
- Инвариант IO: игра и папки модов — **только чтение**; запись — исключительно
  в явно запрошенные пользователем выходные пути.

## Вклад во фронтенд

- Среда: Node.js 20+; фронт React — `gui/tauri-app/frontend-react`
  (`npm ci`, `npm run build` = `tsc -b && vite build`).
- Локальный запуск GUI: `cargo tauri dev` из `gui/tauri-app/src-tauri`
  (дефолтная конфигурация собирает Svelte-fallback; React-вариант —
  `cargo tauri dev --config tauri.react.conf.json`).
- E2E-джорни React — спеки `react-*.spec.ts` в
  `gui/tauri-app/frontend-v2/e2e/wdio-spike/` под wdio-харнессом
  (`testlab/ui_automation/`); фикстуры и правила честного LIVE/MOCK — в
  `docs/design/UI_R1_FINAL_REPORT.md`.
- Правило моков: мок-транспорт не бандлится в прод; ни один мок не должен
  выглядеть как живой режим.

## Адаптеры и мультиигровость

- **RimWorld — первичный адаптер.** Его знания (Keyed/DefInjected, версионные
  папки, LoadFolders, плейсхолдеры) живут в адаптерном слое, идентификатор —
  `rimworld` в `rimloc-domain` (`canonical::adapter_ids`).
- Интерфейс адаптера — каноническая модель проекта
  (`rimloc-domain::canonical::AdapterIdentity`): адаптер подписывает проект
  своим id и версиями схемы; чужой/неизвестный id не открывается молча, а
  падает с диагностикой.
- Новый игровой адаптер (не RimWorld) — сначала обсуждение в issue: каноническая
  модель должна остаться общей, а ядро (редактор, глоссарий, валидация) — не
  обрастать game-specific ветками.

## Коммит-сообщения

В репозитории включён хук `commit-msg` (`.githooks/`, `core.hooksPath`), он
принуждает Conventional Commits:

- заголовок `type(scope): summary` — **до 72 символов**;
- типы: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `ci`, `build`,
  `perf`, `revert`;
- пустая строка после заголовка;
- тело — буллетами, каждый пункт начинается с `- ` (что изменилось и почему);
- для `chore(release):` хук дополнительно требует строку порядка публикации
  `Run publish in order: core -> parsers -> ...`.

Пример:

```
fix(po): keep placeholder order when importing CJK translations

- compare placeholder sets, not sequences, in validate-po
- add reordered-placeholders fixture for ja/zh layouts
```

Только свои файлы в коммит: не включайте попутные переименования, массовое
форматирование или чужие незакоммиченные изменения. Репо-wide reformat —
отдельный PR.

## Pull Request

1. Feature-ветка → **Draft PR** в `main`.
2. CI на PR гоняет: rustfmt+clippy, `cargo test` на Ubuntu/macOS/Windows,
   сборку Tauri GUI, проверку фронтенда, `cargo-deny`, сверку JSON-схем.
   Красный чек = PR не мержится; push-триггеры CI выключены намеренно —
   проверки считаются на PR.
3. В описании PR: что и зачем изменилось, как проверяли (команды, вывод,
   скриншоты UI-правок), затронуты ли доки/i18n-ключи, известные ограничения.
4. Ребейз на свежий `main` перед ревью; обсуждения — по существу и уважительно.

## Тесты

- **Rust**: юнит-тесты рядом с кодом; интеграционные — `crates/rimloc-cli/tests`
  (хелперы в `helpers.rs`), временные пути через `tempfile`, долгоживущие
  фикстуры — в `test/`. Прогон всего: `cargo test --workspace`.
- **Svelte-fallback**: в `gui/tauri-app/frontend-v2` — `npm run check`
  (svelte-check) и `npm test` (Vitest).
- **React**: проверка типов и сборка — `npm run build` в
  `frontend-react`; e2e-джорни — wdio-спеки (см. «Вклад во фронтенд»).
- Меняли JSON-схемы вывода — поднимите `OUTPUT_SCHEMA_VERSION`, перегенерируйте
  схемы (`rimloc-cli schema`) и обновите доки; CI сверяет, что схемы свежие.

## Документация

- Сайт собирается MkDocs (конфиг `mkdocs.yml`): `python -m venv .venv`,
  `pip install -r requirements-docs.txt`, затем `mkdocs serve` для локального
  превью.
- Английские исходники — `docs/en/`, русские — `docs/ru/`; структура папок
  зеркалится, при добавлении страницы заведите оба языка.
- К изменению кода, меняющему поведение CLI/GUI, прилагается правка
  соответствующих страниц `docs/` и i18n-ключей справки (`crates/rimloc-cli/i18n/`;
  английский каталог — источник, русский зеркалится).

## Ожидания по безопасности

- **Никаких секретов в репозитории** — ни токенов, ни ключей, ни
  `.env`-файлов. CI-секреты — только через `secrets.*`; в тестах — заведомо
  синтетические фикстуры (и помеченные как таковые).
- Ключи LLM/провайдеров пользовательские хранятся в системном keychain
  (сервис `rimloc-llm`), не в конфигах и не в переменных окружения кода.
- Уязвимости — только приватно через
  [SECURITY.md](SECURITY.md) (GitHub Private Vulnerability Reporting), не в
  публичных issues.
- Зависимости проверяются `cargo-deny` (advisories/licenses/bans); новый RUSTSEC
  ignore в `deny.toml` — только с обоснованием в комментарии.

## Issues и помощь

- Баг/фича — GitHub Issues с шагами, ожиданием и фактом, окружением
  (`rimloc-cli --version`, ОС). Чек-лист — `docs/en/community/issues.md`.
- Вопросы локализации самого CLI — указывайте локаль и примеры строк.
- Дискуссии и ревью ведутся в PR/Issues — этого канала пока достаточно.

Ещё раз спасибо — каждый аккуратный PR ускоряет RimLoc к бете.
