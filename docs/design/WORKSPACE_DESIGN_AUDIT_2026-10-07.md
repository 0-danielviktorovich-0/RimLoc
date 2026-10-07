# WORKSPACE DESIGN AUDIT — §13 Context/Terms/XML · §14 Related strings · §15 Contextual AI

Дата: 2026-10-07 · Лейн: Workspace-Auditor · Ревизия аудита: `main` @ `b9c53b1`
(worktree `_rimloc-worktrees/ba-main`; React-лейн `gui/tauri-app/frontend-react`,
типизация проверена `npx tsc -b` → exit 0).

Truth-источники: `docs/design/LOVABLE_R1_ADAPTATION.md`, `docs/design/RIMLOC_UI_R1_PRODUCT_CONTRACT.md`,
`docs/design/CURRENT_SVELTE_BASELINE.md` (frozen legacy), код фронтa и контракта; `UI_R1_CHECKPOINT.md` /
`UI_R1_FINAL_REPORT.md` не использовались как правда (исторические). Аудит чтением; код не менялся.

---

## §13. Context / Terms / XML

Lovable-интент (ADAPTATION:47): «Context-tabs (Контекст/Термины/XML) → Контекст = Source Inspector
(PRODUCTION_ALREADY_BETTER); XML-превью — осторожно (не эмуляция семантики)».

### 13.1 Context = живой provenance

**CURRENT.** Реализовано одним табом «Источник» (`ws.tabSource`, i18n `lib/i18n/index.ts:126/514`;
других таб-ключей нет) в `Workspace.tsx:347-412`: file+line (`source_ref` → `src.file`/`src.line`),
winner reason (`selected_by` → `src.selected-by`), view-facts (`provenance.version_selected` →
`src.version`, `conditional_branch` → `src.conditional`, `patch_stage` → `src.patch`), TKey
первичная+прочие локации (`src.tkey*`), корень источника (`src.root` из `snapshot.source_root`).
Отсутствующие данные рендерятся честным «—», никогда не фабрикуются. Провайдер данных — контракт:
`EntrySourceRefDto` (`types.ts:156-160`), `SourceProvenanceDto` (`types.ts:96-106`), `TKeyMetaDto`
(`types.ts:121-130`), `snapshot.source_root` (`types.ts:201-204`); домен —
`rimloc-domain/src/canonical.rs:142-147` (`EntrySourceRef`), `:98-119` (`SourceProvenance`).

**EXPECTED** (Lovable): таб «Контекст» с живым provenance.

**VERDICT: LIVE_SUPERIOR.** Продакшн показывает строго больше прототипа: provenance-факты версии/
условной ветки/патч-стадии, победителя precedence, TKey-локации и корень источника — всё из
контрактного снапшота. ADAPTATION:47 уже фиксировал PRODUCTION_ALREADY_BETTER; React-лейн это
воспроизвёл. Никаких действий.

### 13.2 Terms = релевантные глоссарий-матчи текущей строки

**CURRENT.** Таба нет. Контракт: список глоссария проекта — `project_glossary` (`client.ts:265-267`,
`GlossaryTermDto` `types.ts:478-483`; домен — `Project.glossary: Vec<GlossaryTerm>`,
`canonical.rs:310`); **lookup-операции «по source-тексту» для глоссария в контракте НЕТ**. Зато TM
имеет полноценный ranked lookup: `project_tm_lookup` (`client.ts:314-316`, `TmLookupRequestDto`/
`TmMatchDto` `types.ts:599-619`, exact→normalized→fuzzy, `contract.rs:1292-1330`). Глоссарий-экран
грузит весь список (`Glossary.tsx:23-38`) — проектный глоссарий мал (ручной CRUD + импорт).
Промпт ChatBatch глоссарий НЕ включает (`chat_batch.rs:624-646` — только key/source-пары).

**EXPECTED.** Таб «Термины»: термины глоссария, релевантные source-тексту выбранной строки (+ опционально
TM-матчи), кликабельно к Glossary/TM.

**VERDICT: IMPLEMENT_NOW** (только фронт, без изменений контракта).

**FIX (минимальный план).**
1. Модульный кэш `lib/state/glossary.ts` по образцу `providers.ts:43-113`: `glossaryList(project_id,
   session_epoch)` один раз на проект/эпоху, инвалидация по ревизии снапшота.
2. Таб «Термины» в `context-tabs` (`Workspace.tsx:347`): матчинг case-insensitive подстроки/слова
   `term` в `selected.source` по кэшу — O(термины × длина строки), бесплатно; вывод term→translation+note.
