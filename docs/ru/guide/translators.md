---
title: Для переводчиков — пошагово
---

# Перевод мода с RimLoc (без стресса)

Эта страница объясняет, как перевести мод даже без опыта работы с терминалом. Короткие шаги, команды для копирования, простые пояснения.

Что мы сделаем:
- Просканируем мод и найдём строки для перевода
- Экспортируем единый .po для Poedit (или любого редактора PO)
- Проверим плейсхолдеры (чтобы не сломать строки)
- Импортируем переводы обратно в XML
- Соберём отдельный мод‑перевод для теста в игре

Подготовка (один раз)
1) Установите Rust (даст `cargo` для установки rimloc):
   - Windows/macOS/Linux: https://www.rust-lang.org/tools/install
2) Установите RimLoc CLI:
   - Откройте терминал (Windows: PowerShell) и выполните: `cargo install rimloc-cli`

Сделать RimLoc удобнее (рекомендуется)
- Создайте `rimloc.toml` рядом с модом. Минимум:

```
source_lang = "English"
target_lang = "Russian"
game_version = "1.5"
list_limit = 100
```

Шаг 1 — Выберите папку мода
- Пример пути: `C:/RimMods/MyCoolMod` (Windows) или `~/RimMods/MyCoolMod` (Linux/macOS)

Шаг 2 — Сканирование и проверка
```
rimloc-cli scan --root "C:/RimMods/MyCoolMod" --format json > scan.json
rimloc-cli validate --root "C:/RimMods/MyCoolMod"
```

Шаг 3 — Экспорт .po
```
rimloc-cli export-po --root "C:/RimMods/MyCoolMod" --out-po "C:/RimMods/MyCoolMod.ru.po" --lang ru
```
Откройте .po в Poedit и переводите. Сохраняйте плейсхолдеры (`%d`, `{NAME}` и т.п.).

Шаг 4 — Проверка плейсхолдеров в PO (по желанию)
```
rimloc-cli validate-po --po "C:/RimMods/MyCoolMod.ru.po"
```

Шаг 5 — Импорт переводов обратно
Вариант A (один файл):
```
rimloc-cli import-po --po "C:/RimMods/MyCoolMod.ru.po" --out-xml "C:/RimMods/_Imported.xml" --dry-run
rimloc-cli import-po --po "C:/RimMods/MyCoolMod.ru.po" --out-xml "C:/RimMods/_Imported.xml"
```
Вариант B (по структуре мода):
```
rimloc-cli import-po --po "C:/RimMods/MyCoolMod.ru.po" --mod-root "C:/RimMods/MyCoolMod" --lang ru --report --dry-run
rimloc-cli import-po --po "C:/RimMods/MyCoolMod.ru.po" --mod-root "C:/RimMods/MyCoolMod" --lang ru --report
```

Шаг 6 — Собрать отдельный мод‑перевод
Из .po:
```
rimloc-cli build-mod --po "C:/RimMods/MyCoolMod.ru.po" --out-mod "C:/RimMods/MyCoolMod_RU" --lang ru --dry-run
rimloc-cli build-mod --po "C:/RimMods/MyCoolMod.ru.po" --out-mod "C:/RimMods/MyCoolMod_RU" --lang ru
```
Из готовой структуры Languages/Russian:
```
rimloc-cli build-mod --from-root "C:/RimMods/MyCoolMod" --out-mod "C:/RimMods/MyCoolMod_RU" --lang ru --dry-run
rimloc-cli build-mod --from-root "C:/RimMods/MyCoolMod" --out-mod "C:/RimMods/MyCoolMod_RU" --lang ru
```

Тест в игре
- Перенесите `MyCoolMod_RU` в папку `Mods` RimWorld, включите в списке модов и выберите язык Russian.

Подсказки, если терминал пугает
- Копируйте команды как есть; пути берем в кавычки "..."
- На Windows используйте PowerShell, на macOS — «Terminal», на Linux — любой терминал
- С флагом `--dry-run` RimLoc только показывает план без изменений

Вопросы и ответы
- «rimloc-cli не найден» — откройте новый терминал после установки `cargo install`, проверьте PATH (`~/.cargo/bin`)
- «Плейсхолдеры не совпадают» — исправьте строку в Poedit, снова запустите `validate-po`
- «Нечего импортировать» — все строки пустые; используйте `--keep-empty`, если нужно заложить заглушки

