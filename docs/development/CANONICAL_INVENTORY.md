# Канонический инвентарь — один пайплайн для всех потребителей (гейт B)

**Дата**: 2026-09-23 · **Канонический вход**: `rimloc_services::scan_units_with_defs_and_dict`
(Keyed + Defs через learned/словарь + TKey + merge с DefInjected-переписыванием путей).
`scan_units_auto` = тот же пайплайн с autodiscover. Все convenience-варианты обязаны
делегировать; собственные сборки юнитов запрещены.

## Машиночитаемая таблица consumer × capability

| Потребитель | Точка входа | Effective-mod aware | Version aware | TKey aware | Eligibility (сейчас) | Language aware | Статус |
|---|---|---|---|---|---|---|---|
| CLI scan | `scan_units_with_defs_and_dict` | частично (modview отдельно) | ✅ флаг | ✅ | словари/allowlist | ✅ фильтр | канон |
| CLI compare | `scan_units_with_defs_and_dict` | частично | ✅ | ✅ | словари | ✅ (sets) | канон |
| CLI translate | `scan_units_with_defs_and_dict` | частично | ✅ | ✅ | словари | ✅ | канон |
| CLI version-diff | `scan_units_with_defs_and_dict` | частично | ✅ (по два root) | ✅ | словари | ✅ | канон |
| validate / coverage / cross-language | `validate.rs::scan_canonical` → канон | частично | — | ✅ | словари | ✅ | **унифицировано (bc…/этот коммит)** |
| word-info | `scan_units_auto` → канон | частично | — | ✅ **(теперь; раньше TKey-пасса не было — P1-1 закрыт)** | словари | ✅ | **унифицировано** |
| export-po | `scan_keyed_xml` + defs-meta + TKey-блок (GATE A) | частично | ✅ | ✅ | словари | ✅ | канон (TM-ключи по target-пути) |
| build (from-root) | `scan_keyed_xml` | — | — | n/a (Keyed-дерево) | — | — | осознанно (перенос Keyed) |
| GUI scan/export | те же сервисы через IPC | как CLI | ✅ | ✅ | словари | ✅ | канон |
| diff (extras) | канон + варианты | частично | ✅ | ✅ | словари | ✅ | канон |

Заполнение «Eligibility» сейчас — словари/allowlist; заменяется на eligibility-компонент в гейте J без смены сигнатур потребителей.

## Расхождения, устранённые унификацией (эвиденс)

1. **Double extraction удалён**: parsers-инвентарь дублировал TKey-узлы обычным
   field-путём (`SampleQuest.label` рядом с `SampleQuest.LetterLabelX.slateRef`);
   канонический пайплайн такого не делает (сверка с tkey_audit: двойного извлечения 0).
2. **word-info видел не всё**: `scan_units_auto` не включал TKey-пасс → фикс.
3. **validate/coverage без learned-словаря**: собирал юниты напрямую из parsers →
   поля из learned_defs.json отсутствовали → теперь канон.

Эффект на реальном моде (VWE 1814383360, English→Russian, HEAD-сравнение в worktree):
**до** унификации source=370/translated=142/missing=228 (**38%**), **после**
source=317/translated=144/missing=173 (**45%**) — минус 55 фантомных missing от
двойного извлечения. Число «65%» из раннего догфуда T6 измерялось другой методикой
(до TKey-гейта) и не сравнимо; авторитет для истории — эта таблица.

## Правило
Новый потребитель = вызов канонического входа. Изменение состава инвентаря =
правка одного пайплайна + строки в этой таблице. GUI никогда не зовёт parsers
напрямую (кроме keyed-only TM-подсказок в export-потоке, фиксируется в таблице).

## Gate H — эффективный source view (семантика, не чистка дублей)

Правила precedence по декомпилу 1.6 (GAME_SOURCE_FINDINGS §1.4/§1.5/§2.1), применены
в `services::scan::apply_effective_precedence` + `scan_units_effective`:

| Случай | Правило игры | Реализация |
|---|---|---|
| Keyed, дубликат в одном файле | ошибка `Duplicate keyed translation key`, берётся ПЕРВЫЙ | победитель = первый; дубликат сохраняется для диагностики валидатором |
| Keyed, дубликат между файлами | `SetOrAdd`, побеждает ПОСЛЕДНИЙ загруженный | last-wins по лексикографическому пути (детерминированный стенд-ин для FS-порядка Windows) |
| DefInjected, дубликат ключа | `SetOrAdd` перезапись | last-wins |
| Defs, дубликат defName | обе записи в БД, `GetDefSilentFail` берёт первую | first-file-wins по (def_type, key) с сортировкой путей; второй деф целиком вне инвентаря |
| TKey | Def-семантика (первый файл) + last-wins поля в файле | было в scan_defs_tkey (e4639f7) |
| Precedence никогда не пересекает языковые пакеты | EN и RU — разные LoadedLanguage | фолд партиционируется по языковой папке |
| Пути | case-exact везде, кроме About/LoadFolders (case-insensitive resolve) | воспроизведено; LoadFolders-теги парсятся lowercase-толерантно |

