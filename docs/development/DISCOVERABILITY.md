# Discoverability & SEO — стратегия RimLoc

```yaml
type: development-doc
status: draft
tags: [project/rimloc, topic/seo, kind/strategy]
last-reviewed: 2026-09-23
related: [COMPETITOR_MATRIX.md, MULTILINGUAL_ARCHITECTURE.md]
```

Исследовательский документ: как RimLoc находят и будут находить игроки, переводчики и моддеры.
**Реализация — на фазе docs hardening**, здесь только решения, готовые строки и конфиг-сниппеты.

Факты о найденном проверены на текущем состоянии: `gh api` (топики и аналоги), исходники
`mkdocs-material==9.7.7` и `mkdocs-static-i18n==1.3.1` (pip, версия из `requirements-docs.txt`),
построенный сайт `site/`, веб-поиск по реальной выдаче (сентябрь 2026).

---

## Содержание

1. [Search intent — реальные запросы аудиторий](#1-search-intent)
2. [GitHub metadata — proposal](#2-github-metadata)
3. [README strategy](#3-readme-strategy)
4. [Docs landing pages — по странице на intent](#4-docs-landing-pages)
5. [MkDocs metadata — что уже поддерживает наша связка](#5-mkdocs-metadata)
6. [Release notes как поисковая поверхность](#6-release-notes)
7. [Post-release measurement](#7-post-release-measurement)
8. [Policy — чего SEO не ломает](#8-policy)
<!-- Пункт 9 «Чеклист внедрения» удалён из оглавления аудитом 2026-10-05: раздела #9-чеклист-внедрения в документе нет (см. DOCUMENTATION_AUDIT_2026-10.md). -->

---

## 1. Search intent

Методика: веб-поиск по формулировкам из задачи + расширение по реальной выдаче (кто и чем
отвечает). Оценки объёма/конкуренции качественные (нишевая игровая тема: объёмы везде от
десятков до низких сотен запросов/мес на язык; решает не объём, а точность попадания).
**Intent**: `I` — информационный, `T` — инструментальный (человек ищет инструмент/скачать),
`M` — смешанный.

### 1.1 Английские запросы

| # | Запрос | Intent | Объём | Конкуренция | Кто отвечает сейчас | Комментарий |
|---|--------|--------|-------|-------------|---------------------|-------------|
| 1 | `how to translate a RimWorld mod` | I | средний | низкая | [RimWorld Wiki: Localization](https://rimworldwiki.com/wiki/Localization), Steam-гайды | Главный вход новичка. Wiki отвечает про ручной XML, инструмента не предлагает — наша страница должна отвечать способом «инструментом» и ссылаться на Wiki как источник формата |
| 2 | `RimWorld mod translator` | T | средний | низкая | RimTrans, [RimTranslate](https://github.com/winterheart/RimTranslate), [kelvinauta/Rimworld-Mod-Translator](https://github.com/kelvinauta/Rimworld-Mod-Translator), Steam-мод Kkokoros | Прямой брендовый запрос на нас. У аналогов слабые README без ключевых слов |
| 3 | `RimWorld translation tool` | T | средний | низкая | тот же набор + RimTrans (ludeon) | Тоже прямой. Нужна страница-сравнение/обзор в доках (честная, см. Policy) |
| 4 | `RimWorld language pack` (+ `create`, `install`) | M | средний | низкая | Wiki, Steam-гайды, Ludeon форумы | Двойной intent: игрок хочет готовый пак, автор хочет сделать. Отвечаем обеим веткам на одной странице |
| 5 | `RimWorld AI translation` / `AI translate RimWorld mod` | T/I | растёт | почти нулевая | Обрывки: обсуждения на DTF/Reddit, скрипты на GitHub, in-game мод Auto Translation | Ниша формируется, устоявшегося лендинга нет. Наша страница про AI-перевод может стать стандартным ответом. Окно возможностей |
| 6 | `RimWorld DefInjected` (+ `translation`, `edit`) | I | низкий | низкая | Wiki | Запрос разработчиков/переводчиков. Глоссарий + learn-страница должны его покрывать |
| 7 | `update RimWorld translation after mod update` | I | низкий-средний | низкая | RimTranslate README («run again after update»), RimSort wiki | Боль «перевод умер после апдейта мода» — прямое попадание в `diff-xml`/`update_translations` |
| 8 | `RimWorld localization workflow` | I/T | низкий | низкая | Wiki, форумы, блоги CAT-инструментов | Профессиональный переводчик. Отвечает наш `guide/translators.md` |
| 9 | `RimWorld translation memory` / `RimWorld PO files` | I/T | очень низкий | нулевая | [RimTranslate](https://github.com/winterheart/RimTranslate) (упоминает OmegaT/Poedit) | Крошечный объём, нулевая конкуренция, идеальная точность. Покрывается `guide/po_files.md` |
| 10 | `RimWorld mod localization validation` / `placeholder mismatch` | I | низкий | нулевая | форумы (ошибки загрузки), Wiki | QA-боль: «перевод не подхватился/сломал игру». Отвечают `validate` + troubleshooting |

### 1.2 Русские запросы

| # | Запрос | Intent | Объём | Конкуренция | Кто отвечает сейчас | Комментарий |
|---|--------|--------|-------|-------------|---------------------|-------------|
| 11 | `русификатор модов RimWorld` | M (готовый пак) | средний+ | средняя | [RusPack (Steam, 140+ переводов)](https://steamcommunity.com), [Playground.ru](https://www.playground.ru), top-mods.ru | Игрок хочет готовое. Честная стратегия: не спорить с RusPack, а перехватить того, кто не нашёл свой мод в коллекциях → «как сделать перевод самому» |
| 12 | `перевод модов RimWorld` / `RimWorld перевод мода` | I/M | средний | низкая | [Steam-гайд «Как переводить моды RimWorld?»](https://steamcommunity.com), xgm.guru, Ludeon форумы | Главный RU-вход. Русская версия лендинга #1 из таблицы 1.1 |
| 13 | `как перевести мод RimWorld на русский` | I | средний | низкая | Steam-гайды, xgm.guru | Вариант #12, длинный хвост. Одна страница, разные формулировки в тексте — без спама |
| 14 | `RimTrans` / `RimTrans скачать` | T (бренд) | средний | низкая | top-mods.ru, GitHub RimTrans | Брендовые запросы конкурента. Честная страница «RimLoc vs RimTrans / RimTranslate» перехватывает сравнивающих. Не писать «RimTrans лучше/хуже» голословно — факты возможностей |
| 15 | `программа для перевода модов RimWorld` | T | низкий-средний | низкая | [Pikabu-мини-программы](https://pikabu.ru), TohaOceani/rimworld-automatic-mod-translator (exe-аддон к RimTrans) | Инструментальный RU-запрос. Отвечает главный экран доков + GitHub |
| 16 | `RimWorld перевод мода нейросеть` / `ИИ` | T/I | растёт | нулевая | [обсуждение на DTF](https://dtf.ru) (про Noita, но та же боль), скрипты | RU-версия ниши #5. И там и там конкуренции практически нет |
| 17 | `перевод мода RimWorld сломался после обновления` | I (troubleshoot) | низкий | низкая | форумы, Steam | Боль после апдейта. Отвечает `update_translations` + troubleshooting |
| 18 | `RimWorld DefInjected перевод` / `Keyed файлы` | I | низкий | нулевая | Wiki (англ.), обрывки | RU-версия #6. Глоссарий на русском |

### 1.3 Выводы из выдачи

- **Серийные ответчики — форумы, wiki и Steam-гайды.** Ни один конкурент не имеет
  структурированной SEO-страницы с мета-данными; GitHub-аналоги (`RimTranslate` — 7 звёзд,
  `kelvinauta` — 4) имеют пустые topics и однострочные описания. Даже скромная
  метадата + лендинги выводят RimLoc в топ ниши.
- **Два разных человека за одним словом «перевод»**: игрок (хочет готовое) и переводчик
  (хочет инструмент). Каждая лендинг-страница обязана в первом абзаце развести эти ветки и
  дать игроку ссылку «где взять готовые переводы» (Steam Workshop коллекции, переводы авторов
  модов) — это не потеря, а доверие; часть игроков возвращается делать своё.
- **Термины, по которым нас ищут и не находят**: `DefInjected`, `Keyed`, `Languages/` папка,
  `_Imported.xml`, `translation mod`. Все должны присутствовать в заголовках/тексте
  соответствующих страниц естественно (они и так о них).

---

## 2. GitHub metadata

**Текущее состояние** (проверено `gh api`): description — только RU («RimLoc —
кроссплатформенный инструмент перевода модов для игры RimWorld»), topics — **отсутствуют**,
homepage — пусто, social preview — автовгенерённый GitHub-скриншот.

### 2.1 Описание репозитория (EN, каноническое, 263 символа)

```
Open-source workstation for RimWorld mod localization: extract, translate, validate and package translation mods. Keyed/DefInjected scanning, PO/XLIFF export, placeholder-safe QA, translation memory and AI-assisted translation. Windows, macOS, Linux. Rust + Tauri.
```

### 2.2 Описание репозитория (RU, альтернатива или для зеркал)

```
Открытая станция локализации RimWorld: извлечение строк из модов (Keyed/DefInjected), экспорт в PO/XLIFF, проверка плейсхолдеров, память переводов и ИИ-перевод. Готовые перевод-моды для Windows, macOS и Linux. Rust + Tauri, GPL-3.0.
```

Требования: ключевые слова естественно влиты (`RimWorld mod localization`, `translation`,
`DefInjected`, `PO`, `AI-assisted`), без перечисления через запятую, без эмодзи. Оба ≤350 симв.

### 2.3 Topics (проверено живое существование через `gh api search/repositories`)

Числа — сколько репозиториев на GitHub носят пару `topic:X topic:rimworld` / всего с topic:X
на момент проверки (2026-09-23):

| Topic | rimworld-пар | Всего | Зачем нам |
|---|---|---|---|
| `rimworld` | 1468 | 1468 | обязательный, главный якорь ниши |
| `rimworld-mod` | 387 | 529 | второй якорь, есть в выдаче GitHub-поиска |
| `rimworld-tools` | 1 | 1 | почти пустой, но точный; забираем бесплатно |
| `modding` | 813 | — | широкая аудитория моддеров |
| `localization` | 7 | 6906 | основная категория продукта |
| `translation` | 13 | 11465 | синоним-дубль по охвату |
| `translation-memory` | 0 | 58 | наша фича, ни у кого в нише нет |
| `game-localization` | 0 | 85 | уточнение домена |
| `i18n` | 0 | — | термин, по которому ищут разработчики |
| `gettext` | 0 | 419 | PO-формат, CAT-аудитория |
| `rust` | 6 | 127024 | язык реализации (аудитория Rust-разработчиков) |
| `tauri` | 1 | 13707 | GUI-стек |
| `svelte` | 0 | 17060 | планируемый GUI-фронтенд; добавить, когда Svelte реально появится в репо |
| `gui` | 0 | — | добавить вместе с релизом GUI |

Предлагаемый набор на сегодня (12): `rimworld`, `rimworld-mod`, `rimworld-tools`, `modding`,
`localization`, `translation`, `translation-memory`, `game-localization`, `i18n`, `gettext`,
`rust`, `tauri`. После релиза GUI добавить `gui` (+`svelte` по факту). GitHub лимит — 20.

Не добавлять: `cat-tools` (17 всего, шумно), `ai-translation`/`machine-translation` до
фактического появления AI-функций — topics обязаны не врать (см. Policy, §8).

### 2.4 Homepage URL и Social preview

- **Homepage:** `https://0-danielviktorovich-0.github.io/RimLoc/` — сайт уже жив и
  каноничен; homepage-поле единственное место, где GitHub показывает ссылку отдельной кнопкой.
- **Social preview** (1280×640 PNG): короткий вариант существующего баннера
  `docs/assets/RIMLOC-baner.png`: логотип/название + слоган «Translate RimWorld mods» +
  плашки Win/macOS/Linux. Проверка: превью читается в ленте 200px высотой. Заменить
  автовгенерённый скриншот: Settings → General → Social preview.

---

## 3. README strategy

Принцип: **человек первый, поисковик второй**. README уже хорош по фактам (быстрый старт,
таблица команд), но отвечает «что внутри», а не «зачем мне это» в первых 15 строках.

1. **Первый экран (до бейджей-свалки) отвечает на четыре вопроса:** что (workstation для
   локализации RimWorld), для кого (переводчики модов, авторы перевод-модов), где скачать
   (crates.io / релизы / soon GUI), как выглядит (скриншот GUI — заблокирован пометкой
   «after first public walkthrough» в README; до скриншота — баннер + одна строка-манифест).
2. **Task-oriented заголовки как якоря поиска** — переименовать/добавить секции:
   - `## Translate a RimWorld mod` → 3 команды (scan → export-po → build-mod)
   - `## Update an existing translation after a mod update` → diff-xml сценарий
   - `## AI-assisted translation` → текущее состояние (morph-провайдеры, планы), честно
   - `## Base game and DLC localization` → GAME_LOCALIZATION_SUPPORT.md кратко
   Эти заголовки дублируют будущие лендинги (§4) — GitHub README индексируется и часто
   ранжируется выше собственного сайта, поэтому task-заголовки нужны именно здесь.
3. **Без keyword-stuffing:** слова `translation`, `localization`, `RimWorld`, `mod` и так
   естественно составляют большую часть текста. Ничего не повторять искусственно; в
   альт-тексте баннера заменить пустой `alt="RimLoc banner"` на
   `alt="RimLoc — RimWorld mod translation workstation"`.
4. **Ссылка на docs-лендинги** в блоке Documentation — текстовыми анкорами совпадающими с
   intent («How to translate a RimWorld mod →», а не «см. туториал»).
5. Бейджи оставить после первого абзаца-манифеста, не перед ним.

---

## 4. Docs landing pages

По одной странице на intent-кластер, EN-канон (`docs/en/`) + RU-зеркало (`docs/ru/`).
Структура каждой: **H1 = формулировка запроса человека → однострочный ответ (сразу, без
прелюдий) → 3–7 шагов → что дальше (2–3 ссылки) → limitations честно.** Никаких «Введение»,
«О документе».

| Файл (предложение) | H1 / поисковая цель | Ядро ответа |
|---|---|---|
| `tutorials/translate_mod.md` (существует — усилить) | **How to translate a RimWorld mod** | scan → export-po → перевод в Poedit/OmegaT/чем угодно → import-po/build-mod. Отвечает и «русифицировать мод» |
| новая `guides/language_pack.md` | **How to create a RimWorld language pack** | Languages/Keyed/DefInjected анатомия (кратко, ссылка на learn.md) + init + build-mod; ветка «хотите готовый пак — вот коллекции сообщества» |
| `tutorials/update_translations.md` (существует — усилить) | **How to update a translation after a mod update** | diff-xml с baseline-po: что добавилось/сломалось, только новое переводить |
| новая `guides/ai_translation.md` | **How to use AI to translate RimWorld mods** | экспорт PO → любой LLM/CAT с MT → validate-po (плейсхолдеры!) → import. Честно про риски галлюцинаций терминов и глоссарий; отдельный блок про морфологию (morph) для русского |
| новая `guides/validation.md` | **How to validate a RimWorld translation** | validate/validate-po/xml-health: что ловим (дубли, пустые, плейсхолдеры) и почему игра молча ломается |
| `guide/translators.md` (существует) | **RimWorld localization workflow для CAT/переводчиков** | PO/XLIFF, TM, глоссарий терминов |

Правила: каждая страница имеет `description` во frontmatter (§5.1); перекрёстные ссылки
между лендингами и CLI-референсом; H1 не повторяет site_name; ничего не дублировать
дословно между страницами (каннибализация) — каждый лендинг отвечает только свой вопрос.

---

## 5. MkDocs metadata

Проверено по исходникам `mkdocs-material==9.7.7` и `mkdocs-static-i18n==1.3.1`
(версии из `requirements-docs.txt`) и по построенному `site/`. Главное: **почти всё нужное
включается конфигом, без кастомных подсистем**.

### 5.1 Что уже работает из коробки (проверено в построенном `site/`)

| Возможность | Статус | Механика |
|---|---|---|
| `meta description` per page | **работает** | встроенный в MkDocs `meta`-плагин читает `description:` из frontmatter; `base.html` 9.7.7: `page.meta.description` → `<meta name="description">`, fallback на `site_description` |
| Canonical URL | **работает** | `<link rel="canonical">` автоматически при заданном `site_url`; в CI (`docs.yml`) `SITE_URL` уже выставлен |
| rel="next"/"prev" | **работает** | Material отдаёт автоматически по порядку nav |
| Sitemap + hreflang | **работает** | `sitemap.xml(.gz)` строит MkDocs; **mkdocs-static-i18n 1.3.1 подменяет шаблон на i18n-версию** с `<xhtml:link rel="alternate" hreflang="en|ru">` на каждый URL (подтверждено в `site/sitemap.xml`). Условие: в `docs/overrides/` нет своего `sitemap.xml` — так и есть |
| `<html lang>` per locale | **работает** | `reconfigure_material: true` у i18n-плагина пересобирает Material под каждую локаль |
| Поиск по обеим локалям | работает | `search` + i18n |

### 5.2 Чего не хватает (и как включить конфигом)

1. **OG/Twitter-теги и социальные карточки.** В Material 9.7.7 `base.html` НЕ отдаёт
   `og:`/`twitter:` — их инжектит **встроенный плагин `social`** (проверено: теги из
   layout-шаблона `plugins/social/templates/default.yml` вставляются перед `</head>`,
   но только при заданном `site_url`). Требует extras: `pip install mkdocs-material[imaging]`
   (Pillow + cairosvg):

   ```yaml
   plugins:
     - social:
         cards: true
         cards_layout_options:
           background_color: "#24292f"   # под баннер
           font_family: Roboto
   ```

2. **robots.txt** — в `site/` отсутствует. MkDocs копирует статические файлы из `docs/`,
   поэтому достаточно файла `docs/robots.txt`:

   ```
   User-agent: *
   Allow: /

   Sitemap: https://0-danielviktorovich-0.github.io/RimLoc/sitemap.xml
   ```

3. **Внутренние доки утекают в публичный сайт.** `docs/development/*.md`
   (AUTONOMOUS_PLAN, SECURITY_AUDIT, COMPETITOR_MATRIX…) строятся в публичный
   `/development/` и попадают в публичный `sitemap.xml` с hreflang — то есть поисковикам
   предлагаются внутренние рабочие документы. Исправление в `mkdocs.yml`:

   ```yaml
   exclude_docs: |
     readme/**
     development/**
   ```

   (этот файл `DISCOVERABILITY.md` тоже внутренний — после включения правила он не
   публикуется, и это правильно). Если что-то из development всё же нужно публике —
   переносить в `docs/en/dev/` осознанно.

4. **`site_description` слишком общий.** «Toolkit for working with RimWorld translations» —
   нет ни RimWorld mod, ни ключевых поверхностей. Предложение:

   ```yaml
   site_description: RimLoc — open-source workstation for RimWorld mod localization: extract Keyed/DefInjected strings, translate with PO/XLIFF, validate placeholders, and build translation mods with AI assistance.
   ```

5. **Заголовки страниц.** Построенный `index.html` отдаёт `<title>RimLoc - RimLoc Docs</title>`
   — дублирование имени. Каждому лендингу — осмысленный `title`/H1 во frontmatter, напр.
   `title: How to translate a RimWorld mod`. Material подставит `{{ page.title }} - RimLoc Docs`.

6. **Alt-тексты** у изображений в доках — проверять при усилении страниц (сейчас баннер в
   README без осмысленного alt, в доках проверить `rg '!\[\](\)| ?\()' docs/`).

### 5.3 Чего НЕ делать

- Не тащить отдельный плагин redirect'ов, meta-генераторы и кастомные overrides-частичные
  шаблоны ради SEO: весь объём §5.1–5.2 закрывается родными механизмами.
- hreflang как `<link>` в `<head>` не нужен: Google официально рекомендует
  sitemap-variant (у нас уже так), дублирование в head избыточно.

---

## 6. Release notes

GitHub Releases индексируются и часто ранжируются по запросам вида
`RimWorld 1.6 translation tool update`. Правила:

- Заголовок релиза — описательный, не «v0.7.1»:
  `v0.7.1 — XLIFF export, RimWorld 1.6 support, faster scan (Windows, macOS, Linux)`.
- Первый абзац — человекочитаемое «что изменилось для переводчика», потом техдетали.
- В каждом релизе фиксировать совместимость: поддерживаемые версии RimWorld (1.4–1.6),
  платформы, минимальные требования GUI.
- release-plz уже генерирует CHANGELOG — PR-шаблон релиза дополняется человеком-заголовком
  по правилу выше (категории Keep-a-Changelog сохраняются).

---

## 7. Post-release measurement

План измерений после фазы docs hardening + первого релиза с лендингами.

| Что | Где | Как часто | Цель/сигнал |
|---|---|---|---|
| Organic search: клики/показы/запросы | Google Search Console (нужен верифицированный домен; для `*.github.io` свойства ограничены — кандидат на переезд сайта на свой домен при росте) | еженедельно первый месяц → ежемесячно | рост показов по кластерам §1; по каким запросам находимся |
| GitHub Traffic: views/clones, referrers, popular paths | Insights → Traffic | еженедельно | какие страницы README/доков ведут к клонам; top referrers (Steam? Reddit? Поиск?) |
| Внешний поиск-трафик на сайт | GitHub Pages не даёт аналитику → добавить lightweight-аналитику в docs (`extra.javascript`, Plausible/Umami self-hosted) при фазе hardening | еженедельно | входы по лендингам, конверсия «лендинг → crates.io/релизы» |
| Позиции по якорным запросам | ручная проверка топ-10 из §1 (приватный режим, раз в 2 недели) | раз в 2 недели | попадание в топ-10 по `RimWorld mod translator`, `how to translate a RimWorld mod`, `русификатор модов RimWorld` |
|[hreflang корректность] | Search Console → International targeting / валидатор | раз после настройки | en/ru пары без ошибок |
| Старовый бенчмарк | зафиксировать при внедрении: позиции, трафик, topics-покрытие аналогов | однократно | точка сравнения через 3 месяца |

Метрика успеха первого квартала: топ-10 Google по 3+ якорным запросам из §1 и измеримый
органический вход на лендинги (не только прямой/социальный).

---

## 8. Policy

SEO не жертвует ясностью, фактами и доверием. Конкретно:

1. **EN канонический, RU качественный.** RU-страницы — полноценный перевод человеком
   (или человеком проверенный), не машинная калька; непереведённые CLI-термины и команды
   остаются на английском. Массовый машинный перевод ради «покрыть больше языков» — запрещён:
   он убивает доверие ровно в той аудитории (переводчики), которую мы ищем.
2. **Никаких фейковых обещаний:** topics и description перечисляют только реализованное
   (`svelte`, `ai-translation` — после факта появления в репо; AI-лендинг описывает
   доступное сегодня + roadmap с пометкой).
3. **Честное сравнение с конкурентами** (RimTrans, RimTranslate): только проверяемые
   различия возможностей, без оценочных ярлыков; ссылка на них — уместна.
4. **Каждый лендинг отвечает на свой вопрос в первом экране** — если страница нужна только
   поисковику, её не должно быть.
5. **Развилка «готовый перевод vs сделать самому»** всегда в пользу игрока честно: даём
   ссылки на коллекции сообщества (RusPack и аналоги), а не прячем их.

---

## 9. Чеклист внедрения

Фаза docs hardening. Порядок по зависимости, не по приоритету.

**GitHub (10 мин, без кода)**
- [ ] Description EN (§2.1) в Settings → General
- [ ] Topics ×12 (§2.3)
- [ ] Homepage `https://0-danielviktorovich-0.github.io/RimLoc/`
- [ ] Social preview 1280×640 из баннера (§2.4)

**MkDocs (конфиг + 2 файла)**
- [ ] `site_description` обновить (§5.2.4)
- [ ] `exclude_docs: development/**` (§5.2.3) — закрыть утечку внутренних доков
- [ ] `docs/robots.txt` создать (§5.2.2)
- [ ] `pip install mkdocs-material[imaging]` в requirements-docs + плагин `social` (§5.2.1)
- [ ] Пересобрать и проверить руками: `meta description`, canonical, OG-теги в HTML,
      hreflang в sitemap, отсутствие `/development/` в билде
- [ ] title во frontmatter для index + лендингов (§5.2.5)

**Лендинги (контент, по одному интенту за итерацию)**
- [ ] `translate_mod.md` — усилить под «How to translate a RimWorld mod» (уже существует)
- [ ] `update_translations.md` — усилить под intent «после апдейта мода» (существует)
- [ ] `guides/language_pack.md` — новый (§4)
- [ ] `guides/ai_translation.md` — новый (§4)
- [ ] `guides/validation.md` — новый (§4)
- [ ] RU-зеркала каждой страницы (§8.1)
- [ ] README: task-заголовки + первый экран + alt баннера (§3)

**Релизы**
- [ ] Шаблон описательного заголовка релиза + блок совместимости (§6)

**Измерения**
- [ ] Зафиксировать стартовый бенчмарк позиций/трафика (§7)
- [ ] Добавить lightweight-аналитику в docs (по решению Даниэля)
- [ ] Кэлендарная ревизия через 3 месяца: Search Console + GitHub Traffic + ручные позиции
