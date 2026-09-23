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
