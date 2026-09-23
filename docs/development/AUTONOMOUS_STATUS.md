# AUTONOMOUS STATUS — RimLoc RC campaign

Session: `zcode-rc-campaign` · Updated: 2026-09-23 (после Ф3) · **Не пушить — вся работа локально.**
Recovery point: этот файл + `~/Developing/_rimloc-safety/` (bundle, stash diff, логи).

## Текущее состояние — ИТЕРАЦИЯ 2 ЗАВЕРШЕНА → И3 (PRODUCT & PUBLIC RELEASE, мандат A–AP)

**Чекпоинт И2 (проверен): 114/0 тестов, clippy/deny чисто, HEAD 2e0f799+, дерево чисто, пуша нет.**
Кумулятив И2: слепой бенчмарк (adjudication: 0 реальных миссов) · provenance (Gemini-AI пак; OW.RU.* = RimLangKit-конвейер) · многоязычный промпт · матрица конкурентов v2 · first-party аудит (13 категорий TranslationReport из декомпила DLL 1.6) · modview (LoadFolders) · version-diff · TODO-семантика · capability-aware WordInfo (`rimloc word-info`) · whole-game (Core EN 11549 / RU 16681 / 93%). Блокер: локальная LLM запрещена (ресурсы), платные API не авторизованы; GLM Flash через ZCode = разрешённый zero-cost путь для семантического бенчмарка (§20 мандата).



## И3-GATE: P1-фиксы независимого ревью ChatGPT (ДО глубокой привязки GUI) — В РАБОТЕ

Ревью: CORE TKEY EXTRACTION PASS, 112/112 PASS, FREEZE NOT YET. P1-очередь:
- P1-1 unify inventory (scan_units/scan_units_auto/+dict/+fields — один канонический пайплайн, TKey везде) + регрессия на GUI/service path
- P1-2 TKey round-trip: базовая идентичность `<defName>.<TKey>` НЕ сериализуется игрой — нужен path_hint (.slateRef / .value.slateRef / bare по типу узла) в TransUnit (additive, schema minor) + импорт-райтер + e2e фикстуры 3 форм
- P1-3 кросс-язычные валидаторы (placeholders/lists/orphans) TKey-aware: exact → TKey-алиас ТОЛЬКО для известных TKey-идентичностей; тесты: .slateRef ок, .value.slateRef ок, bare ок, настоящий orphan, не-TKey .slateRef НЕ алиасится
- P1-4 прямой регресс 5016be4: CLI coverage на TKey-фикстуре с target DefInjected: A) base→.slateRef translated=1; B) →.value.slateRef translated=1; C) TODO→missing; D) absent→missing
- P1-5/P1-6/P1-7: один общий резолвер матчинга (coverage/uncovered-sample/placeholder/glossary — один результат), детерминизм (0/1/>1 кандидатов), тесты мультивариантности
- P2-8 scan_defs_tkey external --defs-dir (ходить по defs_root напрямую) + регрессия
- P2-9 tkey_audit.py: TemporaryDirectory, без stale /tmp кеша, reproducibility по полным нормализованным выводам
- P2-10 DLC TKey adjudication (Royalty 208/200 — 8 дублей: исследовать) — субагент
- P2-11 TKey НЕ «новое в 1.6» — система с 1.1 (май 2020, QuestScriptDefs/TipSetDefs): исправить доки/комментарии; 1.6 остаётся primary tested
- P2-12 противоречия в RIMWORLD_REFERENCE_AUDIT (implemented vs not-parsing; identity-формулировки) + точное описание git-состояния («tracked clean; known untracked testlab artifacts»)
- BROADER P1: load precedence (Defs first-wins, Keyed last-wins) — до Pass A, не блокирует Svelte-скаффолд

**АРХИТЕКТУРНЫЙ ГЕЙТ (каноническая проектная модель)**: PO → adapter, не внутреннее состояние. Аудит PO-зависимостей → CANONICAL_PROJECT_MODEL.md; каноническая модель SourceEntry/TranslationUnit/(Project/LanguagePair/status/provenance); TKey round-trip как архитектурный тест; персистентность по evidence (не SQLite-ради-SQLite); GUI/LLM/TM/MCP — только от канонических сервисов; fidelity-матрица адаптеров; недеструктивная миграция. «Если архитектура УЖЕ удовлетворяет — задокументировать доказательство, не переписывать».


## P1-ПРОГРЕСС (И3-GATE): фундамент резолвера готов (fdd3076), интеграция — следующая

