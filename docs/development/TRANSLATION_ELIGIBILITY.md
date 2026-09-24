# Переводимость записей (Gate J): контракт и движок

**Статус:** контракт + движок реализованы; подключение к validate — есть; GUI-explain и AI-адъюдикация — в плане.

Переводимость (eligibility) — это решение «что делать с записью: переводить, не
трогать или отдать на разбор». Его принимает ровно один компонент — движок
eligible (`rimloc-services/eligibility_engine.rs` поверх контракта
`rimloc-domain/eligibility.rs`). Сканер, GUI, LLM-конвейер, compare и import
сами решение НЕ принимают — они спрашивают движок. Это единственный источник
правды; мандаты: GAME_SOURCE_FINDINGS §6, RIMWORLD_REFERENCE_AUDIT §C,
SOURCE_INSPECTOR_MANDATE, CANONICAL_PROJECT_MODEL.

## Решение и уверенность

`Decision` — что делать с записью:

- `translatable` — переводить;
- `non_translatable` — не переводить (технические поля ломают загрузку Def);
- `review` — отдать человеку или AI на разбор (молчаливого «переводить» нет);
- `unknown` — решить не удалось.

`Authority` — класс уверенности, без фейковых чисел:
`deterministic` (семантика первого лица) → `verified` (правило) →
`strong_inference` (структура) → `heuristic` (в т.ч. AI) → `unknown`.

## Evidence и источники правил

Каждое решение несёт цепочку `Evidence { rule_id, source, detail }` и
диагностированные `Conflict`-ы (победитель/проигравший/причина). `EvidenceSource`
— откуда правило: `first_party_attribute`, `mod_assembly_metadata`,
`built_in_rule`, `community_rule`, `user_rule`, `project_override`,
`reference_pack`, `structural_heuristic`, `ai_proposal`.

Инварианты мандата: AI-вывод — только предложение (`ai_proposal`), никогда не
истина по умолчанию; знания сообщества и пользователя — декларативные (без кода);
явный NoTranslate не перекрывается молча.

## Лестница приоритетов и финальность NoTranslate

`PRECEDENCE` — порядок сверху вниз, верхний источник побеждает:

```
first_party_attribute > mod_assembly_metadata > built_in_rule >
community_rule > user_rule > project_override > reference_pack >
structural_heuristic > ai_proposal
```

Две гарантии поверх лестницы:

1. `NON_TRANSLATABLE` с authority `deterministic`/`verified` — **финален**:
   более слабое (или равное по рангу) «переводить» откатывается назад, конфликт
   записывается в `verdict.conflicts`. Так user-пак «хочу перевести texPath» не
   может сломать игру.
2. Строго более сильное `TRANSLATABLE` (first-party MustTranslate-семантика)
   остаётся победителем, а столкновение диагностируется вместо тихого
   разрешения. Всё детерминировано: одинаковый вход — одинаковый вердикт.

## Встроенный seed-пак: 155 правил

`builtin_seed_rules()` собирается декларативно из уже существующих дефолтов
экстракции. Состав зафиксирован тестом
`seed_pack_shape_matches_documented_composition` (любое изменение словаря
сознательно обновляет эту цифру и документ):

| Семейство | Штук | Откуда |
|---|---|---|
| Kind-правила первого лица | 2 | TKey и Keyed/LanguageData существуют, чтобы их переводили |
| NoTranslate-поля | 5 | `defName`, `packageId`, `texPath`, `workerClass`, `defaultDamage` — семейство `[NoTranslate]`/`[Unsaved]` |
| Def-type правила | 126 | пары (def_type, поле) из встроенного словаря `rimloc-parsers-xml assets/defs_fields.json` — те же данные, что у сканера, поэтому движок и экстрактор не расходятся |
| Универсальные leaf-паттерны | 22 | `label`, `title`, `description`, `reportString`, `letterText`, … — человеческие поля вне контекста def-типа |

Экстракция Defs — allowlist- driven, поэтому технические поля из Defs в
инвентарь не попадают; но DefInjected-сайдкары «в дикой природе» их несут, и
движок обязан такие записи отклонять.

## Rule-паки (сообщество / пользователь / проект)

Файл пака: `{"schema_version": 1, "rules": [...]}`, загрузчик `load_rule_pack`
принимает только версию 1 и **отвергает неизвестные поля** (`deny_unknown_fields`
на обёртке и каждом правиле): опечатка в поле падает громко, а не молча сужает
правило. Внутри правила — селекторы (`def_type`, `field_path`, `package_id`,
`entry_kind`, `versions`) и решение (`decision`, `provenance`, `reason`); пустые
`id`/`reason` отклоняются. Пак подключается через
`EligibilityEngine::with_rule_pack`; внешние правила оцениваются до встроенного
фолбэка, но исход всё равно решает лестница и финальность NoTranslate.

## explain-JSON

`EligibilityEngine::explain_json(entry, package_id)` отдаёт полный объясняющий
артефакт — `decision`, `authority`, `evidence[]`, `conflicts[]`. Это контракт
для будущих CLI/GUI поверхностей: пользователь видит не только вердикт, но и
какие правила столкнулись и почему победил именно этот.

## Как подключено сейчас

Пайплайн validate (`rimloc-services/validate.rs`) после канонического скана
прогоняет инвентарь через `evaluate_units` и добавляет сообщение валидации
`kind = "non-translatable-flagged"` **только** для записей, которые движок
объявил `non_translatable`, но которые всё же попали в инвентарь (типичный
случай — DefInjected-сайдкары с `defName`/`texPath`). Каждое такое сообщение —
сигнал расхождения движка и экстрактора, ценная диагностика, а не шум.
Записи `translatable` сообщений не создают; kind `review-eligibility`
зарезервирован для отдельной поверхности разбора.

Winner-reason provenance: в `SourceProvenance` канонической модели живёт
поле `selected_by` — часть pre-freeze контракта Source Inspector:
provenance обязан отвечать «что за файл / где / какой кандидат победил /
почему». Постраничный захват реализован: скан-пайплайн ставит причину
победы в точке решения (`rimloc_core::winner_reason`): «first-file-wins»
(Defs/TKey, в том числе между контент-директориями LoadFolders),
«keyed-last-wins» и «keyed-first-in-file» (Keyed), «definjected-setoradd»
(DefInjected), «tkey-last-assignment» (несколько узлов одного файла на
одном TKey), «patch-applied» (значение произвёл патч-оп). Факты выбора
корней остаются в `version_selected` (разрешённая версия) и
`conditional_branch` (IfModActive-супермножество) той же записи provenance.
Реальный effective source file Defs-записей несёт `SourceContext.file`
(виртуальный DefInjected output path — отдельный канал `TransUnit.path`);
line — только там, где её гарантирует парсер.

## План (отдельные полосы)

- **GUI explain**: показ вердикта и цепочки evidence в Source Inspector поверх
  `explain_json` и `selected_by`.
- **AI-адъюдикация**: разбор `review`-записей по fingerprints — AI предлагает
  (`ai_proposal`), человек подтверждает; знание фиксируется декларативно в
  rule-паках, а не кодом.
