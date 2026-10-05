# Tier C/D/E — north-star референс локализационных инструментов

**Дата:** 2026-10-05 · **Статус лейна:** разведскан (DOC_ONLY), не бета-цели и не парити · **Продукт референса:** RimLoc 0.1.0-alpha.1 (CLI, RimWorld-локализация)

**Методология.** Каждый тулза описан по документации/первоисточникам, где удалось их прочитать в этом заходе; иначе — по устойчивому знанию, честно помечено. Статусы прогона — из словаря лейна. Проверено в этом заходе (WebFetch первоисточников): Weblate checks, Unity Localization, Godot i18n, XUnity.AutoTranslator (GitHub README), Crowdin webhooks, Unreal FText/StringTables (dev.epicgames.com), модель TM-матчей (Wikipedia). Веб-поиск (WebSearch) был рейт-лимичен (HTTP 429) — Gridly, Lingo.dev, Languine и остальные TMS верифицировать вебом не удалось → они DOC_ONLY.

**Заземление по RimLoc (прочитано в этом заходе):** бинарь `rimloc-cli --help` (`/Volumes/Portable-SSD/caches/targets/rimloc-tgbench/release/rimloc-cli`, 27 команд: scan, merge-keyed, validate, validate-po, diff-xml, coverage, learn-defs/learn-keyed/learn-patches, annotate, schema, xml-health, lang-update, morph, init, export-po/export-xliff, import-po/import-xliff, word-info, version-diff, compare, translate, build-mod, doctor). TM — `crates/rimloc-domain/src/tm.rs`: ключ `(source_text, target_locale)`, case-sensitive, статусы Draft/Accepted/Reviewed с rank, provenance AUTO/IMPORT/MANUAL, write-политики A/B/C, **fuzzy уже есть** — bounded Levenshtein, порог `max(2, len/10)` (tm.rs:176–219). Глоссарий — `glossary.rs`: term→translation+note, уникален case-insensitive, лимиты полей. Чеки — `rimloc-validate/src/lib.rs`: mismatch плейсхолдеров, невидимые/bi-di символы, дубли ключей (в файле и кросс-файловые), неоднозначные ссылки, сироты, severity Error/Warning/Info.

---

## Сводная таблица: тулза → что берём на roadmap

