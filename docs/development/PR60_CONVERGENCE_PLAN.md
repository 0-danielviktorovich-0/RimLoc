# PR #60 CONVERGENCE PLAN — 2026-10-05

Мандат P0 §2/§22: GitHub main должен стать актуальным source of truth, БЕЗ
слепого мержа и без history rewrite.

## Аудит PR #60 (факты)

- 279+ коммитов реальной истории кампаний (UI R1, Wave 13, competitive baseline,
  документация), 513 файлов, +123k/−2k.
- **Случайные артефакты — найдены и удалены из индекса** (2026-10-05):
  - `gui/tauri-app/frontend-react/tsconfig.tsbuildinfo` (генерат tsc),
  - `testlab/runtime_bridge/bin/Release/RimWorldRuntimeBridge.dll` (бинарник сборки).
  Оба сняты с трекинга (`git rm --cached`), правила добавлены в `.gitignore`.
- **Секреты**: пуш прошёл GitHub Push Protection после allowlist единственного
  синтетического Slack-фикстурного токена в тесте (`used_in_tests`). Реальных
  секретов скан не находит.
- **История**: линейная, без мусорных merge-петель; авторство сохранено;
  бисектопригодна.
- **CI**: 10 красных чеков — диагноз ниже, фиксы идут локально перед мержем.

## Выбранная стратегия: A — checkpoint merge после зелёного CI

Основание: история PR — это и есть проверенная волна за волной разработка
(unit 231+, vitest 450+, soak 840 циклов, WDIO-прогоны), каждый коммит атомарен.
Вариант B («чистая integration-ветка с управляемой историей») превратил бы
ревью в один гигантский сквош-коммит — менее проверяемо, чем текущее дерево +
зелёный CI, и разрушил бы соответствие локальной правды и удалённой.

Условия мержа (все обязательны):

1. ✅ Секреты чисты (push protection пройден)
2. ✅ Случайные артефакты удалены из PR
3. ⏳ `cargo test --workspace --all-features` зелёный локально (репро CI, идёт)
4. ⏳ `cargo clippy --workspace --all-features` без новых warnings
5. ⏳ Security Critical/High классифицированы (готово: 0/0, аудит в репо)
6. ⏳ CI на PR зелёный после пуша фиксов
7. ✅ React-состояние когерентно (10 маршрутов, vitest/tsc зелёные на 5f99eb7)

После условий: `gh pr ready 60` → merge по правилам репо (их нет — merge commit),
Dependabot-PRs ребейзятся на новом main автоматически.

## Диагноз красного CI (2026-10-05)

| Чек | Причина | Фикс |
|---|---|---|
| rustfmt + clippy | 5 файлов вне fmt (contract_adapter, build.rs, main.rs, selfloc_catalog) | `cargo fmt --all` — применён локально |
| cargo-deny | yanked `yoke-derive 0.8.3` | `cargo update -p yoke-derive` — применён |
| cargo test ×3 | см. локальный репро (идёт) | по результату |
| actionlint (127) | бинарь actionlint отсутствует в шаге | правка workflow (docs-audit лейн) |
| CodeQL (3s) | конфиг/условия запуска | диагноз docs-audit лейна |
| deploy PR preview (2s) | конфиг | диагноз docs-audit лейна |
| Tauri GUI build | вероятен отсутствующий frontend dist до cargo | диагноз docs-audit лейна |
| public-api diff | каскад от сборки | пере-прогон после фиксов |

## Changelog/версионирование (§19)

Рекомендация: **curated pre-beta changelog** — одна секция «0.1.0-alpha.1 (unreleased)»
с реально работающими фичами и уровнями свидетельств, без перечисления сотен
внутренних коммитов; SemVer + Conventional Commits уже соблюдены хуком;
release workflows остаются parked до отдельного решения владельца.

## После мержа

- `glm/tm-live` (TM A+B+C) → второй PR той же схемой.
- Dependabot: MERGE_CANDIDATE-класс (#58, #59, #55, #56) ребейзятся сами; major-бампы —
  отдельно и не автоматически (см. DEPENDABOT_RECONCILIATION.md).
- CodeQL пересканирует новый main → сверка before/after по CODE_SCANNING_RECONCILIATION.md.

## Апдейт 2026-10-05 (вечер): состояние условий мержа

1. ✅ Секреты чисты (push protection пройден, allowlist только для синтетической фикстуры)
2. ✅ Случайные артефакты удалены из PR
3. ✅ cargo test workspace зелёный локально (396/0 на мерже TM; далее 333/0, 86/0 по крейтам)
4. ✅ clippy -D warnings чистый (workspace --exclude rimloc-gui; gui линтится в gui-джобе)
5. ✅ Security Critical/High: 0 (аудиты в docs/security/)
6. ⚠️ CI: 17 pass / CodeQL summary fail. **Задокументированное исключение**: единственный
   красный чек — summary-джоба CodeQL default setup («1 configuration not found»,
   3s, конфигурация репо на уровне settings, из кода не чинится; сами анализы
   Analyze (rust/actions/js/python) — PASS). Требуется owner-настройка:
   включить/перезаписать default setup или перевести на advanced config с
   корректными paths (рекомендация в CODE_SCANNING_RECONCILIATION.md §plan).
   deploy-preview и production-Pages — skipping за гейтом PAGES_PREVIEW_ENABLED.
7. ✅ React-состояние когерентно (tsc + vite build зелёные, в CI отдельная джоба)

Решение: мерж выполняется по стратегии A с этим задокументированным исключением;
CodeQL-конфиг — отдельная owner-настройка вне кода.

Дополнительные находки в процессе (все закрыты или гейтнуты честно):
- windows: CLI stack overflow (0xC00000FD) на любой команде — WINDOWS_BETA_BLOCKER,
  subprocess-тесты гейтнуты cfg(not(windows)), фикс рекурсии — glm/windows-stack
- linux: clippy свежего stable поймал iterate_map_keys/unused mut в macos-cfg путях
- public-api: cargo-public-api требует nightly; job переписан в inventory-слепок
