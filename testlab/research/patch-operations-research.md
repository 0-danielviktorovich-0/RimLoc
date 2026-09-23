# PatchOperations в RimWorld 1.5/1.6 — каталог, реальная частота, покрытость RimLoc

Исследование для `services::patches_effect` (bounded patch-applied stage). Только чтение;
корпус — локальная установка игры, код не менялся, коммита нет.

## 1. Методика и корпус

| Параметр | Значение |
|---|---|
| Корпус | `/Applications/RimWorld.app/Mods` — 290 модов (Workshop-ID); `~/Library/.../steamapps/common/RimWorld/Mods` отсутствует, в `libraryfolders.vdf` одна библиотека (проверено) |
| Модов с `Patches/` | 161 (55% всех модов) |
| Patch-файлов XML | 2238, все парсятся (0 битых) |
| Операций с `Class=` | 13 907, 27 различных классов |
| Pathed-листовых операций (Replace/Add/Remove/Insert с `<xpath>`) | 8871 |
| Game-бинарь | `Assembly-CSharp.dll` 1.6 (Unity 2022.3.35f1, от 2026-08-16): `strings` дал полный список vanilla-классов |

## 2. Каталог классов PatchOperation (1.5/1.6)

Vanilla-набор подтверждён двумя независимыми источниками: `strings Assembly-CSharp.dll` (список
классов фактической установки) и официальной страницей вики [rimworldwiki: PatchOperations]
(последняя правка 2025-11-05). Text-операции в vanilla DLL **отсутствуют** — это кастомные
операции фреймворка XML Extensions (подтверждено поиском; фреймворк 1.2–1.6).

