---
title: RimLoc
---

# RimLoc

[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://0-danielviktorovich-0.github.io/RimLoc/)

RimLoc — это набор инструментов для работы с переводами RimWorld.

- Сканирует XML (`Keyed/DefInjected`) и извлекает единицы перевода.
- Проверка (повторяющиеся ключи, пустые строки, несоответствия плейсхолдеров).
- Экспорт в **PO/CSV**, импорт **PO** обратно в целевой мод (поддерживается режим тестирования).
- Весь вывод CLI локализован с помощью **Fluent**; ресурсы встроены с использованием **rust-embed**.

## Документация

Начните с [обзора CLI](cli/index.md) или переходите сразу к командам:

- [Сканирование XML](cli/scan.md)
- [Проверка (Validate)](cli/validate.md)
- [Проверка PO](cli/validate_po.md)
- [Экспорт / Импорт](cli/export_import.md)

!!! tip "Быстрый старт"
    ```bash
    cargo build -p rimloc-cli
    cargo run -p rimloc-cli -- --help
    ```

### Полезные примеры

```bash
# Сканировать мод и вывести JSON
cargo run -p rimloc-cli -- scan --root ./test/TestMod --format json | jq .

# Проверить мод (читаемый вывод)
cargo run -p rimloc-cli -- validate --root ./test/TestMod

# Проверить плейсхолдеры в PO (JSON)
cargo run -p rimloc-cli -- validate-po --po ./test/bad.po --format json | jq .
```

---
## Вклад в документацию

Нашли опечатку или хотите добавить примеры? [Отредактируйте эту страницу на GitHub](https://github.com/0-danielviktorovich-0/RimLoc/tree/main/docs/ru/index.md).
