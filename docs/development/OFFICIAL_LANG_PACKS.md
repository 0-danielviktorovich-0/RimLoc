# OFFICIAL LANG PACKS — что реально в официальных языковых репозиториях RimWorld

Дата: 2026-09-23 · Researcher-отчёт к [MULTILINGUAL_ARCHITECTURE.md](MULTILINGUAL_ARCHITECTURE.md)
Источники: shallow-клоны `Ludeon/RimWorld-ru`, `Ludeon/RimWorld-de`, `Ludeon/RimWorld-ja`,
`Ludeon/RimWorld-ChineseSimplified`, `Ludeon/RimWorld-Ukrainian` (ветка `master`/`main` на 2026-09-23,
клоны в `/tmp/RimWorld-*`). Каждый факт ниже привязан к пути в репозитории.

**Зачем это RimLoc:** валидировать LanguageCapabilities-модель реальными артефактами официальных
команд, а не нашими допущениями. Всё, что официальные команды делают в продакшене, RimLoc обязан
читать, валидировать и не ломать.

---

## §1. Структура языкового пака

Репозиторий = по одной папке на DLC/модуль: `Core/`, `Royalty/`, `Ideology/`, `Biotech/`,
`Anomaly/`, `Odyssey/` (у ru дополнительно `RimWorldUniverse/` — текстовые брифинги
`cryptosleep_revival_briefing_{orig,trans}.txt`). Внутри каждой DLC-папки:

| Путь | Что это | Кто имеет |
|---|---|---|
| `LanguageInfo.xml` | паспорт языка: `friendlyNameNative`, `friendlyNameEnglish`, `canBeTiny`, опционально `languageWorkerClass`, `credits` (список `CreditRecord_Role`/`CreditRecord_Text`/`CreditRecord_Space`) | все |
| `DefInjected/<DefType>/<SourceFile>.xml` | перевод полей дефов; структура папок и имён файлов **зеркалит** `Defs/` исходного мода (`DefInjected/ThingDef/Apparel_*.xml`, `DefInjected/IncidentDef/Incidents_Map_Disease.xml`), теги = `defName.fieldName` | все |
| `Keyed/<Группа>.xml` | перевод Keyed-строк, теги = ключи (`Keyed/Misc_Gameplay.xml`) | все |
| `Strings/Names/*.txt` | построчные списки имён по полу (`Names/Imperial_First_Female.txt`, `Names/Animal_Unisex.txt`, `Names/ColonyNames.txt`) | все |
| `WordInfo/` | грамматические таблицы (см. §2) | **только ru, de, uk** |
| `LangIcon.png` | иконка языка | все |
| `FriendlyName.txt` | короткое имя (у ja: `日本語`) | ja (у прочих не наблюдается) |

`canBeTiny=true` у всех пяти — ни один язык не «tiny» (не грузится как fallback-подмножество).

### languageWorkerClass

- **ru**: `<languageWorkerClass>LanguageWorker_Russian</languageWorkerClass>` +
  **исходник воркера лежит в корне репо**: `LanguageWorker_Russian.cs` (193 строки,
  `TotalNumCaseCount => 3`, `Pluralize` с fallback, `TryLookUp` в таблицы WordInfo с
  «срезанием хвоста» — ищет ключ по убыванию слов: «mace of steel (norm)» → «mace of steel» →
  «mace»).
- **de**: `<languageWorkerClass>LanguageWorker_German</languageWorkerClass>`, но **.cs в репо нет** —
  класс компилируется в сборку игры; репо только ссылается на него.
- **ja, zh, uk**: элемента `languageWorkerClass` в LanguageInfo.xml нет вовсе — базовый воркер игры.

**Вывод для RimLoc:** `languageWorkerClass` в LanguageInfo.xml — парсимое поле, но его отсутствие
не означает отсутствие морфологии: uk склоняется через `{lookup:}` без воркер-класса
(см. §3). Отражение класса в сборке игры (de) и в репо (ru) — разные модели сопровождения.

### Версионность

- **ru**: `master` = текущая игра (1.6), **теги на каждую версию** (`git tag`: `1.0…1.5`,
  исторические `a-15…b-19`), десятки рабочих веток PR-процесса (`1.6-keyed-3`, `backlog`,
  `automation-tools`, ветки по номерам задач `#1462-…`). README ведёт таблицу «какой архив для
  какой версии игры».
