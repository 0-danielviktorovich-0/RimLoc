# BACKLOG: PO — опциональный interchange, не каноническое представление

- **Дата:** 2026-10-06 · **Статус:** BACKLOG (bounded, не disruptive) · **Приоритет:** после
  стабилизации windows/CI-конвейера, до UX-полировки беты
- **Решение владельца:** PO — опциональный формат обмена/экспорта/импорта. Он НЕ каноническое
  RimLoc-представление проекта и не обязан требоваться для обычного цикла
  «перевод → валидация → сборка».

## Целевая архитектура (северное направление)

```
source (мод)
  → canonical SourceEntry / project state (session store, уже есть)
  → translations (session; TM/glossary как ассистенты)
  → validation / review (validate, checks)
  → build/export ADAPTERS: build-mod | export-po | export-xliff | export-csv | …
```

Форматы (PO, XLIFF, CSV, XML) — суть адаптеры на входе/выходе, не шов конвейера.

## Зафиксированный легаси-шов (факт, 2026-10-06)

`crates/rimloc-cli/src/commands/translate.rs` — LLM-перевод завершается записью
переведённого **PO** (строки 159-179, `translate-po-saved`) специально под существующий
пайплайн import-po/build-mod. То есть автоматический перевод сегодня **по построению**
проходит через PO-файл как обязательный промежуточный артефакт.

## Что уже есть правильного (не трогать)

- `build-mod --from-root` — готовый no-PO путь (source → scan → build напрямую).
- Session-слой (`rimloc-services`, canonical Project, persist-before-ack) — каноническое
  состояние уже живёт не в PO.
- Существующие PO-команды **обязаны остаться** для совместимости: export-po, import-po,
  validate-po; build-mod продолжает принимать PO.

## Bounded- slices (каждый — отдельный маленький PR, без редизайна)

1. **`translate --emit json|po`** (или `--format`): LLM-перевод умеет выгружать результат в
   нейтральный JSON (SourceEntry-совместимый: key/source/target/locale) рядом с текущим PO
   (PO остаётся дефолтом для совместимости). Оценка: ~1 файл + тест.
2. **`apply-translations`** (или расширение import): приём того JSON напрямую в session-проект
   (persist-before-ack, revision), минуя PO. Дальше validate/build-mod работают от проекта.
3. **Аудит хвостов**: grep по CLI на `\.po\b`/`out_po` — найти места, где PO предположительно
   обязателен; задокументировать каждый как «PO-optional» или «PO-required + почему».
   Результат — таблица в этом документе.
4. **(только если 1-3 вскроют спрос)** GUI-поток «LLM-перевод → review → build» без
   промежуточных файлов — это уже UI-задача, решается поверх session-слоя.

## Явные non-goals

- Никакого редизайна CLI в этом бэктлоге; PO-команды не деградируют.
- Не трогать в текущем критическом окне (windows/CI/merge PR #60).
- Не менять формат существующего PO (X-RimLoc-Generator остаётся).
