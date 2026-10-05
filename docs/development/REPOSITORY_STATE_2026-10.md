# REPOSITORY STATE — 2026-10-05

Сверка локальной реальности и GitHub. Составлено по живому git, не по историческим
чекпоинтам (мандат P0 §1).

## Git-правда

| Что | Значение |
|---|---|
| Локальный HEAD (feature/ui-r1-convergence, worktree `ba-main`) | `da2fc77` (плавает — активная кампания) |
| `main` (локальный) | = feature/ui-r1-convergence |
| `origin/main` | `9591f4f` «ci: park release workflows (owner decision)», 2026-09-26 |
| Draft PR #60 | `feature/ui-r1-convergence` → `main`, ~279 коммитов, 513 файлов |
| ahead/behind | локальная линия **на 279+ впереди** origin/main; behind = 0 (fast-forward база) |
| Дивергенция | `codex/pass-b-tails` влит (`11eafaa`): забран handoff-docs, `perf_bench.rs` остался от main (новее) |
| Worktrees | 15 активных; рабочие: `ba-main` (интеграция), `wt-tm-live` (ветка `glm/tm-live`, TM A+B+C), `wt-ui-r1` (рефлекс UI R1); остальные — история волн |
| Push-защита | Branch protection/Rulesets: отсутствуют. GitHub Push Protection: активен — в истории синтетический Slack-фикстурный токен (`frontend-v2/tests/contribution-bundle.test.ts`), allowlisted 2026-10-05 через `secret-scanning/push-protection-bypasses` (reason `used_in_tests`) |

## Frontend-архитектура (текущая)

- **Продакшен**: `gui/tauri-app/frontend-react/` — React 19 + Vite 7 + TS strict + Tailwind 4,
  OKLCH-токены Lovable R1, framework-neutral `RimLocClient` над Tauri IPC. 10 живых маршрутов.
- **Замороженный fallback**: `gui/tauri-app/frontend-v2/` — Svelte 5, не удаляется, собирается
  для benchmark-сравнения. НЕ дефолт.
- **Сборка**: Tauri 2, dual-config (React lane / Svelte default); automation-артефакт ≠
  owner-артефакт (identity gate: source SHA, flavor, automation flag).

## Артефактное состояние

- `rel16-react-r1` v2 — production (React, automation=false), sha256 `d780e275…`
- `rel17-react-r1` — automation (React + automation surface), sha256 `4c597434…`, смоук 5/5
- Runtime Bridge v1.0.0 (testlab, НЕ в проде: cargo-feature `automation-bridge` optional,
  capabilities automation.json удаляются из прод-сборки, release-guard сканирует артефакт)

## Release state

- Тегов/GitHub Releases: нет (запрещено до явного одобрения владельца)
- `release-plz.toml`/`release.toml` — конфигурация есть, workflows «parked» (решение владельца
  на origin/main 9591f4f)
- Версии крейтов 0.1.x, pre-1.0; curated pre-beta changelog — готовится (см. CHANGELOG-план в
  PR60_CONVERGENCE_PLAN.md)

## Security-состояние (на 2026-10-05)

- `docs/security/PRE_BETA_SECURITY_AUDIT.md` — 0 Critical/High, 1 Medium (F-1: out-пути
  5 IPC-команд без containment — фикс в работе), 5 Low, 4 Info; реальных секретов 0
- `docs/security/DEPENDABOT_RECONCILIATION.md` — 15 открытых PR классифицированы
  (0 security-driven; 4 merge-кандидата, 4 major-бампа, 1 superseded, 3 reject)
- cargo-deny (advisories/licenses/bans) — зелёный локально после `cargo update -p yoke-derive`
- Code Scanning (~130 алертов на старом origin/main) — ревизия в `CODE_SCANNING_RECONCILIATION.md`

## CI-состояние на PR #60

10 чеков красные (2026-10-05): rustfmt (исправлен локально `cargo fmt --all`), cargo test ×3 ОС
(локальный репро — в работе), cargo-deny (yanked yoke-derive — обновлён локально), actionlint
(exit 127), CodeQL (3s), deploy PR preview (2s), Tauri GUI build, public-api diff.
Диагноз и план — в PR60_CONVERGENCE_PLAN.md.

## Средовые ограничения (для агентов)

- Внутренний диск 12.9 GiB — cargo только с
  `CARGO_TARGET_DIR=/Volumes/Portable-SSD/caches/targets/rimloc` (создан, 74 GiB)
- На btrfs-SSD cargo требует `CARGO_INCREMENTAL=0` (локи не поддерживаются, os error 45)
- Хук commit-msg: Conventional Commits + буллеты в теле