| Тулза | Категория | Что взять на roadmap (одна строка) | Метка | Статус прогона |
|---|---|---|---|---|
| Gridly | GAME/TMS | Ячеечная модель строки: контекст, статус и character-limit на уровне ячейки, а не файла | ROADMAP (метаданные сегмента) | DOC_ONLY (поиск 429) |
| Crowdin | GAME/TMS | Событийная модель (file.translated, project.*), git round-trip через PR, лимиты вебхуков как контракт CI | ROADMAP (CI-контракт, не сервер) | SOURCE_CONFIRMED_ONLY (support.crowdin.com/webhooks) |
| Lokalise | GAME/TMS | Платформенные теги на ключе + screenshot-контекст рядом со строкой | ROADMAP (теги) | DOC_ONLY |
| Phrase | GAME/TMS | Строгий QA-каталог и match-бэнды enterprise-CAT как эталон проверок | ROADMAP (через секцию CAT) | DOC_ONLY |
| Transifex | GAME/TMS | Дедупликация строк между версиями ресурса («строка живёт, файл меняется») — у нас уже растёт в version-diff | ROADMAP (усилить version-diff) | DOC_ONLY |
| Smartling | GAME/TMS | Визуальный контекст у сегмента (для RimWorld — отрывок XML-дефа рядом со строкой) | ROADMAP (контекст-подсказка) | DOC_ONLY |
| Smartcat | GAME/TMS | Идея «переводчик в один клик» (маркетплейс) как дальняя продуктовая опция | ROADMAP (дальняя) | DOC_ONLY |
| XTM | GAME/TMS | Workflow-стадии проекта (translate→review→sign-off) поверх статусного словаря | ROADMAP (статусы сегментов) | DOC_ONLY |
| POEditor | GAME/TMS | Публичный проект для сообщества: dump-выгрузка, которую волонтёры правят без установки | ROADMAP (экспорт в git = наш публичный проект) | DOC_ONLY |
| Localazy | GAME/TMS | ShareTM: общая TM между проектами — межмодовая TM RimWorld (общие термины pawns/hediffs) | ROADMAP (внешняя TM) | DOC_ONLY |
| Locize | GAME/TMS | Save-missing: неизвестные ключи автоматически падают в бэклог | ROADMAP (частично есть: learn-*) | DOC_ONLY |
| SimpleLocalize | GAME/TMS | Минимализм ценностного предложения, CLI-first дешёвый тир — ориентир позиционирования, не фича | INTENTIONAL_NON_GOAL (нечего копировать фичево) | DOC_ONLY |
| i18nexus | GAME/TMS | Экспорт в Google Sheets как «нулевой порог входа» для сообществ переводчиков | ROADMAP (опционально) | DOC_ONLY |
| Texterify | GAME/TMS | PO-first self-host стек — подтверждение выбора форматов, фичево мало | INTENTIONAL_NON_GOAL | DOC_ONLY |
| Weblate | OPEN SOURCE | Каталог из ~100 QA-чеков по категориям + чек «does not follow glossary» + git-native коммиты в репо | ROADMAP (чек-каталог — прямой бэклог validate) | SOURCE_CONFIRMED_ONLY (docs.weblate.org/user/checks) |
| Tolgee | OPEN SOURCE | In-context ревью (клик по элементу) — для RimWorld аналог: контекст из дефа/скриншот в метаданных строки | ROADMAP (контекст, не UI) | DOC_ONLY |
| Mozilla Pontoon | OPEN SOURCE | Терминология как отдельная панель переводчика, командные проекты поверх VCS | ROADMAP (терминология в UI) | DOC_ONLY |
| Traduora | OPEN SOURCE | API-first дизайн (всё, что умеет UI, умеет и CLI/API) — инвариант для RimLoc GUI | ROADMAP (принцип) | DOC_ONLY |
| Lingo.dev | OPEN SOURCE | CI-паттерн «дифф только изменённых строк → LLM → PR», компилятор как zero-key-менеджмент | ROADMAP (translate --changed-only + Action) | DOC_ONLY (поиск 429) |
| Languine | OPEN SOURCE | CLI-first AI-локализация с детектом изменённых ключей — подтверждение тренда, того же что Lingo.dev | ROADMAP (тот же паттерн) | DOC_ONLY (поиск 429) |
| Trados Studio | CAT | Эталон всего: TM-бэнды, pre-translate с локом exact, concordance, termbase, QA Checker, статусы сегментов | ROADMAP (north star секции A) | DOC_ONLY (docs.rws.com отдали 403; концепты — Wikipedia) |
| memoQ | CAT | LiveDocs: сырой корпус как референс без импорта в TM — workshop-корпус как референсный корпус | ROADMAP (concordance по корпусу) | DOC_ONLY |
| Wordfast | CAT | Лёгкий txt-TM и переносимость — минимум; ничего уникального | INTENTIONAL_NON_GOAL | DOC_ONLY |
| OmegaT | CAT | Git-нативные team-проекты и TMX-обмен в открытом стеке — доказательство жизнеспособности | ROADMAP (TMX-обмен) | DOC_ONLY |
| CafeTran Espresso | CAT | macOS-нативность как ниша (совпадает с нашей аудиторией GUI), hitlist-предобработка | ROADMAP (GUI-приоритет macOS) | DOC_ONLY |
| MateCat | CAT | Массовый веб-воркфлоу для волонтёров с MT-подсказками; сам сервер — не наш дом | INTENTIONAL_NON_GOAL (сервер) | DOC_ONLY |
| Poedit | CAT | Одиночно-файловый UX «открыл .po → перевёл → сохранил» — наш GUI-поток export-po→edit→import-po | ROADMAP (UX-поток, почти есть) | DOC_ONLY |
| KDE Lokalize | CAT | Открытый PO-стек с TM/глоссарием/QA — подтверждение PO-first выбора | ROADMAP (через PO-first) | DOC_ONLY |
| XUnity.AutoTranslator | ENGINE/RUNTIME | Файловый формат `оригинал=перевод` из `Translation\{Lang}\Text\*.txt` — знакомый фан-переводчикам таргет экспорта | ROADMAP (опциональный экспортёр) | SOURCE_CONFIRMED_ONLY (github.com/bbepis/XUnity.AutoTranslator) |
| Unity Localization | ENGINE/RUNTIME | XLIFF/CSV как игровые обменные форматы (мы уже export-xliff — подтверждено) + pseudo-localization как QA-проход | ROADMAP (pseudo-loc) | SOURCE_CONFIRMED_ONLY (docs.unity3d.com) |
| Unreal localization | ENGINE/RUNTIME | Тройка Namespace+Key+SourceString и LocRes-компиляция: SourceString как stale-детектор — ровно наш diff-xml | ROADMAP (инвариант дизайна) | SOURCE_CONFIRMED_ONLY (dev.epicgames.com, FText-страница) |
| Godot localization | ENGINE/RUNTIME | gettext PO как первоклассный игровой формат + tr()/TranslationServer runtime-модель | ROADMAP (через PO-first) | SOURCE_CONFIRMED_ONLY (docs.godotengine.org) |

