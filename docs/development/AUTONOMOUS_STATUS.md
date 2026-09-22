# AUTONOMOUS STATUS — RimLoc RC campaign

Session: `zcode-rc-campaign` · Updated: 2026-09-23 (Ф0 in progress)
Recovery point: this file + `~/Developing/_rimloc-safety/` (bundle, stash diff, logs).

## Current milestone
**F0 — immutable baseline** (в работе: baseline build/test идёт в фоне).

## Git state (после интеграции)
- `main` = `2c5a47e` — 43 локальных коммита rebase на origin/main (e774d67 + веб-коммиты `35746bb` dependabot.yml, `e14d709` SECURITY.md). Rebase чистый, конфликтов нет. **AHEAD 43, не пушилось — и не должно.**
- Safety: bundle `~/Developing/_rimloc-safety/rimloc-baseline-2026-09-23.bundle`, tag `f0-baseline-20260923`.
- Stash `pre-purge-wip` (2025-09-30): добавлял `tauri-plugin-dialog` + `tauri-plugin-shell` в GUI (Cargo.toml/main.rs +2/+2, сгенерённые схемы). **НЕ применён, НЕ удалён** — дифф сохранён `stash-pre-purge-wip.diff`; суперсидится в Ф2 свежими версиями тех же плагинов.
- Ветка `codex/revise-definjected-functionality-in-rimloc` (06cf502) — старая, не тронута.
- Новые origin-ветки: 2 dependabot pip PR (mkdocs-static-i18n 1.3.1, pymdown-extensions 12) + release-plz branch от 20.09.
- `.github/dependabot.yml` на месте (cargo root + cargo gui + pip + github-actions; daily/weekly; labels; limit 10/10/5/10).

## Completed (F0)
- Прочитан RimLoc/AGENTS.md; хуки включены (setup-git-hooks.sh); сессия zcode-rc-campaign начата.
- Git-инспекция полная; stash проанализирован детально (без разрушения).
- Bundle + safety-тег + сохранённые артефакты.
- Rebase 43/2 — clean; дивергенция устранена локально.

## In progress
- Baseline `cargo build/test/fmt/clippy` в фоне → лог `~/Developing/_rimloc-safety/baseline-build-test.log`.

## Known issues (from reconnaissance, pre-baseline)
- P0: Tauri `csp:null`; `open_path` (cmd-injection), `save_text_file` (arbitrary write), `load_plugin_cmd` (native code from WebView path); zip-slip в services/extras/lang_update.rs; CI не гоняет тесты.
- P1: устаревшие зависимости (reqwest 0.11/hyper 0.14/rustls 0.21, zip 0.6, image 0.24, schemars 0.8, двойной libloading в lock, двойной Cargo.lock); GUI-монолиты (index.js 2513 строк, src-tauri main.rs 3928); CLI main.rs 1805.
- P2: заглушка scan_keyed_xml (validate), legacy RimLocError, 67 unwrap+11 expect в либах, 7 крейтов без тестов, доки отстают от CLI (coverage/export-xliff/merge-keyed/learn-*).
- 29 открытых Dependabot PR на GitHub (мажорные бампы) + 2 pip PR.
- GUI i18n vendor-тесты были известны как падающие в старых логах коммитов — проверить в baseline.

## Next (по порядку)
1. Дождаться baseline, записать результаты сюда, закоммитить статусные файлы.
2. F1: testlab/ + манифесты (1814383360, 2927850179, 2126925929, ваниль-тар) + hash-скрипт + начальный `rimloc compare` дизайн + CLI-догфуд тикеты.
3. F2 security по списку выше (суперсидить stash свежими плагинами).

## Blockers
Нет. (Baseline-результаты — ожидание, не блокер.)

## Decisions log
- GUI-стек: Svelte 5 + TS (выбор Даниэля делегирован; критерий: красота/практичность/вес/долгосрок).
- LLM: Anthropic + OpenAI-compatible (Z.AI пресет). Платных вызовов НЕТ — только MockProvider.
- Stash: сохранить, суперсидить в Ф2. 43 коммита: rebase (выполнен, clean).
- Push: только по явному одобрению Даниэля в конце.
