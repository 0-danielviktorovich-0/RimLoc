---
title: Для переводчиков — пошагово
---

# Перевод мода с RimLoc (без стресса) 🎯

Эта страница объясняет, как перевести мод даже без опыта работы с терминалом. Короткие шаги, команды для копирования, простые пояснения.

[:material-download: Установка](../install.md){ .md-button .md-button--primary }
[:material-play-circle: Запуск скачанной сборки](../install_run.md){ .md-button }
[:material-github: Релизы на GitHub](https://github.com/0-danielviktorovich-0/RimLoc/releases){ .md-button }

Что мы сделаем:
- Просканируем мод и найдём строки для перевода
- Экспортируем единый .po для Poedit (или любого редактора PO)
- Проверим плейсхолдеры (чтобы не сломать строки)
- Импортируем переводы обратно в XML
- Соберём отдельный мод‑перевод для теста в игре

Как получить RimLoc CLI (выберите один вариант)
- Вариант A — Скачать готовую сборку (рекомендуется): откройте страницу Установка и следуйте гайду запуска.
  - Гайд: Установка → Запуск скачанной сборки.
  - Windows: запускайте из папки `.\\rimloc-cli`; macOS/Linux: `./rimloc-cli`.
- Вариант B — Установка через Cargo (нужен Rust):
  - Поставьте Rust: https://www.rust-lang.org/tools/install
  - Затем: `cargo install rimloc-cli`

Зачем это знать?
- Если вы скачали релизную или dev‑сборку, вам НЕ нужны Rust и Cargo.
- В обоих случаях команда одна и та же: `rimloc-cli`.

Простым языком (микро‑глоссарий)
- Терминал: окно, где выполняются команды. Windows → PowerShell; macOS → Terminal; Linux → любой терминал.
- Корень мода: верхняя папка мода (ту, что копируют в RimWorld `Mods/`).
- PO‑файл: один файл перевода с парами «исходник → перевод», открывается в Poedit.
- Плейсхолдер: кусочки вроде `%d`, `%s`, `{NAME}` — их нельзя менять; RimLoc умеет проверять их.
- Dry‑run: безопасный прогон без изменений — показывает план действий.

Сделать RimLoc удобнее (рекомендуется)
Почему это помогает?
- Конфиг `rimloc.toml` экономит ввод флагов: RimLoc подставляет дефолты за вас. Создайте файл рядом с модом. Минимум:

```
source_lang = "English"
target_lang = "Russian"
game_version = "1.5"
list_limit = 100
```

Шаг 1 — Выберите папку мода 📁
- Пример пути: `C:/RimMods/MyCoolMod` (Windows) или `~/RimMods/MyCoolMod` (Linux/macOS)
Зачем?
- Командам нужен корневой каталог мода, чтобы найти папки `Languages/*` и XML‑файлы `Keyed`/`DefInjected`.

Шаг 2 — Сканирование и проверка 🔎✅
```
rimloc-cli scan --root "C:/RimMods/MyCoolMod" --format json > scan.json
rimloc-cli validate --root "C:/RimMods/MyCoolMod"
```
Зачем два шага?
- `scan` просто перечисляет всё, что подлежит переводу (можно сохранить в `scan.json` и посмотреть глазами).
- `validate` заранее показывает типичные проблемы: пустые строки, дубликаты ключей и подозрительные плейсхолдеры. Это экономит время и снижает риск «сломать» перевод.

Шаг 3 — Экспорт .po 📤📝
```
rimloc-cli export-po --root "C:/RimMods/MyCoolMod" --out-po "C:/RimMods/MyCoolMod.ru.po" --lang ru
```
Откройте .po в Poedit и переводите. Сохраняйте плейсхолдеры (`%d`, `{NAME}` и т.п.).
Зачем?
- Получаете один удобный файл для Poedit/локализаторов. В нём есть исходный текст и поле для перевода — не нужно бегать по множеству XML.

Шаг 4 — Проверка плейсхолдеров в PO (по желанию) 🧪
```
rimloc-cli validate-po --po "C:/RimMods/MyCoolMod.ru.po"
```
Зачем?
- Плейсхолдеры (`%d`, `%s`, `{NAME}`) — «дырки», куда игра подставляет числа/имена. Если их удалить/исказить, строка сломается. Проверка ловит такие случаи до импорта.

Шаг 5 — Импорт переводов обратно ⬅️📄
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

Шаг 6 — Собрать отдельный мод‑перевод 📦
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
Почему это важно?
- Только так можно увидеть, как строки выглядят «в бою»: переносы, длины, контекст. Если что-то не так — вернитесь к .po, поправьте и снова импортируйте.

Подсказки, если терминал пугает
- Копируйте команды как есть; пути берём в кавычки "...".
- Windows: PowerShell; macOS: Terminal; Linux: любой терминал.
- Добавляйте `--dry-run`, чтобы сначала посмотреть план без изменений.

Вопросы и ответы
- «rimloc-cli не найден»
  - Устанавливали через Cargo: откройте новый терминал или проверьте PATH (`~/.cargo/bin`, в Windows `%USERPROFILE%\\.cargo\\bin`).
  - Скачивали релиз: запускайте из папки, куда распаковали — `.\\rimloc-cli` (Windows) или `./rimloc-cli` (macOS/Linux), либо добавьте папку в PATH.
- «Плейсхолдеры не совпадают» — исправьте строку в Poedit, снова запустите `validate-po`.
- «Нечего импортировать» — все строки пустые; используйте `--keep-empty`, если нужно заложить заглушки.
Что выбрать и почему?
- Вариант A — всё в `_Imported.xml`. Проще для ревью/быстрой проверки.
- Вариант B — правильно для релиза: строки раскладываются по тем же файлам/папкам, что у оригинала. Легче сопровождать и обновлять.
Зачем?
- Это «упаковка» перевода в самостоятельный мод, который можно включить в игре, не меняя оригинал. Подходит для публикации и командной работы.
