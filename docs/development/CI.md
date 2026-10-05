# CI — дизайн production-конвейера

Дата: 2026-09-27 · Обновлено: 2026-10-06 · Статус: **живой дизайн + имплементация**. Тяжёлый core CI по-прежнему PR/manual; точечные push-триггеры разрешены только там, где они реально нужны (GitHub Pages после docs-merge и Codecov baseline после code push).
Связанное: [BRANCH_STRATEGY.md](BRANCH_STRATEGY.md) (гейты на PR, trunk-based), [VERSIONING.md](VERSIONING.md) (каноническая схема версий), [SECURITY_AUDIT.md](SECURITY_AUDIT.md).

## 1. Аудит до редизайна (evidence: 8 активных + 2 запаркованных)

| Workflow | Триггеры | Находка аудита | Вердикт |
|----------|----------|----------------|---------|
| `ci.yml` | PR (без фильтра ветки) + dispatch + **schedule еженедельный** | нет frontend-гейта (svelte-check/vitest); schedule вне триггерной политики; actions не запинены; permissions по умолчанию | переписан |
| `docs.yml` | PR + dispatch | top-level `pages: write, id-token: write` доступен и build-job'у; deploy-prod висел на push (мёртвый код после снятия push-триггеров); глобальный `concurrency: pages` | переписан |
| `publish.yml` | dispatch | **токен crates.io через workflow_dispatch input** (inputs видны в UI/логах); `|| true` на 7 из 8 позиций публикации — пачка «зеленела» наполовину опубликованной; лишний `packages: write` | переписан |
| `release-plz.yml` | dispatch + **PR** | release_pr срабатывал на каждый PR (диф не от main, шум, минуты); release-конкурентность с `cancel-in-progress: true` (риск оборвать публикацию) | переписан |
| `semver.yml` | PR→main + dispatch | чист; нет пинов/конкурентности | дополнен |
| `public-api.yml` | PR→main | информационный, никогда не красный (`\|\| true` и на fetch, и на сам диф); дублирует сюжет semver | **удалён**, job переехал в `semver.yml` |
| `schema-check.yml` | PR + dispatch | чист; отдельный файл ради одного job'а | **удалён**, job переехал в `ci.yml` |
| `changelog-check.yml` | PR→main | чист | запинен |
| `release-dev.yml` (parked) | dispatch | бинарные dev-релизы; при разморозке потребует пиннинга и ревизии | припаркован, не тронут |
| `release-dev-auto.yml` (parked) | dispatch (develop) | то же; ссылается на ветку develop, которой по BRANCH_STRATEGY нет | припаркован, не тронут |

Итог: 8 активных → **6** (`ci`, `changelog-check`, `semver`, `docs`, `publish`, `release-plz`). История удалённых файлов сохранена в git.

## 2. Рамка владельца (не пересматривалась)

- **Тяжёлый core CI не запускается на каждый push**: `ci.yml`, semver и release-пути остаются PR/manual. После ревизии 2026-10 точечный `push` используется только для дешёвых/необходимых post-merge действий: публикация MkDocs Pages при изменении docs на `main` и запись Codecov baseline на `main`; PR coverage при этом работает и для `main`, и для активной convergence-ветки без отдельного duplicate push-run.
- **Релизные workflow припаркованы** (9591f4f) — `.github/workflows-disabled/` не тронут; при разморозке см. §7.

## 3. Полномочия релиза (кто что публикует)

| Субъект | Действие | Статус |
|---------|----------|--------|
| `release-plz.yml` → jobs `release_pr` / `release` | версии + теги + публикация крейсов в crates.io | **единственный автоматический владелец**. `workflow_dispatch` требует явный `mode`: `release-pr` создаёт/обновляет Release PR; после его merge отдельный dispatch с `mode=release` тегает/публикует. Один запуск физически не может сделать оба шага. |
| `publish.yml` | повторная публикация крейсов | **только ручной recovery** (например, crates.io 429 прервал пачку). Токен — только repo secret; `dry_run=true` по умолчанию; для реальной публикации дополнительно требуется literal `confirm_publish=PUBLISH`; retries/timeouts ограничены. Когда release-plz обкатается на 2-3 релизах — кандидат на удаление. |
| `release-dev*.yml` (parked) | GitHub Releases с бинарниками CLI/GUI | разморозка — отдельное решение владельца; до неё не трогаем |
| `docs.yml` → job `deploy-prod` | публикация сайта на Pages | strict build на docs PR; после docs-изменений в `main` Pages деплоится автоматически, manual dispatch остаётся для контролируемого redeploy; preview opt-in. |