3. Вторым рядом — `tmLookup({source_text: selected.source, target_locale: folderForm(st.targetLocale),
   limit: 3})` с пометкой match_kind; честное «—»/«нет совпадений» при пустом результате.
4. Новые i18n-ключи `ws.tabTerms`, `ws.terms.*` (en/ru parity, как `ws.tabSource`).

### 13.3 XML/raw — только честный фрагмент source

**CURRENT.** Таба нет — и правомерно: (а) в контракте нет ни одной операции чтения фрагмента файла
(полный список `ContractMethod` `types.ts:387-449`; production invoke-handler `main.rs:3965-4024` —
только контрактные ops + «safe read-only legacy extras» без файлового чтения); (б) у webview НЕТ
fs-доступа: capability `default.json` — только `core:default` + `dialog:*`; (в) единственный
«просмотрщик source» в истории продукта — Svelte W7 **синтетические фикстуры**
(`frontend-v2/src/lib/source/fixtures.ts:1-4`: «SYNTHETIC source fixtures… demo DATA»), что честности
контракта (`PRODUCT_CONTRACT:38-41`) не подходит как образец. `source_ref.file` — путь ОТНОСИТЕЛЬНО
корня проекта (`canonical.rs:137-140`), абсолютный корень есть в `snapshot.source_root`, но на wire
фрагмент не ездит.

**EXPECTED** (по заданию): XML/raw ТОЛЬКО если backend честно отдаёт реальный фрагмент; иначе —
задокументированный reject, не эмуляция.

**VERDICT: INTENTIONALLY_REJECTED.** Backend не отдаёт фрагмент; читать фронтому нечем и нельзя;
эмуляция запрещена мандатом (ADAPTATION:47). Reject фиксируется этим документом.

**Предусловие пересмотра (не сейчас):** read-only контрактная op вида `source_fragment`
(`project_id` + `entry` identity → N строк вокруг `source_ref.line` из `source_root` открытой
сессии, bound + read-only, отказ если source_root неизвестен/файл недоступен). Только после неё
таб XML/raw становится реализуемым честно.

---

## §14. Related strings (label ↔ description одного Def)

**CURRENT.** Группировки нет: grep по `frontend-react/src` на related-группировку пуст (класс
`.related-string` в `r1.css` используется только для TKey-блока, `Workspace.tsx:382`).
Каноническая идентичность в контракте ЕСТЬ и она полна: `SourceEntryId { kind, key, def_type }`
(`canonical.rs:23-36`, wire `SourceEntryIdDto` `types.ts:32-36`), ключи строятся детерминированно
как `{defName}.{field}` (`rimloc-parsers-xml/src/lib.rs:891,1555,1869,2199,…`; TKey —
`{defName}.{TKey}`, `rimloc-core/src/lib.rs:65`), `defName` берётся из XML `<defName>`
(`parsers-xml:804-809`). Клиент уже держит полную идентичность в `WorkspaceEntry.identity`
(`project.ts:17-37`). Быстрый переход существует: `projectStore.select(key)` (`project.ts:198-200`)
→ `selectedKey` (`Workspace.tsx:54-55`).

**EXPECTED** (ADAPTATION:48): «related по canonical identity (defName-группа)», без хардкода модов.

**VERDICT: IMPLEMENT_NOW** (только фронт).

**FIX (минимальный план).**
1. Группировка в `useMemo` над `st.entries` (один проход на снапшот, O(n)):
   группа = `(def_type, key.split('.')[0])` для `kind ∈ {def_injected, t_key}`; `keyed/strings/
   backstories/patch_derived` — без defName-структуры, группу не получают. `def_type` участвует
   в ключе группы — контрактив: «два def type с одним ключом — два разных Def»
   (`canonical.rs:29-34`) соблюдён. Хардкода модов нет — только contract identity.
2. Секция «Связанные строки» в контент-блоке редактора (рядом с `src.tkey`): строки группы —
   suffix поля (`key` после первого `.`), target-статус, клик → `projectStore.select(key)`
   (переход уже работает, `Workspace.tsx:138`). label/description не «угадываются» —
   показывается реальный suffix поля (`.label`, `.description`, …) из identity.
3. i18n `ws.related*`, testid `ws.related.*`; кап на видимые строки (например 8) + «+N ещё».

---

## §15. Contextual AI (ChatBatch — отдельный экран; вход из Workspace)

