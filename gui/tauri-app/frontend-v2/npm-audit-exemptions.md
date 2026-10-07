# npm audit exemptions — frontend-v2

Статус: **documented dev-only risk — ожидает явного подтверждения владельца** (frontend-v2 — замороженный legacy fallback ТОЛЬКО для WDIO-спек `e2e/wdio-spike/`; production = `frontend-react`). Новые фичи и мажорные апгрейды сюда не заносить.

> Формулировка для release-документации: «0 known production/runtime npm vulnerabilities; 16 documented high-severity dev-tool advisories remain under explicit exemption (pending owner confirmation)». Владелец либо принимает эти 16 как non-shipping test-tool risk, либо заказывает миграцию tooling (WDIO 10) — до решения нельзя писать «принятые владельцем».

- Дата аудита: 2026-10-07
- Инструменты: node 26.10.0, npm 11.19.1, `npm audit --json`
- До работ: 0 critical / **23 high** · После работ: 0 critical / **16 high** (все — задокументированы ниже)
- Ветка: `chore/npm-audit`, worktree `wt-npmaudit`

## Почему это не влияет на продакшен

Все остатки живут в `devDependencies` — цепочка WDIO/e2e-тестов (`@wdio/*`, `webdriverio`, `mocha`, `@puppeteer/browsers`). В runtime-бандл (`npm run build` → `vite build`) они не попадают: `vite` бандлит только `src/` приложения. Пакет `rimloc-gui-frontend-v2` — `private: true`, в npm не публикуется. Собранные артефакты GUI шипятся из `frontend-react`.

## Что было исправлено (не exemption)

