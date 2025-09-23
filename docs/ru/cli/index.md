---
title: CLI команды
---

# Обзор команд CLI

## Описание

RimLoc CLI предоставляет инструменты для сканирования, проверки и конвертации переводов.

## Команды

| Команда         | Описание                                                        | Ссылка                   |
|-----------------|-----------------------------------------------------------------|--------------------------|
| Scan            | Извлекает единицы перевода из файлов `Keyed/DefInjected`.       | [scan](scan.md)          |
| Validate        | Проверяет на дубликаты, пустые строки и ошибки в заполнителях.  | [validate](validate.md)  |
| Validate PO     | Сравнивает заполнители между `msgid` и `msgstr`.                | [validate_po](validate_po.md) |
| Export / Import | Конвертирует в/из форматов PO/CSV.                              | [export_import](export_import.md) |

---

## Примеры

```bash
# Показать справку
cargo run -p rimloc-cli -- --help
```
Отображает справочную информацию о CLI.

```bash
# Сканирование (по умолчанию CSV)
cargo run -p rimloc-cli -- scan --root ./test/TestMod
```
Сканирует указанную директорию мода и извлекает единицы перевода.

```bash
# Проверка (JSON)
cargo run -p rimloc-cli -- validate --root ./test/TestMod --format json | jq .
```
Проверяет переводы и выводит результаты в формате JSON. Для удобства чтения результат передаётся через `jq`.

```bash
# Строгая проверка PO (код выхода 1 при несоответствии)
cargo run -p rimloc-cli -- validate-po --po ./test/bad.po --strict
```
Проверяет PO-файл в строгом режиме; при обнаружении несоответствий возвращает код выхода 1.

---

## См. также

- [Scan](scan.md)
- [Validate](validate.md)
- [Validate PO](validate_po.md)
- [Export / Import](export_import.md)