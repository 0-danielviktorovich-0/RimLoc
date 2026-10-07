---
type: reference
status: current
tags:
  - project/rimloc
  - kind/autonomous-process
last-reviewed: 2026-10-07
related:
  - "[[RELEASE_GATE]]"
  - "[[FINAL_IDENTITY]]"
  - "[[RELEASE_CONVERGENCE_STATE]]"
---

# AUTONOMOUS FAILURE PATTERNS

Постоянное правило волны R4 (мандат 2026-10-07): **любой дефект, обнаруженный
после предыдущего «final/owner-gated» вердикта, считается одновременно
PRODUCT DEFECT и PROCESS DEFECT.** Помимо кода определяется, какой gate
должен был его поймать, и этот gate добавляется/усиливается.

Формат записи: DEFECT → WHY PRODUCT FAILED → WHY AUTONOMY FAILED →
MISSING GATE → NEW PERMANENT GATE → REGRESSION (как проверяется).

---

## 1. Stale rel21/rel22 evidence в rel23 handoff-пакете

- **DEFECT**: handoff-lite v1 содержал IDENTITY-rel22, rel22
  OWNER_TEST_PACKET и устаревшую parity-матрицу рядом с rel23-заявлениями.
- **WHY PRODUCT FAILED**: evidence-каталог накапливает эпохи; сборщик копировал
  по стабильным путям, не по «текущему кандидату».
- **WHY AUTONOMY FAILED**: пакер не сверял содержимое пакета с заявленным RC.
- **MISSING GATE**: состав-гейт пакета.
- **NEW PERMANENT GATE**: build-handoff-lite v2 — whitelist текущего
  артефакта; EVIDENCE_REQUEST-список как контракт состава.
- **REGRESSION**: packet-self-test (stale top-level rel21/rel22 = 0) + независимый
  аудит reviewer'а каждого пакета.

## 2. Self-referential MANIFEST (broken self-check)

- **DEFECT**: MANIFEST.sha256 содержал собственную запись с SHA пустого файла —
  формально не проходил проверку собственного содержимого.
- **WHY PRODUCT FAILED**: `find . -type f` во время записи манифеста включал
  сам манифест.
- **WHY AUTONOMY FAILED**: манифест генерировался, но никем не верифицировался.
- **MISSING GATE**: манифест-гейт.
- **NEW PERMANENT GATE**: packet-self-test распаковывает ZIP и сверяет каждый
  entry; сам манифест исключён из собственного списка; checksum доставляется
  вне полосы (в delivery-сообщении).
- **REGRESSION**: self-test RUN на каждом пакете (MANIFEST = PASS обязателен).

## 3. AppleDouble-мусор в handoff ZIP

- **DEFECT**: 27 `._*`-записей внутри доставленного ZIP при заявлении
  «junk-free»; пост-чек по рабочей директории их не видел.
- **WHY PRODUCT FAILED**: ditto -k материализует xattr как AppleDouble-записи
  внутри архива; файлы рабочей директории чисты.
- **WHY AUTONOMY FAILED**: gate доверял промежуточной директории, а не
  конечному артефакту.
- **MISSING GATE**: внешняя верификация конечного артефакта.
- **NEW PERMANENT GATE**: xattr-strip всех файлов пакета; `zip -r -X`; пост-чек
  `unzip -l` (unzip-recount) с отказом при любом мусоре; заявленные файлы ⊆
  фактический состав ZIP.
- **REGRESSION**: EXTERNAL CHECK строка сборщика + независимый unzip-аудит
  reviewer'а (итерация 2 доказала ловушку).

## 4. Premature OWNER_GATE

- **DEFECT**: «Source Inspector live» стоял в ownerGates, хотя был технически
  выполним и был реализован (C2); provider «ready» трактовался как готовность.
- **WHY PRODUCT FAILED**: owner-gate использовался как полка для отложенных
  задач, не как честный барьер.
- **WHY AUTONOMY FAILED**: не было критерия допуска в OWNER_GATE.
- **MISSING GATE**: admission gate.
- **NEW PERMANENT GATE**: OWNER_GATE допускается ТОЛЬКО с reason enum:
  SUBJECTIVE_VISUAL · OWNER_CREDENTIAL · EXTERNAL_ACCOUNT_ACTION ·
  SIGNING_NOTARIZATION · PUBLISH_AUTHORIZATION ·
  PHYSICAL_OR_GAME_VISUAL_CONFIRMATION. Технически выполнимая задача
  owner-gate'ом быть не может — агент продолжает автономную реализацию.
  Машинная проверка: `.rimloc-release-state.json` ownerGates сверяется со
  скриптом `testlab/check-owner-gates.py` (reason обязателен для каждой записи).
- **REGRESSION**: пересмотр ledger при каждом RC (current: Source Inspector
  исключён; provider-live = OWNER_CREDENTIAL; level-7 =
  PHYSICAL_OR_GAME_VISUAL_CONFIRMATION; publish = PUBLISH_AUTHORIZATION).

## 5. Locale seam `ru` / `Russian`

- **DEFECT**: применённые чат-батчем/импортом переводы (folder-form
  `Russian`) не отображались в Workspace, который сравнивал с сырым `ru`.
- **WHY PRODUCT FAILED**: две формы локали в записи, одна в чтении.
- **WHY AUTONOMY FAILED**: 385+ зелёных тестов проверяли бэкенд; ни один
  behavioral тест не прогонял полный UI-цикл «применить → увидеть».
- **MISSING GATE**: behavioral E2E пользовательского цикла.
- **NEW PERMANENT GATE**: chatbatch-acceptance WDIO (полный UI-цикл select →
  create → export → import → apply → workspace) — именно он поймал дефект.
