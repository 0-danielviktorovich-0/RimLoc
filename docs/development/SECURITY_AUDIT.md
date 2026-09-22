# SECURITY AUDIT — RimLoc

Дата: 2026-09-23 · Область: Tauri GUI (trust boundary WebView → IPC → Rust → fs/process/native), сервисы, архивы.

## Модель угроз
WebView рендерит контент, производный от чужих модов (результаты сканов, превью XML). Компрометация WebView (XSS через мод-данные) не должна давать: произвольную запись файлов, запуск shell, исполнение нативного кода, чтение произвольных файлов.

## Находки и статус

| # | Серьёзность | Находка | Статус |
|---|-------------|---------|--------|
| S1 | **Critical** | `csp: null` в `gui/tauri-app/src-tauri/tauri.conf.json` — инъекция скрипта из мод-данных исполняется без ограничений | **FIXED**: CSP `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src ipc: http://ipc.localhost` |
| S2 | **Critical** | `save_text_file(path, content)` — запись произвольного файла по пути из WebView | **FIXED**: команда заменена на `save_text_via_dialog(default_path, content)` — путь подтверждается нативным диалогом на стороне Rust; WebView путь не поставляет. Все 6 мест фронтенда переведены |
| S3 | **High** | `collect_diagnostics(out_path)` — произвольная запись | **FIXED**: `collect_diagnostics_via_dialog` — путь только через нативный диалог |
| S4 | **High** | `load_plugin_cmd` → `libloading::Library::new` по пути из WebView = исполнение произвольного нативного кода | **FIXED**: гейт-эскалация — загрузка только `.dylib/.so/.dll`, канонизированный путь должен быть в allowlist `~/.rimloc/plugins-allow.json` (`{"allow":[...]}`); по умолчанию запрещено. CLI-флаги без изменений (явное действие пользователя в своём процессе) |
| S5 | **High** | `open_path` на Windows: `cmd /C start` без квотинга — инъекция метасимволов; на всех ОС — спавн shell-хелпера | **FIXED**: заменён на крейт `open` (LaunchServices/ShellExecute/xdg-open напрямую, без интерпретатора) + проверка существования пути |
| S6 | **High** | Zip-slip: `lang_update.rs` джойнил имена записей ZIP без санитизации (`../` выводит запись за пределы каталога) | **FIXED**: `sanitize_zip_rel()` отсекает `..`/absolute/backslash/NUL в плане и при распаковке; негативные тесты `apply_plan_never_writes_outside_target_dir`, `sanitize_rejects_traversal_and_absolute` |
| S7 | Medium | `tauri-plugin-shell` в capabilities (`shell:allow-open`) — WebView мог открывать произвольные URL/пути напрямую | **FIXED**: плагин удалён (фронтенд его не использовал — `tauriShell()` был мёртвым кодом), permissions из capabilities убраны, зависимость убрана из Cargo.toml |
| S8 | Low | `assetProtocol: enable` при отсутствии использования | **FIXED**: `enable: false`, feature `protocol-asset` снята |
| S9 | Info | 8× `innerHTML` во фронтенде | **AUDITED**: все интерполяции проходят `escapeHtml` / статические бейджи; `renderEnHighlighted` экранирует до разметки. CSP — второй рубеж |
| S10 | Info | XXE через XML модов | **Not vulnerable**: quick-xml/roxmltree не разворачивают внешние сущности/DTD |
| S11 | Info | Секреты | В коде/конфигах отсутствуют; localStorage хранит только UI-настройки |

## Остаточные риски (принятые, документированные)
- `open_path` открывает существующий путь системным обработчиком; открытие злоумышленнического `.app` теоретически возможно при уже скомпрометированном WebView и заранее подложенном файле. Смягчение: CSP (S1) + отсутствие произвольной записи (S2/S3) закрывают цепочку доставки.
- Динамические плагины остаются исполняемым кодом по своей природе; allowlist снижает риск до «осознанное действие пользователя».
- `blocking_save_file`/`blocking_pick_folder` блокируют поток команды — UX-ограничение Tauri, не安全问题.

## Проверки
- `cargo test --workspace` — 82 passed / 0 failed (включая zip-slip негативные).
- `cargo build -p rimloc-gui` — чисто; `cargo fmt` clean.