- **ГОТОВО**: services::matching::SourceMatcher — ЕДИНЫЙ резолвер (P1-6): exact→TKey-fallback ТОЛЬКО для известных TKey-идентичностей (P1-5), детерминированный мультивариантный выбор (P1-7: non-TODO предпочтение, сортировка), 3 теста. 120 тестов зелёные.
- **СЛЕДУЮЩИЙ КОММИТ (интеграция)**: coverage_report → SourceMatcher (замена tgt_exact/tgt_canonical словарей; TODO-правило через resolve); compare metrics/uncovered-sample/placeholder/glossary → тот же резолвер (P1-6 частично остался); валидаторы cross-language → SourceMatcher (P1-3); P1-4 регрессии A-D через CLI coverage на test/TKeyMod + Languages/Russian/DefInjected TKey-suffixed цели.
- **ПОТОМ**: P1-2 round-trip (TransUnit.tkey_suffix additive + import-po writer .slateRef/.value.slateRef/bare по типу узла; проверка по 208 Royalty узлам — субагент DLC-adjudication в полёте); P1-1 unify (scan_units_auto += TKey); P2-8 external defs_root; P2-11 TKey с 1.1 (НЕ 1.6!) — исправить доки; P2-12 противоречия аудита; P2-9 audit-script hardening.
- **АРХИТЕКТУРНЫЙ ГЕЙТ** (каноническая модель, PO=adapter): аудит → CANONICAL_PROJECT_MODEL.md; «если уже удовлетворяет — задокументировать доказательство».

## LATEST — DLC TKey ADJUDICATION ЗАВЕРШЕН (суффикс-правило закрыто)

- `DLC-TKEY-ADJUDICATION.md`: 351 узел / 343 пары, 0 исключений. Суффикс = функция КОНТЕКСТА: `parms` (QuestNode_SubScript) → `.value.slateRef` (закрыт Core-подслучай!); TipSetDef `li` → bare; остальные → `.slateRef`. 8 дублей Royalty: 2 deliberate + 6 accidental копипаст (официальный пак = 1 запись на пару → дедуп (defName,TKey) подтверждён). 3 Odyssey structural-алиаса = известное ограничение матчёра. 3 реальных gap официального RU.
- Реализация в `scan_defs_tkey`: детекция суффикса по контексту (parms/li/прочие) + `TransUnit.tkey_suffix` (P1-2) — СЛЕДУЮЩИЙ КОММИТ перед GUI-фризом.

## LATEST — SUPPLEMENT БАНДЛ ГОТОВ (50f9483 → cf60ee7) + inventory-аудит 22 команд

- **Supplement**: `~/Developing/_rimloc-review/RimLoc-TKey-50f9483-to-cf60ee7-supplement.zip` (20 файлов; SHA-256 `6d22fcfc92fa0b17d96585a364dd03cec3e767a7c18f0508e99f84c7292bcf26`; sha256-файл проверен shasum -c OK). COVERAGE-TKEY-FOLLOWUP.md: дефект (coverage = путь B scan_all_units* без TKey; scan/translate/compare = путь A с TKey), фикс в КОРНЕ семейства пути B (все потребители автоматически), полная таблица аудита 22 CLI-команд.
- **Inventory-аудит итог**: 7 команд TKey-aware (scan/coverage/validate/compare/translate/version-diff/diff-xml); 15 — TKey неуместен по дизайну (build/import/export/annotate/learn-*/morph/…); 2 честных наблюдения не-дефектного класса: word-info отсекает TKey фильтрами независимо от сканера; export-po — Keyed-only ограничение PO-контура (кандидат в тикет).
- **Расхождение с коммит-сообщением 5016be4** зафиксировано честно: фикс НЕ затронул word-info (слово в сообщении избыточно).
- Оба бандла готовы к загрузке в ChatGPT: primary (60 файлов) + supplement (20 файлов).

## LATEST — REVIEW BUNDLE READY + фоновые агенты завершены (2026-09-23)