LoadFolders: LoadFolders-моды больше не ре-рутятся в `root/<версию>` (терялся Common);
`scan_units_effective` берёт Languages из effective-директорий и Defs СТРОГО из
effective-корней версии (IfModActive — по документированной offline-superset политике).

**Real-mod эвиденс (VWE 1814383360, LoadFolders v1.4/1.5/1.6)**: до H скан был union
(317 юнитов при любой --game-version, Defs версий терялись в merge); после H —
**195 юнитов @1.5 vs 247 @1.6**, версии дают разные effective-инвентари.

Синтетические регрессии: def first-file-wins (A/B файлы), keyed last-file + in-file
first с сохранённым дубликатом-диагностикой, LoadFolders-фикстура
`test/LoadFoldersMod` (Versioned.label выбирается по версии; OnlySixteen только @1.6).

**Известный хвост (Gate I)**: coverage-команда пока без `--game-version` — для
LoadFolders-модов считает по mod-root; переходит на типизированный inventory-сервис
с контекстом (версия/языки) при Gate I. Исторические числа снабжаются
methodology-блоком (inventory_semantics = `canonical-v2-effective-precedence`),
транспортный schema_version при этом НЕ меняется.

## Exact vs POTENTIAL: различие контекстов (уточнение владельца, 24.09)

Инвентарь обязан честно помечать, ЧТО он представляет:

| Контекст | Вид | Метка |
|---|---|---|
| Известны версия + активные моды/DLC/load order (RimSort/modlist) | **EXACT effective view** | authoritative |
| Только папка мода; `IfModActive`-условия не проверяемы | **POTENTIAL/CONDITIONAL** (superset) | НЕ называется runtime-истиной |

`InventoryContext` (Gate I) несёт, где доступно: target RimWorld version · active DLC ·
active mods · load order. Off-line политика IfModActive-включения остаётся superset-политикой
и маркируется POTENTIAL, пока контекст не поднят до EXACT.

### Конвейер источника (каноническая архитектура)
```
Raw source → version/LoadFolders effective content → patch-applied effective content
→ translation eligibility → canonical SourceEntry inventory
```

### Patch-этап (реализованный ограниченный поднабор)
**Аудит**: игра применяет патчи к объединённому XML ДО создания дефов
(GAME_SOURCE_FINDINGS §1.4/8) ⇒ пост-патч Defs = то, что видит переводчик.
RimLoc сегодня сканирует PRE-patch Defs + text-кандидаты патчей (learn/patches,
opt-in `with_patches`) — т.е. инвентарь был pre-patch superset'ом.
**Реализовано** (`services::patches_effect`): `replace`/`add`/`remove` (включая
`<operations>`-списки и канонический `Class="PatchOperationX"`) с literal-xpath
`/Defs/Tag[defName="X"]/field…`; add с element-value создаёт новые юниты полей.
**Классифицировано как unsupported** (счётчик + sample в PatchReport): прочие
xpath-формы (атрибутные предикаты, `ancestor::`, функции), прочие классы операций
(TextOperations и т.п.), add к корню /Defs. Unsupported ⇒ coverage=Partial ⇒ вид
остаётся POTENTIAL — pre-patch данные никогда не выдаются за runtime-истину.

### Порядок файлов — RimLoc stand-in, не игровой контракт
Runtime НЕ гарантирует порядок перечисления файлов в папке
(GAME_SOURCE_FINDINGS §1.4: «сортировки по алфавиту нет — порядок определяется ФС»).
Лексикографический порядок RimLoc — детерминизм для воспроизводимости, а не
эквивалентность рантайму; для ключей с разным текстом в разных файлах (Keyed)
поведение RimLoc детерминировано + in-file дубликаты отдаются валидатору как
диагностика. Claim «exact» для таких случаев не делается.

### Provenance SourceEntry (требование к Gate I)
Canonical SourceEntry должен уметь объяснить происхождение: raw source ·
selected version/content root · conditional LoadFolders branch · patch-transformed ·
overridden source context. В GUI новичку эта сложность не показывается — слой
диагностики/расширенного контекста переводчика.
