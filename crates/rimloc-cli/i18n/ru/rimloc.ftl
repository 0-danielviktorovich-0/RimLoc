## ====================================================================
## RimLoc FTL стандарт оформления (RU)
## --------------------------------------------------------------------
## 1) Источник истины  -  английский файл en/rimloc.ftl.
##    Все локали ДОЛЖНЫ содержать те же самые ключи.
## 2) Порядок секций  -  ЗАФИКСИРОВАН:
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
##    - Если появляется новый блок  -  добавьте весь блок В КОНЕЦ файла с заголовком.
## 4) Плейсхолдеры:
##    - Имена плейсхолдеров ($var) должны совпадать во всех локалях.
##    - Нельзя переименовывать/удалять плейсхолдеры без обновления всех локалей.
## 5) Локализация справки CLI:
##    - Топ-уровень: help-about, help-no-color, help-ui-lang.
##    - Для подкоманд: help-&lt;cmd&gt;-about и help-&lt;cmd&gt;-&lt;arg&gt;.
##    - Имена  -  в kebab-case, совпадают с флагами/аргументами CLI (например, help-importpo-out-xml).
## 6) Тесты:
##    - `all_locales_have_same_keys` проверяет совпадение ключей с EN.
##    - `each_locale_runs_help_successfully` использует help-ключи для проверки вывода.
## ====================================================================

app-started = rimloc запущен - версия={ $version } - logdir={ $logdir } - RUST_LOG={ $rustlog }

scan-csv-stdout = Печать CSV в stdout...
scan-csv-saved = CSV сохранён в { $path }
scan-json-stdout = Печать JSON в stdout...
scan-json-saved = JSON сохранён в { $path }

validate-clean = Всё чисто, ошибок не найдено

export-po-saved = PO сохранён в { $path }
export-po-tm-coverage = TM автозаполнение: { $filled } / { $total } ({ $pct }%)
export-po-missing-definj-suggested = Не найдено English/DefInjected; примените { $path }, скопировав в Languages/{ $lang_dir }/DefInjected
export-po-missing-definj-learned = Не найдено English/DefInjected; воспользуйтесь обученными данными из { $path }
export-po-missing-definj-generate = Папка Languages/{ $lang_dir }/DefInjected пуста; запустите «rimloc-cli learn-defs --lang-dir { $lang_dir }» или добавьте шаблоны перед экспортом

import-dry-run-header = DRY-RUN план:
import-total-keys = ИТОГО: { $n } ключ(ей)
import-only-empty = PO содержит только пустые строки. Добавьте --keep-empty, если хотите импортировать заглушки.
import-nothing-to-do = Нечего импортировать (все строки пустые; добавьте --keep-empty, если нужны заглушки).
import-done = Импорт выполнен в { $root }

dry-run-would-write = DRY-RUN: записали бы { $count } ключ(ей) в { $path }
annotate-dry-run-line = DRY-RUN: { $path } (добавить={ $add }, удалить={ $strip })

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
learn-defs-summary = Обучение по Defs: кандидатов={ $candidates }, принято={ $accepted } → missing_keys.json={ $missing }, suggested.xml={ $suggested }

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
schema-dumped = Схемы сохранены в { $path }

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


# === детальные сообщения валидации (на элемент) ===
# Стандартизированные плейсхолдеры по локалям:
# - $validator : короткое имя валидатора, напр. DuplicateKey, EmptyKey, Placeholder
# - $path      : путь к файлу
# - $line      : номер строки (число)
# - $message   : человекочитаемое объяснение (может быть локализовано самим валидатором)
validate-detail-duplicate = [duplicate] { $validator } ({ $path }:{ $line })  -  { $message }
validate-detail-empty = [empty] { $validator } ({ $path }:{ $line })  -  { $message }
validate-detail-placeholder = [placeholder-check] { $validator } ({ $path }:{ $line })  -  { $message }

