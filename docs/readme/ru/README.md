# RimLoc

<p align="center">
  <img src="../../assets/RIMLOC-baner.png" alt="RimLoc баннер" />
 </p>

[English version](../../README.md)

[![Build](https://github.com/0-danielviktorovich-0/RimLoc/actions/workflows/build.yml/badge.svg)](https://github.com/0-danielviktorovich-0/RimLoc/actions/workflows/build.yml) [![Crates.io](https://img.shields.io/badge/crates.io-rimloc--cli-blue?logo=rust&logoColor=white)](https://crates.io/crates/rimloc-cli) [![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://0-danielviktorovich-0.github.io/RimLoc/) [![License](https://img.shields.io/badge/license-GNU%20GPL-blue)](../../LICENSE)

RimLoc — это инструмент на Rust для локализации и управления переводами модов RimWorld. Он объединяет извлечение строк, проверку качества и экспорт в PO/CSV в едином рабочем процессе на Linux, macOS и Windows.

## Почему RimLoc?

- Автоматически находит все строки `Keyed`/`DefInjected` и держит их в актуальном виде.
- Предупреждает о дубликатах, пустых значениях и несоответствиях плейсхолдеров до релиза.
- Конвертирует XML в удобные для переводчиков форматы PO и CSV и обратно.
- Сразу собирает отдельный мод-перевод из готового `.po` файла.
- CLI уже локализован (английский и русский) и использует стек Fluent.

## Быстрый старт за 5 минут

```bash
cargo install rimloc-cli
git clone https://github.com/0-danielviktorovich-0/RimLoc.git
cd RimLoc
rimloc-cli scan --root ./test/TestMod --format json | jq '.[0]'
rimloc-cli validate --root ./test/TestMod
rimloc-cli export-po --root ./test/TestMod --out-po ./logs/TestMod.po --lang ru
rimloc-cli build-mod --po ./logs/TestMod.po --out-mod ./logs/TestMod-ru --lang ru --dry-run
```

1. Установите CLI из crates.io.
2. Воспользуйтесь тестовым модом `test/TestMod` (или своим модом).
3. `scan` выводит найденные строки; с `jq` удобно смотреть структуру.
4. `validate` подсвечивает пустые значения, дубликаты и ошибки плейсхолдеров (код возврата 1 при ошибках).
5. `export-po` формирует единый `.po` для передачи переводчикам.
6. `build-mod` в режиме `--dry-run` показывает, каким будет готовый мод-перевод.

Нужно подготовить пакет для переводчиков?

```bash
rimloc-cli export-po --root ./test/TestMod --out-po ./logs/TestMod.po --lang ru
```

Хотите собрать отдельный мод-перевод?

```bash
rimloc-cli build-mod --po ./logs/TestMod.po --out-mod ./logs/TestMod-ru --lang ru
```

## Основные команды

| Команда | Когда использовать | Пример |
|---------|--------------------|--------|
| `rimloc-cli scan` | Собрать строки из модов в CSV или JSON. | `rimloc-cli scan --root ./path/to/mod --format json --out-json ./logs/scan.json` |
| `rimloc-cli validate` | Проверить XML на дубликаты, пустоты и плейсхолдеры. | `rimloc-cli validate --root ./path/to/mod --format text` |
| `rimloc-cli validate-po` | Убедиться, что переводы в PO сохранили плейсхолдеры. | `rimloc-cli validate-po --po ./translations/ru.po --strict` |
| `rimloc-cli export-po` | Подготовить единый PO-файл для переводчиков. | `rimloc-cli export-po --root ./path/to/mod --out-po ./out/mymod.po --lang ru` |
| `rimloc-cli import-po` | Вернуть переводы из PO обратно в XML. | `rimloc-cli import-po --po ./out/mymod.po --mod-root ./path/to/mod --dry-run` |
| `rimloc-cli build-mod` | Собрать автономный мод-перевод. | `rimloc-cli build-mod --po ./out/mymod.po --out-mod ./ReleaseMod --lang ru` |

### Версионные папки

Если мод использует поддиректории версий (например, `1.4`, `1.5`, `v1.6`), можно явно выбрать нужную или обработать все:

```bash
# Сканировать конкретную версию (если не указано — выбирается последняя)
rimloc-cli scan --root ./Mods/MyMod --game-version 1.4

# Проверить все версии под корнем мода
rimloc-cli validate --root ./Mods/MyMod --include-all-versions

# Экспорт с учётом версионной структуры
rimloc-cli export-po --root ./Mods/MyMod --out-po ./out/MyMod.po --game-version v1.6
```

<!-- Демо и скриншоты будут добавлены после записи первого walkthrough. -->

## Документация и поддержка

- Полная документация: [RimLoc Docs](https://0-danielviktorovich-0.github.io/RimLoc/)
- Справка по командам лежит в `docs/en/cli/` и `docs/ru/cli/`.
- Примеры и фикстуры для экспериментов находятся в каталоге `test/`.
- Сообщить об ошибке или предложить улучшение можно через [Issues](https://github.com/0-danielviktorovich-0/RimLoc/issues).

## Как помочь проекту

Для новых контрибьюторов есть гайд [AGENTS.md](../../AGENTS.md) — там описаны структура репозитория, инструменты и правила ревью.

Хотите обновить документацию? Запустите `mkdocs serve` и редактируйте файлы в `docs/`, синхронизируя английскую и русскую версии.

---

## Лицензия

GNU GPL — см. [LICENSE](../../LICENSE).
