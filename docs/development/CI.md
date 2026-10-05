# CI — дизайн production-конвейера

Дата: 2026-09-27 · Статус: **дизайн-документ + имплементация волны**. Push-триггеры не возвращены — решение владельца (b102f3e) сохранено.
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

- **Push-триггеры сняты** (b102f3e «ci: disable push triggers, keep manual dispatch») — все workflow реагируют только на `pull_request` (в main) и/или `workflow_dispatch`. Еженедельный `schedule` у CI в ту же рамку не влезает и снят; сниппет для возврата — §6.
- **Релизные workflow припаркованы** (9591f4f) — `.github/workflows-disabled/` не тронут; при разморозке см. §7.

## 3. Полномочия релиза (кто что публикует)

| Субъект | Действие | Статус |
|---------|----------|--------|
| `release-plz.yml` → job `release` | версии + теги + публикация крейсов в crates.io | **единственный автоматический владелец**. Двухшаговый ручной процесс: (1) dispatch на main → Release PR; (2) смержить Release PR → ещё dispatch на main → теги + publish |
| `publish.yml` | повторная публикация крейсов | **только ручной recovery** (например, crates.io 429 прервал пачку). Токен — только repo secret; dry_run по умолчанию `true`; любая позиция пачки обязательна. Когда release-plz обкатается на 2-3 релизах — кандидат на удаление (решение владельца) |
| `release-dev*.yml` (parked) | GitHub Releases с бинарниками CLI/GUI | разморозка — отдельное решение владельца; до неё не трогаем |
| `docs.yml` → job `deploy-prod` | публикация сайта на Pages | ручной dispatch на main (раньше — push, мёртвый после b102f3e); preview — для PR из upstream |

## 4. Целевая архитектура

| Workflow | Триггеры | Jobs | Роль |
|----------|----------|------|------|
| `ci.yml` | PR→main, dispatch | `lint` (fmt+clippy), `test` (ubuntu/macos/windows), `gui` (Tauri build), `frontend` (svelte-check + vitest), `deny` (cargo-deny), `schema` (drift), `workflows-lint` (actionlint) | единый quality gate |
| `semver.yml` | PR→main, dispatch | `semver` (cargo-semver-checks, гейт), `public-api` (информационный диф) | стабильность API |
| `changelog-check.yml` | PR→main | `verify` | CHANGELOG сопровождает пользовательские изменения (лейбл `internal-only` выключает) |
| `docs.yml` | PR→main, dispatch | `build`, `deploy-prod`, `deploy-preview` | MkDocs |
| `publish.yml` | dispatch | `publish` | recovery-публикация crates.io |
| `release-plz.yml` | dispatch | `release_pr`, `release` | версионирование и релизы |

Покрытие мандата: rustfmt/clippy/tests — `ci.lint/test`; frontend typecheck/tests — `ci.frontend`; dependency/security audit — `ci.deny` (cargo-deny: RustSec-advизории + лицензии + bans; cargo-audit не добавлен как дубликат — см. §6); actionlint — `ci.workflows-lint`; CodeQL — default setup репозитория (см. §6); mkdocs build — `docs.build`.

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
   | `MarcoIeni/release-plz-action` | `v0.5` | `b8d6b54b02889ff2ae2bb82e8b57c3a8fc1683a5` |

   Замечание про `dtolnay/rust-toolchain`: запинен код экшена, но НЕ тулчейн — действие вызывает `rustup`, и `stable` резолвится на стороне GitHub в момент прогона, так что новые стабильные версии Rust продолжают приходить без правки workflow.

2. **Минимальные permissions.** На верхнем уровне каждого workflow `permissions: {}`, у каждого job'а — свой минимум (`contents: read` почти везде; `pages: write, id-token: write` — только у deploy-job'ов docs; `contents: write, pull-requests: write` — только у release-plz).

3. **Никаких `|| true` на обязательных шагах.** Убраны из `publish.yml` (позиции пачки), `public-api` (диф; остаётся информационным, но ошибка инструмента теперь видна). Легитимные `|| true`-паттерны, оставшиеся в запаркованных файлах (`find ... | head` при упаковке), — предмет правки при разморозке.

4. **Секреты — только через `secrets.*`.** Input `token` у `publish.yml` удалён (inputs читаемы в UI и логах). Пустой `CARGO_REGISTRY_TOKEN` при `dry_run=false` теперь явная ошибка с понятным сообщением, а не невнятный отказ cargo.

5. **Конкурентность.** Быстрые гейты (`ci`, `semver`, `docs.build`) — `cancel-in-progress: true` по `github.ref`; необратимые операции (`crates-publish`, `docs-pages`, `release-plz-release`) — очереди без отмены: оборванная публикация хуже отложенной.

## 6. Осознанно отложенное (позже, по решению владельца)

- **Coverage (`cargo-llvm-cov` + vitest `--coverage`).** Спроектировано, не имплементировано: матчер — job `ci.coverage` на ubuntu (`cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info` + `npm test -- --coverage` в `gui/tauri-app/frontend-v2`), загрузка в Codecov или отдельный артефакт; пороги покрытия — после накопления базовой линии, иначе гейт родится красным. Включается одним job'ом в `ci.yml`, когда появится baseline.
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

`actionlint` (1.7.12) прогоняется и локально, и в CI (`ci.workflows-lint` ставит его через `go install ...@v1.7.12` — верификация модуля через sum.golang.org, без сторонних actions). Локально:

```bash
brew install actionlint   # или go install github.com/rhysd/actionlint/cmd/actionlint@v1.7.12
actionlint -color .github/workflows/*.yml
```

## 9. Рекомендации владельцу (в этой волне не сделано)

1. **Dependabot: ecosystem `github-actions`** (в `.github/dependabot.yml` сейчас только cargo+pip) — иначе пины SHA устаревают вручную. PR от dependabot будет поднимать SHA и comment-теги разом.
2. **Branch protection на main** (сейчас не защищена, rulesets пусты): после включения — required checks `CI / lint`, `CI / test (ubuntu-latest)`, `CI / deny`, `CI / schema`, `API stability / cargo-semver-checks (gating)`, `changelog / verify` (минимальный набор; matrix-OS и GUI — по вкусу).
3. **Удалить `publish.yml`**, когда release-plz проведёт 2-3 релиза без сбоев, — второй путь публикации стоит держать только пока первый не обкатан.
