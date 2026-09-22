# AUTONOMOUS STATUS — RimLoc RC campaign

Session: `zcode-rc-campaign` · Updated: 2026-09-23 (после Ф3) · **Не пушить — вся работа локально.**
Recovery point: этот файл + `~/Developing/_rimloc-safety/` (bundle, stash diff, логи).

## Текущее состояние
**ИТЕРАЦИЯ 1 ЗАВЕРШЕНА** (Ф0–Ф6 + Ф9-acceptance + compare). CLI-пайплайн = RC-кандидат. Итоги: docs/development/FINAL_ACCEPTANCE.md.
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
