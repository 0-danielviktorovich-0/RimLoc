# VENDOR_AUDIT — supply-chain аудит vendored noactivate-форков RimLoc

- **Дата**: 2026-10-07
- **Аудитор**: supply-chain security audit (read-only, без изменений кода)
- **Объект**: `gui/tauri-app/vendor/wry-0.57.0-noactivate`, `gui/tauri-app/vendor/tao-0.37.1-noactivate`
- **База сравнения**: `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wry-0.57.0/` и `.../tao-0.37.1/` (registry-извлечения, чексуммы верифицированы cargo при распаковке)
- **Worktree**: `/Users/danielviktorovich/Developing/_rimloc-worktrees/ba-main`
- **Мандат**: §31; пересадка 06.10.2026 (коммиты `796619b` wry, `00eb252` tao; перепривязка патча `eb67315` 06.10 16:49)

---

## 1. Diff surface audit — ПОВЕРХНОСТЬ ЧИСТА

Метод: полный рекурсивный `diff -rq` (включая dotfiles) форк ↔ registry-извлечение, затем построчные `diff -u` по каждому отличию. Файловые наборы идентичны (ни одного «Only in»).

### wry-0.57.0-noactivate — отличаются ровно 2 файла

| Файл | Хунок (fork-строки) | Суть |
|---|---|---|
| `src/wkwebview/mod.rs` | `@@ -691,16 +691,17 @@` (~691–707) | Удалён безусловный `NSApplication::activate` (macOS 14+) / `activateIgnoringOtherApps` (старее) при создании webview. Заменён на no-op: `let app = NSApplication::sharedApplication(mtm);` + `let os_major_version = os_major_version;` с `#[allow(unused_variables)]`. 19 строк диффа |
| `Cargo.toml` | +5 в конце | `[lints.rust] warnings = "allow"` + комментарий |

Проверено: изменённый блок целиком внутри `#[cfg(target_os = "macos")]`; iOS-ветка и `makeFirstResponder` (внутриоконный first responder, активацию приложения не вызывает) не тронуты. Остальное дерево — побайтово идентично апстриму.

### tao-0.37.1-noactivate — отличаются ровно 4 файла

| Файл | Хунки (fork-строки) | Суть |
|---|---|---|
| `src/platform_impl/macos/app_state.rs` | `@@ -288,9 +288,11 @@` и `@@ -445,8 +447,14 @@` | (1) На старте приложения скип `activateIgnoringOtherApps(ignore)` — заменён инертным `let _ = ...aux_state...;`; (2) в `window_activation_hack` для видимых окон `makeKeyAndOrderFront` → `orderFront`. 18 строк диффа |
| `src/platform_impl/macos/util/async.rs` | `@@ -213,7 +213,8 @@` и `@@ -237,9 +238,13 @@` | `make_key_and_order_front_sync`: → `orderFront`; `set_focus`: → `orderFront` без key-граба, убран `msg_send![app, activateIgnoringOtherApps: YES]` |
| `src/platform_impl/macos/window.rs` | `@@ -626,12 +626,12 @@` | `set_visible`: обе ветки (focused/unfocused) → `orderFront`, `let _ = focused;` |
| `Cargo.toml` | +5 в конце | `[lints.rust] warnings = "allow"` + комментарий |

### Заключение по поверхности

**Бэкдоров и посторонних правок нет.** Ни одна правка не затрагивает security-значимые поверхности: IPC, навигационную политику, валидацию URL, custom protocol, JS-инъекции, куки/хранилище, пермишены. Все 5 содержательных хунков — только вызовы активации окна AppKit (`activate`, `activateIgnoringOtherApps`, `makeKeyAndOrderFront` → `orderFront`) плюс комментарии-маркеры `RimLoc fork (P0 …)`. Соответствует заявленному мандату (noactivate, P0 zero-focus-stealing) один в один.

**Задокументированное поведенческое отклонение** (не уязвимость): публичный хелпер tao `set_focus()` теперь не переводит окно в key-состояние — app-код, вызывающий `WebviewWindow::set_focus()`, получит только orderFront. Осознанная цена P0-политики; при rebase об этом помнить.

---

## 2. Upstream security advisories — ЧИСТО, ПЕРЕКРЁСТНО

| Источник | wry | tao | Примечание |
|---|---|---|---|
| OSV (`api.osv.dev`, агрегирует RUSTSEC + GHSA) | 0 уязвимостей | 0 уязвимостей | Sanity-check метода: тот же запрос по `chrono` вернул RUSTSEC-2020-0159 — запрос рабочий, пустота не артефакт |
| RustSec advisory-db (github.com/rustsec/advisory-db, `crates/wry`, `crates/tao`) | каталога нет → advisories нет | каталога нет → advisories нет | 404 по обоим |
| GitHub global advisories (`advisories?ecosystem=rust&affects=…`) | 0 | 0 | |
| GH репозиторные advisories (`repos/tauri-apps/{wry,tao}/security-advisories`) | нет опубликованных | нет опубликованных | |
| Yanked-версии на crates.io | нет | нет | 0.57.0 и 0.37.1 не отозваны |

**Актуальность версий**: wry **0.57.0 — последняя** версия на crates.io (издана 2026-09-08); tao **0.37.1 — последняя** (2026-09-26). Форки стоят на самых свежих апстрим-релизах — security-фиксов после этих версий не существует по определению.

**Changelog 0.55.1 → 0.57.0 / 0.35.3 → 0.37.1** (проверены CHANGELOG wry и тела GH-releases обоих): security-записей нет. Единственное близкое к security — в wry 0.56.1 устранён паник IPC-хендлера на невалидном `http::Uri` (crash/DoS-класс, Android-путь); пересадка с 0.55.1 этот фикс **подхватила** — плюс к устойчивости.