- **REGRESSION**: suite зелёный на каждом RC (59/59 включает его).

## 6. Provider «ready» ≠ connected

- **DEFECT-КЛАСС (R4-волна, исправляется)**: GUI считал `ready = has_key ||
  local` и называл офлайн-валидацию «Test».
- **WHY PRODUCT FAILED**: статус конфигурации выдан за статус подключения.
- **WHY AUTONOMY FAILED**: UI-состояния не были покрыты behavioral-тестами
  (замена network-call моком).
- **MISSING GATE**: semantic-гейт UX-статусов.
- **NEW PERMANENT GATE**: R4 §7 — разделение CONFIGURED / CONNECTION_UNKNOWN /
  TESTING / CONNECTED / AUTH_FAILED / NETWORK_FAILED / MODEL_NOT_FOUND /
  LOCAL_OFFLINE; кнопка «Проверить подключение» вызывает реальный bounded
  `provider-test`; behavioral-тест: offline → AUTH/NETWORK_FAILED, а не
  зелёный ready.
- **REGRESSION**: WDIO providers-спека (R4).

## 7. CodeQL workflow-success ≠ healthy configuration

- **DEFECT**: «success» джобы сосуществовал с banner «configuration error» и
  1 файлом extraction error (generate_context! без dist).
- **WHY PRODUCT FAILED**: dist gitignored, конфигурация не создавала контекст.
- **WHY AUTONOMY FAILED**: exit-код принимался за health; tool status не
  проверялся.
- **MISSING GATE**: tool-status гейт.
- **NEW PERMANENT GATE**: RC-чекер читает extraction-метрики последнего
  анализа (currently 323 clean / 1 vendor-only documented) и state
  default-setup; success ≠ healthy без сверки.
- **REGRESSION**: C5-коррекция в reviewer-цикле (323/1 принят как documented
  limitation); при каждом RC — свежий codeql-tool-status.txt в пакете.

## 8. Stale parity matrix

- **DEFECT**: матрица утверждала «Chat batch IMPLEMENT_NOW» после реализации;
  IfModActive описан как over-включение после R2.
- **WHY PRODUCT FAILED**: truth-документы не обновлялись вместе с кодом.
- **WHY AUTONOMY FAILED**: не было требования синхронизации truth при
  финализации.
- **MISSING GATE**: truth reconciliation.
- **NEW PERMANENT GATE**: перед FINAL — автоматический reconcile (git HEAD,
  product_source_sha, artifact SHA, CI SHA, CodeQL SHA, RELEASE_GATE,
  RELEASE_PARITY_MATRIX, release-parity.json, release-state,
  OWNER_TEST_PACKET, handoff packet); IMPLEMENT_NOW ≠ 0 или UNKNOWN → FINAL
  запрещён.
- **REGRESSION**: reviewer-итерация 1 поймала; итог — 0 IMPLEMENT_NOW.

## 9. Exact-final-SHA CI gap

- **DEFECT**: SUCCESS был на промежуточных SHA; финальная комбинация не
  прогонялась.
- **WHY PRODUCT FAILED**: CI диспатчился не на каждый финальный коммит.
- **WHY AUTONOMY FAILED**: «недавно зелёный» считался покрытием.
- **MISSING GATE**: exact-SHA правило.
- **NEW PERMANENT GATE**: workflow_dispatch CI на точный финальный SHA перед
  owner packet (записывается в final-ci-summary.txt с SHA).
- **REGRESSION**: run 37627700035 @ 24d1d1f SUCCESS; правило в Part D цикла.

## 10. Hidden owner desktop capture

- **DEFECT**: две пробные desktop-съёмки захватили окна владельца (окно
  приложения на скрытом Space).
- **WHY PRODUCT FAILED**: region-capture не знает о принадлежности окон.
- **WHY AUTONOMY FAILED**: отсутствовал privacy-гейт capture-метода.
- **MISSING GATE**: выбор канала съёмки.
- **NEW PERMANENT GATE**: для UI-доказательств — только page-image
  (browser.takeScreenshot) или окно приложения с проверенной принадлежностью;
  desktop-регион запрещён без явной owner-разрешённости. Найденные захваты
  удалялись немедленно, в артефакт не вошли.
- **REGRESSION**: rel23-скриншоты сняты takeScreenshot (18/18).

## 11. Feature-specific cfg build bug (panels v4 × Source Inspector)

- **DEFECT**: авто-мерж двух лейнов скомпилировался semantically неверно
  (props-типы panels v4 vs старый вызов).
- **WHY PRODUCT FAILED**: оба лейна зелёные по отдельности, комбинация не
  тестировалась до мержа.
- **WHY AUTONOMY FAILED**: интеграционный локальный tsc не был обязательным
  шагом.
- **MISSING GATE**: пост-мерж семантическая сборка.
- **NEW PERMANENT GATE**: после каждого интеграционного мержа — локальные
  tsc+vite+cargo-check ДО пуша (выполнено: конфликт пойман и исправлен
  41c63e0 до CI).
- **REGRESSION**: интеграция R3 прошла через обязательный локальный гейт.

## 12. Reviewer Bridge packaging (итоговая форма §1-§3)

- **DEFECT**: сводный класс итерации 1 (пакет/манифест/джанк) — см. выше.
- **NEW PERMANENT GATE**: packet-self-test + declared ⊆ actual + внешний
  unzip-recount — все три обязательны на каждый пакет.
- **REGRESSION**: итерация 3 mini-proof PASS; правило «gate проверяет
  конечный артефакт снаружи» — в multi-workflow-orchestrator skill.