---

## Раздел A. Профессиональный CAT-workflow: что должен уметь RimLoc когда-нибудь

Это ядро лейна. Профессиональный переводческий процесс в Trados/memoQ держится на шести механизмах; ниже — каждый, его смысл и маппинг на наши TM (`crates/rimloc-domain/src/tm.rs`), глоссарий (`glossary.rs`) и чеки (`rimloc-validate/src/lib.rs`).

### A1. TM-матчи с процентными бэндами и pre-translate — ROADMAP

В CAT каждый новый сегмент сравнивается с TM: **exact match (100%)** — посимвольное совпадение; **context match / ICE (101%)** — exact плюс совпадение окружения (сегмент выше, файл); **fuzzy (обычно 50–99%)** — нечётное совпадение с процентом схожести; бэнды 50–74 / 75–99 / 100 условно делят «низкое/высокое/точное» совпадение (конвенция индустрии; Wikipedia подтверждает типы exact/ICE/fuzzy, но принципиально не пиннит конкретные цифры — они несопоставимы между системами; docs.rws.com в этом заходе не открылся, 403). **Pre-translate** — прогон всего файла через TM до открытия: exact подставляются и **лочатся** (сегмент сразу «translated»), fuzzy подставляются черновиком. Для RimLoc: fuzzy-lookup в TM уже реализован (bounded Levenshtein, порог `max(2, len/10)`), ROADMAP — добавить **процент схожести в выдачу** и команду `pretranslate` c политиками «exact → применить со статусом Reviewed, fuzzy ≥ X% → применить как Draft» (поверх существующих TmStatus/Ack-политик). XLIFF уже умеет нести `state` и `state-qualifier`, PO — флаг `fuzzy`: новый формат не нужен.

### A2. Concordance search — ROADMAP (наш козырь)

Поиск выбранной фразы **внутри сегментов TM** — переводчик видит, как похожие фразы уже переводились, без терминологической базы. Для RimLoc это особенно ценно: корпус мастерских модов (`steamcmd-root/.../294100/`) — фактически бесплатный референсный корпус. МемоQ-паттерн **LiveDocs** (корпус как референс *без* обязательного импорта в TM) переносится один-в-один: `rimloc concordance "query"` по TM проекта + по выгруженным переводам других модов. ROADMAP; интент — именно подсказка переводчику, не автоподстановка.

### A3. Termbase / глоссарий с принуждением — ROADMAP

У нас глоссарий есть (term→translation+note, case-insensitive, валидация ввода). В CAT и Weblate он работает **двусторонне**: подсветка терминов в исходнике при переводе и **QA-чек «перевод не следует глоссарию»** (у Weblate чек «does not follow glossary» подтверждён докой). ROADMAP: чек в `rimloc-validate` — «в исходнике есть term, в переводе нет translation» (с allow-list и учётом морфологии — у нас уже есть `morph` для русской падежной системы, что большинство CAT-тулз не умеет вовсе).

### A4. Каталог QA-чеков — ROADMAP (прямой бэклог)

Our `validate` уже покрывает: плейсхолдеры, невидимые/bi-di символы, дубли, сироты, xml-health, validate-po. Каталог Weblate (прочитан, docs.weblate.org/user/checks) даёт систематизацию недостающего по категориям: **маркап** (BBCode/XML/Markdown-синтаксис), **пунктуация** (двойной пробел, mismatched colon/question mark, ellipsis, множественные заглавные), **whitespace** (leading/trailing space/newline, mismatched line breaks), **консистентность** (inconsistent translations — одинаковый исходник переведён по-разному; reused translation; unchanged translation), **дубли слов подряд**, **максимальная длина/число строк**. ROADMAP: добить whitespace/двойные пробелы/unchanged/inconsistent в первую волну; length-limits — опционально (RimWorld UI не даёт жёстких лимитов, поле может нести подсказку из метаданных строки — см. Gridly-строку таблицы).

### A5. Статусная модель сегмента — ROADMAP

Профессиональный CAT живёт статусами сегмента: Untranslated → Draft → Translated → Reviewed → Signed-off, и pre-translate локирует только подтверждённые. Наши TmStatus (Draft/Accepted/Reviewed, rank с монотонным усилением — более сильный статус не ослабляется слабой записью) — уже правильный примитив. ROADMAP: протащить статусы **в форматы обмена** — PO `fuzzy`-флаг и `#`-комментарии, XLIFF 1.2 `state` (`needs-translation`/`translated`/`reviewed`/`final`) — тогда раунд-трип с Trados/memoQ/OmegaT сохраняет рабочий процесс, а не только строки.

