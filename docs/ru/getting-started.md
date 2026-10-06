---
title: Начало работы
---

# Начало работы с RimLoc

У RimLoc есть десктопный project workflow и CLI. Для обычного переводчика основным путём должен быть GUI; CLI нужен для автоматизации, CI, диагностики и обмена с внешними CAT-инструментами.

!!! warning "Pre-beta"
    React-интерфейс ещё проходит hardening. Если тестируете development build, убедитесь, что artifact явно относится к React/REACT_PROD, а не к legacy fallback.

## Вариант A — Desktop workflow

### 1. Создайте или откройте проект

На Home:

- **Новый перевод** — новый источник;
- **Открыть/обновить существующий** — если перевод уже есть и его надо сохранить при обновлении источника.

Для RimWorld адаптер может представлять разные типы источников: мод, Core/DLC, language pack, существующий перевод — в зависимости от реально включённых capabilities сборки.

### 2. Выберите целевой язык

Один RimLoc-проект может содержать несколько target locale. Source inventory общий, а переводы изолированы по языкам.

### 3. Переводите прямо в RimLoc

Редактируйте target-текст в workspace. PO для этого не нужен.

Project state и revisions сохраняются в канонической модели RimLoc.

### 4. Validation и review

Перед сборкой пройдите Checks/validation. Исправьте findings по плейсхолдерам, структуре, source drift и другим правилам адаптера.

### 5. Build/export

Выбирайте отдельный output-каталог. Исходные каталоги игры/мода считаются read-only.

Конкретные действия build/export зависят от capabilities адаптера.

## Вариант B — CLI

Для автоматизации можно начать с встроенной фикстуры:

~~~bash
cargo build -p rimloc-cli
cargo run -p rimloc-cli -- scan --root ./test/TestMod --format json
cargo run -p rimloc-cli -- validate --root ./test/TestMod
~~~

### PO — опционально

PO удобно использовать с Poedit/CAT:

~~~bash
cargo run -p rimloc-cli -- export-po \
  --root ./test/TestMod \
  --out-po ./logs/TestMod.po \
  --lang ru
~~~

Импорт обратно — отдельный interoperability workflow:

~~~bash
cargo run -p rimloc-cli -- import-po \
  --po ./logs/TestMod.po \
  --mod-root ./test/TestMod \
  --lang ru \
  --dry-run
~~~

### Сборка без PO

Если уже есть готовое дерево RimWorld <code>Languages/&lt;язык&gt;</code>:

~~~bash
cargo run -p rimloc-cli -- build-mod \
  --from-root ./Mods/MyTranslatedMod \
  --out-mod ./dist/MyTranslatedMod-RU \
  --lang ru \
  --dry-run
~~~

Поэтому PO — формат обмена, а не обязательное внутреннее хранилище RimLoc.

## Что дальше

- [Desktop GUI](guide/gui.md)
- [Гайд переводчика](guide/translators.md)
- [CLI](cli/index.md)
- [Обновление существующего перевода](tutorials/update_translations.md)
- [Решение проблем](troubleshooting.md)
