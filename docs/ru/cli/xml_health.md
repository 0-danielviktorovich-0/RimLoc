---
title: XML Health
---

# Команда XML Health

Проверяет XML под `Languages/` на ошибки структуры/чтения. Удобно в CI с флагом `--strict`.

## Синопсис

```bash
rimloc-cli xml-health --root <MOD> [--format text|json] [--lang-dir <DIR>] [--strict]
```

## Опции
- `--root <MOD>`: корень мода (обязательно)
- `--format`: text (по умолчанию) или json
- `--lang-dir <DIR>`: ограничить конкретной языковой папкой
- `--strict`: ненулевой код выхода при найденных проблемах