## 4. Целевая архитектура

| Workflow | Триггеры | Jobs | Роль |
|----------|----------|------|------|
| `ci.yml` | PR→main, dispatch | `lint` (fmt+clippy), `test` (ubuntu/macos/windows), `gui` (Tauri build), `frontend` (svelte-check + vitest), `deny` (cargo-deny), `schema` (drift), `workflows-lint` (actionlint) | единый quality gate |
| `semver.yml` | PR→main, dispatch | `semver` (cargo-semver-checks, гейт), `public-api` (информационный диф) | стабильность API |
| `changelog-check.yml` | PR→main | `verify` | CHANGELOG сопровождает пользовательские изменения (лейбл `internal-only` выключает) |
| `workflow-lint.yml` | workflow-file PR→main/convergence + dispatch | `actionlint` | быстрый синтаксис/semantics-гейт GitHub Actions отдельно от тяжёлого CI |
| `docs.yml` | docs PR→main + docs push→main + dispatch | `build`, `deploy-prod`, optional `deploy-preview` | strict MkDocs + Pages; docs-only PR не гоняет тяжёлый Rust matrix |
| `coverage.yml` | relevant code PR→main/convergence + push→main + dispatch | `config`, `rust`, `gui-rust` | cargo-llvm-cov + Codecov OIDC, два coverage-family report; Components делят один отчёт по подсистемам |
| `publish.yml` | dispatch | `publish` | recovery-публикация crates.io |
| `release-plz.yml` | dispatch | `release_pr`, `release` | версионирование и релизы |

Покрытие мандата: rustfmt/clippy/tests — `ci.lint/test`; React typecheck/build — `ci.frontend-react`; frozen Svelte regression — `ci.frontend`; dependency/security audit — `ci.deny`; coverage — отдельный `coverage.yml` (Rust workspace + Tauri Rust, Codecov Components/Flags, OIDC); actionlint — `ci.workflows-lint`; CodeQL — default setup репозитория с path-конфигом; mkdocs — strict `docs.build`.

## 5. Правила безопасности, общие для всех workflow

1. **Пин сторонних actions на полные SHA.** Комментарий рядом с SHA — какой тег/ветка был запинен и когда. Пины сняты с той же ревизии, на которую резолвились старые `@vX`-ссылки — поведение не изменилось.

   | Action | Было | Запинено (2026-09-27) |
   |--------|------|----------------------|
   | `actions/checkout` | `v7` | `3d3c42e5aac5ba805825da76410c181273ba90b1` |
   | `actions/setup-python` | `v7` | `5fda3b95a4ea91299a34e894583c3862153e4b97` |
   | `actions/setup-node` | (новый) `v6` | `249970729cb0ef3589644e2896645e5dc5ba9c38` |
   | `actions/cache` | `v6` | `55cc8345863c7cc4c66a329aec7e433d2d1c52a9` |
   | `actions/upload-pages-artifact` | `v5` | `fc324d3547104276b827a68afc52ff2a11cc49c9` |
   | `actions/deploy-pages` | `v5` | `368f82528645a54fb793d4d04e342629a3f51346` |
   | `dtolnay/rust-toolchain` | `stable` | `6bed0761d98439e5a578e2877258200ad565ba87` |
   | `Swatinem/rust-cache` | `v2` | `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` |
   | `EmbarkStudios/cargo-deny-action` | `v2` | `3c6349835b2b7b196a839186cb8b78e02f7b5f25` |
   | `codecov/codecov-action` | `v7.1.1` | `303a32d7a59b442fa8d48b6a1cc6825c09c847a5` |
   | `taiki-e/install-action` | `v2.87.25` | `183e4297cca2404691e9380e1307288dced5c82a` |
   | `MarcoIeni/release-plz-action` | `v0.5` | `b8d6b54b02889ff2ae2bb82e8b57c3a8fc1683a5` |

   Замечание про `dtolnay/rust-toolchain`: запинен код экшена, но НЕ тулчейн — действие вызывает `rustup`, и `stable` резолвится на стороне GitHub в момент прогона, так что новые стабильные версии Rust продолжают приходить без правки workflow.

2. **Минимальные permissions.** На верхнем уровне каждого workflow `permissions: {}`, у каждого job'а — свой минимум (`contents: read` почти везде; `pages: write, id-token: write` — только у deploy-job'ов docs; `contents: write, pull-requests: write` — только у release-plz).

