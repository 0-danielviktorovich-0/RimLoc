# AUTONOMOUS STATUS — RimLoc RC campaign

Session: `zcode-rc-campaign` · Updated: 2026-09-23 (Ф0 in progress)
Recovery point: this file + `~/Developing/_rimloc-safety/` (bundle, stash diff, logs).

## Current milestone
**Ф1 — testlab + догфуд** (F0 завершён, baseline зелёный). Дальше: фикс P0-тикетов T5/T6 (сломанное ядро scan/coverage), затем Ф2-безопасность.

## Dogfood results (Ф1, до правок)
Полные тикеты: `testlab/dogfood-tickets.md`. Ядро:
- **T5 P0**: `scan --lang` не фильтрует — микс всех `Languages/*` (1625 записей / 760 уникальных, коллизии).
- **T6 P0**: `coverage` = 0% даже на синтетическом фикстуре — команда нерабочая.
- T7 P1: validate по всем языкам, false-positive дубли (li-нормализация), шум placeholder-check на EN.
- T1 P1: дублирующийся Fluent-ключ diffxml-summary — ERROR ×2 на каждый запуск.
- T2 P1: `scan --format json` не пишет в stdout без --out-json (непайпуем).
- T3 P2: `--help` сабкоманд в stderr · T4 P2: дефолт scan = переводы вместо источника · T8 P3: `:0` вместо строк.
- Манифесты: `testlab/manifests/{installed-mods,real-mods}.json` (287 модов, 186 VE, 161 VE без RU); hash-база 1066 файлов `source-mods-hashes.sha256` — read-only гарантия проверена (OK).
- 6 конкурентов клонированы в `~/Developing/_competitors-rimloc/`.

## Git state (после интеграции)
- `main` = `2c5a47e` — 43 локальных коммита rebase на origin/main (e774d67 + веб-коммиты `35746bb` dependabot.yml, `e14d709` SECURITY.md). Rebase чистый, конфликтов нет. **AHEAD 43, не пушилось — и не должно.**
- Safety: bundle `~/Developing/_rimloc-safety/rimloc-baseline-2026-09-23.bundle`, tag `f0-baseline-20260923`.
- Stash `pre-purge-wip` (2025-09-30): добавлял `tauri-plugin-dialog` + `tauri-plugin-shell` в GUI (Cargo.toml/main.rs +2/+2, сгенерённые схемы). **НЕ применён, НЕ удалён** — дифф сохранён `stash-pre-purge-wip.diff`; суперсидится в Ф2 свежими версиями тех же плагинов.
- Ветка `codex/revise-definjected-functionality-in-rimloc` (06cf502) — старая, не тронута.
- Новые origin-ветки: 2 dependabot pip PR (mkdocs-static-i18n 1.3.1, pymdown-extensions 12) + release-plz branch от 20.09.
- `.github/dependabot.yml` на месте (cargo root + cargo gui + pip + github-actions; daily/weekly; labels; limit 10/10/5/10).

## Completed (F0)
- **Baseline: build OK · tests all pass (exit 0) · fmt clean · clippy 4 warning (parsers-xml ×2, services ×1, gui ×1 — будут закрыты при вводе -D гейта).**
- Прочитан RimLoc/AGENTS.md; хуки включены; сессия zcode-rc-campaign.
- Git-инспекция; stash проанализирован (сохранён); bundle+тег `f0-baseline-20260923`; rebase 43/2 clean → main=2c5a47e; коммит статусов 2b50aec.
- testlab создан: манифесты, hash-скрипты, тикеты догфуда; структура manifests/synthetic/adversarial/expected/minimized-regressions/scripts.

## In progress
- Baseline `cargo build/test/fmt/clippy` в фоне → лог `~/Developing/_rimloc-safety/baseline-build-test.log`.

## Known issues (from reconnaissance, pre-baseline)
- P0: Tauri `csp:null`; `open_path` (cmd-injection), `save_text_file` (arbitrary write), `load_plugin_cmd` (native code from WebView path); zip-slip в services/extras/lang_update.rs; CI не гоняет тесты.
- P1: устаревшие зависимости (reqwest 0.11/hyper 0.14/rustls 0.21, zip 0.6, image 0.24, schemars 0.8, двойной libloading в lock, двойной Cargo.lock); GUI-монолиты (index.js 2513 строк, src-tauri main.rs 3928); CLI main.rs 1805.
- P2: заглушка scan_keyed_xml (validate), legacy RimLocError, 67 unwrap+11 expect в либах, 7 крейтов без тестов, доки отстают от CLI (coverage/export-xliff/merge-keyed/learn-*).
- 29 открытых Dependabot PR на GitHub (мажорные бампы) + 2 pip PR.
- GUI i18n vendor-тесты были известны как падающие в старых логах коммитов — проверить в baseline.

## Next (по порядку)
1. Закоммитить testlab + тикеты.
2. **P0-фиксы ядра**: T5 (lang-фильтр scan), T6 (coverage) — с регрессионными тестами на реальном кейсе мультиязычного мода + минимизированные фикстуры в testlab.
3. T7 (validate по языкам/li-дубли), T1 (Fluent-дубль), T2 (json stdout).
4. Ф2 security (CSP, open_path, save_text_file, плагины, zip-slip) — суперсидить stash свежими tauri-plugin-dialog/shell.
5. Ф3 CI+зависимости.

## Blockers
Нет. (Baseline-результаты — ожидание, не блокер.)

## Decisions log
- GUI-стек: Svelte 5 + TS (выбор Даниэля делегирован; критерий: красота/практичность/вес/долгосрок).
- LLM: Anthropic + OpenAI-compatible (Z.AI пресет). Платных вызовов НЕТ — только MockProvider.
- Stash: сохранить, суперсидить в Ф2. 43 коммита: rebase (выполнен, clean).
- Push: только по явному одобрению Даниэля в конце.
