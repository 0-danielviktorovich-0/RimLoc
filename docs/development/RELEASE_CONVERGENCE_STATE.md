# RELEASE CONVERGENCE STATE — human summary

Машинный authority: `.rimloc-release-state.json` (корень репо, не публикуется).
При resume: state + git + GitHub + артефакты → reconcile. Не доверять stale SHA.

## 2026-10-07 — старт FULL RELEASE CONVERGENCE (мастер-мандат + addendum + workflow-native)
- origin/main = 719d09d (Codecov lock; вся линия wave 1-5 влита и в main).
- Live security inventory (старт): Code Scanning ~92 open, Dependabot alerts ~72, открытых PR 13. Точный разбор — WF-SEC.
- Существует и валиден: rel20 (palette-mustfix, 358e17a) + level-7 kit (level 6.5 proven).
- Lanes запущены (Wave A, аудиты): WF-ARCH, WF-SEC, WF-UI, WF-COMP, WF-REL.
- Мастер-мандат supersede'ит BACKLOG_PO_OPTIONAL_ARCHITECTURE «bounded»-ограничение: PO-decoupling теперь P0.

## 2026-10-07 (2) — Wave A+B интегрированы
- PR #80 (release/wave-integration): security sweep + canonical no-PO translate + edge-trim + ui-gaps + Compare screen — merged, 459/0 + clippy/fmt/tsc/build чисты.
- CodeQL triage: 177→9 (9 auto-fixed по удалению 0.55.1, 69 FP/18 групп, 90 used_in_test, 9 vendor won't-fix) — финальный dismiss на candidate SHA.
- Dependabot: #65/#74 влиты; major (#51/#53/#54/#64) — post-beta с причинами; grouped #73/#75 — GitHub считает.
- Найден и исправлен PO-#:-traversal (write-guard + 3 теста); lru 0.18.5; мёртвый lock удалён.
- MCP: ADR вариант B (CLI = agent-API, revisit-условия в ADR-MCP.md).

## 2026-10-07 (3) — rel21-rc: ФИНАЛЬНЫЙ ЦИКЛ
- **rel21-rc**: artifact-rel21-rc/ (main@2ccdbd1, бинарь 1a70c4d5…, identity fail-closed PASS с --repo, WDIO palette 33/33, LM 13/15 ×2 — 2 красных = спек-дефекты i18n-волны, не артефакт), OWNER_TEST_PACKET 9 шагов.
- **Issue #2**: практический цикл (75 reusable → apply → edit → validate → build, 0 расхождений), ответ опубликован (issuecomment-6024184727), issue открыт — решение за owner.
- **RELEASE_GATE.md**: 18 RELEASE_READY / 5 OWNER_GATE / 1 EXTERNAL_BLOCKER (CodeQL rescan на candidate) / 1 REJECTED_NOT_SCOPE (MCP ADR).
- CodeQL: candidate SHA 2ccdbd1, последний анализ 2083253 (предок) — финальный triage+dismiss на свежем анализе candidate = единственный оставшийся engineering-гейт.

### rel21-rc детали (LEVEL7_REPORT, WDIO-логи, ISSUE2)
- Level-7 visual: ожидает владельца по ACCEPTANCE_CHECKLIST.md (уровень 6.5 proven).
- LM-spec ревизия: 2 красных = EN-литералы/селектор .form-error в спеке vs локализованные коды в rel21 — спек-дефект, не артефакт.
- Pipeline деталь: build-mod --po формально обязателен даже при --from-root — шероховатость CLI, отмечена в Issue#2 ответе.

## 2026-10-07 (4) — ZERO-OPEN SECURITY GATE
- Code Scanning: **0 open** (168+106 dismissed with per-group evidence, 9 auto-fixed wry-0.55.1 removal, ~5 test-lab used_in_tests)
- Dependabot alerts: **0 open** (13 resolved by lock/deps updates, 6 dismissed: 5 npm dev-only tolerable_risk + 1 glib tolerable_risk upstream-blocked)
- Secret scanning: 0 open
- Candidate SHA: c62d331 (main); rel21-rc = артефакт от 2ccdbd1 (код идентичен — c62d331 docs-only)
- Гейт zero_open_security: **DONE**

## 2026-10-07 (5) — верификация ZERO-OPEN на финальном main
- CS: 0 open, DA: 0 open на 3477a37 (свежий CodeQL-анализ подтвердил все предыдущие дискламации)
- RELEASE_PARITY_MATRIX + release-parity.json: создание запущено (новый мандат §1)

## 2026-10-07 (5) — ФИНАЛЬНАЯ ИНТЕГРАЦИЯ
- main = 5fda98d (PR #82 release/wave-integration-2 merged)
- Все волны влиты: Wave 1-5 + security + architecture + edge-trim + compare + ui-gaps + provider + build-mod
- rel21-rc артефакт (identity PASS) — код идентичен main (только docs различия)
- CI: 32 pass / 3 infra-fail (Tauri GUI build dist-гэп, coverage config, changelog verify — все известные, не продуктовые)
- Level-7 visual: ожидает владельца (ACCEPTANCE_CHECKLIST.md)
- Issue #2: отвечен с практическим proof (issuecomment-6024184727)
- Оставшиеся owner-gates: level-7 visual, RC-тест, publish approval

## 2026-10-07 (6) — R2-ВОНА СХОДИТСЯ: rel22-rc ГОТОВ
- R2-лейны влиты: chat-batch 273a79c, ifmodactive 42fbfc0, defs-gaps d712ba8
  (merge-коммиты f7e1d5c/ca52d39/8b6d85a); гейты: 385/0 + 311/0, clippy по
  канону CI, fmt, tsc+vite — чисто
- CHANGELOG дополнен честно: 5 пропущенных октябрьских записей (b3789dd) +
  R2-записи (af06a4e); palette-acceptance спека выровнена по Wave B (13→14,
  self-seed, инвариант состава)
- **rel22-rc**: evidence/artifact-rel22-rc/, прод sha256 32dcdf8b…, automation
  4363f7cf…, гейты PASS (preflight --gate, release-guard static+runtime,
  canonical tree hashes); DMG-грабля rel19-22 повторилась (задокументирована),
  поставка = .app
- **WDIO 49/49** (palette 34/34 + LM 15/15; rel21 был 46/48); скриншоты 18/18
  через browser.takeScreenshot() — desktop-захват снял бы окна владельца
  (окно приложения на скрытом Space; две пробные region-съёмки удалены
  немедленно, в артефакт не вошли)
- **Security cycle на af06a4e**: CI full matrix SUCCESS (37596486274,
  workflow_dispatch — push-триггер выключен владельцем); CodeQL дал 6 новых
  High на R2-коде → все разобраны в §9 CODE_SCANNING_RECONCILIATION.md и
  закрыты по одному с evidence → **CS 0 open / DA 0 open**; Dependabot 6 PR
  post-beta по мандату (проверено: TS7/vite8/lucide1 — мажоры)
- RELEASE_GATE.md: rel22-дельта (chat-batch → RELEASE_READY; итог 19/5/1/0/0);
  .rimloc-release-state.json = RC_READY
- Owner-only остатки: level-7 visual (чек-лист в OWNER_TEST_PACKET.md),
  publish-апрув, AI-провайдеры live, Source Inspector live

## 2026-10-07 (7) — РЕСПИН: ШОВ ЛОКАЛЕЙ ПОЙМАН НОВЫМ ГЕЙТОМ
- chatbatch-acceptance (полный UI-цикл, новый гейт) поймал: mapSnapshot сравнивал
  'ru' наивно, а контрактные записи пишут folder-форму 'Russian' (P1-2) →
  применённые переводы не показывались в workspace на свежей сессии (файл и TM —
  корректны). Классический UX-дефект, маскировавшийся ручным переключением цели.
- Фикс f99fc19: чтение — обе формы, commit() — folderForm. Бинарная дельта
  против af06a4e — только project.ts (crates/src-tauri diff пуст).
- Респин 2: прод 2305d564, automation 884baa33; все гейты заново (preflight,
  release-guard static+runtime, WDIO 54 зелёных: palette 34 + LM 15 +
  chatbatch 4 + multitarget 1 самодостаточный); скриншоты пересняты (18).
- multitarget-спека переписана самодостаточно (commit→показ→uk-изоляция→
  round-trip) — старая держалась на утраченном [R1-smoke] фикстуре старого профиля.

## 2026-10-07 (8) — C-ГЕЙТЫ ЗАКРЫТЫ (reviewer-мандат)
- **C1 provider**: точная граница доказана живыми прогонами — OAuth/api-key/DevPack
  отвергнуты api.z.ai (6 комбинаций, evidence C1_PROVIDER_BOUNDARY.md);
  ZCode-сессия живёт за собственным шлюзом. Поверхность provider-test доказана;
  live-E2E ждёт только owner-ключ (1 минута в консоли Z.AI); массовые прогоны —
  mock по мандату.
- **C2 Source Inspector**: ЖИВОЙ в production React — SOURCE-блок (file+line,
  selected_by, version, conditional, patch stage, TKey primary+other), бэкенд
  не тронут (данные уже реальные), WDIO 5/5 с доказательными значениями,
  сверенными с CLI-scan.
- **C3 Dependabot**: все 6 PR разрешены (4 миграции: sha2 0.11, toml 1.1,
  ico 0.5, thiserror 2; supersede: svelte-мажоры, TS7 — ignore-правила с
  причинами и датами); новые патчи #86/#87 влиты. Открытых PR: 0.
- **C4 npm audit**: react 0 vulns; v2 23→16 high — все 16 задокументированы
  (dev-only WDIO-цепочка, upstream-фиксов нет, npm-audit-exemptions.md).
- **C5 CodeQL**: default setup здоров — 298/298 файлов (было 297+1 error):
  frontend-v2/dist закоммичен (generate_context! требует его на свежем
  чекауте); advanced-путь невозможен (default enabled + токен без
  security_events) — вариант 1 по мандату. Продакшн-инвариант сохранён:
  каждый tauri build пересобирает dist, prod собирается react-конфигом с
  tree-hash верификацией.
- **C6 DMG**: root-cause — Portable-SSD это NFS (hdiutil EEXIST, AppleDouble
  «._*.toml» паниковал tauri-utils acl; bash -x + дифф-тест NFS/APFS).
  Формальное решение: macOS artifact = .app + .app.zip (build-production.sh
  --bundles app, exit 0), vendor-патч фильтра по прецеденту wry/tao,
  условие возврата DMG — в RELEASE_GATE.
- **Интеграция**: 4 ветки R3 влиты без конфликтов; semantic-конфликт panels
  v4×SOURCE-блок пойман локальным tsc и исправлен (v4 props = числа-проценты);
  CI-матрица SUCCESS на 41c63e0; CodeQL §10 (10 новых — 7 vendor + 3
  dev-tool, dismissed по одному) → CS 0 / DA 0.

## 2026-10-08 — REVIEWER BRIDGE: 3 ИТЕРАЦИИ → OWNER_ONLY
- Канал: browser-direct ВСТУПИЛ В СИЛУ (owner авторизовал IAB и открыл
  RimLoc-чат; attach через DataTransfer-drop, текст через ProseMirror
  execCommand, send через evaluate-click) — A1-граница пересмотрена.
- **Итерация 1** (пакет 2305d56): CONDITIONAL_PASS — 7 DO_NOW, среди них
  смешанный rel22/rel23 пакет, self-entry манифеста, устаревшая parity.
- **Итерация 2** (пакет 7781dd3 v2, 27 файлов): PRODUCT RC APPROVED;
  1 blocker = packaging/truth gate (27 ._ записей в доставленном zip,
  FINAL_IDENTITY с mutable main).
- **Итерация 3** (mini-proof): OWNER_ONLY подтверждён — reviewer независимо
  проверил конечный ZIP (0 junk, MANIFEST 26/26 PASS, rel23-final, 59/59,
  live GitHub SUCCESS, 0 open PR) и подтвердил переход к owner-тестированию.
- Перманентные фиксы моста: xattr-strip + zip -r -X + внешняя верификация
  (unzip-recount) + declared ⊆ actual + packet-self-test gate
  (распаковка и пофайловая сверка манифеста); mutable main убран из
  FINAL_IDENTITY (repo_head_at_packaging живёт в metadata пакета);
  урок «gate проверяет конечный артефакт снаружи» — в orchestration skill.
- Owner-контур (по reviewer): запустить rel23-final и пройти
  пользовательский сценарий; level-7 RimWorld (HugsLib); UI/UX замечания;
  решение по 16 dev-only npm high; опционально Z.AI key, signing, publish.

## 2026-10-08 — R4 ВОЛНА: UI/PRODUCT TRUTH CLOSURE → rel24-candidate
- Автономная система обновлена ПЕРВОЙ: AUTONOMOUS_FAILURE_PATTERNS.md (12
  классов), owner-gate admission enum + machine ledger (fail-closed поймал
  2 записи), правила в skill.
- Аудит: 5 лейнов, 44 IMPLEMENT_NOW findings с файл:строка evidence
  (UI_R1_FINAL_PARITY / CONTROLS_AUDIT / S7+S8 / WORKSPACE_DESIGN).
- Имплементация: 5 лейнов по непересекающимся файлам; dogfood-редteam
  (16 шагов) нашёл 6 дефектов — все исправлены (включая high: zai-шаблон
  на anthropic-эндпоинте → auth_failed недостижим).
- CI-фича-матрица поймала cfg-баг Http-руки (79dfa09) — exact-SHA правило
  работает. Финальный CI SUCCESS на 2c50dcb.
- **rel24-candidate**: прод 33457117 (self-report fcb82d7 чисто), appzip
  e99b750a; гейты PASS; WDIO 59/59; скриншоты 18; §21 visual parity:
  0 fixes required.
- Owner-only: level-7, субъективный UI pass, решение по 16 dev-only
  advisories, опционально Z.AI key и signing, publish approval.
