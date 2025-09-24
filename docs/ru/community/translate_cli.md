---
title: Перевод RimLoc (без кода)
---

# Перевод RimLoc (без кода)

Помогите сделать RimLoc доступным на вашем языке! Ниже — короткая и дружелюбная инструкция. Перевод делается в обычных текстовых файлах (Fluent/FTL) и может быть выполнен прямо в браузере, без установки Rust.

## Что переводим

В каждой локали два файла:

- `crates/rimloc-cli/i18n/<lang>/rimloc.ftl` — тексты CLI (help и сообщения)
- `crates/rimloc-cli/i18n/<lang>/rimloc-tests.ftl` — небольшие тестовые строки

Источник истины — английские файлы: `crates/rimloc-cli/i18n/en/`.

## Самый быстрый способ (через GitHub)

1) Откройте папку EN: `crates/rimloc-cli/i18n/en/`
2) Создайте рядом папку с кодом вашего языка (например, `es`, `de`, `fr`).
3) Скопируйте из `en/` оба файла в новую папку:
   - `rimloc.ftl`
   - `rimloc-tests.ftl`
4) Переведите только значения — ключи и плейсхолдеры не меняйте.
5) Нажмите “Commit changes” → “Create pull request”. В PR укажите код языка.

Готово! CI запустит проверки; если чего‑то не хватает, мы подскажем.

## Хотите локально? (опционально)

```bash
git clone https://github.com/0-danielviktorovich-0/RimLoc.git
cd RimLoc
# копируем en → <lang>
cp -R crates/rimloc-cli/i18n/en crates/rimloc-cli/i18n/<lang>
# редактируем файлы в crates/rimloc-cli/i18n/<lang>

# (опционально) локальная проверка
cargo test --package rimloc-cli -- tests_i18n
cargo test --workspace
```

Проверить локализованный help (нужен установленный Rust):

```bash
cargo run -q -p rimloc-cli -- --ui-lang <lang> --help
```

## Плейсхолдеры — не трогаем

Плейсхолдеры — это метки вроде `{count}` или `%s`. Их нужно оставить строго как в EN. Подробности — в “Гайды → Плейсхолдеры”.

Примеры (токены не меняем):

```
EN: Found {count} files
RU: Найдено {count} файлов

EN: Invalid value: %s
RU: Неверное значение: %s
```

## Чек‑лист

- [ ] Ключи без изменений (переводим только значения)
- [ ] Плейсхолдеры сохранены (`{…}`, `%…`)
- [ ] Два файла на месте: `rimloc.ftl`, `rimloc-tests.ftl`
- [ ] В PR указан код языка и, по возможности, скрин `--help`

## Нужна помощь?

- Создайте issue (Community → Issue Guidelines)
- Откройте черновой PR — мы поможем довести до мерджа

Спасибо, что делаете RimLoc удобнее для всех!

