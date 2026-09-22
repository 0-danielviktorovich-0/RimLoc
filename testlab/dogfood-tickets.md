# Dogfood tickets — CLI на реальных модах (Ф1, до правок)

Дата: 2026-09-23 · Бинарь: 0.1.0-alpha.1 (baseline 2c5a47e) · Мод: VWE `1814383360` (4 языка: EN, RU, NL, JA), фикстура test/TestMod.
Артефакты: `testlab/artifacts/dogfood-*/`.

| ID | Приоритет | Симптом | Как воспроизвести | Ожидание |
|----|-----------|---------|-------------------|----------|
| T1 | P1 | Двойной `ERROR Overriding {diffxml-summary}` при каждом запуске CLI | любая команда; stderr | Дубликат Fluent-ключа устранён, stderr чистый |
| T2 | P1 UX | `scan --format json` без `--out-json` не пишет ничего в stdout — вывод непайпуем | `scan --root M --format json \| jq` | JSON в stdout, `--out-json` опционален |
| T3 | P2 | `--help` сабкоманд уходит в stderr | `scan --help 2>/dev/null` → пусто | help в stdout (или выяснить и задокументировать) |
| T4 | P2 UX | Дефолтный scan (без `--lang`) собирает переводы, а не EN-источник; концепция «источник перевода» не выражена | `scan --root M` | Явная семантика source/target, вменяемый дефолт |
| T5 | **P0** | `--lang` не фильтрует: scan смешивает ВСЕ `Languages/*` (Dutch 402 + EN 325 + JA 501 + RU 397 = 1625 записей, уникальных ключей 760 — коллизии) | `scan --root VWE --lang en --format json --out-json f` → пути всех языков | Только запрошенный язык |
| T6 | **P0** | `coverage` = 0/0/0 (0%) даже на синтетическом test/TestMod; на реальном VWE так же | `coverage --root test/TestMod --source-lang-dir … --target-lang-dir …` | Честный отчёт переведено/пропущено |
| T7 | P1 | `validate` гоняет по всем языкам включая EN-источник: placeholder-check шумит на исходных строках; li-нормализация даёт cross-file duplicate false-positives (`*.tools.li.label` в RangedMedieval и RangedNeolithic) | `validate --root VWE` | Валидация переводов против источника; дубликаты по реальным совпадениям ключа |
| T8 | P3 | DefInjected-сообщения validate имеют `:0` вместо номера строки | вывод validate | Линия из источника |

Вывод: **основной сценарий «scan → coverage → validate» на реальном моде с несколькими языками не работает** (T5/T6) — это ядро P0, чинится до/вместе с безопасностью Ф2.

## Статусы после фикса (2026-09-23)
- **T5 FIXED**: `--lang` фильтрует скан на один Languages/<dir> (English дополнительно тянет Defs-строки под целевым путём). Регрессия: `scan_lang_flag_filters_to_requested_language` + фикстура MultiLangMod.
- **T6 FIXED**: аргументы-каталоги нормализуются (полный путь → имя папки) в coverage и всех кросс-язычных проверках. VWE: source=370, translated=142 (38%). Регрессия: `coverage_accepts_full_language_dir_paths`.
- **T7 FIXED**: (а) per-row validate целяется в перевод (--lang-dir/--lang/cfg.target_lang), а не в EN-источник через протечку конфига; (б) `duplicate-global` скоупится на языковую папку — один ключ в EN и RU больше не «дубликат». Регрессия: `validate_does_not_flag_same_key_across_language_folders`. Снапшот snapshot_validate_json обновлён (удалены 5 кросс-язычных false-positive).
- **T1 FIXED**: дублирующийся Fluent-ключ `diffxml-summary` удалён из en+ru (паритет ключей сохранён).
- **T2 NOT-A-BUG**: `--format json` без `--out-json` пишет в stdout (проверено на тёплом бинаре); первоначальная пустота — артефакт холодной компиляции.
- **T3 NOT-A-BUG**: `--help` сабкоманд в stdout (проверено на тёплом бинаре).
- T4: принято решение — дефолт (без флагов) остаётся «все языки», явная семантика через `--lang`/`--source-lang`; пересмотреть в контексте редактора GUI.
- T8: отложено (P3, вместе с observability Ф4).

Итог прогонов: cargo test --workspace — 79 passed / 0 failed; fmt clean; новых clippy-предупреждений нет.