- **zh**: релизы-теги с точным билдом (`1.5.4409`, `1.4.3901`), ветки по мейнтейнерам
  (`AlyxMS`, `zorbathut`).
- **ja**: `master` + `wip`. **de**: только `master`. **uk**: `main`/`master` + `backstories-update`.
- Публикация в игру: Ludeon пакует перевод в `Data/<DLC>/Languages/<Имя>.tar`; README de и zh
  описывают установку свежих файлов «удали `<Язык>.tar`, положи папку `<Язык> (Нативное)`» —
  наличие папки рядом с tar заставляет игру читать папку.

---

## §2. WordInfo-воркфлоу

**WordInfo существует у ru, de, uk. У ja и zh папки нет** (ожидание подтвердилось) — CJK-языки
не склоняются, их воркеры в игре занимаются только переносом строк/регистром.

Формат у всех одинаковый по духу: UTF-8 текстовые CSV с разделителем `;`, строки-комментарии
`//`, пустые строки допустимы, группы строк помечены комментарием-заголовком. Но состав таблиц
и дисциплина сильно различаются.

### ru (`Core/WordInfo/` и в каждом DLC)

| Файл | Формат |
|---|---|
| `Case.txt` | 6 колонок `;`-разделитель: именительный + 5 падежей. Без BOM, строки начинаются сразу с кириллицы |
| `Plural.txt` | `ед.; мн.` с группами `// ThingDef`, `// OrderedTakeGroupDef` (источник — тип дефа) |
| `Gender/Male.txt`, `Female.txt`, `Neuter.txt`, `Plural.txt` | по одному слову в строке, группы `// BodyDef` |
| `Imperfect.txt` | пары `соверш.; несоверш.` (виды глагола) — **ru-уникальная таблица** |
| `SkillDef_subject.txt` | кастомная 3-колоночная таблица с дока-комментарием формата внутри файла |

### de (`Core/WordInfo/` и в каждом DLC) — самый «инженерный»

| Файл | Формат |
|---|---|
| `decline.txt` | CSV-**заголовок колонок** `NOM;1_GEN;2_DAT;3_ACC;4_NOM_DEF;5_GEN_DEF;6_DAT_DEF;7_ACC_DEF` (8 колонок: неопр./опр. артикль), BOM, блоки `//`-док автогенератора и `// DefInjected\PawnKindDef\* (.+\.label(Male|Female)?)` — комментарий-группа указывает **исходный путь и regex поля**. Есть строки с пустыми ячейками (`Bache;`) — непросклонённые, ждут ручного заполнения |
| `plural.txt` | заголовок `SINGULAR;PLURAL`, примечание «для склонения мн.ч. см. plural_decline.txt» |
| `plural_decline.txt` | склонение форм мн.ч. |
| `compound.txt` | `NOM;1_COM_PRE` — **de-уникально**: формы слова как префикса составного слова («Piraten» в «Piratengruppe») |
| `Gender/Male.txt`, `Female.txt`, `Neuter.txt`, `Other.txt`, `new_words.txt` | `new_words.txt` — очередь нераспознанных автоматически слов для ручного разбора |
| `capacity.txt`, `hediff.txt`, `ritual.txt` | кастомные таблицы `KEY;1_OUT` для точечных улучшений текста |

**Автоматизация de** (README «Grammar Resolving» + `.github/workflows/update-wordinfo.yml`):
каждый push в `master` запускает `update-wordinfo.ps1`, который гоняет
`update-wordinfo-{gender,decline,plural}.ps1` — скрипты сканируют DefInjected по regex, складывают
labels (lowercase) в Gender-файлы по родам, а нераспознанные — в `new_words.txt`, и пополняют
`decline.txt`/`plural.txt`. Скелет генератора — `utils.ps1` (`New-CommentBlock` шьёт
`// Usage syntax: {lookup: TEXT; FILE_NO_EXT; INDEX}` прямо в файл).

### uk (`Core/WordInfo/` и в DLC) — самый «человеческий»