3. **Никаких `|| true` на обязательных шагах.** Убраны из `publish.yml` (позиции пачки), `public-api` (диф; остаётся информационным, но ошибка инструмента теперь видна). Легитимные `|| true`-паттерны, оставшиеся в запаркованных файлах (`find ... | head` при упаковке), — предмет правки при разморозке.

4. **Секреты — только через `secrets.*`.** Input `token` у `publish.yml` удалён (inputs читаемы в UI и логах). Пустой `CARGO_REGISTRY_TOKEN` при `dry_run=false` теперь явная ошибка с понятным сообщением, а не невнятный отказ cargo.

5. **Конкурентность и timeouts.** Быстрые гейты (`ci`, `semver`, `docs.build`, coverage) отменяют stale-run по ref; необратимые операции (`crates-publish`, `docs-pages`, `release-plz-release`) не отменяются. Для jobs выставлены разумные `timeout-minutes`, чтобы зависший runner не жил бесконечно.

## 6. Осознанно отложенное (позже, по решению владельца)

- **React coverage.** Rust coverage уже имплементирован отдельным `coverage.yml` и реально загружается в Codecov (PR #62; validator + workspace + Tauri Rust jobs прошли). React R1 пока не имеет стабильного unit/component coverage runner в `package.json`, поэтому фальшивый 0% не публикуется. Контракт на будущее: `npm run test:coverage` → `frontend-react/coverage/lcov.info` → отдельный Codecov flag/component.
- **cargo-audit не добавлен**: `cargo-deny` (job `deny`, конфиг `deny.toml`) уже проверяет RustSec-advизории по всем lockfile'ам (включая `gui/tauri-app/src-tauri`), плюс лицензии и bans. Второй сканер тех же advisory-DB — дубль без новой гарантии.
- **CodeQL** работает через default setup репозитория («dynamic») с конфигом `.github/codeql/codeql-config.yml` (`security-extended`). Workflow-файл `codeql.yml` НЕ добавлять: GitHub запрещает default setup и workflow одновременно. При переходе на advanced setup — мигрировать явно, не дублировать.
- **Schedule-прогон** (еженедельная проверка дрейфа зависимостей/clippy) снят вместе с push-триггерами. Вернуть можно сниппетом в `ci.yml`:
  ```yaml
  schedule:
    - cron: "0 4 * * 1"  # понедельник 04:00 UTC
  ```

## 7. При разморозке release-dev*

Перед возвратом файлов в `workflows/`: (1) запинить `softprops/action-gh-release` и все actions (сейчас там bare `@v3`/`@stable`); (2) заменить `if sudo apt-get install ...; then ... else ...` и `find | head || true` на строгие проверки — это release-артефакты, тихая деградация здесь дороже всего; (3) решить судьбу `release-dev-auto.yml` — он ссылается на ветку `develop`, которой по BRANCH_STRATEGY.md нет.

## 8. Локальная валидация

`actionlint` (1.7.12) прогоняется и локально, и в отдельном `workflow-lint.yml` (устанавливается через `go install ...@v1.7.12` — верификация модуля через sum.golang.org, без стороннего action). Локально:

```bash
brew install actionlint   # или go install github.com/rhysd/actionlint/cmd/actionlint@v1.7.12
actionlint -color .github/workflows/*.yml
```

## 9. Рекомендации владельцу (в этой волне не сделано)

1. **Dependabot** уже расширен: Rust + React npm + frozen Svelte npm + pip + GitHub Actions; routine updates сгруппированы и cadence снижена, чтобы не плодить десятки PR.
2. **Branch protection / rulesets:** REST `rulesets` сейчас возвращает пустой список; branch-protection endpoint недоступен GitHub App без admin permission. После стабилизации baseline стоит включить required checks `CI / lint`, `CI / test (ubuntu-latest)`, `CI / deny`, `CI / schema`, `API stability / cargo-semver-checks (gating)`, `changelog / verify` (минимальный набор; matrix-OS и GUI — по вкусу).
3. **Codecov gate:** пока project/patch statuses informational. После нескольких репрезентативных PR — включить blocking `target: auto` с малым допустимым regression threshold и затем разумный patch target; не ставить случайный глобальный «80%».
4. **Удалить `publish.yml`**, когда release-plz проведёт 2-3 релиза без сбоев, — второй путь публикации стоит держать только пока первый не обкатан.
