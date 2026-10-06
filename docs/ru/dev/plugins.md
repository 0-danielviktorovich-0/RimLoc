---
title: Legacy Scan Plugins
---

# Legacy scan plugins

В RimLoc есть **экспериментальный legacy-механизм scan plugins**, который расширяет XML scanning.

Это **не** новая LocalizationAdapter architecture и не стабильный публичный extension API.

Исторически такие плагины подключались через <code>RIMLOC_PLUGINS</code> / <code>--with-plugins</code> и возвращали JSON-compatible translation units через native C ABI.

Для новых игровых/приложенческих интеграций начинайте с [гайда по адаптерам](adapters.md).

## Почему это разные вещи

Scan plugin только добавляет extracted units. Полноценному LocalizationAdapter могут понадобиться:

- source discovery;
- versions/dependencies;
- existing-translation import;
- provenance;
- adapter-specific validation;
- build/export;
- runtime acceptance.

Не проектируйте новый публичный ecosystem вокруг неверсированного native dynamic-library ABI.

Исторические детали смотрите в <code>rimloc-plugin-api</code> и существующих plugin crates.
