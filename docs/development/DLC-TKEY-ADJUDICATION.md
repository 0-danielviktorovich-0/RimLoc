# TKey-адъюдикация DLC — кампания RimLoc

**Дата**: 2026-09-23 · **Скоуп**: TKey-узлы Defs всех корней, сопоставление с официальным RU, дубли Royalty, суффикс-правило.
**Данные**: `/Applications/RimWorld.app/Data/{Core,Royalty,Ideology,Biotech,Anomaly,Odyssey}/Defs` (READ-ONLY, не менялись).
**Официальный RU**: тары `Russian (Русский).tar` из той же установки; первоначально распакованы в `/tmp/adjudic-ru/<root>/` (временный путь), теперь воспроизводимо — `testlab/scripts/tkey_population_reconcile.py` сам распаковывает в `testlab/run/official-ru-extract/<root>/`.
**Инструменты**: сканер `/tmp/adjudic/tkey_scan.py` (исходная методика, временный путь) и — **воспроизводимая замена** — `testlab/scripts/tkey_population_reconcile.py` с отчётом `testlab/reports/tkey-population-reconcile.json`; методика `testlab/scripts/tkey_audit.py` (узлы с непустым текстом; canonical-фоллбэк — снятие `.value.slateRef`/`.slateRef`).
**Калибровка**: на Core сканер воспроизвёл известный эталон точно — 112 узлов / 112 пар, 90 exact + 22 canonical (20 `.slateRef` + 2 `.value.slateRef`), 0 unmatched. Методике доверять можно.

Пути ниже — относительно `/Applications/RimWorld.app/Data/`, если не указано иное.

---

## 1. Все корни — сводная таблица

| Корень | Узлов всего | С текстом | Уник. пар | Групп дублей | Типы дефов (узлов) | Типы узлов (с текстом) | RU-ключей DefInjected |
|---|---|---|---|---|---|---|---|
| Core | 112 | 112 | 112 | 0 | TipSetDef (90), QuestScriptDef (22) | li 90, label 9, text 9, customLetterLabel 2, customLetterText 2 | 19 145 |
| **Royalty** | **210** | **208** | **200** | **8** | QuestScriptDef (188), TipSetDef (20) | label 64, text 64, customLetterText 16, li 20, customLetterLabel 10, value 12, inspectString 9, expiryInfoPart 4, expiryInfoPartTip 4, arrivingPawnsLabelDef 3, introText 1, endingText 1 | 3 188 |
| Ideology | 3 | 3 | 3 | 0 | TipSetDef (3) | li 3 | 9 260 |
| Biotech | 5 | 5 | 5 | 0 | TipSetDef (5) | li 5 | 4 379 |
| Anomaly | 3 | 3 | 3 | 0 | TipSetDef (3) | li 3 | 3 559 |
| Odyssey | 27 | 27 | 27 | 0 | QuestScriptDef (21), TipSetDef (6) | label 10, text 10, li 6, raidLetterText 1 | 4 737 |
| **Итого** | **360** | **358** | **350** | **8** | | | |

Разница «узлов всего» vs «с текстом» = 2 TKey-контейнера без собственного текста в Royalty (см. §5). Counts Core/Royalty/Ideology/Biotech/Anomaly/Odyssey = 112/208/3/5/3/27, всего 358 — совпадают с зафиксированными в `RIMWORLD_REFERENCE_AUDIT.md`.

## 1.1. Авторитетные определения популяций (single source of truth)

Числа 358, 351 и 343 — не противоречащие замеры, а **разные популяции одного корпуса**. Единственное определение (воспроизводится `testlab/scripts/tkey_population_reconcile.py`, машинный отчёт `testlab/reports/tkey-population-reconcile.json`):

| # | Популяция | Определение | Всего |
|---|---|---|---|
| P1 | **Raw TKey nodes** | каждый XML-узел с атрибутом `TKey` в Defs-дереве принадлежащего Def'а | **360** |
| P2 | **Translatable TKey nodes** | P1 ∩ {defName-владелец непуст} ∩ {собственный непустой текст} | **358** |
| P3 | **Unique logical identities** | уникальные пары `(defName, TKey)` внутри P2 (несколько узлов могут делить идентичность — см. §5) | **350** |
| P4 | **Serialization-tested** | идентичности, чей кандидат `<defName>.<TKey>{,․slateRef,․value.slateRef}` найден в официальном RU: 351 узел / **343 идентичности** | **343 из 350** |

