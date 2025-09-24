---
title: Перевод RimLoc (i18n)
---

# Перевод RimLoc (i18n)

CLI‑сообщения RimLoc локализуются через Fluent (FTL) и встраиваются на этапе сборки. Ниже — как добавить или обновить перевод.

## Структура

- `crates/rimloc-cli/i18n/en/rimloc.ftl` — английский источник истины.
- `crates/rimloc-cli/i18n/<lang>/rimloc.ftl` — другие локали зеркалируют ключи EN.
- `crates/rimloc-cli/i18n/<lang>/rimloc-tests.ftl` — тестовые сообщения.

Для `<lang>` используйте IETF/ISO коды (`ru`, `de`, `fr`). RimLoc подбирает языки по языковому коду; региональные теги игнорируются.

## Добавление новой локали

1) Скопируйте английские файлы:

```
crates/rimloc-cli/i18n/en/rimloc.ftl → crates/rimloc-cli/i18n/<lang>/rimloc.ftl
crates/rimloc-cli/i18n/en/rimloc-tests.ftl → crates/rimloc-cli/i18n/<lang>/rimloc-tests.ftl
```

2) Переведите значения, ключи и плейсхолдеры оставьте как в EN.
   - Ключи: строчные с дефисами.
   - Плейсхолдеры: `{name}`, `{0}`, `%s`, `%d` — не менять.

3) Запустите тесты:

```bash
cargo test --package rimloc-cli -- tests_i18n
cargo test --workspace
```

4) Проверьте локализованный help:

```bash
rimloc-cli --ui-lang <lang> --help
```

Если всё прошло — язык подключится автоматически (доп. регистрации не требуется).

## Обновление строк

- Сначала правьте EN (добавление/удаление ключей), затем синхронизируйте другие локали.
- Набор ключей во всех локалях должен совпадать — это проверяется тестами.
- Для координации изменений можно завести issue (см. «Issue Guidelines»).

## Плейсхолдеры

Смотрите раздел [Плейсхолдеры](../guide/placeholders.md). Несовпадающие или испорченные плейсхолдеры будут ловиться `validate-po --strict`.