- **Ревью-бандл готов**: `~/Developing/_rimloc-review/RimLoc-TKey-50f9483-review.zip` (60 файлов; SHA-256 `3e06b443dc1e385c85f40b1e7d06cb9578e07bf92676cc765490153d4bd5aacb`). Ревьюер проверяет: TKey-извлечение, reconciliation 112/226/358, двойное извлечение, canonical-нормализацию, коллизии, coverage>1.0 фикс, независимость тестов, EN/RU-допущения, безопасность фриза. Загрузить ZIP в ChatGPT.
- **Post-bundle фикс (5016be4)**: coverage шёл через scan_all_units* без TKey-прохода (находка пакинг-агента) → TKey в полном инвентаре с дедупом. Ревьюерам оценивать 50f9483 + follow-up.
- **DISCOVERABILITY.md готов** (SEO-стратегия): топ-интенты, description/topics proposal (12 живых topics), MkDocs-возможности (meta/canonical/sitemap+hreflang из коробки; OG через social-плагин; robots.txt статикой), **находка: docs/development/** утекает в публичный sitemap → exclude_docs на фазе hardening**.
- **Docs-UX knowledge base** — субагент завершил: создана в AI-OS по конвенциям (База-знаний/Темы/Документация-UX: INDEX, references, patterns (24+), framework-bakeoff, starter-checklist) + скилл .agents/skills/docs-product-design/SKILL.md. Детали — в отчёте агента (интеграция/валидация скилла — при первой docs-фазе).
- Следующая фаза: GUI RC (дизайн через ui-ux-pro-max → Svelte 5 + TS → i18n ru/en → E2E).



## LATEST — TKey integrity audit готов, review-бандлы готовы, Docs-UX база создана (2026-09-23)

- **Два ревью-бандла для ChatGPT**: primary `RimLoc-TKey-68b5381-to-50f9483` (60 файлов) и supplement `RimLoc-TKey-50f9483-to-cf60ee7-supplement` (20 файлов) в `~/Developing/_rimloc-review/` + .sha256. Ревьюер проверяет: TKey-семантику, reconciliation чисел (112/226/358; «+226» = арифметическая ошибка двух баз), двойное извлечение (0), canonical-нормализацию, коллизии, coverage>1.0 фикс, независимость тестов, EN/RU-допущения, безопасность фриза.
- **TKey реализован и прошёл integrity audit**: `scan_defs_tkey` (+226 записей в Core), идентичность `<defName>.<TKey>`, канонические суффиксы нормализуются fallback-only; 0 двойных/коллизий/необъяснённых. Коллизионные тесты поймали coverage>1.0 баг — исправлен.
- **Inventory-аудит 22 CLI-команд**: 7 TKey-aware, 15 неуместны по дизайну; word-info/export-po нюансы задокументированы честно (5016be4 commit-message упоминал word-info избыточно — зафиксировано).
- **DISCOVERABILITY.md** (SEO): топ-10 интентов, 12 topics, description, MkDocs-возможности; находка: docs/development/** утекает в sitemap → exclude_docs на фазе hardening.
- **Docs-UX knowledge base создана в AI-OS** (кросс-проект): База-знаний/Эксперты/Документация-UX (references OpenClaw/Z.AI/OpenAI/Claude + Starlight/Fumadocs/Rspress/MkDocs, patterns ×25, bake-off ×19 критериев, starter-checklist) + скилл .agents/skills/docs-product-design (валидаторы зелёные, в AI-OS незакоммичено — норма Obsidian). Bake-off: RimLoc остаётся на MkDocs Material сейчас; AI-нативность — хуком билда; триггеры миграции документированы.
- **Следующая фаза: GUI RC** — дизайн через ui-ux-pro-max, Svelte 5 + TS, редактор SOURCE|TARGET, i18n ru/en, tauri-driver E2E, полный джорни. Entry model заморожена (см. RIMWORLD_REFERENCE_AUDIT гейт-таблицу).

## И3 — ФАЗЫ ПО §AO (строгий порядок, не менять без причины)
1. **GUI RC** (И2-E переносится сюда): Svelte 5 + TS, «Translate a mod» beginner-кнопка (§B), progressive disclosure 3 слоя (§C), first-run onboarding 3-5 экранов (§D), UI i18n ru/en (§F), редактор SOURCE|TARGET, E2E tauri-driver, полный GUI-джорни без CLI-fallback, Windows=first-class (§G), plain-language copy (§E)
2. **Функциональные Pass A/B** (§35/36 предыдущего мандата)
3. **Beginner-player UX acceptance** (§27)
4. **Repo/docs hardening**: README=продуктовая лендинг (§I), docs-IA по аудиториям (§H), SECURITY.md реальный (§L), CONTRIBUTING (§M), CoC (§N), AGENTS-рефактор + nested (§O), CHANGELOG чистка (§P), SUPPORT.md (§T), issue/PR-формы (§S), позиционирование «RimWorld Localization Workstation» (AU)
5. **Agent/CLI skill hardening**: agent-friendly CLI (§V: JSON/exit-codes/--non-interactive), RimLoc AI USER SKILL (§U, ≠ AGENTS.md), MCP thin adapter если останется scope (§X)
6. **WordInfo/MCP** — по остатку
7. **Branch cleanup audit** → BRANCH_CLEANUP.md (§Q/AL, без удалений до одобрения)
8. **Release engineering**: единый release-authority (§AT/Y), Codecov (§AQ: llvm-cov, flags rust/frontend, patch-gate), security-аудит workflows (§AS: SHA-pin, OIDC, без || true в гейтах), RELEASE_ENGINEERING.md (§AM)
9. **Cross-platform RC** (§AA/AB): Win/macOS/Linux артефакты + smoke, signing prepared-not-faked (§AC), SBOM/attestations (§AD)
10. **Release notes + PUBLIC_RELEASE_READINESS.md** (§AE/AF/AN), RimSort-interop → RIMSORT_INTEROP.md (§AV, не блокирует), community testing plan (§AJ, без рассылки)
11. **STOP** → decision packet (§AP), ждать одобрения

**Новые документы к созданию**: RELEASE_ENGINEERING.md · PUBLIC_RELEASE_READINESS.md · BRANCH_CLEANUP.md · RIMSORT_INTEROP.md · ai-user-skill (упаковка по конвенциям агентов) · .github/release.yml
**Жёсткие правила И3**: без пуша/релиза/удалений; Windows проверять на реальных артефактах; не рекламировать непротестированное; без AI-маркетингового воды; «high-confidence inferred provenance» для отпечатков.

## И2 (архив — всё закрыто):

**Новый мандат Даниэля (2026-09-23): FIRST-PARTY RimWorld reference audit (§A–H)** — RimWorld как первоисточник (декомпил в .app/Source, Ludeon-репо, Translation Report как оракул), effective mod view, provenance-иерархия документации.

**Готово в этом мандате:**
- `crates/rimloc-services/src/modview.rs` — **effective mod view (§B)**: парс LoadFolders.xml (BOM-толерантный, теги v1.x, `/`=корень, IfModActive→conditional отдельно), классический fallback 1.x-папок, `defs_roots()` для скоупинга извлечения. 3 теста. База для validate-орфанов и строгого patch-резолва.
- `docs/development/COMPETITOR_MATRIX.md` v2 (субагент): implementation/design-матрица + **workflow-археология: OW.RU.* паки — high-confidence inferred provenance = конвейер автора RimLangKit** (детерминированные отпечатки EncodingFixer/CommentInserter/CaseCreator в 191/191 файлах; прямого подтверждения автора нет). Следов RimTrans/Text-grabber нет. Топ-5 adopt/adapt: формат сообщества как дефолт вывода, source-text TM, единый ExtractionFilter, About/discover/loadFolders, дозакрытие LLM-движка.
- `docs/development/TRANSLATION_BENCHMARK.md` + слепой бенчмарк (§8): 0 реальных извлекательных миссов на VWE (adjudication: VEF-derived/version-skew/speculative); Genetics пак↔база 82.7%; provenance референса = AI Gemini + human (заявлено в About пака).

**OFFICIAL_LANG_PACKS.md — готов и интегрирован (0af951f):**
- WordInfo только у ru/de/uk; ja/zh — без воркера и WordInfo
- de: `{replace:}`-макрос (вложенный), автогенерация WordInfo через GitHub Actions; uk: 7 падежей без воркер-класса; ja/zh: легитимное переупорядочивание плейсхолдеров
- Валидатор: set-based сравнение (порядок не важен для CJK) + внутренние плейсхолдеры `{replace:}` — зафиксировано 3 тестами
- 5 first-party фикстур в testlab/fixtures-official/ с PROVENANCE.md (только парсинг-регрессия, не переводческий корпус — лицензия Ludeon-репо не объявлена)
- Формат нестабилен даже в официальных репо (BOM ±, CRLF/LF) — парсер обязан быть толерантным

**GAME_SOURCE_FINDINGS.md — готов (декомпил DLL 1.6.4871, не community-2018):** 13 категорий TranslationReport дословно; load rules (Languages из каждой content-папки, li descending+дедуп, Keyed last-wins, регистронезависимые теги, фолбэк ≤); TKey-система (1.6-новое). Поправки: отчёт пишется на Desktop; Backstories legacy мёртв.

**RIMWORLD_REFERENCE_AUDIT.md (§H) — создан** (0e01c74): иерархия источников, маппинг 13 категорий → подсистемы RimLoc (✓/частично/✗ с приоритетами), 3 задокументированных расхождения «документация vs рантайм».

**ВЕРСИОННАЯ ПОЛИТИКА (мандат Даниэля):** 1.6 — primary tested target; 1.5/1.4 — compatibility; старше — best-effort без заявлений о поддержке. Резолвер принимает явную версию (modview + resolve_game_version_root); union-скан — только явный `--include-all-versions` (maintenance-режим), по умолчанию всегда версия-скоупед. **`rimloc version-diff --from --to`** (ddaf141): unchanged/changed/new/removed + review-очередь, JSON/MD; на реальном VWE 1.5→1.6 = 317 unchanged. Project-level target version = `game_version` в rimloc.toml (уже был) + Auto (latest ≤).

**WHOLE-GAME SUPPORT (мандат §A-H доп.):** Data-корни (Core/DLC) работают как источники — Core EN scan = 11549, официальный RU тар = 16681 (93% пересечения); резолвер принимает корни без About.xml; GAME_LOCALIZATION_SUPPORT.md (LocalizationSource-маппинг + translation-maintainer workflow + остатки: tar-адаптер, Strings/Backstories-извлечение, NoTranslate/TKey, DLC-прогон NOT TESTED). Гейт §1 ЗАКРЫТ (TODO-семантика + capability-aware WordInfo `rimloc word-info`: на VWE 158 лейблов без WordInfo).

**TKEY INTEGRITY AUDIT ПРОЙДЕН (79abe52, финальная коррекция):** machine-evidence `testlab/scripts/tkey_audit.py` — 358 TKey-узлов по всем корням (Core 112 / Royalty 208 / Ideology 3 / Biotech 5 / Anomaly 3 / Odyssey 27), скан воспроизводим, эмиттировано ровно 112 (дедуп 0), двойного извлечения 0, официальный RU: 90 exact + 22 canonical + 0 unmatched + 0 коллизий + 0 необъяснённых. Коллизионные тесты поймали и исправили реальный баг (coverage>1.0 при множественных суффикс-вариантах одной TKey-идентичности). «+226» из старого лога — арифметика двух баз; авторитетные числа — из аудита.

**ГЕЙТ §1 ПЕРЕЗАКРЫТ по финальной коррекции (поправка Даниэля: TKey ≠ BLOCKED):**
- TKey **РЕАЛИЗОВАН** (7646e9a): семантика выведена из installed 1.6 данных + официального RU-пака — `<defName>.<TKey>` + канонические суффиксы; `scan_defs_tkey` (+226 записей Core), фикс. тест, canonical-нормализация в compare/coverage; нераскрытый подслучай задокументирован (полный список типов узлов с `.value.`-вариантом — 2 из 112 наблюдений)
- NoTranslate: регрессия `scan_never_emits_nontranslatable_technical_fields` (allowlist-гарантия «невозможно по конструкции»); дифф с Translation Report — при следующем acceptance-прогоне
- Entry identity: SourceEntryId/TranslationUnit разделение задокументировано как доказуемо эквивалентное (TransUnit target-независим; pair-scoping на слоях перевода) — MULTILINGUAL_ARCHITECTURE.md
- Семейства: Keyed/DefInjected/TKey(реализован)/Backstories(=DefInjected)/RulePack(через Defs-словарь) — модель покрывает; **Strings (.txt) — единственный известный формат вне модели, адаптер зафиксирован как остаток** (не блокирует: ключ-представление совместимо)
- 116/0 тестов

**ГЕЙТ §1 ЗАКРЫТ (первичная классификация):** NoTranslate=закрыт (allowlist)+P2-диагностика; TKey=BLOCKED явно (0 употреблений в корпусе 287 модов, фикстур не построить — зарезервированный extension point); field-not-found=закрыт (orphan-диагностика, в GUI validation suite всегда); TODO=закрыт. **Entry model заморожена**: источники Keyed/DefInjected/Defs/Patches + TKey-reserved; статусы untranslated/translated/TODO-missing/orphan-invalid/sourceChanged/pending-review; идентичность (source kind, key, language pair). AQ-AU персистентны в AUTONOMOUS_PLAN.md (§И3).

**Следующие задачи (по приоритету):**
1. И2-E GUI Svelte 5 — крупнейший незакрытый кусок (редактор SOURCE|TARGET, i18n ru/en, E2E tauri-driver)
2. P2 из reference audit: NoTranslate-атрибуты, TKey-пути, TODO-семантика в coverage, поле-не-найдено валидация — каждая с фикстурой
3. WordInfo-генератор (ru/uk/de capability) с missing-WordInfo диагностикой (§F)
4. И2-F acceptance 2+ кейса (поправка: отчёт на Desktop) → Pass A/B → финал

**И2 завершённые фазы (архив):**

**Дальше (по завершении субагентов):**
1. Интегрировать находки → docs/development/RIMWORLD_REFERENCE_AUDIT.md (§H, provenance-классификация)
2. Translation Report категории → новые validate-правила (§C): missing DefInjected/Keyed, unnecessary translations, argument discrepancies
3. WordInfo: генератор + missing-WordInfo диагностика + официальные паки как regression-корпус (§F)
4. И2-E GUI (Svelte 5, редактор SOURCE|TARGET, i18n ru/en, E2E) — крупнейший оставшийся кусок
5. И2-F RimWorld acceptance 2+ кейса → И2-G Pass A/B → финал

**И2 ранние фазы (архив):** (продолжение по расширенной спецификации Даниэля)

**Реверификация чекпоинта И1 (2026-09-23, независимо повторена):**
- git: clean, ahead 62, HEAD=3019135 ✓
- `cargo test --workspace`: 99 passed / 0 failed ✓
- `cargo clippy -D warnings` чисто, `verify-source-hashes.sh` OK (1066 файлов) ✓
- `compare` на VWE: human 45.4% ✓; `translate` жив ✓

**БЛОКЕР (решение Даниэля 2026-09-23): локальные LLM не запускать — не хватает ресурсов.** → §9 (семантический бенчмарк реальной генерации) заблокирован; все структурные/извлекательные/GUI/конкурентные работы продолжаются. MockProvider остаётся для детерминированных тестов. Оllama-инфраструктура в коде осталась (keyless localhost + автодетект модели, коммит 34c9eb4) — пригодится, когда ресурсы позволят.

**Найден и исправлен P1-баг**: version-резолвер падал на плоских модах — fallback на корень ТОЛЬКО при наличии версии в About/supportedVersions (защита от опечаток сохранена; 2 старых теста подтвердили семантику) + регрессионный тест.

**Сделано в И2 к текущему моменту:**
- §8 СЛЕПОЙ БЕНЧМАРК: corpus VWE/Genetics(base↔pack)/VFE/QEA; adjudication — ноль реальных извлекательных миссов на VWE (reference-extra = VEF-derived рецепты + version-skew + спекулятивные записи); Genetics: RimLoc 2340 ключей, пак перевёл 82.7%; docs/development/TRANSLATION_BENCHMARK.md + testlab/scripts/blind_benchmark.py
- §4 PROVENANCE: Genetics-пак сам заявляет «AI Gemini 2.5 Pro + ручная правка» — слово gold убрано из лексикона
- §1 МНОГОЯЗЫЧНОСТЬ: LLM-промпт параметризован (нейтральное ядро + capability-блоки ru/ja/uk/de/generic, выбор по коду и имени папки); аудит всех слоёв → docs/development/MULTILINGUAL_ARCHITECTURE.md; TM/glossary pair-scoping записан как требование F7
- §5: субагент дописывает implementation-матрицу конкурентов (файл COMPETITOR_MATRIX.md — его владение)
- §9 БЛОКЕР: локальная LLM запрещена Даниэлем (ресурсы), платные API не авторизованы

И1 (архив):** (Ф0–Ф6 + Ф9-acceptance + compare). CLI-пайплайн = RC-кандидат. Итоги: docs/development/FINAL_ACCEPTANCE.md.
**СЛЕДУЮЩАЯ ИТЕРАЦИЯ (высший приоритет): Ф7 GUI** — Svelte 5 + TS бутстрап в gui/tauri-app: (1) frontend-v2 + vite + TS, (2) IPC-поверхность src-tauri ЗАМОРОЖЕНА — фронт ходит через существующие 38 команд, (3) экраны: мод-пикер → обзор проекта → редактор EN|RU (таблица, фильтры статусов, инлайн-правка, бейджи, LLM/TM-кнопки) → validate/diff/build/compare → провайдер-конфиг, (4) дизайн-проход скиллом ui-ux-pro-max, (5) tauri-driver E2E + полный GUI-джорни, (6) расщепление src-tauri/main.rs на command-модули при переносе.
Затем: Ф8 бенчмарки → Ф10 формальные Pass A/B до чистых проходов → финальный отчёт → пуш по одобрению.

## Git (локальный main, AHEAD origin — не пушить)
Коммиты кампании (сверху вниз):
- `feat(cli): add translate command wiring the rimloc-llm subsystem` (517b226) — **e2e на VWE: 317 строк, mock-перевод → import-po → покрытие 38%→65%**, dry-run без вызовов, чекпоинт-резюм подтверждён
- `feat(core): add rimloc-llm translation subsystem` + `llm: elide lifetime` — Provider trait, Anthropic + OpenAI-compat (openai/zai/ollama пресеты), MockProvider, движок (батчи/ретраи/чекпоинты/глоссарий/строгая валидация плейсхолдеров), 11 unit-тестов без сети
- `ci: add CI quality gates and cargo-deny policy` — ci.yml (fmt/clippy/test×3OS/gui/deny), deny.toml, лицензии крейтам
- `chore(deps): bump roxmltree 0.21, libloading 0.9, docs stack and actions`
- `chore(deps): bump schemars 0.8 -> 1.2 and regenerate JSON schemas`
- `chore(deps): bump quick-xml 0.36 -> 0.42` (API-миграция: str-QName, xml10_content, Event::GeneralRef)
- `chore(deps): bump zip 0.6 -> 8.6` (SimpleFileOptions в тестах)
- `chore(deps): refresh lockfile; reqwest 0.13, dirs 7, image 0.25`
- `fix(gui): harden tauri trust boundaries and fix zip-slip` (8efa526) — CSP, save_text_via_dialog, plugin-allowlist, open-крейт, zip-slip guard + негативные тесты; SECURITY_AUDIT.md
- `fix(validate): fix language scoping in scan, coverage and validate` — T5/T6/T7 + MultiLangMod фикстура + 3 регрессии
- `chore(tests): testlab: manifests, hash guard, dogfood tickets`
- `chore(repo): docs(repo): autonomous plan/status` (2b50aec)
База: rebase 43 локальных коммитов на origin/main (+2 веб-коммита), clean. Bundle: `~/Developing/_rimloc-safety/rimloc-baseline-2026-09-23.bundle`, тег `f0-baseline-20260923`.

## Состояние качества (после Ф6)
- `cargo test --workspace`: зелёный (82+11 llm) · clippy -D warnings чисто · fmt clean · cargo-deny ok
- Команда `rimloc translate` в CLI: mock/anthropic/openai/zai/ollama; FTL en+ru паритет
- Полный цикл доказан: scan → translate(mock) → PO → import-po → validate → coverage 65%

## Артефакты кампании
- `docs/development/AUTONOMOUS_PLAN.md` — дорожная карта Ф0–Ф10 + ограничения
- `docs/development/SECURITY_AUDIT.md` — S1–S11: все Critical/High FIXED, негативные тесты
- `docs/development/COMPETITOR_MATRIX.md` — код-левел матрица 6 конкурентов, топ-10 гэпов
- `testlab/dogfood-tickets.md` — T1–T8 со статусами (5 fixed, 2 not-a-bug, T4/T8 отложены осознанно)
- `testlab/research/rimworld-isolated-run.md` — процедура acceptance: `-savedatafolder=`, batch-прогон (прецедент rwmt/Multiplayer HostUtil), APFS-клон .app для тест-модов, grep-маркеры ошибок локализации, ручной smoke-чеклист
- `testlab/manifests/{installed-mods,real-mods}.json` — 287 модов, 186 VE, 161 VE без RU

## ДАЛЕЕ (по порядку)
1. **СЕЙЧАС**: compare-сервис пишет субагент (владение: crates/rimloc-services/src/extras/compare.rs + mod.rs) → я интегрирую CLI-команду `rimloc compare` и прогоняю на VWE (human RU vs mock-LLM RU vs непереведённое).
2. **Ф4-лёгкий**: CLI main.rs уже модульный по сути (clap-определения + commands/); расщепление GUI main.rs — в Ф7. Data-safety: write_atomic уже в services — аудит применений. Observability: run-report как JSON для translate/compare.
3. **Ф7**: Svelte 5 + TS бутстрап в gui/tauri-app (frontend-v2 + vite), IPC-поверхность src-tauri заморожена; редактор EN|RU; ui-ux-pro-max дизайн-проход; tauri-driver E2E.
4. **Ф8**: бенчмарки на VWE (малый) и SOS2 (большой), потом точечные оптимизации.
5. **Ф9**: acceptance по research-файлу: APFS-клон .app + savedatafolder + batch + grep лога.
6. **Ф10**: Pass A (engineering) + Pass B (hostile) до чистых проходов; FINAL_ACCEPTANCE.md; СТОП, ждать одобрения пуша.

## Решения/уроки (накопленное)
- INSTA_UPDATE=always не сработал — снапшоты править файлом напрямую + rm *.snap.new
- RimLoc-агентскрипт agent-commit сам добавляет `type(scope):` — в --subject префикс не писать
- AI-OS Layer 7 блокирует `git add -u` — только явный pathspec (или скрипты RimLoc)
- `--lang` ранее был только CSV-колонкой; Defs-юниты получают путь Languages/English/DefInjected-цели (is_source_for_lang_dir("English") их ловит)
- duplicate-global скоупится на языковую папку (EN+RU одного ключа = норма)
- stash pre-purge-wip: сохранён, суперсэжден Ф2 (dialog/shell плагины вошли в Ф2 коммит)

## LATEST · 2026-09-23 · финальная коррекция TKey (4 пункта) ЗАКРЫТА, P1-3/P1-4 частично
HEAD 3ea0e19 (после b29ef68, e4639f7, 3ea0e19) · 128/0 тестов · clippy 0 · fmt ok · НЕ пушено.

**Коррекция-1 (популяции)**: `testlab/scripts/tkey_population_reconcile.py` + отчёт `testlab/reports/tkey-population-reconcile.json`. Авторитетные определения в `DLC-TKEY-ADJUDICATION.md §1.1`: P1 raw=360, P2 translatable=358 (−2 textless-контейнера PawnLend DutyRules), P3 идентичности=350, P4 serialization-tested=351 узел/343 идентичности. 7 unmatched машинно перечислены: 4 Royalty gap (дока исправлена: было «3») + 3 Odyssey structural. Официальный RU распаковывается скриптом в gitignored `testlab/run/`.

**Коррекции-2/3/4 (e4639f7)**: `TKeyMeta {strategy, suffix, contexts}` на TransUnit (key = логическая идентичность; метадата типизированная, не string-спецкейс). scan_defs_tkey выводит стратегию из контекста: TipSetDef li→bare, parms-потомок QuestNode_SubScript→parms_value_slate_ref (`.value.slateRef`), остальное→slate_ref. Дубли (defName,TKey): первый файл владеет идентичностью, внутри файла last-wins (RimWorld field-assignment), contexts сохранён. Машино-воспроизведение: Core 112 = 90 bare+20 slate+2 parms; все корни дают P3=350; Royalty 200 юнитов, 8 дуплов contexts=2. `TKeyRegistry {identities, aliases}` в matching.rs: порядок exact → proven alias → known-suffix; structural-алиасы = данные реестра, не shape-эвристика (Odyssey-кейс покрыт тестом).

**P1-3 (3ea0e19)**: coverage_report и compare гейтят canonical-фоллбэк через реестр (форма `identity_for`), shape-stripping без гейта устранён в coverage+compare. **P1-4**: test/TKeyMod расширен (parms-узел + RU DefInjected на 3 суффикс-формы); CLI-регрессии: coverage 5/4/4/1 (4 TKey сматчены, ordinary label — missing), scan эмитит parms-стратегию.

**Далее**: P1-2 остаток (import-po writer строит путь из key+TKeyMeta.suffix; e2e round-trip фикстуры), P1-1 unify scan-вариантов, P2-8 (scan_defs_tkey внешний defs_dir), canonical project-model audit → затем GUI RC (ui-ux-pro-max → Svelte 5).

## BACKEND GATE CHECKLIST (A–L, по именам — авторитетный остаток; обновлять по мере работы)
| # | Задача | Статус | Эвиденс |
|---|--------|--------|---------|
| A | TKey output/round-trip end-to-end (writer по TKeyMeta, 4 стратегии, мульти-контексты) | **ACTIVE** — метадата/реестр/стратегии готовы (e4639f7); writer+e2e — впереди | e4639f7; §1.1 adjudication |
| B | Каноническая унификация inventory/сервисов (+таблица consumer×capability) | **DONE** — один пайплайн (validate/coverage/word-info делегируют; double extraction устранён; VWE 38%→45%); таблица в CANONICAL_INVENTORY.md | 98db4b2 |
| C | TKey-aware кросс-язычные валидаторы на каноническом матчёре | **DONE** — один typed Resolution (Matched{Exact/ProvenAlias/SuffixFallback}/Ambiguous/Unmatched); placeholders/lists/orphans на общем резолве, ambiguity = диагностика; sourceChanged — на канон-модели Gate I | c634ee0 |
| D | D | **DONE** — кейсы A/B/C/D в CLI-регрессии (source=6/translated=4/missing=2: TODO=missing, absent=missing) | 3ea0e19 + 65af0f4 |
| E | E | **DONE** — scan_defs_tkey ходит по явному defs_dir напрямую; внешняя директория покрыта тестом | 65af0f4 |
| F | Hardening аудит-скриптов (tmp-каталоги, воспроизводимость) | **DONE** — tkey_audit.py в свежем TemporaryDirectory, RU-тар извлекает с check=True каждый прогон; reconcile самодостаточен | этот коммит |
| G | G | **DONE** — гейт-таблица и комментарии кода: система с 1.1/2020, 1.6 = primary tested | 65af0f4 |
| H | Effective RimWorld load precedence (Def first-wins / Keyed last-wins / версии / DLC) | **OPEN** | — |
| I | Каноническая project model / persistence / адаптеры (Mandate 2) | **OPEN** | — |
| J | Eligibility/knowledge архитектура (Mandate 3) | **OPEN** | — |
| K | Existing-translation maintenance (Mandate 4 §1–7) | **OPEN** | — |
| L | Contributor/debugging/observability (Mandate 4 §8–39) | **OPEN** | — |

Порядок: A→B→C→D→E→F→G→H → I → J → K → L → freeze → delta-бандл. GUI-дизайн/моки — параллельно, без привязки к legacy-состоянию.

## УТОЧНЕНИЯ ВЛАДЕЛЬЦА (24.09) — границы GATE A и правило no-PO
- **GATE A (8595129) доказывает**: семантику TKey-сериализации, все доказанные
  стратегии, round-trip текущего PO-адаптера, корректность писателя. **НЕ** доказывает
  приемлемость PO как канонической внутренней архитектуры.
- **Gate I обязан включить no-PO acceptance**: source → canonical project state →
  translation/editor/TM/MockProvider → validation → RimWorld writer → корректный TKey
  output. Отдельно сохраняется interop-тест: canonical project → PO export/import →
  canonical project (fidelity-матрица).
- Текущий PO-based TKey E2E остаётся регрессией и станет эвиденсом совместимости
  PO-адаптера после миграции.
- **Темы GUI**: Light / Dark / System(Auto) — все три; «dark primary» = первичный
  визуальный референс/дефолт, НЕ dark-only продукт.
- **5 открытых вопросов GUI_DESIGN_SPEC** классифицированы (см. файл): обратимые
  дефолты решаются автономно; к пользователю — только существенные продукт-решения.
- **Issue #2 остаётся OPEN** до полного канонического цикла: existing translation →
  import → preserve reusable → sourceChanged/new/obsolete/orphan классификация →
  edit → close/reopen → validate → build. TM-prefill round-trip — поддерживающий
  эвиденс, не закрытие.

## АРХИТЕКТУРНЫЕ УТОЧНЕНИЯ (24.09, после Gate B; позиция ЧПТ + наша)
1. **`scan_units_with_defs_and_dict` — реализация, не доменный контракт.** Каноническая
   РЕАЛИЗАЦИЯ сегодня; типизированный canonical inventory service/context — часть миграции
   Gate I (не переименовать ради эстетики сейчас). Наша добавка: при Gate I типизированный
   сервис ОБОРАЧИВАЕТ эту реализацию (один шов), а лестница `scan_units / _with_defs /
   _with_defs_and_fields / _with_defs_and_dict / _auto` сворачивается в options-структуру.
2. **Provenance бенчмарков обязателен.** Каждый coverage/benchmark-отчёт фиксирует:
   версию семантики инвентаря, матчёра, целевую версию RimWorld, identity корпуса
   (hash/manifest), provenance референсов, версию eligibility/ruleset. Исторические числа
   НЕ перезаписывать как одну методику: 38% = parsers+double-extraction+без словарей
   (пре-B), 45% = canonical v1 (пост-B), 65% = догфуд T6 до TKey-гейта (другая методика,
   superseded). Наша добавка: meta-блок в coverage/compare JSON — аддитивный
   (schema_version не ломаем, пустые поля skip).
3. **Словари → eligibility-слой с provenance (Gate J).** Инвентарь вправе включать
   верифицированные learned-правила, но скрытое изменение инвентаря словарём без
   объяснимого evidence запрещено структурно в J. Не блокирует C. Наша добавка:
   у AutoDefsContext уже есть learned_sources (пути файлов) — в J они становятся
   evidence в explain-выдаче.
4. **Шрифты в GUI.** Бандл допустим (офлайн + детерминированный E2E), но перед public
   release — проверка лицензий перераспределения, нотисов и размера пакета. IBM Plex Sans
   и JetBrains Mono — SIL OFL 1.1: перераспределение внутри приложения допустимо, в репо
   кладутся файлы лицензий + THIRD-PARTY-NOTICES. Пункт в чек-листе repo/docs hardening.