### A6. Обменные форматы TM/глоссария — ROADMAP

CAT-индустрия обменивается **TMX** (памяти переводов) и **TBX** (терминология); XLIFF — формат задания на перевод. Мы уже двусторонне работаем с PO и XLIFF 1.2. ROADMAP: `export-tmx`/`import-tmx` (наша TM → чужая CAT и обратно — сразу открывает аудиторию переводчиков-профессионалов) и `export-tbx`/CSV для глоссария. OmegaT (открытый CAT) подтверждает жизненность TMX-центричного стека.

### Явные INTENTIONAL_NON_GOAL секции A

- **Полноценный сегментный редактор командой** (параллельное редактирование, assign, lexxer/verification-настройки Trados/memoQ) — ядро RimLoc остаётся CLI+лёгкий GUI; полноценный CAT-GUI не строим.
- **In-context клик-редактирование** (Tolgee/Locize) — web-паттерн, к RimWorld-файлам неприменим напрямую.
- **Серверная TM-синхронизация в реальном времени** — локальный файл проекта остаётся источником правды.
- **Fragment recall** (Trados UpLift — матч на уровне под-сегментов) — дальняя ROADMAP-звезда, не цель ближайших волн.

---

## Раздел B. Continuous localization: триггеры и webhooks

**Механика Crowdin (прочитано в этом заходе).** Вебхук = имя + набор событий + URL (2XX за 30 секунд), метод GET/POST, опциональные заголовки и batch; до 20 эндпоинтов на проект; доставка однократная **без retry**; 100+ фейлов (4xx/5xx) за 24 часа автоматически отключают вебхук. События вида `project.translated`, `file.translated`, `task.completed` (полный список в developer-портале; на прочитанной странице не расписан). CI/CD-интеграции (GitHub/GitLab/Bitbucket) — это VCS-синк: исходники тянутся из репо, переводы возвращаются **merge-реквестами**.

**Модель Weblate** — git-native: компонент = URL репозитория, TMS сам коммитит переводы обратно (push-модель). **Модель Lingo.dev/Languine** (DOC_ONLY — верификация упёрлась в рейт-лимит поиска) — CLI/GitHub Action в CI: детект изменённых ключей → переводятся только они → PR.

**Маппинг на RimLoc.** RimLoc — локальный CLI, свой вебхук-сервер и облачный хостинг = INTENTIONAL_NON_GOAL. Но контракт *триггеров* берём целиком — ROADMAP:

1. **GitHub Action-обёртка** (`rimloc diff-xml`/`version-diff`/`coverage` как CI-шаг с осмысленными exit-кодами и машиночитаемым отчётом) — триггер: push в репо мода. Это наш ответ на Crowdin/VCS-синк.
2. **Дифф-only LLM-перевод**: `rimloc translate` уже есть; добавить `--changed-only` (по baseline PO из diff-xml) — паттерн Lingo.dev, триггер: тот же push/PR.
3. **Ночной cron-режим**: пересбор покрытия и бэклога (`learn-keyed`/`learn-patches` уже производят missing-отчёты) — аналог «save-missing» Locize.
4. **Релизный гейт**: `version-diff` + `validate` как обязательный шаг перед публикацией новой версии мода (триггер: тег/релиз); «переводы отстают от инвентаря исходников» — машиносчитываемый вердикт.
5. **Контракт отчёта**: стабильные коды чеков + severity (уже есть в ValidationMessage) — предпосылка для GitHub Annotations/SARIF-подобной подачи; событийная таксономия Crowdin — справочник имён, если когда-нибудь понадобится внешняя интеграционная шина (INTENTIONAL_NON_GOAL сегодня).

---

## Раздел C. Runtime-механика игр: форматы

**RimWorld (наш дом).** Перевод живёт в `Languages/<Язык>/`: `Keyed/` (ключ-значение XML), `DefInjected/` (пути к полям дефов), `Backstories/`, `Words/`. Мод-перевод = обычный мод с этой структурой, подгружается игрой нативно — это и есть наш «runtime delivery»: `rimloc build-mod` уже собирает standalone-мод из .po. Drop-in без кода — преимущество, которого нет ни у одной из платформ ниже.

