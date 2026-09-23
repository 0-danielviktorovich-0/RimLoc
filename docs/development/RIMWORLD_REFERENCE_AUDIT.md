# RIMWORLD REFERENCE AUDIT — источники правды для RimLoc

Дата: 2026-09-23 · Итерация 2 · Версия игры-референса: **1.6.4871 rev573** (установленная GOG).

## Иерархия источников (§H)

| Уровень | Источник | Доказательная сила |
|---|---|---|
| 1 | **Декомпилированная DLL установленной 1.6.4871** (`/tmp/rw_decomp`, ILSpy; UTF-16 литералы из бинаря) | первоисточник поведения; помечено `[1.6 · Verse/Xxx.cs#Method]` в GAME_SOURCE_FINDINGS.md |
| 1 | **Живые данные установки** (`Russian (Русский).tar`, `/Applications/RimWorld.app/Data/*`) | `[install]` |
| 2 | Официальные репо Ludeon `RimWorld-{ru,de,ja,zh,uk}` (shallow-клоны, OFFICIAL_LANG_PACKS.md) | first-party данные; workflow команд |
| 3 | RimWorld Wiki / форумы | community-documented — **не использовались без проверки кодом/данными** |
| 4 | Реальные моды/переводы (287 в библиотеке) | корpus-доказательства |
| 5 | Конкуренты (COMPETITOR_MATRIX.md) | только идеи, не истина о формате |

Важно: `/Applications/RimWorld.app/Source/` оказался нерепрезентативным для локализации (43 файла JobDrivers) — вместо community-декомпилов 2018 года декомпилирована **текущая DLL**. Ни один факт не принят из устаревших источников без сверки с 1.6.

## A. Модель локализации (проверено по 1.6)

- `Languages/<lang>/`: DefInjected (папки = типы дефов), Keyed, Strings, Backstories (**DefInjected/BackstoryDef** — legacy-формат целиком устарел, категория отчёта №12), WordInfo (только ru/de/uk-семейство), LanguageInfo.xml (`languageWorkerClass`).
- DefInjected-путь: `DefType/defName.поле[.вложение]`; списки — именные узлы и li-N хэндлы; `.slateRef`; TKey-система (**новое в 1.6**, парсится до патчей).
- Keyed: **последний файл перезаписывает**; дубликаты в одном файле — ошибка.
- Encoding: UTF-8 (BOM факультативен даже в официальных репо).

## B. Load semantics → реализовано в RimLoc (services::modview)

| Правило игры (1.6) | Статус RimLoc |
|---|---|
| Languages/ собирается из **каждой** content-папки (root/Common/1.6/IfModActive) | ✓ `EffectiveModView.languages_dirs()` (P1-дыра закрыта: моды с `1.6/Languages/Russian` больше не пропускаются) |
| Теги версий регистронезависимы (`<V1.6>`) | ✓ |
| Фолбэк версии: наибольший тег ≤ запрошенной | ✓ |
| `IfModActive`/`IfModActiveAll`/`IfModNotActive` → условные папки | ✓ (отдельный список; офлайн-извлечение — супермножество) |
| Сумма `версия/ + Common/ + корень` (не «или») | ✓ content_dirs |
| Порядок `li` descending + дедуп (первый Defs-путь выигрывает; Keyed — последний перезаписывает) | ⚠ частично: извлечение — супермножество (детерминированная сортировка); приоритеты дедупа — todo следующей итерации (влияет только на подсчёт дублей, не на полноту) |
| Регистр путей на macOS/Windows | учитывается существованием путей; отдельного теста нет — mac-only среда |

## C. Translation Report как оракул (§C)

Все **13 категорий** отчёта 1.6 зафиксированы дословно из DLL в GAME_SOURCE_FINDINGS.md §2. Сопоставление с RimLoc:

| Категория отчёта 1.6 | RimLoc | Gap |
|---|---|---|
| General load errors | `xml-health`, `validate` | ✓ |
| DefInjected load errors (def/поле не найдено, индексы, NoTranslate, коллизии normalizedPath) | частично (`validate` empty/duplicate/orphan) | поле-не-найдено и normalizedPath-коллизии — todo P2 |
| Missing keyed translations (TODO = отсутствие) | `coverage`/`learn-keyed` | ⚠ TODO-семантику добавить в coverage (P2) |
| Def-injected missing (пропускает generated/однословные/без пробела) | `compare` | учесть правила пропуска для сопоставимости чисел (P2) |
| Backstory missing/errors | не реализовано (legacy-формат мёртв) | осознанно rejected |
| Unnecessary DefInjected (NoTranslate) | ✗ | P2: парсить [NoTranslate]/[Unsaved] атрибуты |
| Old/renamed defs (BackCompatibility) | ✗ | требует back-compat таблиц игры — P3 |
| Argument count mismatches (только EN `{x}` ⊆ перевода) | `validate` placeholder-check (строже: множества) | RimLoc строже игры — оставить, задокументировать разницу |
| Unnecessary keyed / Keyed matching English | `validate` (empty/duplicate) + compare len-ratio | частично |
| Syntax suggestions (li-N, TKey) | ✗ | P3 |
| TKey system errors | ✗ | P2 (1.6-новое) |

Полное покрытие всех категорий — план следующей итерации; каждая новая категория сопровождается фикстурой и тестом (§8-петля).

