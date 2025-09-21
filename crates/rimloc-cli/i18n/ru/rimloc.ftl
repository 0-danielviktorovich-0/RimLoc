# ====================================================================
# RimLoc FTL стандарт оформления (RU)
# --------------------------------------------------------------------
# 1) Источник истины — английский файл en/rimloc.ftl.
#    Все локали обязаны содержать те же самые ключи.
# 2) Порядок: ключи сгруппированы блоками. Внутри блоков порядок фиксированный.
#    Текущая последовательность блоков:
#      - (A) общие сообщения запуска/сканирования/валидатора/экспорта/импорта/xml
#      - (B) build-dry-run/build-done и т.п.
#      - (C) === validate-po ===
#      - (D) === import-po аргументы ===
#      - (E) === build-mod details ===
#      - (F) === warnings / errors ===
#      - (G) === validation kinds (короткие метки) ===
#      - (H) === validation categories ===
# 3) Добавление новых ключей:
#      - Если это существующий блок — добавляйте НОВЫЕ ключи В КОНЕЦ блока.
#      - Если появляется новый функциональный блок — добавляйте весь блок В КОНЕЦ файла
#        с комментарием-заголовком `# === new-feature ===`.
# 4) Плейсхолдеры и аргументы: используйте те же имена и порядок, что и в en/rimloc.ftl.
#    Пример: { $path }, { $count }, { $ctxt }, { $reference } и т.д.
# 5) Тесты:
#    - `all_locales_have_same_keys` проверяет, что в каждой локали есть все ключи из EN.
#    - При несовпадении тест укажет, какие ключи отсутствуют.
# ====================================================================
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
ui-lang-unsupported = Неподдерживаемый код языка интерфейса; используется системная/дефолтная локаль
warn-unsupported-ui-lang = ⚠ Неподдерживаемый язык интерфейса: { $lang }. Использую системный.
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