Исключения на каждом шаге перечислены машинно в отчёте: P1→P2 теряет ровно 2 textless-контейнера (`PawnLend.DutyRulesAny/Royal`, §6); P3→P4 не сматчивает ровно **7 идентичностей** — 4 Royalty gap + 3 Odyssey structural-addressed (§4). Любой другой документ с TKey-числами обязан ссылаться на эти определения или использовать их имена.

## 2. Сопоставление с официальным RU (per root)

Матч считается по уникальной паре `(defName, TKey)`; кандидаты: exact `<defName>.<TKey>`, далее canonical-суффиксы.

| Корень | Пар | exact (без суффикса) | +`.slateRef` | +`.value.slateRef` | unmatched | Причина unmatched |
|---|---|---|---|---|---|---|
| Core | 112 | 90 | 20 | 2 | **0** | — |
| Royalty | 200 | 20 | 157 | 19 | **4** | §4: 3 gap официального RU + 1 пара fallback-дефолтов |
| Ideology | 3 | 3 | 0 | 0 | **0** | — |
| Biotech | 5 | 5 | 0 | 0 | **0** | — |
| Anomaly | 3 | 3 | 0 | 0 | **0** | — |
| Odyssey | 27 | 6 | 18 | 0 | **3** | §4: RU перевёл структурными путями, не TKey-путями |
| **Итого** | **350** | **127** | **195** | **21** | **7** | |

Exact-пару дают только `TipSetDef` (`li`-узлы): 90+20+3+5+3+6 = 127, без исключений.

## 3. Суффикс-правило: подтверждено, но уточнено — суффикс зависит не от тега, а от контекста

Кросс-таблица «тег узла × класс-родитель × сматченный суффикс» (Core+Royalty+Odyssey). Счёт **по узлам** (дубли Royalty входят дважды): 358 узлов с текстом − 7 узлов unmatched-пар = **351 сматченный узел, 0 исключений из правила**.

| Тег | Контекст (класс-родитель) | Суффикс в RU | Узлов |
|---|---|---|---|
| `li` (TipSetDef) | — | нет (exact) | 127 |
| `label`, `text` | QuestNode_Letter, QuestNode_Message | `.slateRef` | 164 |
| `customLetterLabel/Text` | QuestNode_ManhunterPack, QuestNode_PawnsArrive, QuestNode_GiveRewards | `.slateRef` | 10 |
| `customLetterLabel/Text` | QuestNode_SubScript (внутри `<parms>`) | `.value.slateRef` | 19 |
| `arrivingPawnsLabelDef` | QuestNode_SubScript (внутри `<parms>`) | `.value.slateRef` | 2 |
| `value` | QuestNode_Set | `.slateRef` | 10 |
| `expiryInfoPart/Tip`, `inspectString` | QuestNode_Delay, QuestNode_InspectString | `.slateRef` | 17 |
| `introText`, `endingText` | QuestNode_EndGame | `.slateRef` | 2 |
| `raidLetterText` | QuestNode_Root_SurveyScanner | (miss, см. §4) | 1 |

Итого сматчено: 127+164+10+19+2+10+17+2 = 351. На уровне уникальных пар (без дублей) те же группы дают 343 сматченные пары из 350 (§2).

**Правило (финальная формулировка для RimLoc):**
1. Идентичность узла = `(defName, TKey)`; DefInjected-база пути = `<defName>.<TKey>`.
2. Суффикс определяется **сериализацией поля**, а не именем тега:
   - узел-потомок `<parms>` у `QuestNode_SubScript` → **`.value.slateRef`** (19 пар Core+Royalty, 0 исключений; слот parms-словаря элидируется из пути);
   - любой другой прямой SlateRef-узел QuestScriptDef (включая `value` у QuestNode_Set и RulePack-контейнеры `rules`) → **`.slateRef`**;
   - `li` у TipSetDef → **без суффикса**.
3. Доказательство на реальном файле: одинаковый тег `customLetterLabel` даёт разный суффикс в зависимости от контекста — `+slateRef` у `QuestNode_PawnsArrive`/`ManhunterPack`, но `+value.slateRef` в parms (Royalty/Defs/QuestScriptDefs/Hospitality/Script_Hospitality_Worker.xml:874 → `Hospitality_Util_Worker.LetterLabelLodgersArrived.value.slateRef` в RU).
4. `Core`-эталон («22 canonical») этим правилом покрывается целиком: 20 `.slateRef` (label/text/customLetter* на своих классах) + 2 `.value.slateRef` (customLetter* в parms у QuestNode_SubScript). Закрывается и недосказанный в аудите «подслучай `.value.`-варианта» (наблюдение 2 из 112): это именно parms-потомки SubScript.