# === локализация справки CLI ===
# Топ-уровень
help-about = Набор инструментов локализации RimWorld (Rust)
help-no-color = Отключить цветной вывод
help-ui-lang = Язык интерфейса (ru или en; по умолчанию системный)
help-quiet = Скрыть стартовый баннер и несущественные сообщения в stdout (алиас: --no-banner)

# scan
help-scan-about = Сканировать папку мода и извлечь записи Keyed XML
help-scan-root = Путь к корню мода RimWorld для сканирования
help-scan-out-csv = Сохранить извлечённые записи в CSV-файл
help-scan-out-json = Сохранить извлечённые записи в JSON-файл (используйте вместе с --format json)
help-scan-lang = Код языка файлов для сканирования (например, en, ru)
help-scan-source-lang = Код исходного языка для перекрёстных проверок
help-scan-source-lang-dir = Путь к директории исходного языка для перекрёстных проверок
help-scan-format = Формат вывода: «csv» (по умолчанию) или «json»
help-scan-game-version = Папка версии игры (например, 1.6 или v1.6); по умолчанию выбирается самая новая под корнем
help-scan-include-all = Включить все подпапки версий (отключить авто‑выбор последней)

# validate
help-validate-about = Проверить строки на ошибки/предупреждения
help-validate-root = Путь к корню мода RimWorld для проверки
help-validate-source-lang = Код исходного языка для сравнения
help-validate-source-lang-dir = Путь к директории исходного языка для сравнения
help-validate-format = Формат вывода: «text» (по умолчанию) или «json»
help-validate-game-version = Папка версии игры (например, 1.6 или v1.6); по умолчанию выбирается самая новая
help-validate-include-all = Включить все подпапки версий (отключить авто‑выбор последней)

# validate-po
help-validatepo-about = Проверить согласованность плейсхолдеров в .po (msgid vs msgstr)
help-validatepo-po = Путь к .po файлу для проверки
help-validatepo-strict = Строгий режим: вернуть ошибку (код 1), если найдены несовпадения
help-validatepo-format = Формат вывода: «text» (по умолчанию) или «json»

# export-po
help-exportpo-about = Экспортировать извлечённые строки в единый .po файл
help-exportpo-root = Путь к корню мода RimWorld с извлечёнными строками
help-exportpo-out-po = Путь к выходному .po файлу
help-exportpo-lang = ISO-код языка перевода (например, ru, ja, de)
help-exportpo-pot = Вместо локализованного PO записать POT-шаблон (пустой заголовок Language)
help-exportpo-source-lang = ISO-код исходного языка для экспорта (например, en, ru, ja)
help-exportpo-source-lang-dir = Имя папки исходного языка (например, English). Перекрывает --source-lang
help-exportpo-tm-root = Путь(и) к базам переводов (флаг повторяемый). Каждая база: Languages/<язык> или корень мода. Автозаполняет msgstr и помечает fuzzy
help-exportpo-game-version = Папка версии игры для сканирования (например, 1.6 или v1.6); по умолчанию — самая новая
help-exportpo-include-all = Включить все подпапки версий (может привести к дублям)

# import-po
help-importpo-about = Импорт .po  -  в один XML или по структуре существующего мода
help-importpo-po = Путь к .po файлу для импорта
help-importpo-out-xml = Путь выходного XML (режим одного файла)
help-importpo-mod-root = Корень мода для обновления импортированными строками (структурный режим)
help-importpo-lang = Код целевого языка для импорта (например, ru)
help-importpo-lang-dir = Папка целевого языка (переопределяет авто‑соответствие)
help-importpo-keep-empty = Импортировать пустые строки как заглушки
help-importpo-game-version = Подпапка версии игры, в которую писать (например, 1.6 или v1.6); по умолчанию выбирается последняя, если есть
help-importpo-single-file = Записать все импортированные строки в один XML‑файл
help-importpo-backup = Создавать .bak‑резервные копии при перезаписи
help-importpo-dry-run = Ничего не записывать; только показать план действий
help-importpo-format = Формат вывода для отчётов/DRY‑RUN: «text» (по умолчанию) или «json»
help-importpo-report = Показать сводку: создано/обновлено/пропущено файлов и всего записано ключей
help-importpo-incremental = Не перезаписывать файлы, если содержимое не изменилось
import-report-summary = Сводка импорта: создано={ $created }, обновлено={ $updated }, пропущено={ $skipped }, ключей={ $keys }
help-importpo-only-diff = Записывать только изменённые/новые ключи по файлам (пропускать неизменённые)