| Файл | Формат |
|---|---|
| `case.txt` | 7 колонок `; ` (с пробелом и висячими `;`): 6 падежей + **кличный (вокатив)**; док-комментарии с примерами `{lookup: {термін}; Case; 3}`, список категорий. **CRLF, без BOM** |
| `rules.txt` | **12 колонок** (6 падежей × ед./мн.) для подстановки в `*.rulePack.rulesStrings` через `{lookup: [Animal]; Rules; 6-12}`; дока с нумерацией колонок в комментарии. **CRLF** |
| `skilldo.txt`, `weaponq.txt` | кастомные семантические таблицы (глагол к навыку; род качества) — самодеятельные расширения uk-команды |

Именование регистром нестабильно: ru — `Case.txt`/`Plural.txt`, uk — `case.txt` (нижний регистр),
de — `decline.txt`/`plural.txt`. Это работает в игре (резолюция таблицы по имени нечувствительна
к регистру на целевых платформах), но парсер RimLoc должен принимать оба регистра и не
канонизировать один.

**Автоматизация ru** — не генерация, а **CI-валидация** (`.github/workflows/main.yml` + 6
Python-скриптов): `check_utf-8.py` (требует UTF-8 **с BOM**!), `check_xml_format.py`,
`worldinfo_case.py` (валидация формата групп `//` в Case.txt), `check_pawn_gender.py`
(согласованность значений макроса `{PAWN_gender ? …}`), `check_report_string_dot.py`
(нет точки в конце reportString), `check_dash_format.py` (неразрывный пробел перед тире).

---

## §3. Мультиязычные различия → карта LanguageCapabilities

Сколько файлов DefInjected/Keyed реально используют каждый механизм (grep по ветке master, 2026-09-23):

| Язык | Worker | WordInfo | `{lookup:}` | `{replace:}` | `_numCase ?` | `{PAWN_gender ?}` | Особое |
|---|---|---|---|---|---|---|---|
| ru | в репо (`LanguageWorker_Russian.cs`) | да (6 падежей) | 110 файлов | 13 файлов | 42 файла | повсеместно (75+ вхождений `? рос : росла` в одном типе) | `Imperfect.txt`, `SkillDef_subject.txt` |
| de | вшит в игру | да (8-кол. склонение + compound) | 95 файлов | **132 файла** | нет | нет (род только через WordInfo) | `{replace:}` regex-замены («sind»→«ist» по числу), `compound.txt`, `new_words.txt` |
| uk | нет вообще | да (7 падежей + вокатив) | **127 файлов** | 11 файлов | нет | нет | `rules.txt` 12 колонок для rulePack; кастомные `skilldo`/`weaponq` |
| ja | вшит в игру | **нет** | 0 | 0 | 0 | 0 | переупорядочивание плейсхолдеров `{0}…{5}` в другом порядке; CJK-пунктуация (в одном файле и CJK-запятая, и ASCII-запятая: `ja/Core/Keyed/Dates.xml`) |
| zh | вшит в игру | **нет** | 0 | 0 | 0 | 0 | «人名翻译缺少接口» — имена персонажей без интерфейса перевода (README); CRLF в LanguageInfo.xml |

Живые примеры из репо:

- ru: `<Disease_OrganDecay.letterText>У {PAWN_labelShort} в {lookup: {2}; Case; 5} … в течение
  {3_numCase ? дня : дней : дней} …` (`ru/Core/DefInjected/IncidentDef/Incidents_Map_Disease.xml`) —
  lookup, numCase и глобальный placeholder в одной строке.
- de: `{replace: ^{0} deiner {lookup: {1}; plural_decline; 5} sind; …sind-"…"…ist; …}`
  (`de/Core/DefInjected/IncidentDef/Incidents_Map_Disease.xml`) — вложенный lookup внутрь replace.
- uk: `{0} потрапив до засідки {lookup: {1}; Case; 1} з фракції {2}!`
  (`uk/Core/DefInjected/IncidentDef/Incidents_Caravan_All.xml`).
- ja: `EN: Days passed… {0}\nCurrent quadrum: {4}\nLocal season: {2}…{3}…{5}` →
  перевод сохраняет те же токены, но **в порядке японского синтаксиса**
  (`ja/Core/Keyed/Dates.xml`).

**Следствия для LanguageCapabilities-модели RimLoc:**

1. Capabilities — **свойство target-языка**, задаются конфигом per-язык, не эвристикой:
   `wordinfo` (ru/de/uk), `inflection` (ru: 6; uk: 7 с вокативом; de: 4×2 артиклевых),
   `macro_lookup` (ru/uk/de), `macro_replace` (de прежде всего, но 13 файлов есть и в ru),
   `macro_numcase` (только ru), `macro_gender_conditional` (только ru), `verb_aspect` (только ru),
   `compound_prefix` (только de).