## D. Грамматика/аргументы

- Set-based сравнение плейсхолдеров (позиции легитимно меняются в ja/zh) — тест `reordered_placeholders_are_legitimate_for_cjk`.
- `{lookup:}`/`{replace:}`: обёртки не токены, внутренние аргументы считаются — тесты `lookup_wrapper_inner_placeholder_is_preserved`, `german_replace_macro_inner_placeholders_counted`.
- `{X_numCase ? …}`/`{PAWN_gender ? …}` (ru) — тот же принцип: внутренние токены считаются, конструкция сохраняется.
- WordInfo-формат: `;`-таблицы, `//`-секции, `Gender/Neuter.txt → Gender.None` — зафиксировано для генератора (Ф-WordInfo, следующая итерация).

## E/F/G. Официальные паки

См. OFFICIAL_LANG_PACKS.md (структура, воркфлоу, 5 фикстур в `testlab/fixtures-official/` с PROVENANCE). Использование: только парсинг-регрессия и моделирование capabilities; **не** источник утечки в blind-бенчмарки и не переводческий корпус (лицензия Ludeon-репо не объявлена).

## Классификация остаточной семантики перед заморозкой GUI-контрактов (гейт §1)

| Правило | Влияет на entry model? | Классификация |
|---|---|---|
| **NoTranslate/Unsaved** (атрибуты полей в коде игры) | Нет: извлечение RimLoc — allowlist-словарь переводимых полей; NoTranslate-поля в него не входят и не попадают в entries | **ЗАКРЫТО** (исключение по умолчанию) + P2-диагностика «unnecessary entries» для уже существующих переводов (парсинг атрибутов требует рефлексию/список — офлайн-curated список) |
| **TKey (1.6)** | **Да** — создаёт записи с иной идентичностью ключей | **РЕАЛИЗОВАНО + ИНТЕГРИТИ-АУДИТ ПРОЙДЕН** (`testlab/scripts/tkey_audit.py`, machine-evidence): узлы с текстом — Core 112, Royalty 208, Ideology 3, Biotech 5, Anomaly 3, Odyssey 27 = **358 всего** (350 уникальных пар); скан воспроизводим (11775=11775); эмиттировано ровно 112 TKey-записей в Core (дедуп 0); **двойного извлечения нет (0)** — обычный field-путь TKey-узла не эмиттируется; официальный RU: 90 exact + 22 canonical + **0 unmatched, 0 коллизий, 0 необъяснённых**. Подслучай суффикса ЗАКРЫТ DLC-adjudication (P4-популяция: 351 узел / 343 идентичности из 350, 0 исключений правила, `DLC-TKEY-ADJUDICATION.md` §1.1 — **авторитетные определения популяций**: P1 raw 360 / P2 translatable 358 / P3 идентичности 350 / P4 serialization-tested 343, воспроизводятся `testlab/scripts/tkey_population_reconcile.py`): суффикс — функция контекста — `parms`-узлы QuestNode_SubScript → `.value.slateRef`; TipSetDef `li` → bare; остальные QuestScriptDef-узлы → `.slateRef`. 8 дублей Royalty: 2 deliberate (идентичный EN) + 6 accidental копипаст (официальный пак содержит ровно одну запись на пару → дедуп по (defName,TKey) подтверждён; рантайм-семантика и все контексты — `DLC-TKEY-ADJUDICATION.md` §5). 7 unmatched = 4 Royalty gap + 3 Odyssey structural-path алиаса (адресация валидна в рантайме; поддержка алиасов в матчёре — в P1-2). Коллизии покрыты тестами `tkey_suffix_variants_stay_distinct_as_units`, `canonical_fallback_does_not_shadow_exact_match_of_other_entry` |
| **Field-not-found** (target-ключ ссылается на несуществующее поле/def) | Да — статус записи «invalid/orphan» | **ЗАКРЫТО давно**: `validate_orphans_cross_language` (target-ключи без source) — GUI-контракт обязан всегда включать orphan-проверку в validation suite (не за флагом) |
| **TODO = отсутствие перевода** | Да — статус untranslated, не translated | **ЗАКРЫТО** (coverage + тест, этот же гейт) |
| **generated-дефы / однословные строки без пробела** (правила пропуска missing-отчёта) | Влияет на сопоставимость coverage с числами игры, не на существование записи | **P2-паритет**: реализовать как режим «game-parity coverage» при сверке с Translation Report |

**Итог гейта**: entry model заморожена в составе — источники (Keyed/DefInjected/Defs-fields/Patches-derived; TKey = зарезервированное расширение), статусы (untranslated/translated/TODO-missing/orphan-invalid/sourceChanged/pending-review), идентичность = (source kind, key, language pair). Контракт готов к GUI.

## Расхождения «документация vs текущее поведение» (зафиксировано)

1. Backstories как `Backstories/Backstories.xml` — мёртвый legacy (1.6 = DefInjected/BackstoryDef); RimLoc не реализует старый формат — сознательно.
2. TranslationReport в 1.6 пишется на **Desktop**, а не в Config песочницы — поправка к процедуре isolated-run (research-файл обновить при следующем acceptance).
3. TKey-система (1.6) отсутствует во всех community-гайдах — реализована только в DLL; RimLoc пока не парсит TKey-пути (P2).