---

## 3. Rebase-safety — СЕГОДНЯ НЕ ТРЕБУЕТСЯ, ПОРТИРУЕМОСТЬ ХОРОШАЯ

- Rebase не нужен: обе базовые версии = latest upstream.
- На будущее: дифф — 5 хунков в 4 файлах, все якорятся на текстово-уникальные вызовы AppKit (`makeKeyAndOrderFront`, `activateIgnoringOtherApps`, `NSApplication::activate`) — стабильные objc2-биндинги, переименование маловероятно. Перенос механический: та же замена → orderFront / удаление activate.
- Зона риска конфликтов: `wkwebview/mod.rs` (самая живая часть wry, но конкретный регион — macos-создание webview — исторически стабилен) и `app_state.rs::window_activation_hack`.
- **Специфический триггер**: в tao открыт PR **#1210** «fix(macos): create non-focused window when `focus` or `focusable` to false» — апстрим решает ту же проблему, но иначе (уважает флаг focus, а не тотальный запрет кражи фокуса, как у RimLoc). Если его смёржат, регион `window.rs ~626` изменится → конфликт при rebase почти гарантирован, а семантика RimLoc (строже апстрима) потребует ручного слияния. В wry открытых PR по теме поиском не найдено — заявленный в комментарии форка «Upstream PR pending» не подтверждается (возможно, PR не открыт или сформулирован иначе).

---

## 4. Supply-chain гигиена

| Пункт | Состояние | Оценка |
|---|---|---|
| `[patch.crates-io]` | `/ba-main/Cargo.toml:59-61`, обе записи → path внутри `gui/tauri-app/vendor/`; с комментарием о P0 | корректно |
| Cargo.lock | Записи `wry 0.57.0` / `tao 0.37.1` **без** `source = "registry…"` и **без** checksum — каноничное патч-состояние; lock не может «сдрейфить» на registry-версию молча | корректно |
| Целостность vendor | Форки закоммичены в git (406 файлов), дерево чистое; неизменённые файлы побайтово совпадают с checksum-верифицированным registry-извлечением | корректно |
| `lints.rust warnings = "allow"` | Приемлемо для вендоренного кода (не ронять workspace `-D warnings` на чужом коде). Побочный эффект: глушит и deprecation-предупреждения — при будущем rebase сигналы гнили кода исчезнут. Низкий риск | приемлемо, осознанно |
| Cargo.lock внутри папок форков | Из пакетного тарбола; воркспейсом не читается (path-deps) | безвредно |
| `deny.toml` (корень) | `[advisories] v2`, `yanked = "deny"`, `unknown-registry/git = "deny"`; wry/tao **не** в ignore-листе (игнорируются только транзитивные unic-*/paste/proc-macro-error) | хорошо |
| **Старые форки** | `wry-0.55.1-noactivate` (996K) и `tao-0.35.3-noactivate` (1.5M) всё ещё в vendor/ и в git. Ссылок из toml/rs/json на них нет — чистый мёртвый груз | **найти** |
| **Документационный дрейф** | `docs/security/DEPENDABOT_RECONCILIATION.md` (строка #57) и `docs/security/PRE_BETA_SECURITY_AUDIT.md` (F-10) всё ещё описывают патч как указывающий на 0.55.1/0.35.3 | **найти** |

Дополнительные найденные обстоятельства: конфиг CodeQL исключает `gui/…` несуществующим префиксом `RimLoc/gui/…` — vendor-код сканируется как собственный (массовые алерты), зафиксировано в `docs/development/DOCUMENTATION_AUDIT_2026-10.md` (к аудиту форков напрямую не относится, но увеличивает шум по vendor-путям).

---

## 5. Verdict: **SAFE_TO_KEEP**

Оба форка: дифф-поверхность точно равна заявленной (только noactivate), advisories отсутствуют по всем пяти независимым источникам, версия = latest upstream, патч-механика (patch.crates-io + lock без registry-source) корректна. Security-блокеров нет, rebase не требуется.

### Рекомендации (приоритет по убыванию)

1. **Удалить старые форки** `wry-0.55.1-noactivate` и `tao-0.35.3-noactivate` из vendor/ (`git rm`, физически — через корзину по Правилу 1). Убирает 2.5 МБ мёртвого кода и риск случайного ре-referencing; заодно уменьшает поверхность ложных CodeQL-алертов.
2. **Обновить два устаревших места в security-доках**: `docs/security/DEPENDABOT_RECONCILIATION.md` #57 и `docs/security/PRE_BETA_SECURITY_AUDIT.md` F-10 — заменить 0.55.1/0.35.3 → 0.57.0/0.37.1, иначе следующий Depandabot-разбор пойдёт по неактуальной инструкции.
3. **Зафиксировать процедуру rebase форков** (при выходе нового wry/tao): извлечь registry-версию → `diff -rq` перенести 5 хунков → `diff -rq` форк vs новый апстрим (должен показать ровно те же 4/2 файла) → обновить пути в `[patch.crates-io]` → cargo check. Следить за tao PR #1210: при мерже регион `window.rs` изменится.
4. **Периодический cargo-deny advisories** по корневому lock (конфиг уже готов): при каждом Depandabot-пинге или раз в месяц — единственный способ поймать будущий advisory на 0.57.x/0.37.x, если апстрим зарелизит фиксированную версию.
5. **Опционально**: оформить настоящий upstream PR в wry/tao (в комментарии форка заявлен «pending», фактически не обнаружен) — сокращает lifetime вендор-патча; либо поправить комментарий на честный статус.
