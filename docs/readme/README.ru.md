

# RimLoc

[English](../../README.md) | [Русский](README.ru.md)

[![Build](https://github.com/0-danielviktorovich-0/RimLoc/actions/workflows/build.yml/badge.svg)](https://github.com/0-danielviktorovich-0/RimLoc/actions/workflows/build.yml) [![Crates.io](https://img.shields.io/crates/v/rimloc)](https://crates.io/crates/rimloc) [![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://0-danielviktorovich-0.github.io/RimLoc/) [![License](https://img.shields.io/badge/license-GNU%20GPL-blue)](LICENSE)

RimLoc — это инструмент на Rust для локализации и управления переводами модов RimWorld. Он помогает моддерам сканировать XML, проверять качество переводов и экспортировать/импортировать их в PO/CSV на Linux, macOS и Windows.

## Установка

```bash
cargo install rimloc-cli
```

## Возможности

- Сканирование XML RimWorld и извлечение переводимых единиц  
- Проверка дубликатов, пустых строк, плейсхолдеров  
- Экспорт/импорт в PO / CSV  
- Локализация CLI с помощью Fluent (английский + русский)  

## Пример

```
rimloc-cli scan --root ./TestMod
```

### Demo (asciinema)

[![asciicast](https://asciinema.org/a/your-demo-id.svg)](https://asciinema.org/a/your-demo-id)

### Screenshot

![CLI validation example](../demo-validation.png)

<!-- TODO: Add screenshot or asciinema demo of CLI output once available -->

## Документация

👉 Полная документация: [RimLoc Docs](https://0-danielviktorovich-0.github.io/RimLoc/)

---

## Лицензия

GNU GPL — см. [LICENSE](LICENSE).