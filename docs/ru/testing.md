---
title: Тестирование и отчёты
---

# Тестирование и отчёты

В RimLoc есть несколько уровней доказательств. Зелёные unit tests не равны доказанному desktop workflow или результату в игре.

## Уровни доказательств

Используйте самый сильный реально полученный уровень:

1. source/code inspection;
2. unit tests;
3. integration tests;
4. built desktop/CLI E2E;
5. same-corpus differential;
6. in-game/runtime proof.

Наличие красивого UI-экрана само по себе ничего не доказывает.

## Rust workspace

~~~bash
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
~~~

GUI crate имеет platform/frontend prerequisites; CI проверяет его отдельным job.

## React frontend

~~~bash
cd gui/tauri-app/frontend-react
npm ci
npx tsc --noEmit
npm run build
~~~

Semantic/WDIO acceptance React R1 идёт отдельно от обычного build/typecheck.

## Svelte fallback

Только если меняется замороженный fallback:

~~~bash
cd gui/tauri-app/frontend-v2
npm ci
npm run check
npm test
npm run build
~~~

## CLI smoke

Используйте встроенную fixture или изолированную копию реального мода:

~~~bash
rimloc-cli scan --root ./test/TestMod --format json
rimloc-cli validate --root ./test/TestMod
~~~

PO-тесты проверяют interoperability, а не весь продукт:

~~~bash
rimloc-cli export-po --root ./test/TestMod --out-po ./logs/TestMod.po --lang ru
rimloc-cli validate-po --po ./logs/TestMod.po --strict
~~~

Есть и no-PO build:

~~~bash
rimloc-cli build-mod \
  --from-root ./work/MyTranslatedMod \
  --out-mod ./dist/MyTranslatedMod-RU \
  --lang ru \
  --dry-run
~~~

## Desktop acceptance

При owner/user тесте проверяйте exact artifact identity:

- source SHA;
- frontend flavor;
- automation flag;
- app/binary SHA, если дан.

Не тестируйте stale/automation build как production candidate.

Автоматизация должна использовать отдельный data/profile, чтобы synthetic projects не попадали в Recent Projects владельца.

## Security checks

Для security-sensitive изменений нужны релевантные проверки:

- cargo-deny/advisories;
- JS dependencies;
- CodeQL;
- secret scan;
- Tauri capabilities/IPC;
- path traversal/symlink containment;
- отсутствие automation bridge в production artifact.

Внутренние audit-артефакты — в <code>docs/security/</code>, публичная политика — [SECURITY.md](https://github.com/0-danielviktorovich-0/RimLoc/blob/main/SECURITY.md).

## Документация

~~~bash
python -m venv .venv
source .venv/bin/activate
pip install -r requirements-docs.txt
mkdocs build --strict
~~~

## Багрепорт

Укажите:

- exact build/version/commit;
- ОС;
- область (React desktop, CLI, adapter, build/export и т.д.);
- шаги;
- expected/actual;
- sanitized logs;
- маленькую fixture/project, если возможно.

Никогда не публикуйте API keys, tokens, private paths или непроверенный diagnostics bundle.