## 4. 7 пар unmatched — вердикты

| Пара | Источник | Вердикт |
|---|---|---|
| `Util_SpawnSiteThreat.LetterLabelSiteAppeared` | Royalty/Defs/QuestScriptDefs/Utility/Scripts_Utility_Threats.xml:183 (`<value>` в QuestNode_Set) | **Легитимный untranslated**: RU-файл дефа существует (`Scripts_Utility_Threats.xml`, переведены только `questDescriptionRules`), TKey-ключей нет. Fallback-дефолты внутри QuestNode_IsNull-веток. Ожидаемый по правилу суффикс `.slateRef` — проверки по RU нет. Побочный квирт EN: TKey-имена свапнуты относительно содержимого (`LetterLabelSiteAppeared` держит `[defaultSiteAppearedLetterText]`, и наоборот, стр. 183/193) — TKey-имя не обязано совпадать с семантикой. |
| `Util_SpawnSiteThreat.LetterTextSiteAppeared` | там же, :193 | как выше |
| `ThreatReward_Infestation_ItemPod.LetterTextInfestationArrived` | Royalty/Defs/QuestScriptDefs/RewardThreat/Scripts_ItemPodThreat.xml:240 (`<customLetterText>` в `<parms>` SubScript) | **Gap официального RU** (в RU-файле переведены только rules). Правило предсказывает `.value.slateRef`. |
| `EndGame_RoyalAscent.ArrivingPawnsDefiniteLabel` | Royalty/Defs/QuestScriptDefs/Hospitality/Script_EndGame_RoyalAscent.xml:199 (`<arrivingPawnsLabelDef>` в `<parms>`) | **Gap официального RU** (grep по `/tmp/adjudic-ru/Royalty` — пусто). Правило предсказывает `.value.slateRef` (у близнеца `Hospitality_Util_Worker.ArrivingPawnsDefiniteLabel.value.slateRef` в RU так и есть). |
| `SurveySite.LetterLabelSurveySiteQuestCompleted` | Odyssey/Defs/QuestScriptDefs/Script_Site.xml:191 | **Не gap — структурная адресация**: RU перевёл тем же текстом по структурному пути `SurveySite.root.nodes.AllSignals.node.node.nodes.Letter.label.slateRef` (`/tmp/adjudic-ru/Odyssey/DefInjected/QuestScriptDef/Script_Site.xml:63`). Обе схемы адресации валидны в рантайме. |
| `SurveySite.LetterTextSurveySiteQuestCompleted` | там же, :193 | как выше (RU: `…nodes.Letter.text.slateRef`, файл RU:65) |
| `SurveySite.LetterTextSurveySiteRaid` | там же, :173 (`<raidLetterText TKey=…>`) | как выше (RU: `SurveySite.root.nodes.Root_SurveyScanner.raidLetterText`, файл RU:61) |

Показательно: в одном файле RU (`Script_Site.xml` Odyssey) сосуществуют обе схемы — TKey-адресация для `LetterLabelQuestExpired` (:57) и структурная для completion-письма (:63). Значит structural-путь — легитимный алиас идентичности TKey-узла, а не мусор.

## 5. 8 дублей Royalty — вердикты

208 узлов / 200 пар: 8 групп по 2 узла с одинаковым `(defName, TKey)`. Все 8 — пары label+text письма `QuestNode_Letter`, оба узла в одном файле, в соседних signal-ветках. Во всех 8 случаях официальный RU содержит **ровно одну запись на пару** — коллапс идентичности в рантайме подтверждён данными, это свойство ванили, а не ошибка подсчёта.