**CURRENT (экран).** ChatBatch — полноценный ОТДЕЛЬНЫЙ экран: маршрут `#/chatbatch`
(`App.tsx:79,361-362`), пункт nav (`App.tsx:98`), команда палитры (`App.tsx:52`); полный живой цикл
create→export prompt→copy→paste import→preview→apply через контракт (`ChatBatch.tsx:130-201`;
`client.ts:353-392`; `contract.rs:1490-1653`; гейты identity/revision/source-hash на бэкенде,
apply = origin=import persist-before-ack). Скриншот-данные не имитируются.

**CURRENT (вход из Workspace).** Контекстного входа НЕТ: в Workspace одиночный выбор
(`selectedKey`, `Workspace.tsx:138`; чекбоксов на строках нет), футер — только «назад»+статус
(`Workspace.tsx:181-187`); CSS-класс Lovable-полосы `.translation-action` в React-лейне не
используется (grep по components — пусто). ChatBatch держит выбор в локальном `useState`
(`ChatBatch.tsx:33`), handoff-механизма нет; ключи выбора — displayIdentity
(`ChatBatch.tsx:24-26`, 1:1 с бэкендом `canonical.rs:43-56`), а не `e.key` воркспейса.
LLM-движка в GUI-контракте НЕТ: `ContractMethod` не содержит translate; `rimloc_llm` в сервисах
используется только keychain/presets (`providers.rs:17,53,69-79`); `engine::translate`
(`rimloc-llm/src/engine.rs:74`) живёт в CLI (`rimloc-cli/src/lib.rs:298`). Провайдеры в GUI —
redacted config-only (`has_key`, `types.ts:637-647`). Мандаты: «никогда не звать платное только
потому что ключ есть» (ADAPTATION:63, §34); «полоса машинного перевода ведёт в реальный AI/no-API
flow (chatbatch)» (ADAPTATION:49).

**VERDICTS.**
- Экран ChatBatch: **LIVE_PARITY** (отдельный экран есть, живой, канонический).
- Контекстный вход (selection → ChatBatch; текущая строка → batch-of-1): **IMPLEMENT_NOW**.
- In-editor «AI draft» автозвоном/фоновым вызовом платного провайдера: **INTENTIONALLY_REJECTED** —
  контракта на LLM-вызов в GUI нет, автоматический платный вызов запрещён мандатом §34. Пересмотр —
  только после появления явной user-initiated контрактной op (решение владельца), не shortcut'ом.

**FIX (минимальный план входа, только фронт).**
1. Мульти-выбор в Workspace: `selectedKeys: Set<string>` + чекбокс в `entry-row`
   (`Workspace.tsx:192-241`), «выбрать все видимые» в тулбаре фильтров.
2. Действие «Перевести через чат-батч» в футере редактора/списка: пишет handoff и переходит
   `#/chatbatch`. Handoff-модуль `lib/state/chatBatchHandoff.ts` (set/take-once), переносит
   `displayIdentity(e.identity)` — тот же хелпер, что `ChatBatch.tsx:24-26` (вынести в общий
   модуль, чтобы ключи выбора совпадали с `entry_keys` контракта).
3. ChatBatch на маунте: `takeHandoff()` → `setSelected(new Set(keys))`; если среди перенесённых
   есть строки с переводом — снять `untranslatedOnly` (`ChatBatch.tsx:34,47-50`), иначе перенос
   невидим. Текущая строка = частный случай (один чекбокс): create+export — два явных клика
   пользователя, ничего не зовётся автоматически.
4. Провайдер НЕ гейтит вход: ChatBatch — no-API flow, провайдеры к нему не относятся; при
   настроенных инстансах можно показать нейтральную подсказку «провайдеры используются CLI
   translate» со ссылкой на `#/providers`. i18n `ws.ai.*` / `cb.handoff*`.

---

## Сводка

| # | Пункт | Вердикт | Backend-работа |
|---|---|---|---|
| 1 | §13 Context (SOURCE-блок) | LIVE_SUPERIOR | не нужна |
| 2 | §13 Terms таб | IMPLEMENT_NOW | не нужна (glossary list + tm_lookup уже в контракте) |
| 3 | §13 XML/raw таб | INTENTIONALLY_REJECTED | предусловие: read-only `source_fragment` op |
| 4 | §14 Related strings | IMPLEMENT_NOW | не нужна (identity kind/key/def_type уже в снапшоте) |
| 5 | §15 ChatBatch экран | LIVE_PARITY | не нужна |
| 6 | §15 Contextual entry (selection → ChatBatch) | IMPLEMENT_NOW | не нужна (chat_batch_create принимает entry_keys) |
| 7 | §15 In-editor автоматический AI draft | INTENTIONALLY_REJECTED | мандат §34; нужен явный контракт-op при пересмотре |
