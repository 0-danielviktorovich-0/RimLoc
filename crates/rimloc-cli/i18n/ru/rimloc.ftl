## ====================================================================
## RimLoc FTL стандарт оформления (RU)
## --------------------------------------------------------------------
## 1) Источник истины — английский файл en/rimloc.ftl.
##    Все локали ДОЛЖНЫ содержать те же самые ключи.
## 2) Порядок секций — ЗАФИКСИРОВАН:
##    - Общие сообщения (app-started, scan, validate, export, import, dry-run, xml, build)
##    - блок validate-po
##    - проверка аргументов import
##    - детали build-mod
##    - предупреждения / ошибки
##    - виды валидаций (validation kinds)
##    - категории валидаций (validation categories)
##    - локализация справки CLI (help-*, сгруппированные по подкомандам)
## 3) Добавление новых ключей:
##    - В существующем блоке добавляйте НОВЫЕ ключи В КОНЕЦ блока.
##    - Если появляется новый блок — добавьте весь блок В КОНЕЦ файла с заголовком.
## 4) Плейсхолдеры:
##    - Имена плейсхолдеров ($var) должны совпадать во всех локалях.
##    - Нельзя переименовывать/удалять плейсхолдеры без обновления всех локалей.
## 5) Локализация справки CLI:
##    - Топ-уровень: help-about, help-no-color, help-ui-lang.
##    - Для подкоманд: help-&lt;cmd&gt;-about и help-&lt;cmd&gt;-&lt;arg&gt;.
##    - Имена — в kebab-case, совпадают с флагами/аргументами CLI (например, help-importpo-out-xml).
## 6) Тесты:
##    - `all_locales_have_same_keys` проверяет совпадение ключей с EN.
##    - `each_locale_runs_help_successfully` использует help-ключи для проверки вывода.
## ====================================================================

app-started = rimloc запущен • версия={ $version } • logdir={ $logdir } • RUST_LOG={ $rustlog }

scan-csv-stdout = Печать CSV в stdout…
scan-csv-saved = CSV сохранён в { $path }

validate-clean = Всё чисто, ошибок не найдено

export-po-saved = PO сохранён в { $path }

import-dry-run-header = DRY-RUN план:
import-total-keys = ИТОГО: { $n } ключ(ей)
import-only-empty = PO содержит только пустые строки. Добавьте --keep-empty, если хотите импортировать заглушки.
import-nothing-to-do = Нечего импортировать (все строки пустые; добавьте --keep-empty, если нужны заглушки).
import-done = Импорт выполнен в { $root }

dry-run-would-write = DRY-RUN: записали бы { $count } ключ(ей) в { $path }

xml-saved = XML сохранён в { $path }

build-dry-run-header = === DRY RUN: сборка мода перевода ===
build-built-at = Мод перевода собран в { $path }
build-done = Мод перевода собран в { $out }

# === тестовые маркеры (для интеграционных тестов) ===
test-app-started = rimloc app_started маркер
test-dry-run-marker = DRY-RUN

# === validate-po ===
validate-po-ok = ✔ Плейсхолдеры в порядке ({ $count } строк)
validate-po-mismatch = ✖ Несовпадение плейсхолдеров { $ctxt } { $reference }
validate-po-msgid = msgid: { $value }
validate-po-msgstr = msgstr: { $value }
validate-po-expected = ожидалось: { $ph }
validate-po-got = получено: { $ph }
validate-po-total-mismatches = Всего несовпадений: { $count }
validate-po-report-line = { $ctxt } → { $reference }
validate-po-summary = Итого несовпадений: { $count }

# === import-po аргументы ===
import-need-target = Ошибка: нужно указать либо --out-xml, либо --mod-root
import-dry-run-line = { $path }  ({ $n } ключ(ей))

# === build-mod details ===
build-name = Имя мода: { $value }
build-package-id = PackageId: { $value }
build-rw-version = RimWorld версия: { $value }
build-mod-folder = Папка мода: { $value }
build-language = Язык: { $value }
build-divider = -----------------------------------
build-summary = ИТОГО: { $n } ключей будет записано

