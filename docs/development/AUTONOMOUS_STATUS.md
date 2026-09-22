# AUTONOMOUS STATUS — RimLoc RC campaign

Session: `zcode-rc-campaign` · Updated: 2026-09-23 (после Ф3) · **Не пушить — вся работа локально.**
Recovery point: этот файл + `~/Developing/_rimloc-safety/` (bundle, stash diff, логи).

## Текущее состояние
**Ф3 завершена. Следующая — Ф4 (архитектурный аудит + data-safety + observability), затем Ф5-имплементация гэпов из готовой матрицы.**

## Git (локальный main, AHEAD origin — не пушить)
Коммиты кампании (сверху вниз):
- `ci: add CI quality gates and cargo-deny policy` — ci.yml (fmt/clippy/test×3OS/gui/deny), deny.toml, лицензии крейтам
- `chore(deps): bump roxmltree 0.21, libloading 0.9, docs stack and actions` — + requirements-docs + workflows actions v7/v6/v5/v3
- `chore(deps): bump schemars 0.8 -> 1.2 and regenerate JSON schemas`
- `chore(deps): bump quick-xml 0.36 -> 0.42` (API-миграция: str-QName, xml10_content, Event::GeneralRef)
- `chore(deps): bump zip 0.6 -> 8.6` (SimpleFileOptions в тестах)
- `chore(deps): refresh lockfile; reqwest 0.13, dirs 7, image 0.25`
- `fix(gui): harden tauri trust boundaries and fix zip-slip` (8efa526) — CSP, save_text_via_dialog, plugin-allowlist, open-крейт, zip-slip guard + негативные тесты; SECURITY_AUDIT.md
- `fix(validate): fix language scoping in scan, coverage and validate` — T5/T6/T7 + MultiLangMod фикстура + 3 регрессии
- `chore(tests): testlab: manifests, hash guard, dogfood tickets`
- `chore(repo): docs(repo): autonomous plan/status` (2b50aec)
База: rebase 43 локальных коммитов на origin/main (+2 веб-коммита), clean. Bundle: `~/Developing/_rimloc-safety/rimloc-baseline-2026-09-23.bundle`, тег `f0-baseline-20260923`.

## Состояние качества
- `cargo test --workspace`: **82 passed / 0 failed** · `cargo fmt --check` clean · `cargo clippy --workspace --all-targets -- -D warnings` чисто · `cargo deny check` — все 4 категории ok (игноры задокументированы: lru заперт tauri ^0.12; unic-семейство unmaintained через tauri/wry)
- Read-only гарантия: `testlab/scripts/verify-source-hashes.sh` — 1066 файлов, OK после всех прогонов
- CLI догфуд: VWE scan --lang en = 325 EN-строк; coverage 370/142 (38%); validate скоупится на RU
- Docs build (mkdocs 9.7.7 + i18n 1.3.1) — OK
- GitHub: 30 Dependabot-PR закрыты с комментарием «superseded»; release-plz-ветка осталась

## Артефакты кампании
- `docs/development/AUTONOMOUS_PLAN.md` — дорожная карта Ф0–Ф10 + ограничения
- `docs/development/SECURITY_AUDIT.md` — S1–S11: все Critical/High FIXED, негативные тесты
- `docs/development/COMPETITOR_MATRIX.md` — код-левел матрица 6 конкурентов, топ-10 гэпов
- `testlab/dogfood-tickets.md` — T1–T8 со статусами (5 fixed, 2 not-a-bug, T4/T8 отложены осознанно)
- `testlab/research/rimworld-isolated-run.md` — процедура acceptance: `-savedatafolder=`, batch-прогон (прецедент rwmt/Multiplayer HostUtil), APFS-клон .app для тест-модов, grep-маркеры ошибок локализации, ручной smoke-чеклист
- `testlab/manifests/{installed-mods,real-mods}.json` — 287 модов, 186 VE, 161 VE без RU

## ДАЛЕЕ (по порядку)
1. **Ф4** — архитектурный аудит (CLI main.rs 1805 строк, GUI main.rs ~3900, фронт index.js 2513 — расщепление), atomic write-утилита уже есть (util::write_atomic — проверить применение), run-reports, ресьюм lang-операций. Рефактор только с регрессионным покрытием (82 теста + джорни).
2. **Ф5** — имплементация гэпов по COMPETITOR_MATRIX (приоритеты там: LLM-подсистема (Ф6 перекрывает), Strings-скан (частично есть: strings inventory panel), WordInfo-канонический формат (проверить Keyed/_Case.xml vs WordInfo/Case.txt — M), encoding normalization (BOM/CRLF) S, wl/bl-фильтры S, About.xml-паспорт M, source-text TM M, mod discovery M). rimloc compare — сюда же (JSON+MD: EN vs human vs LLM).
3. **Ф6** — rimloc-llm: TranslationEngine/Provider/BatchPlanner/Glossary/Validator/RetryPolicy/CheckpointStore/CostEstimator; Anthropic + OpenAI-compat (пресеты OpenAI/Z.AI/Ollama) + MockProvider; keychain+env; БЕЗ платных вызовов.
4. **Ф7** — Svelte 5 + TS редактор (ui-ux-pro-max дизайн), tauri-driver E2E, полный GUI-джорни.
5. **Ф8** — бенчмарки на VWE (малый) и Multiplayer/SOS2 (большой), потом точечные оптимизации.
6. **Ф9** — acceptance по research-файлу: APFS-клон .app + savedatafolder + batch + grep лога.
7. **Ф10** — Pass A (engineering) + Pass B (hostile) до чистых проходов; FINAL_ACCEPTANCE.md; СТОП, ждать одобрения пуша.

## Решения/уроки (накопленное)
- INSTA_UPDATE=always не сработал — снапшоты править файлом напрямую + rm *.snap.new
- RimLoc-агентскрипт agent-commit сам добавляет `type(scope):` — в --subject префикс не писать
- AI-OS Layer 7 блокирует `git add -u` — только явный pathspec (или скрипты RimLoc)
- `--lang` ранее был только CSV-колонкой; Defs-юниты получают путь Languages/English/DefInjected-цели (is_source_for_lang_dir("English") их ловит)
- duplicate-global скоупится на языковую папку (EN+RU одного ключа = норма)
- stash pre-purge-wip: сохранён, суперсэжден Ф2 (dialog/shell плагины вошли в Ф2 коммит)