**XUnity.AutoTranslator + BepInEx** (README прочитан). Runtime-плагин для Unity-игр: хукает текстовые компоненты (UGUI, NGUI, TextMeshPro, FairyGUI, Utage; IMGUI/TextMesh — опционально) и подменяет текст на лету; онлайн-эндпоинты (Google, Bing, DeepL, Papago, Yandex, LLM через кастомный HTTP) с rate-limit и кэшем; ручные переводы — **`Translation\{Lang}\Text\*.txt`, формат пар `оригинал=перевод`**, auto-файл имеет низший приоритет, есть regex-правила (`r:`) и scoping-директивы; загрузчики BepInEx/MelonLoader/IPA. Для RimLoc: этот формат — узнаваемый стандарт фан-переводов; ROADMAP-опционально — экспортёр в него (низкая цена, нестратегически). Сам плагин — чужой дом, не наш.

**Unity Localization** (дока прочитана). Пакет `com.unity.localization`: строковые таблицы на локаль (StringTable), Smart Strings (плейсхолдеры и плюралы в строках), локализация ассетов (текстуры/звук на локаль), pseudo-localization для тестов до переводов, импорт/экспорт **XLIFF, CSV, Google Sheets** (PO докой не упомянут). Runtime — выбор локали через LocalizationSettings. Подтверждение для нас: XLIFF — реально игровой обменный формат (наш `export-xliff` выбран верно). Взять: **pseudo-локализацию как QA-проход** — ROADMAP (генерить синтетические переводы с сохранением плейсхолдеров и раздуванием длины, прогонять `validate` — ловит проблемы до игры).

**Unreal** (страница FText/StringTables прочитана; страницы pipeline отдали 404/403 — нижеследующее по pipeline помечено как знание). Локализуемый текст = **Namespace + Key + Source String** (NSLOCTEXT/LOCTABLE); Source String хранится рядом и служит детектором устаревания перевода; строковые таблицы централизуют ключи (в т.ч. для модов/плагинов); собранные переводы — LocRes-ресурсы на культуру; polyglot-данные позволяют добавлять локализацию в рантайме. Pipeline (DOC_ONLY): Localization Dashboard собирает текст (Gather Text) в манифест .locmeta → архивы .archive → компилирует .locres; переводчики работают с **.po**. Для нас: тройка «путь-дефа = namespace, ключ, исходная строка» — ровно наша модель DefInjected+EN-annotate, а stale-детект по Source String — это уже работающий `diff-xml`/`version-diff`. Вывод: дизайн подтверждён, менять нечего — ROADMAP-пункта нет, инвариант.

**Godot** (дока прочитана). Переводы — ресурсы **CSV** или **gettext PO**; автоперевод контролов по ключу, `tr()`/`tr_n()` (плюралы и контексты), `TranslationServer.set_locale()` — смена локали на лету; ремапы ассетов на локаль; при экспорте проекты бандлят переводы автоматически. Для нас: ещё одно независимое подтверждение PO как первоклассного игрового формата — PO-first стратегия RimLoc устойчива. Взять фичево нечего сверх PO-first (уже сделано).

---

## Итог: топ-5 идей worth stealing

1. **Каталог QA-чеков Weblate** (категории: плейсхолдеры/маркап/пунктуация/whitespace/консистентность/длина) — систематический бэклог для `rimloc-validate`, расширение существующей системы кодов.
2. **TM-бэнды + pre-translate с локом exact** (Trados-модель) поверх уже реализованного Levenshtein-fuzzy — процент схожести в выдаче + команда `pretranslate` с политиками по статусам.
3. **Concordance по корпусу мастерских модов** (memoQ LiveDocs-паттерн) — поиск «как это уже переводилось» по TM проекта и по выгрузкам чужих переводов; уникальное преимущество RimLoc, которого нет у TMS.
4. **Git-native continuous loop** (Weblate push-модель + Lingo.dev diff-only): GitHub Action с `diff-xml`/`version-diff`/`coverage`, `translate --changed-only`, релизный гейт на validate.
5. **Статусы сегмента в обменных форматах** (PO fuzzy-флаг, XLIFF state) + **TMX/TBX экспорт** — совместимость с профессиональным CAT-миром без изобретения новых форматов.

*INTENTIONAL_NON_GOAL (зафиксировано): свой веб-сервер/хостинг/вебхуки, OTA-SDK, маркетплейс переводчиков, полноценный командный CAT-редактор, in-context клик-редактор, GDN-прокси. RimLoc остаётся локальным CLI-first инструментом с лёгким GUI.*

---
*Прогоны инструментов не выполнялись (все статусы — DOC_ONLY / SOURCE_CONFIRMED_ONLY по словарю). Платных API-вызовов нет. Корпус не модифицировался. Выход — только этот файл в /tmp/w3-cde/.*