| # | Пара | Файл (Defs), узлы | EN-текст двух узлов | RU-запись | Вердикт |
|---|---|---|---|---|---|
| 1 | `Hospitality_Util_Worker.LetterLabelGuestLost` | QuestScriptDefs/Hospitality/Script_Hospitality_Worker.xml:1140 и :1160 | идентичный: «Guest lost: {SUBJECT_definite}» | одна («Гость пропал…») | **Deliberate shared identity** — ветки `lodgers.LeftMap` и `lodgers.LeftBehind` намеренно дают одно письмо; общий TKey корректен |
| 2 | `Hospitality_Util_Worker.LetterTextGuestLost` | там же, :1142 и :1162 | идентичный | одна | **Deliberate** — как №1 |
| 3 | `Intro_Wimp.LetterLabelGuestDied` | QuestScriptDefs/Intro/Script_Intro_Wimp.xml:172 и :192 | «Guest died…» vs «Guest left behind…» | одна («Гость погиб…») | **Accidental shared identity** — разные события («погиб» vs «оставлен»), TKey не обновили при копировании ветки; RU-игрок видит «погиб» и для left-behind |
| 4 | `Intro_Wimp.LetterTextGuestDied` | там же, :174 и :194 | «…has died…» vs «…has been left behind…» | одна | **Accidental** — как №3 |
| 5 | `Intro_Wimp.LetterLabelShuttleDestroyed` | там же, :312 и :331 | «Shuttle destroyed» vs «Shuttle left behind» | одна («Челнок уничтожен») | **Accidental** |
| 6 | `Intro_Wimp.LetterTextShuttleDestroyed` | там же, :314 и :333 | «…destroyed…» vs «…left behind…» | одна | **Accidental** |
| 7 | `PawnLend.LetterLabelShuttleDestroyed` | QuestScriptDefs/Script_PawnLend.xml:222 и :238 | «Shuttle destroyed» vs «Shuttle abandoned» | одна («Челнок уничтожен», Script_PawnLend.xml:97 RU) | **Accidental** |
| 8 | `PawnLend.LetterTextShuttleDestroyed` | там же, :223 и :239 | «…has been destroyed.» vs «…has been left behind.» | одна (RU:99) | **Accidental** |

Итог по дублям: 2 deliberate (идентичный EN-текст, легитимный реюз) + 6 accidental (копипаста веток signal-обработчиков без смены TKey; имя TKey «ShuttleDestroyed»/«GuestDied» не соответствует второму узлу). Для RimLoc это не проблема, а **требуемое поведение**: дедуп по `(defName, TKey)` при скане, coverage считать по парам — тогда RimLoc бит-в-бит повторяет и тексты, и квирты официального пака.

## 6. Аномалии

- **TKey без текста**: ровно 2 узла, оба — контейнеры RulePack: `<rules TKey="DutyRulesAny">` и `<rules TKey="DutyRulesRoyal">` (Royalty/Defs/QuestScriptDefs/Script_PawnLend.xml:114, :146, класс QuestNode_ResolveTextNow). В официальный счётчик (208) не входят; в RU живут как `<defName>.<TKey>.slateRef` с `li`-детьми (`PawnLend.DutyRulesAny.slateRef`, RU Script_PawnLend.xml:67–89). Если RimLoc сканирует «узлы с текстом» — они корректно выпадают; при будущем поддержке правил их суффикс — тоже `.slateRef`.
- **TKey без defName-владельца**: 0 во всех корнях. TKey внутри абстрактных дефов (Name/Abstract без defName): 0. Вложенных Def-владельцев: 0.
- **IfModActive-контексты вокруг TKey-узлов**: 0 во всех корнях. `MayRequire` на TKey-узлах: 0. TKey в Patches-деревьях всех корней: 0 — патч-слой TKey не трогает.
- **Дубли вне Royalty**: 0 (Core/Ideology/Biotech/Anomaly/Odyssey — все пары уникальны).
- **Семантические квирты EN** (не блокер, но помнить при тестах на «осмысленность»): свап Label/Text в `Util_SpawnSiteThreat` (§4) и «погиб vs оставлен» у дублей №3–8.

## 7. Итог для RimLoc

1. **Правило идентичности подтверждено на DLC**: идентичность TKey-узла = `(defName, TKey)`; дедуп при скане обязателен и достаточен (8/8 дублей Royalty имеют одну RU-запись; случай «один узел — много RU-записей» не встречается ни разу).
2. **Суффикс-правило подтверждено и уточнено** (351 узел / 343 пары сматчены без единого исключения): `TipSetDef li` → без суффикса; parms-потомок `QuestNode_SubScript` → `.value.slateRef`; остальной QuestScriptDef (включая `value` QuestNode_Set и `rules` RulePack) → `.slateRef`. Суффикс — функция контекста, не тега.
3. **Новое знание против Core-эталона**: официальный RU может адресовать TKey-узел структурным путём (`root.nodes.…` с именами по Class) — Odyssey, 3 пары. Matcher'у нужен structural-алиас (или статус `sourceChanged`/`pending-review` вместо ложного `unmatched`), иначе Odyssey-подобные паки будут «терять» перевод.
4. **4 пары официального RU действительно нет** (§4: Util_SpawnSiteThreat ×2, ThreatReward_Infestation_ItemPod ×1, EndGame_RoyalAscent ×1) — легитимный кейс `untranslated`, фикстура на него нужна. Вместе с 3 Odyssey structural это ровно 7 unmatched-идентичностей из §1.1.