# build-mod
help-buildmod-about = Собрать отдельный мод‑перевод из .po файла
help-buildmod-po = Путь к .po файлу для сборки
help-buildmod-out-mod = Путь выходной папки мода
help-buildmod-lang = Код языка перевода
help-buildmod-from-root = Собрать из уже существующей структуры Languages/<язык> в этом корне вместо .po
help-buildmod-from-game-version = При --from-root учитывать только файлы внутри перечисленных подпапок версий (список через запятую)
help-buildmod-name = Отображаемое имя мода перевода
help-buildmod-package-id = PackageId мода перевода
help-buildmod-rw-version = Целевая версия RimWorld
help-buildmod-lang-dir = Имя языковой папки внутри мода (необязательно)
help-buildmod-dry-run = Ничего не записывать; только вывести план сборки
help-buildmod-dedupe = Удалять дублирующиеся ключи в одном XML (последний имеет приоритет)

# diff-xml
help-diffxml-about = Сравнить присутствие ключей в исходнике и переводе; при наличии baseline PO найти изменившиеся исходные строки
help-diffxml-root = Путь к корню мода RimWorld для анализа
help-diffxml-source-lang = ISO-код исходного языка (соответствует папке RimWorld)
help-diffxml-source-lang-dir = Имя папки исходного языка (например, English). Перекрывает --source-lang
help-diffxml-lang = ISO-код языка перевода (соответствует папке RimWorld)
help-diffxml-lang-dir = Имя папки языка перевода (например, Russian). Перекрывает --lang
help-diffxml-baseline-po = Базовый PO (предыдущий экспорт) для выявления изменившихся исходных строк
help-diffxml-format = Формат вывода: «text» (по умолчанию) или «json»
help-diffxml-out-dir = Папка для записи txt-отчётов (ChangedData.txt, TranslationData.txt, ModData.txt)
help-diffxml-game-version = Папка версии игры для анализа (например, 1.6 или v1.6); по умолчанию выбирается самая новая
help-diffxml-strict = Строгий режим: вернуть ошибку, если найдены отличия

diffxml-saved = Результаты diff сохранены в { $path }
diffxml-summary = Сводка diff: изменившиеся={ $changed }, только-в-переводе={ $only_trg }, только-в-моде={ $only_src }

# annotate
help-annotate-about = Добавлять или удалять комментарии с оригинальным текстом в переводных XML
help-annotate-root = Путь к корню мода RimWorld
help-annotate-source-lang = ISO-код исходного языка (например, en); сопоставляется с именем папки
help-annotate-source-lang-dir = Имя папки исходного языка (например, English). Перекрывает --source-lang
help-annotate-lang = ISO-код языка перевода (например, ru)
help-annotate-lang-dir = Имя папки языка перевода (например, Russian). Перекрывает --lang
help-annotate-dry-run = Ничего не записывать; только показать какие файлы изменятся
help-annotate-backup = Создавать .bak перед перезаписью XML
help-annotate-strip = Удалять существующие комментарии вместо добавления новых
help-annotate-game-version = Папка версии игры под корнем мода (например, 1.6 или v1.6)
help-annotate-comment-prefix = Префикс комментария перед оригинальным текстом (по умолчанию: "EN:")
annotate-would-write = DRY-RUN: добавили бы комментарии в { $path }
annotate-summary = Аннотирование завершено. Обработано={ $processed }, прокомментировано={ $annotated }

