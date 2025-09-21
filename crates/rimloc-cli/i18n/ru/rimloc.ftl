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