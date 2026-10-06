# Dependabot reconciliation — 15 открытых PR

- **Дата:** 2026-10-05
- **HEAD:** `11eafaa1027822913f55dbfefcb3a9ed88beafa6` (ветка `feature/ui-r1-convergence`)
- **Источник списка:** `gh pr list --state open --limit 100 --json number,title,headRefName,mergeable,updatedAt`
- **Важно:** удалённый стейт отстаёт от локального HEAD. Каждый PR сверён с **локальными** `Cargo.toml`/`Cargo.lock` (корневой workspace-lock — канонический; `gui/tauri-app/src-tauri/Cargo.lock` — устаревший вложенный артефакт, см. вывод ниже).
- **Ничего не мержено.** Только классификация и рекомендации.

## Сверка с локальным стейтом (что где лежит)

| Зависимость | Локальный манифест | Локальный Cargo.lock (корневой) | PR хочет |
|---|---|---|---|
| tauri | `2` (src-tauri) | 2.11.6 | 2.12.0 (#57) |
| tauri-build | `2` (src-tauri) | 2.6.3 | 2.7.0 (#58) |
| tauri-plugin-dialog | `2` (src-tauri) | 2.7.3 | 2.8.0 (#59) / 2.7.3 (#47) |
| ico | `0.3` (src-tauri build-deps) | 0.3.0 **и 0.5.0** (0.5 транзитивно через tauri-codegen) | 0.5.0 (#54) / 0.4.0 (#45) |
| toml | `0.8` (rimloc-config) | 0.8.x прямой + 1.1.6+spec транзитивно (via tauri-стек) | 1.1.6+spec-1.1.0 (#53) |
| keyring | `3` optional (rimloc-llm) | 3.6.3 | 4.2.0 (#52) |
| sha2 | `0.10` (rimloc-services) | 0.10.9 **и 0.11.0** (оба уже в графе) | 0.11.0 (#51) |
| lru | `0.12` (rimloc-cli, rimloc-services); ^0.12 держит и tauri 2.x | 0.12.5 | 0.18.5 (#44) |
| thiserror | `1` (workspace + rimloc-llm + src-tauri) | 1.0.69 | 2.0.20 (#40) / 2.0.17 (#39) |
| tauri-plugin-shell | **удалён** из src-tauri/Cargo.toml | только в мёртвом вложенном lock (2.3.1) | 2.3.6 (#36) |
| markdown / pymdown-extensions | `==3.10.3` / `==12.0.1` (requirements-docs.txt) | — (pip) | 3.11 (#55) / 12.1 (#56) |

Ключевое наблюдение: `gui/tauri-app/src-tauri/Cargo.lock` **не используется** сборкой (src-tauri — member workspace, реальный lock — корневой), но Dependabot продолжает обновлять его (PR #36, #47 правят только этот мёртвый файл). Вложенный lock удалён из репо (2026-10-07): cargo metadata из src-tauri резолвит корневой workspace без него, CI-пути читают только корневой lock.

## Сводка категорий

| Категория | Кол-во | PR |
|---|---|---|
| MERGE_CANDIDATE | 4 | #58, #59, #55, #56 |
| ROUTINE_UPDATE (с оговорками) | 2 | #57, #51 |
| MAJOR_UPDATE — плановый миграционный PR, не автопMerge | 4 | #40, #53, #52, #54 |
| CONFLICT (+ major) — заблокировано | 1 | #44 |
| SUPERSEDED | 1 | #47 |
| REJECT_WITH_REASON | 3 | #36, #45, #39 |
| SECURITY_RELEVANT (есть advisory) | **0** | — |

`cargo deny check advisories` не показывает ни одного открытого advisory на затронутые крейты (только yanked `yoke-derive`, не связанный с PR), и в списке нет advisory-driven («security») веток Dependabot — поэтому категория SECURITY_RELEVANT пустая.

## Постатовая классификация

| PR | Заголовок | mergeable | Категория | Обоснование и рекомендация |
|---|---|---|---|---|
| #57 | tauri 2.11.6 → 2.12.0 | MERGEABLE | **ROUTINE_UPDATE** (minor), с обязательной проверкой | Файлы: корневой Cargo.lock. Локально 2.11.6 → не superseded, advisory на 2.11.6 нет. **Оговорка:** workspace держит вендор-патч `wry 0.55.1-noactivate` / `tao 0.35.3-noactivate` (`[patch.crates-io]`, P0 zero-focus-stealing). Если 2.12.0 поднимет минимальный wry/tao, патч «осиротеет» и сборка/патч-семантика сломаются. Мержить только с локальной сборкой + прогоном `testlab/release-guard.sh --runtime`. |
| #58 | tauri-build 2.6.3 → 2.7.0 | MERGEABLE | **MERGE_CANDIDATE** | Build-time зависимость, minor-бамп, локально 2.6.3. Риск минимален; CI решит. |
| #59 | tauri-plugin-dialog 2.7.3 → 2.8.0 | MERGEABLE | **MERGE_CANDIDATE** | Minor-бамп плагина нативных диалогов; локально 2.7.3. Сверить с #57 (версии tauri-плагинов идут в ногу с ядром — если мержится #57, мержить и #59, и наоборот проверять совместимость). |
| #55 | markdown 3.10.3 → 3.11 (pip) | MERGEABLE | **MERGE_CANDIDATE** | Только `requirements-docs.txt` (сборка mkdocs-документации). На приложение не влияет. |
| #56 | pymdown-extensions 12.0.1 → 12.1 (pip) | MERGEABLE | **MERGE_CANDIDATE** | Аналогично #55. |
| #51 | sha2 0.10.9 → 0.11.0 | MERGEABLE | **ROUTINE_UPDATE** (semver-ломкий 0.x-бамп, не major из списка) | Файлы: Cargo.lock + `crates/rimloc-services/Cargo.toml`. Локально 0.10.9; при этом 0.11.0 уже есть в графе транзитивно — бамп может убрать дубли. Публичный API (`Digest`, `Sha256`) стабилен; прогнать тесты rimloc-services. |
| #53 | toml 0.8.2 → 1.1.6+spec-1.1.0 | MERGEABLE | **MAJOR_UPDATE** (0.8 → 1.x) | Файлы: Cargo.lock + `crates/rimloc-config/Cargo.toml`. 1.1.6 уже в lock транзитивно, но прямой деп rimloc-config — 0.8; API-поверхность (error types, `Document`) между 0.8 и 1.x менялась. Не автопMerge: отдельный миграционный PR с полными тестами сериализации конфига. После беты. |
| #52 | keyring 3.6.3 → 4.2.0 | MERGEABLE | **MAJOR_UPDATE** (3 → 4) | Файлы: Cargo.lock + `crates/rimloc-llm/Cargo.toml` (optional-фича). keyring 4 реорганизовал платформенные credential-store'ы и убрал устаревшее — гарантированный миграционный заход в коде `rimloc-llm` (хранение API-ключей провайдеров). Проверить маков keychain-поведение. Не автопMerge; после беты. |
| #54 | ico 0.3.0 → 0.5.0 | MERGEABLE | **MAJOR_UPDATE** (0.3 → 0.5, build-dep) | Файлы: Cargo.lock + src-tauri/Cargo.toml. Затрагивает только build.rs (конвертация иконок). Бонус: унифицирует дубли `ico` 0.3.0+0.5.0 в lock (0.5.0 уже тянется через tauri-codegen). Проверить API build.rs (`ICOEncoder` и т.п.), собрать GUI. После беты или вместе с ней при удобном окне. |
| #40 | thiserror 1.0.69 → 2.0.20 | CONFLICTING | **MAJOR_UPDATE** (1 → 2) + конфликт | Major-бамп; ветка отрезана от глубоко устаревшей базы — в files дифа вся репа (десятки файлов), мержить нельзя. thiserror 2 почти drop-in, но это плановая миграция workspace+src-tauri одним заходом. Рекомендация: закрыть, пересоздать вручную после беты. |
| #44 | lru 0.12.5 → 0.18.5 | CONFLICTING | **CONFLICT** (+ multi-major 0.12→0.18) | lru `^0.12` держат манифесты rimloc-cli/rimloc-services И tauri 2.x (комментарий в deny.toml). Бамп очистил бы игноры RUSTSEC-2026-0002/0253 (soundness), но упирается в tauri. Закрыть/держать открытым до момента, когда tauri разрешит lru >0.12; тогда один PR «lru + пересмотр deny-игноров». |
| #45 | ico 0.3.0 → 0.4.0 (src-tauri) | CONFLICTING | **REJECT_WITH_REASON** | Superseded #54 (0.5.0): локальная цель уже перекрыта более новой версией, ветка конфликтует. Закрыть в пользу #54. |
| #39 | thiserror 1.0.69 → 2.0.17 (src-tauri) | CONFLICTING | **REJECT_WITH_REASON** | Superseded #40: дубль той же миграции по другому манифесту; миграцию thiserror делать одним workspace-заходом (#40-преемник), не по кускам. |
| #47 | tauri-plugin-dialog 2.4.0 → 2.7.3 (src-tauri) | MERGEABLE | **SUPERSEDED** | Правит ТОЛЬКО устаревший вложенный `gui/tauri-app/src-tauri/Cargo.lock`; канонический корневой lock уже на 2.7.3 (манифест `2` резолвит новее). Мержить бессмысленно; закрыть + удалить вложенный lock (иначе Dependabot продолжит его обновлять). |
| #36 | tauri-plugin-shell 2.3.1 → 2.3.6 (src-tauri) | MERGEABLE | **REJECT_WITH_REASON** | Зависимость `tauri-plugin-shell` **удалена** из src-tauri/Cargo.toml (открытие путей — через крейт `open`, что и безопаснее). PR обновляет запись в мёртвом вложенном lock, поддерживая иллюзию живого плагина. Закрыть + удалить вложенный lock. |

Вне списка: **#60** (UI R1 React convergence) — не Dependabot, сама фичеветка, в reconciliation не входит.

## Рекомендуемый порядок действий (после беты, в порядке дешевизны)

1. Удалить `gui/tauri-app/src-tauri/Cargo.lock` (мёртвый файл — источник PR #36/#47 и ложных обновлений), закрыть #36, #47, #45, #39 с указанием причин.
2. `cargo update -p yoke-derive` — гасит единственную ошибку `cargo deny check advisories` (см. PRE_BETA_SECURITY_AUDIT.md, F-4).
3. Мержить MERGE_CANDIDATE пачкой: #58, #59, #55, #56 (+ #51 после тестов services).
4. #57 (tauri 2.12) — отдельным заходом: проверить, что вендор-патч wry/tao ещё резолвится, собрать, прогнать release-guard.
5. Плановые миграции (каждый отдельным PR, с полным прогоном): toml 1.x (#53) → ico 0.5 (#54) → keyring 4 (#52) → thiserror 2 (новый PR вместо #40) → lru 0.18 при готовности tauri (#44-преемник, с пересмотром deny-игноров).

## Вердикт

Среди 15 открытых Dependabot-PR **ни один не несёт security-фикса** (0×SECURITY_RELEVANT) и ни один не блокирует бету. Слепо не мержится ничего из major-четвёрки (#40/#52/#53/#54) и конфликтующего #44; четыре MERGE_CANDIDATE безопасны; #57 — рутинный, но с жёсткой привязкой к вендор-патчу wry. Четыре PR (#36, #45, #39, #47) рекомендуется закрыть как superseded/obsolete, почистив мёртвый вложенный lockfile.