# xml-health
help-xmlhealth-about = Проверить XML под Languages/ на ошибки чтения/структуры
help-xmlhealth-root = Путь к корню мода RimWorld для сканирования
help-xmlhealth-format = Формат вывода: «text» (по умолчанию) или «json»
help-xmlhealth-lang-dir = Ограничить проверку конкретной языковой папкой (например, Russian)
help-xmlhealth-strict = Строгий режим: вернуть ошибку при наличии проблем
help-xmlhealth-only = Список категорий через запятую для включения (parse,tag-mismatch,invalid-char)
help-xmlhealth-except = Список категорий через запятую для исключения
xmlhealth-summary = Проверка XML: проблем не обнаружено
xmlhealth-issues = Проверка XML: обнаружены проблемы (см. список выше)
xmlhealth-issue-line = { $path } — { $error }

# init
help-init-about = Создать заготовку перевода в Languages/<язык> с пустыми значениями
help-init-root = Путь к корню мода RimWorld
help-init-source-lang = ISO-код исходного языка (например, en)
help-init-source-lang-dir = Имя папки исходного языка (например, English). Перекрывает --source-lang
help-init-lang = ISO-код языка перевода (например, ru)
help-init-lang-dir = Имя папки языка перевода (например, Russian). Перекрывает --lang
help-init-overwrite = Перезаписывать существующие файлы, если они есть
help-init-dry-run = Ничего не записывать; показать только план
help-init-game-version = Папка версии игры (например, 1.6 или v1.6)
init-summary = Init завершён: создано файлов — { $count } для { $lang }

# lang-update
help-langupdate-about = Обновить официальную локализацию из GitHub в Data/Core/Languages
help-langupdate-game-root = Путь к корню игры RimWorld (содержит папку Data)
help-langupdate-repo = Репозиторий GitHub в формате owner/name (по умолчанию Ludeon/RimWorld-ru)
help-langupdate-branch = Имя ветки для загрузки (если не задано — ветка по умолчанию)
help-langupdate-zip = Локальный zip‑архив вместо загрузки из сети
help-langupdate-source-lang-dir = Имя папки исходного языка в репозитории (например, Russian)
help-langupdate-target-lang-dir = Имя целевой папки языка под Data/Core/Languages (например, Russian (GitHub))
help-langupdate-dry-run = Ничего не записывать; только показать план
help-langupdate-backup = Создавать резервную копию (.bak) целевой папки перед записью
langupdate-dry-run-header = === DRY RUN: обновление локализации ===
langupdate-dry-run-line = { $path }  ({ $size } байт)
langupdate-summary = Локализация обновлена: файлов={ $files }, байт={ $bytes }, путь={ $out }

# === scan ===
test-csv-header = CSV заголовок должен присутствовать

# === проверка стартового сообщения ===
test-startup-text-must-appear = Стартовое сообщение должно появляться для локали { $loc }
# morph
help-morph-about = Сгенерировать файлы Case/Plural/Gender с помощью провайдера морфологии (dummy/morpher/pymorphy2)
help-morph-root = Путь к корню мода RimWorld
help-morph-provider = Провайдер: dummy (по умолчанию), morpher или pymorphy2
xmlhealth-hint-line = подсказка: { $hint }
help-morph-lang = ISO-код языка перевода
help-morph-lang-dir = Имя папки языка перевода
help-morph-filter = Регулярное выражение для фильтрации ключей (Keyed)
help-morph-limit = Ограничить число обрабатываемых ключей
help-morph-game-version = Подпапка версии игры
help-morph-timeout = Таймаут HTTP для провайдеров, мс (по умолчанию 1500)
help-morph-cache-size = Размер кэша провайдера (по умолчанию 1024)
help-morph-pym-url = URL сервиса Pymorphy2 (перекрывает PYMORPHY_URL)
morph-summary = Сгенерировано форм: { $processed } для { $lang }
morph-provider-morpher-stub = Провайдер Morpher API пока не реализован; применяется dummy-логика
