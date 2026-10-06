---
title: Localization Adapters
---

# Как добавить другую игру или приложение

RimLoc сейчас RimWorld-first, но generic localization core строится вокруг адаптеров.

Цель: разработчик или coding agent должен уметь добавить новую среду, не переписывая editor, TM, glossary, validation framework, project history и multi-target model.

## Что уже есть

Границу уже проверяют две разные adapter identity:

- RimWorld — основной production target;
- RimLoc application/self-localization — второй, структурно другой источник.

То есть core project model не должен быть равен “RimWorld XML”.

Стабильного публичного third-party adapter ABI/marketplace пока нет.

## Что принадлежит core

Generic core знает:

- Project;
- SourceEntry;
- Translation;
- Locale;
- revision/history;
- TM;
- glossary;
- review;
- findings;
- provenance;
- build/output artifacts.

Adapter знает:

- discovery/detection;
- parsing;
- effective-content/version resolution;
- mapping в canonical SourceEntry;
- existing translation import;
- adapter-specific validation;
- build/export;
- optional runtime verification.

Не добавляйте ThingDef, Minecraft keys, Steam Workshop rules или object model другой платформы в generic core.

## Минимальный checklist адаптера

Для in-tree adapter:

1. Выберите стабильный adapter ID и schema/API version.
2. Опишите поддерживаемые source kinds.
3. Реализуйте detection/discovery.
4. Выдавайте deterministic canonical entries со stable IDs.
5. Сохраняйте source location/provenance.
6. Реализуйте existing-translation import, если он имеет смысл.
7. Реализуйте update semantics: unchanged/new/source-changed/obsolete/ambiguous — где применимо.
8. Объявите capabilities.
9. Реализуйте безопасный build/export.
10. Добавьте fixtures и conformance tests.

## Capability-driven UI

UI должен спрашивать adapter, что он умеет, а не захардкоживать один workflow.

Например:

- existing translation import;
- build/export;
- source context;
- dependencies;
- versions/content roots;
- runtime validation.

Unsupported capability не должна выглядеть как рабочая кнопка.

## Conformance

Минимально полезно проверить:

- deterministic inventory;
- stable IDs;
- отсутствие записи в source;
- target-locale isolation;
- persistence/restart;
- update behavior;
- build/round-trip;
- path containment;
- чистый отказ для неизвестной/unsupported semantics.

## Классы расширений

Долгосрочно возможны:

1. **простые/declarative adapters** для JSON/CSV/gettext-like ресурсов;
2. **built-in Rust adapters** для сложных игровых семантик;
3. **external sandboxed/versioned adapters**, если появится реальная ecosystem-потребность.

Не стоит фиксировать Rust dynamic-library ABI как публичный долгосрочный plugin contract. Будущий внешний boundary лучше делать versioned/language-neutral (process/RPC или WASM) после отдельного дизайна.

## Что не нужно до beta

Minecraft/Paradox/Terraria и другие среды не нужны ради галочки.

Первая beta должна доказать RimWorld workflow и то, что архитектура не мешает добавить следующий adapter позже.

## Для coding agents

Дайте агенту:

- эту страницу;
- [канонический architecture doc](../../architecture/LOCALIZATION_ADAPTERS.md);
- близкий reference adapter;
- fixtures;
- conformance tests;
- read-only/output rules.

Ожидаемый результат — локальная adapter-реализация, а не game-specific правки по всему проекту.