# === warnings / errors ===
ui-lang-unsupported = Неподдерживаемый код языка интерфейса
err-placeholder-mismatches = обнаружены несовпадения плейсхолдеров
validate-po-error = обнаружены несовпадения плейсхолдеров

# === validation kinds (короткие метки) ===
kind-duplicate = дубликат
kind-empty = пустое
kind-placeholder-check = проверка-плейсхолдеров

# === validation categories ===
category-duplicate = дубликат
category-empty = пустое
category-placeholder-check = проверка-плейсхолдеров

# === локализация справки CLI ===
# Топ-уровень
help-about = Набор инструментов локализации RimWorld (Rust)
help-no-color = Отключить цветной вывод
help-ui-lang = Язык интерфейса (ru или en; по умолчанию системный)

# scan
help-scan-about = Сканировать папку мода и извлечь записи Keyed XML
help-scan-root = Путь к корню мода RimWorld для сканирования
help-scan-out-csv = Сохранить извлечённые записи в CSV-файл
help-scan-lang = Код языка файлов для сканирования (например, en, ru)
help-scan-source-lang = Код исходного языка для перекрёстных проверок
help-scan-source-lang-dir = Путь к директории исходного языка для перекрёстных проверок

# validate
help-validate-about = Проверить строки на ошибки/предупреждения
help-validate-root = Путь к корню мода RimWorld для проверки
help-validate-source-lang = Код исходного языка для сравнения
help-validate-source-lang-dir = Путь к директории исходного языка для сравнения

# validate-po
help-validatepo-about = Проверить согласованность плейсхолдеров в .po (msgid vs msgstr)
help-validatepo-po = Путь к .po файлу для проверки
help-validatepo-strict = Строгий режим: вернуть ошибку (код 1), если найдены несовпадения

# export-po
help-exportpo-about = Экспортировать извлечённые строки в единый .po файл
help-exportpo-root = Путь к корню мода RimWorld с извлечёнными строками
help-exportpo-out-po = Путь к выходному .po файлу
help-exportpo-lang = ISO-код языка перевода (например, ru, ja, de)
help-exportpo-source-lang = ISO-код исходного языка для экспорта (например, en, ru, ja)
help-exportpo-source-lang-dir = Имя папки исходного языка (например, English). Перекрывает --source-lang

# import-po
help-importpo-about = Импорт .po — в один XML или по структуре существующего мода
help-importpo-po = Путь к .po файлу для импорта
help-importpo-out-xml = Путь выходного XML (режим одного файла)
help-importpo-mod-root = Корень мода для обновления импортированными строками (структурный режим)
help-importpo-lang = Код целевого языка для импорта (например, ru)
help-importpo-lang-dir = Папка целевого языка (переопределяет авто‑соответствие)
help-importpo-keep-empty = Импортировать пустые строки как заглушки
help-importpo-single-file = Записать все импортированные строки в один XML‑файл
help-importpo-backup = Создавать .bak‑резервные копии при перезаписи
help-importpo-dry-run = Ничего не записывать; только показать план действий

# build-mod
help-buildmod-about = Собрать отдельный мод‑перевод из .po файла
help-buildmod-po = Путь к .po файлу для сборки
help-buildmod-out-mod = Путь выходной папки мода
help-buildmod-lang = Код языка перевода
help-buildmod-name = Отображаемое имя мода перевода
help-buildmod-package-id = PackageId мода перевода
help-buildmod-rw-version = Целевая версия RimWorld
help-buildmod-lang-dir = Имя языковой папки внутри мода (необязательно)
help-buildmod-dry-run = Ничего не записывать; только вывести план сборки

# === scan ===
test-csv-header = CSV заголовок должен присутствовать

# === проверка стартового сообщения ===
test-startup-text-must-appear = Стартовое сообщение должно появляться для локали { $loc }