2. **`{lookup:}` ортогонален worker-классу**: uk использует его активнее всех без всякого
   воркера. Валидатор плейсхолдеров RimLoc обязан понимать `{lookup: …}`/`{replace: …}` для
   любого target, не только «склоняемых».
3. **`{replace:}` вложен в `{lookup:}`** (de) — валидатор не может быть линейным регэкспом,
   нужна рекурсивная грамматика макросов.
4. **Переупорядочивание плейсхолдеров — норма для любого языка** (ja/zh): проверка «множество
   токенов совпало, порядок свободен» вместо позиционного сравнения.
5. Worker-классы влияют на игру (плюрализация/регистр), но не на формат строк — RimLoc их
   не исполняет, максимум — метаданные из LanguageInfo.xml.

---

## §4. Инструменты команд переводчиков

| Команда | Основной инструмент | Автоматизация | Качество |
|---|---|---|---|
| ru | GitHub PR-процесс (десятки веток-задач `#NNNN-…`), wiki, чат Telegram + VK | CI `main.yml`: 6 Python-валидаторов на каждый PR; сторонний автоапдейтер установки для игроков (README) | жёсткие письменные правила: BOM, NBSP+тире, без точки в reportString, единые значения `{PAWN_gender}` |
| de | ручное редактирование + PowerShell | GitHub Actions `update-wordinfo.yml` **генерирует** WordInfo-таблицы из DefInjected при каждом push; `install.bat` для игроков | слова-сироты не теряются: `Gender/new_words.txt`; моды Rim Language Hot Reload / RimLiveTongue для просмотра в игре (README «Helpful links») |
| uk | **CrowdIn** (crowdin.com/project/rimworld-ukr) — «основне джерело перекладів» (README) | `Notes/scripts/download-translatef-files.py` — выкачивает архив из CrowdIn API v2 и распаковывает в репо (токен через env) | `Notes/known_issues.txt` — ручной трекинг MOCK-заглушек и непереведённых остатков EN |
| zh | Git + релизы; ссылка на RimTrans (github.com/duduluu/RimTrans — инструмент автора zh-мейнтейнера duduluu) как внешняя утилита | ветки мейнтейнеров, релизы-теги с билдами | README фиксирует терминологию (отказ от «环世界») и ограничения движка по именам |
| ja | минимум: README из 4 строк; ветка `wip` | нет | нет |

**RimTrans нигде не встроен в воркфлоу** — упомянут только в zh README как сторонний инструмент.
Официальные команды живут на: GitHub-PR (ru), генерация скриптами (de), CrowdIn (uk).

Для RimLoc: конкуренты — это в первую очередь «CSV-структура + генерация из DefInjected» (de) и
«CrowdIn-синхронизация» (uk); RimLoc-сценарий `morph`/WordInfo не имеет конкурирующего OSS-аналога
среди инструментов команд.

---

## §5. Формат файлов — реальные реалии (не догмы)

Проверено побайтово (`xxd`/`file`) на ветках master:

| Факт | Где |
|---|---|
| UTF-8 везде, но **BOM непоследователен даже внутри одного репо** | `ru/Core/LanguageInfo.xml` — с BOM; `de/Core/LanguageInfo.xml` — без BOM; `de/Core/WordInfo/*.txt` — с BOM; `ru/Core/WordInfo/Case.txt` — без BOM (начинается с кириллицы); `ru/Core/WordInfo/Plural.txt` — без BOM (начинается с `// `) |
| **ru-CI требует BOM** (`check_utf-8.py` ругает файлы без `ef bb bf`) — при том, что в чужих репо BOM не обязателен | `ru/.github/workflows/check_utf-8.py` |
| **CRLF живёт рядом с LF** | `zh/Core/LanguageInfo.xml` — CRLF; `uk/Core/WordInfo/case.txt`, `rules.txt` — CRLF; остальное LF |
| Отступы XML: 2 пробела — де-факто стандарт; ru LanguageInfo.xml — табы | все репо |
| EN-оригинал сохраняется **комментарием перед тегом**: `<!-- EN: Died on the {0} -->` | `Keyed/*.xml` и `DefInjected` всех репо |
| `\n` внутри значений — литеральная двухсимвольная последовательность | `Incidents_Map_Disease.xml` (ru/uk), `Dates.xml` (ja/zh) |
| CSV-разделитель `;`, но стиль разный: `a;b` (ru/de), `a; b;` (uk — с пробелом и висячим `;`) | WordInfo |
| Комментарий-группа в WordInfo может быть: типом дефа (`// ThingDef`, ru), исходным путём с regex (`// DefInjected\PawnKindDef\* (.+\.label(Male|Female)?)`, de), просто дока (uk) | — |
| Заголовок-строка колонок в CSV есть только у de (`NOM;1_GEN;…`) | `de/Core/WordInfo/decline.txt`, `plural.txt`, `compound.txt` |
| Пустые/неполные ячейки — норма (`Bache;` в decline.txt) | `de/Core/WordInfo/decline.txt` |

