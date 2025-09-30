---
title: Начало работы
---

# 🚀 Начало работы с RimLoc

RimLoc — ваш умный помощник для переводов RimWorld. Он собирает строки из модов, проверяет ошибки и помогает быстро готовить переводы. В этом разделе — простая пошаговая дорожная карта для новичка.

📌 Не знаете термин? Загляните в [Словарь RimLoc](glossary.md).

## 1) Установка (1–2 минуты)

- Через Cargo:
  ```bash
  cargo install rimloc-cli
  ```
- Или скачайте готовый бинарник из Releases и запустите его напрямую.

Подробнее: install.md · install_run.md

## 2) Первый запуск — проверим мод

Пусть ваш мод лежит в `./Mods/MyMod`.

```bash
rimloc-cli scan --root ./Mods/MyMod --format json > scan.json
rimloc-cli validate --root ./Mods/MyMod
```

- `scan` инвентаризирует строки и сохраняет их в `scan.json` (для быстрого взгляда).
- `validate` ищет типичные проблемы: пустые строки, дубликаты ключей и ошибки [плейсхолдеров](glossary.md#плейсхолдер).

💡 Совет: Добавляйте `--format json`, если хотите использовать вывод в CI или делиться отчётом.

## 3) Экспорт `.po` для переводчика

```bash
rimloc-cli export-po --root ./Mods/MyMod --out-po ./MyMod.ru.po --lang ru
```

Откройте `MyMod.ru.po` в Poedit или любимом CAT-инструменте и переводите.

📌 Подробнее см. в разделе: cli/export_import.md

## 4) Проверка `.po` перед импортом (рекомендуется)

```bash
rimloc-cli validate-po --po ./MyMod.ru.po --strict --format text
```

Так вы заранее поймаете несоответствия [плейсхолдеров](glossary.md#плейсхолдер) между `msgid` и `msgstr`.

📌 Подробнее: cli/validate_po.md

## 5) Импорт перевода обратно в мод

Вариант A: один файл (удобно для ревью)
```bash
rimloc-cli import-po --po ./MyMod.ru.po --out-xml ./Mods/MyMod/_Imported.xml --dry-run
rimloc-cli import-po --po ./MyMod.ru.po --out-xml ./Mods/MyMod/_Imported.xml
```

Вариант B: разложить по структуре мода (правильно для релиза)
```bash
rimloc-cli import-po --po ./MyMod.ru.po --mod-root ./Mods/MyMod --lang ru --report --dry-run
rimloc-cli import-po --po ./MyMod.ru.po --mod-root ./Mods/MyMod --lang ru --report
```

## 6) Собрать отдельный мод‑перевод (по желанию)

Из `.po`:
```bash
rimloc-cli build-mod --po ./MyMod.ru.po --out-mod ./MyMod_RU --lang ru --dry-run
rimloc-cli build-mod --po ./MyMod.ru.po --out-mod ./MyMod_RU --lang ru
```

📌 Подробнее: cli/build_mod.md

## 7) Что дальше?

- Перевод с нуля: tutorials/translate_mod.md
- Обновление старого перевода: tutorials/update_translations.md
- Частые вопросы: faq.md
- Если что-то пошло не так: troubleshooting.md

!!! tip "Почему RimLoc экономит время?"
    Вы не бегаете по XML. Один экспорт, один импорт, одна проверка — меньше рутины, больше фокуса на тексте.