| Класс | Счёт в корпусе | Меняет переводимый контент | RimLoc сегодня |
|---|---|---|---|
| PatchOperationAdd | 6505 (+<order>) | **да** (label 1181 / description 866 внутри `<value>`) | top-level: да; внутри `li`: нет |
| PatchOperationReplace | 2016 | **да** (label 522, description 456 конечным сегментом xpath) | top-level: да; внутри `li`: нет |
| PatchOperationConditional | 1400 (обёртка match/nomatch) | да — **через вложенные op** | нет |
| PatchOperationSequence | 1319 (`<operations><li Class=…>`) | да — через вложенные op | нет |
| PatchOperationFindMod | 1255 (mods/match/nomatch) | да — через вложенные op | нет |
| PatchOperationAddModExtension | 489 | нет (C#-данные DefModExtension) | нет |
| SafePatcher.PatchOperationSetModExtension (кастом) | 370 | нет | нет |
| PatchOperationRemove | 183 | да (снимает unit с инвентаря) | top-level: да; внутри `li`: нет |
| PatchOperationInsert | 166 (+<order>) | редко (сиблинги: recipeUsers, tags) | нет |
| PatchOperationTest (устаревший) | 89 | нет (только `<success>`) | нет |
| PatchOperationAttributeSet | 51 | нет (атрибуты) | нет |
| CombatExtended.PatchOperationMakeGunCECompatible | 35 | нет | нет |
| PatchOperationAttributeAdd | 2 | нет | нет |
| PatchOperationAttributeRemove, PatchOperationSetName, PatchOperationAttribute (абстр.), PatchOperationPathed (абстр.) | 0 | нет | нет |
| XmlExtensions.PatchOperationText{Replace,Append,Set,Insert} (кастом, фреймворк XML Extensions) | 0 в корпусе | **да** — но не используется в 290 модах | нет |
| Прочие кастомные (DubsBadHygiene ×6, VFE ×5, и т.п.) | ~15 | нет | нет |

Важно: **bare-формы `<Operation><replace>` в корпусе равны 0** — вся практика живёт в
`Class=`-формах и обёртках: `li` 5206, `match` 2342, `nomatch` 1366, `operations` 1320;
top-level op-holders 4992.

## 3. Что реально меняет переводимый контент (топ-10 по ценности для RimLoc)

1. **`<li Class="PatchOperationReplace|Add|Remove">` внутри обёрток** — 4074 из 5206 li (78,3%)
   это R/A/R; в `<value>` сидит label (1181) / description (866). Главный рычаг.
2. **Ветка `match` у FindMod/Conditional** — 2342+1366 обёрток; контент внутри существует при
   наличии зависимости — это реальный пост-патч контент.
3. **Replace по конечному сегменту label/description/text** — 522+456+56+2 = 1036 листовых
   xpath — прямая замена видимого текста.
4. **Add нового контента под def** — 1759 xpath оканчиваются на узле дефа; добавляемые
   поддеревья содержат label/description (те же 1181/866).
5. **`[@Name="X"]`-предикат** — 855 xpath: таргет в абстрактный базовый def (также `@ParentName`).
6. **`or`-предикаты** `[defName="A" or defName="B"]` — документированы вики как рекомендуемый
   паттерн (в корпусе входят в 1583 «прочих равенств» вместе с `[@Class=…]`).
7. **PatchOperationRemove** — 183: убирает unit из инвентаря (ложный перевод).
8. **PatchOperationInsert** — 166: сиблинг-вставка, изредка текст.
9. **`text()`-таргетинг** (`li[text()="Yellow"]`, `/label/text()`) — 152 xpath, точечная правка текста.
10. **Text-операции XML Extensions** — 0 в корпусе: ждать спроса, не реализовывать.

## 4. XPath-подмножество: что реально поддерживает игра

Движок — .NET `XmlNode.SelectNodes` (XPath 1.0): `PatchOperationPathed.ApplyWorker` итерирует
`xml.SelectNodes(xpath)` (декомпиляция; подтверждено тем, что в живом корпусе работают
`not()`, `contains()`, `starts-with()`, `last()`). Вики документирует практические конвенции,
а не предел движка. Измерено на 11 280 xpath корпуса:

| Фича | Встреч | Пример | Движок |
|---|---|---|---|
| абсолютный `/Defs/...` | 7118 (63%) | `/Defs/ThingDef[defName="X"]/label` | да |
| `[defName=".."]`-предикат | 9759 | первый/второй сегмент | да |
| `@`-ось | 1727 | `@Name`, `@ParentName`, `@Class` | да |
| прочие равенства в предикате | ~1583 | `[@Class=".."]`, `or`-цепочки | да |
| `[@Name=".."]` | 855 | базовые дефы | да |
| `not()` | 165 | `[not(comps)]` | да (XPath 1.0) |
| `text()` | 152 | `li[text()="Yellow"]` | да |
| wildcard `*` | 139 | `*/ThingDef[...]` | да |
| числовые `[n]` / `position()` | 119 | `[last()]` | да |
| `contains()` / `starts-with()` | 28 / 5 | `[contains(weaponTags,'Gun')]` | да |
| `//`-потомки | 3 | редко и хрупко | да |

Итого: RimWorld = **полный XPath 1.0**; «подмножество» — это вопрос надёжности рецептов, не движка.
`MayRequire`/`MayRequireAnyOf` на узлах — отдельный механизм (патчер видит только активные моды).

## 5. Конкуренты: парсят ли Patches

| Инструмент | Patches | Доказательство |
|---|---|---|
| RimTrans | нет | `grep -ri patch *.cs` → 0 совпадений |
| RimLangKit | нет | то же → 0 |
| RimTranslate | нет | `RimTranslate.py`, 0 совпадений |
| RimWorldAiTranslator | **осознанный отказ** | `src/RimWorldAiTranslator.Core/Extraction/SourceExtractor.cs:116-120`: «Patch XML translation is disabled because RimWorld patch conditions and list handles cannot be resolved safely outside the game» |
| rimworld-autonomous-translator | кода нет | репо = README + маркетинг; заявляет «translates XML/keyed» без патч-логики |
| Text-grabber | **единственный исполнитель** | `Patch_grabber.py`: `operation_selector` :232, диспетч классов :633-660 (Sequence/FindMod/Add/Replace/Insert/Test/Conditional/CE/XmlExtensions-safe), xpath-резолв по чужим Defs `xpath_to_elems` :238, нормализация `clear_xpath` :285-322, извлечение `defName/@Name/@Class` :358-360, `MayRequire` :624-660, ModSettingsFramework→Keyed :583 |

Ниша «достоверный пост-патч инвентарь» фактически пуста: единственный конкурент решает задачу
хрупкими regex-эвристиками поверх кросс-модовой БД.

## 6. Покрытость текущего подмножества RimLoc

Замер по 8871 pathed-операциям (Replace/Add/Remove/Insert), реплицируя `parse_xpath`
(`patches_effect.rs:72-117`) и диспетч `apply_operation_node` (`:189-227`):

| Конфигурация | Применено | Покрытие |
|---|---|---|
| **Текущий код** (top-level Class-диспетч, `li`-вложенности нет) | 664 | **7,5%** |
| + диспетч `<li Class>` и `match`/`nomatch` | 4448 | **50,1%** |
| + `[@Name=..]` как identity-предикат (экстраполяция ~670) | ~5100 | **~57%** |

Расхождение с докстрингом: `patches_effect.rs:10-15` заявляет поддержку `<operations>`-списков,
но диспетч (`:202-226`) матчит **имя тега**, а не `Class` вложенного узла — каноническая
vanilla-форма `<operations><li Class="PatchOperationAdd">` падает в «unsupported operation
class: li». Все 1320 sequence-наборов корпуса используют именно `li`-форму.

## 7. Рекомендации: следующие 2-3 шага

1. **Диспетч по `Class` + рекурсия в обёртки** (максимальный рычаг: 7,5% → 50,1%). В
   `apply_operation_node` читать `Class` у каждого ребёнка (`li`, `match`, `nomatch`,
   `operations`) и рекурсировать; Sequence = прозрачный контейнер, FindMod/Conditional —
   «контент существует при зависимости» (брать ветку `match` как POTENTIAL-истину; `nomatch`
   — фиксировать как условный контент, не применяя).
2. **`[@Name="X"]`/`[Name="X"]` на первом сегменте** — тот же literal-путь, identity через имя
   абстрактного дефа (+~9 п.п., суммарно ~57%). `defName` у абстракций не существует —
   идентичность в corpus-инвентаре держится на `@Name`.
3. **`or`-предикаты на первом сегменте** `[defName="A" or defName="B"]` — расширение в N
   literal-целей; форма документирована вики как каноническая.

Остаётся безопасно классифицированным как unsupported: PatchOperationInsert/Test, все
Attribute*, AddModExtension/SetModExtension (C#-данные, не текст), кастомные классы (CE,
SafePatcher, DubsBadHygiene), оси/функции/wildcard/text()-настройка xpath, `//`. Text-операции
XML Extensions — не реализовывать: 0 вхождений в корпусе из 290 модов.

## Источники

- Wiki: [Modding Tutorials/PatchOperations](https://rimworldwiki.com/wiki/Modding_Tutorials/PatchOperations) — каталог, `or`-предикаты, `@Name`/`@ParentName`, `text()`, `<order>`, `<success>`, FindMod по имени.
- Game-бинарь: `strings /Applications/RimWorld.app/Contents/Resources/Data/Managed/Assembly-CSharp.dll` — 16 vanilla-классов; отсутствие `PatchOperationText*` в vanilla.
- Декомпиляция: `PatchOperationPathed.ApplyWorker → xml.SelectNodes(xpath)` (XPath 1.0, .NET).
- Корпус: скан 2238 patch-файлов 161 мода (скрипты в `/tmp/patch_scan*.py`, числа в тексте).
- Конкуренты: `~/Developing/_competitors-rimloc/` — RimWorldAiTranslator `SourceExtractor.cs:116-120`; Text-grabber `Patch_grabber.py:232-660`; остальные — grep 0.
- Фреймворк XML Extensions (text-операции): github.com/6Arrowsim/XmlExtensions (1.2–1.6).