**Вывод:** парсер RimLoc должен принимать: BOM и отсутствие BOM; LF и CRLF; `;` с пробелом и без;
висячие разделители; пустые ячейки; комментарии `//` в любой позиции строки-начала; `<!-- EN: … -->`
как источник оригинала. Жёсткую BOM-политику (как ru-CI) RimLoc копировать не должен — она ломает
интероп с чужими паками.

---

## §6. Регрессионная ценность — фикстуры для тестов RimLoc

Кандидаты (клоны в `/tmp/RimWorld-*`, постоянные пути — добавить те же файлы в `test/` RimLoc):

1. **`/tmp/RimWorld-ru/Core/DefInjected/IncidentDef/Incidents_Map_Disease.xml`** — максимальная
   плотность ru-механизмов в одном файле: `{lookup: {2}; Case; 5}` + `{3_numCase ? дня : дней : дней}`
   + `{PAWN_labelShort}` + литеральные `\n` + EN-комментарии. Эталон для парсера макросов и
   coverage-подсчёта.
2. **`/tmp/RimWorld-de/Core/WordInfo/decline.txt`** — WordInfo de: BOM, CSV-заголовок колонок,
   8 колонок, автоген-доки, комментарии-пути с regex, **пустые ячейки** (`Bache;`) — тест
   толерантности парсера к неполным данным.
3. **`/tmp/RimWorld-ja/Core/Keyed/Dates.xml`** — переупорядоченные плейсхолдеры
   (`{0},{4},{2},{3},{5}`), CJK-текст, смешение CJK- и ASCII-пунктуации, `\n`-цепочки — тест
   «множество токенов сохранено, порядок свободен».
4. **`/tmp/RimWorld-zh/Core/Keyed/Dates.xml`** — тот же ключ `DateReadoutTip` в zh: тот же тест
   на кросс-языковую согласованность токенов (ru/de/ja/zh пары одного ключа) + CRLF-толерантность
   репозитория zh.
5. **`/tmp/RimWorld-de/Core/DefInjected/IncidentDef/Incidents_Map_Disease.xml`** — вложенные
   `{replace: … {lookup: …} …}`-цепочки с regex-литералами и кавычками — стресс-тест для
   рекурсивной грамматики макросов (сейчас это валит любой линейный регэксп).

Запасной: `/tmp/RimWorld-uk/Core/WordInfo/case.txt` (CRLF, 7 падежей с вокативом, `; ` с висячими
разделителями) — если нужна uk-фикстура или тест на «регистр имени таблицы».

---

## Итог (коротко)

**WordInfo имеют: ru, de, uk.** Не имеют: ja, zh.

**Уникальное по языкам для LanguageCapabilities:**
- ru — воркер-класс в репо; 6-падежная `Case.txt`; verb aspect (`Imperfect.txt`);
  `macro_numcase`; `macro_gender_conditional` (`{PAWN_gender ? …}`);
- de — 8-колоночное склонение с CSV-заголовками; compound-prefixes; `macro_replace`
  (132 файла, вложенность в lookup); автогенерация WordInfo скриптами + `new_words.txt`;
- uk — 7 падежей с вокативом без воркер-класса; 12-колоночный `rules.txt` для rulePack;
  CrowdIn-конвейер; самые «нестрогие» файлы (CRLF, `; `, кастомные таблицы);
- ja/zh — только базовые плейсхолдеры с переупорядочиванием; CJK-особенности;
  zh — известное ограничение движка по именам персонажей.

**5 фикстур** — список в §6.
