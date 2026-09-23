# AUTONOMOUS STATUS — RimLoc RC campaign

Session: `zcode-rc-campaign` · Updated: 2026-09-23 (после Ф3) · **Не пушить — вся работа локально.**
Recovery point: этот файл + `~/Developing/_rimloc-safety/` (bundle, stash diff, логи).

## Текущее состояние — ИТЕРАЦИЯ 2 ЗАВЕРШЕНА → И3 (PRODUCT & PUBLIC RELEASE, мандат A–AP)

**Чекпоинт И2 (проверен): 114/0 тестов, clippy/deny чисто, HEAD 2e0f799+, дерево чисто, пуша нет.**
Кумулятив И2: слепой бенчмарк (adjudication: 0 реальных миссов) · provenance (Gemini-AI пак; OW.RU.* = RimLangKit-конвейер) · многоязычный промпт · матрица конкурентов v2 · first-party аудит (13 категорий TranslationReport из декомпила DLL 1.6) · modview (LoadFolders) · version-diff · TODO-семантика · capability-aware WordInfo (`rimloc word-info`) · whole-game (Core EN 11549 / RU 16681 / 93%). Блокер: локальная LLM запрещена (ресурсы), платные API не авторизованы; GLM Flash через ZCode = разрешённый zero-cost путь для семантического бенчмарка (§20 мандата).


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
