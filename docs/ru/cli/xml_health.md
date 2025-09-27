---
title: XML Health
---

# Команда XML Health

Проверяет XML под `Languages/` на ошибки структуры/чтения. Удобно в CI с флагом `--strict`.

## Синопсис

```bash
rimloc-cli xml-health --root <MOD> [--format text|json] [--lang-dir <DIR>] [--strict] \
  [--only <CSV>] [--except <CSV>]
```

## Опции
- `--root <MOD>`: корень мода (обязательно)
- `--format`: text (по умолчанию) или json
- `--lang-dir <DIR>`: ограничить конкретной языковой папкой
- `--strict`: ненулевой код выхода при найденных проблемах
- `--only`: включить только указанные категории (CSV)
- `--except`: исключить указанные категории (CSV)

### Категории

- `encoding` — файл не читается как UTF‑8
- `encoding-detected` — объявлена кодировка отличная от UTF‑8 в `<?xml ... encoding=...?>`
- `invalid-char` — присутствуют управляющие символы < 0x20 (кроме TAB/LF/CR)
- `tag-mismatch` — несоответствие/перепутанные XML‑теги
- `invalid-entity` — некорректные сущности/экранирование (например, «голый» `&`)
- `unexpected-doctype` — найден `<!DOCTYPE ...>` (не нужен для LanguageData)

В текстовом выводе добавлены короткие подсказки по исправлению типовых проблем.