### Регрессионные фикстуры (минимальные сниппеты из реальных данных)

**Фикстура A — `.slateRef` + дубль TKey с разным текстом** (из `Royalty/Defs/QuestScriptDefs/Script_PawnLend.xml:222–239`, сокращено):

```xml
<li Class="QuestNode_Signal">
  <inSignal>pickupShipThing.Destroyed</inSignal>
  <node Class="QuestNode_Sequence"><nodes>
    <li Class="QuestNode_Letter">
      <label TKey="LetterLabelShuttleDestroyed">Shuttle destroyed</label>
      <text TKey="LetterTextShuttleDestroyed">The shuttle sent to collect colonists has been destroyed.</text>
    </li>
  </nodes></node>
</li>
<li Class="QuestNode_Signal">
  <inSignal>pickupShipThing.LeftBehind</inSignal>
  <node Class="QuestNode_Sequence"><nodes>
    <li Class="QuestNode_Letter">
      <label TKey="LetterLabelShuttleDestroyed">Shuttle abandoned</label>
      <text TKey="LetterTextShuttleDestroyed">The shuttle sent to collect colonists has been left behind.</text>
    </li>
  </nodes></node>
</li>
```
Ожидание: 2 TKey-узла → 1 пара на каждый TKey; RU-ключ `PawnLend.LetterLabelShuttleDestroyed.slateRef` (одна запись) матчит обе ветки; дедуп не теряет ни одного узла из отчёта покрытия.

**Фикстура B — parms у SubScript даёт `.value.slateRef`** (из `Royalty/Defs/QuestScriptDefs/Hospitality/Script_Hospitality_Worker.xml:866–877`, сокращено):

```xml
<li Class="QuestNode_SubScript">
  <def>Util_ArriveByDropPodsOrShuttle</def>
  <parms>
    <arrivingPawns>$arrivingPawns</arrivingPawns>
    <customLetterLabel TKey="LetterLabelLodgersArrived">[lodgersLabelSingOrPlural] arrived</customLetterLabel>
  </parms>
</li>
```
Ожидание: путь `Hospitality_Util_Worker.LetterLabelLodgersArrived.value.slateRef` (не `.slateRef`); слот `customLetterLabel` в пути не участвует. Регрессия: наивное правило «тег → суффикс» даст ложный unmatched.

**Фикстура C — structural-алиас в RU при живом TKey** (из `Odyssey/Defs/QuestScriptDefs/Script_Site.xml:190–194`, сокращено; RU — `/tmp/adjudic-ru/Odyssey/DefInjected/QuestScriptDef/Script_Site.xml:63–65`):

```xml
<li Class="QuestNode_Letter">
  <label TKey="LetterLabelSurveySiteQuestCompleted">Quest completed</label>
  <text TKey="LetterTextSurveySiteQuestCompleted">You have successfully completed the quest '[resolvedQuestName]'!</text>
</li>
```
```xml
<SurveySite.root.nodes.AllSignals.node.node.nodes.Letter.label.slateRef>Задание выполнено</SurveySite.root.nodes.AllSignals.node.node.nodes.Letter.label.slateRef>
<SurveySite.root.nodes.AllSignals.node.node.nodes.Letter.text.slateRef>Вы успешно выполнили задание «[resolvedQuestName]»!</SurveySite.root.nodes.AllSignals.node.node.nodes.Letter.text.slateRef>
```
Ожидание: пара не `unmatched`, а сматчена через structural-алиас (или помечена `pending-review`), EN-текст в комментарии подтверждает соответствие узла.

Кандидаты на четвёртую/пятую фикстуру, если понадобится: RulePack-контейнер `<rules TKey="DutyRulesAny">` → `.slateRef` с li-детьми (Script_PawnLend.xml:114) и gap-пара `ThreatReward_Infestation_ItemPod.LetterTextInfestationArrived` → ожидаемый `untranslated` с предсказанным `.value.slateRef`.

---
*Машинный отчёт сканера: `/tmp/adjudic/tkey-report.json`; скрипт: `/tmp/adjudic/tkey_scan.py`; распакованный RU: `/tmp/adjudic-ru/`. Игра не модифицировалась; файл в репозиторий не коммитился.*