| Пакет | Было | Стало | Advisory | Механизм |
|---|---|---|---|---|
| `source-map-js` | 1.2.1 | 1.2.2 | [GHSA-68fv-2mgg-jv7q](https://github.com/advisories/GHSA-68fv-2mgg-jv7q) (high) | `npm audit fix` (небрейкинг, через css-tree/jsdom и postcss/vite) |
| `deepmerge-ts` | 7.1.6 | 8.0.2 | [GHSA-ggr8-5vv4-36mx](https://github.com/advisories/GHSA-ggr8-5vv4-36mx) (high) | override `^8.0.2` |
| `serialize-javascript` | 6.0.2 | 7.0.5+ | [GHSA-5c6j-r48x-rmvq](https://github.com/advisories/GHSA-5c6j-r48x-rmvq) (high), GHSA-qj8w-gfj5-8c6v (moderate) | override `^7.0.5` |
| `basic-ftp` | 5.3.1 | 6.2.2 | [GHSA-c475-qrg2-pj4r](https://github.com/advisories/GHSA-c475-qrg2-pj4r) (high) | override `^6.2.2` |

Обоснования override'ов (в lockfile точечно, без мажорного апгрейда WDIO):

- **deepmerge-ts 7→8**: `@wdio/config@9.32.0` сам объявляет `deepmerge-ts: ^8.0.0` — комбинация «WDIO 9 + deepmerge-ts 8» валидирована upstream внутри того же мажора WDIO. Загрязнена была только вложенная копия под `@wdio/tauri-service → webdriverio@9.30.1`.
- **serialize-javascript 6→7**: используется mocha только в parallel-воркерах (`lib/nodejs/worker.js`, `lib/nodejs/buffered-worker-pool.js` — режим `mocha --parallel`), который `@wdio/mocha-framework` не использует. CJS-`require('serialize-javascript')` в 7.x проверен (main: `index.js`).
- **basic-ftp 5→6**: get-uri использует только `new Client()` + `access/lastMod/list/downloadTo/close` — все присутствуют в 6.2.2 (сверено с `dist/Client.d.ts`), CJS-экспорт сохранён.

## Принятые exemption'ы: 2 корневых advisory без upstream-фикса

Из 16 остатков high **ни один не имеет патченной версии**: `fixAvailable: false` либо только переход на webdriverio/WDIO **10.0.0** (semver-major, ломает замороженный стек — запрещён решением владельца). Оба корня проверены реестром npm: версий новее не существует, т.е. НИКАКОЙ пакет не может зависеть от пропатченной версии.

### E1. `braces` ≤ 3.0.3 — нет фикса upstream

- Advisory: [GHSA-vfj7-8cjw-p6xm](https://github.com/advisories/GHSA-vfj7-8cjw-p6xm) — stack-exhaustion DoS через глубоко вложенные glob-паттерны (high, dev/test-time)
- Установлено: `braces@3.0.3` — **последняя опубликованная версия** (проверено `npm view braces versions`; фикса нет в 3.x, мажор 4 не выпущен)
- Путь: `@wdio/mocha-framework@9.32.0 → mocha@10.8.2 → chokidar@3.6.0 → braces@3.0.3`
- Флаги-потомки: `chokidar@3.6.0`, `mocha@10.8.2`, `@wdio/mocha-framework`
- Почему не чиним: chokidar 4 (без braces) требует mocha 11+, mocha 11 — мажор внутри `@wdio/mocha-framework@9` (объявляет `mocha ^10`), а WDIO 10 — запрещённый мажор. Watch-режим mocha завязан на chokidar 3 API.
- Эксплуатируемость: glob-паттерны поступают из наших конфигов, не из недоверенного ввода; код исполняется только на машине разработчика при e2e-прогоне.

### E2. `extract-zip` ≤ 2.0.1 — нет фикса upstream

- Advisories: [GHSA-jmr9-qjv8-65gv](https://github.com/advisories/GHSA-jmr9-qjv8-65gv) (unvalidated symlink path traversal), [GHSA-7pqw-9j4j-h8q3](https://github.com/advisories/GHSA-7pqw-9j4j-h8q3) (arbitrary file writes через symlink-записи) — high
- Установлено: `extract-zip@2.0.1` — **последняя опубликованная версия** (проверено `npm view extract-zip versions`; 3.x не существует)
- Путь: `@wdio/utils@9.32.0 → @puppeteer/browsers@2.13.2 → extract-zip@2.0.1` (та же цепочка в вложенном `webdriverio@9.30.1 → @wdio/utils@9.30.1` под `@wdio/tauri-service`)
- Флаги-потомки: `@puppeteer/browsers`, `@wdio/utils`, `@wdio/config`, `webdriver`, `webdriverio`, `@wdio/globals`, `expect-webdriverio`, `@wdio/cli`, `@wdio/runner`, `@wdio/local-runner`, `@wdio/tauri-service`
- Почему не чиним: `@puppeteer/browsers@2.x` объявляет `extract-zip ^2.0.1`; переход на фиксованную версию невозможен (её нет), апгрейд `@puppeteer/browsers` 3.x — мажор внутри `@wdio/utils@9`, WDIO 10 — запрещённый мажор.
- Эксплуатируемость: extract-zip вызывается только при скачивании browser-бинарников (`@puppeteer/browsers install`) с официальных CDN-эндпоинтов; zip берётся не из недоверенного источника.

### Итоговый список 16 флагов (все покрыты E1/E2)

`braces`, `chokidar`, `mocha`, `@wdio/mocha-framework` (→ E1) · `extract-zip`, `@puppeteer/browsers`, `@wdio/utils`, `@wdio/config`, `webdriver`, `webdriverio`, `@wdio/globals`, `expect-webdriverio`, `@wdio/cli`, `@wdio/runner`, `@wdio/local-runner`, `@wdio/tauri-service` (→ E2)

## План пересмотра

Exemption'ы пересматриваются при: (а) выходе `extract-zip`/`braces` с патчем; (б) релизе `@wdio/tauri-service`/`@wdio/tauri-plugin` под WDIO 10 — тогда стек v2 поднимается одной согласованной волной, а не точечными override'ами; (в) выводе v2 из эксплуатации.
