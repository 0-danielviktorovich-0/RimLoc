---
title: Перевод RimLoc
---

# Перевод самого RimLoc

RimLoc постепенно переходит к **self-localization через ту же каноническую project model**, что используется для других источников.

Долгосрочный contributor workflow:

1. открыть application catalog RimLoc как localization project;
2. выбрать target locale;
3. переводить/review в обычном editor;
4. проверить placeholders/select/plural rules;
5. собрать contribution bundle;
6. отправить перевод на review.

Это второй реальный adapter direction после RimWorld и важная проверка, что core не RimWorld-only.

## Текущий pre-beta статус

Self-localization ещё интегрируется в React product workflow. Если текущая сборка показывает **Translate RimLoc**, используйте этот путь.

Если UI-путь в конкретной сборке недоступен, repository sources остаются developer fallback, а не финальным UX.

## CLI messages

Rust CLI использует Fluent (FTL). English — source locale CLI message catalogs.

~~~text
crates/rimloc-cli/i18n/en/
crates/rimloc-cli/i18n/ru/
...
~~~

При ручной правке FTL:

- переводите values, не keys;
- сохраняйте placeholders;
- держите одинаковый набор keys;
- запускайте i18n tests.

~~~bash
cargo test --package rimloc-cli -- tests_i18n
~~~

## UI/application catalog

Application UI catalog имеет отдельный canonical bridge/project path. Не считайте CLI FTL единственным источником строк React UI.

Для implementation details смотрите текущую self-localization architecture в repository docs.

## Перевод документации

Публичные docs сейчас поддерживают EN/RU деревья <code>docs/en</code> и <code>docs/ru</code>.

Для новой locale:

- зеркально создайте структуру страниц;
- переводите текст, не меняя technical identifiers/commands;
- добавьте locale в MkDocs i18n;
- запустите <code>mkdocs build --strict</code>.

## Checklist

- target locale указан;
- placeholders/select/plural syntax сохранены;
- source IDs не изменены;
- validation зелёный;
- bundle не содержит secrets/private paths;
- при изменении поведения EN/RU docs обновлены вместе.
