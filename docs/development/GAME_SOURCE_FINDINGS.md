# GAME_SOURCE_FINDINGS — семантика игры из декомпила установленной 1.6

Дата: 2026-09-23 · Владелец: researcher-сессия RC-кампании · Только для RimLoc.

**Как проверялось.** Первоисточник — *установленная* RimWorld GOG **1.6.4871 rev573**
(`Assembly-CSharp.dll`, build 1.6.9676.17433, Unity 2022.3.35f1):

1. Полная декомпиляция DLL через ILSpy (`ilspycmd 11.0`) в `/tmp/rw_decomp` — все цитируемые
   классы прочитаны из этого декомпила установленной сборки, не из веба.
2. Извлечение UTF-16 строковых литералов из того же DLL (диагностические сообщения и
   заголовки отчёта взяты дословно из бинаря).
3. Живые файлы официального языка: `Data/Core/Languages/Russian (Русский).tar` из установки
   (распакован в /tmp только на чтение).

Пометка источников ниже: `[1.6 · Namespace/File.cs#Method]` = декомпил установленной 1.6;
`[1.6 · DLL literal]` = строка из Assembly-CSharp.dll; `[install]` = файл из установленной игры.
`/Applications/RimWorld.app/Source/` — моддерский сабсет из 43 игровых файлов (JobDrivers,
ThingComps), **код загрузки/локализации там отсутствует** — подтверждено полным листингом.
Community-источники не понадобились: всё нашлось в декомпиле этой же установки.

---

## 1. LOAD SEMANTICS — эффективный вид мода

### 1.1 Состав активных модов

- Активный список и порядок — только `ModsConfig.xml` (`<activeMods>` + `<loadFolders>`-секция
  самого конфига), флагов `-mod=...` не существует `[install, см. testlab/research]`.
- `LoadedModManager.InitializeMods` идёт по `ModsConfig.ActiveModsInLoadOrder`, присваивая
  `loadOrder` по порядку; мод с несуществующим `RootDir` **само-деактивируется** с warning
  `Failed to find active mod ...` `[1.6 · Verse/LoadedModManager.cs#InitializeMods]`.
- Определение «мода» у игры шире, чем у RimLoc: мод, у которого есть хоть один файл под
  `Languages/` в любой content-папке, считается полноценным модом (`ModContentPack.AnyTranslationsLoaded`,
  `[1.6 · Verse/ModContentPack.cs#AnyTranslationsLoaded]`). Перевод-мод без Defs — валиден.

### 1.2 LoadFolders.xml — формат

Класс-парсер: `Verse.ModLoadFolders` (`LoadDataFromXmlCustom`), вызывается из `ModMetaData.Init`
через `DirectXmlLoader.ItemFromXmlFile<ModLoadFolders>` `[1.6 · Verse/ModMetaData.cs#Init]`.

- Корень `<loadFolders>`; версия — **имя тега-ребёнка**: `v1.6`, но `v` опционален и регистр
  приведён к нижнему (`childNode.Name.ToLower()`, префикс `v` срезается). Тег `<V1.6>` валиден.
  Внутри — только элементы `<li>`, текст = относительный путь; `XmlComment`-узлы пропускаются.
- `/` **или** `\` = корень мода (`LoadFolder` с пустым folderName). Прочие пути: при наличии
  альтернативного разделителя `\` заменяется на `/` (поведение платформенное).
- Три условных атрибута на `<li>` `[1.6 · Verse/ModLoadFolders.cs#LoadDataFromXmlCustom]`:
  - `IfModActive="pkg1,pkg2"` — грузить, если активен **хотя бы один** (`AnyModActiveNoSuffix`);
  - `IfModActiveAll="pkg1,pkg2"` — если активны **все** (`AllModsActiveNoSuffix`);
  - `IfModNotActive="pkg1"` — если **ни один** не активен.
  Значения — списки через запятую с trim; суффикс версии в packageId (`Author.Mod.1.5`)
  матчится без суффикса (NoSuffix-хелперы `Verse/ModLister.cs`).
- Спецтег `default` существует, но помечен игрой как устаревший
  (`ModLoadFolderDefaultDeprecated`, `[1.6 · DLL literal]`).

### 1.3 Выбор версии (ModContentPack.InitLoadFolders)

`[1.6 · Verse/ModContentPack.cs#InitLoadFolders]`, вызывается из конструктора ModContentPack:

1. Если LoadFolders.xml определил версии — попытки по порядку:
   a) точный тег `VersionControl.CurrentVersionString` = `Major.Minor.Build` (напр. `1.6.4871`);
   b) иначе — **наибольший** определённый тег ≤ текущей версии (теги без точки, пустые и
      `default` при этом пропускаются);
   c) иначе — `default`.
2. Если LoadFolders.xml нет/ничего не подошло — **конвенционный фолбэк**:
   - `RootDir/1.6` (точное совпадение с `CurrentVersionStringWithoutBuild`, т.е. `1.6`), иначе
     наибольший каталог, имя которого парсится как версия ≤ текущей;
   - плюс `RootDir/Common` (`ModContentPack.CommonFolderName == "Common"`);
   - плюс **корень всегда**.

Итог в `mod.foldersToLoadDescendingOrder` — единый список папок-контента для Defs/, Patches/,
Textures/, Assemblies/ **и Languages/**.

### 1.4 Порядок и приоритет (descending order)

`AddFolders` пишет папки **в обратном порядке** `[1.6 · Verse/ModContentPack.cs#AddFolders]`:

- из LoadFolders.xml: последний `<li>` имеет **высший** приоритет (первый в списке загрузки);
- конвенция: `1.6` → `Common` → корень (специфичное выше общего);
- корень грузится всегда, даже когда есть версионная папка — они **суммируются**, а не «или».

Дедуп по относительному пути — **первый загруженный выигрывает**, остальные молча
игнорируются, в трёх местах:

- XML-ассеты Defs/Patches: `DirectXmlLoader.XmlAssetsInModFolder` (словарь `TryAdd` по
  пути относительно папки) `[1.6 · Verse/DirectXmlLoader.cs#XmlAssetsInModFolder]`;
- файлы контента: `ModContentPack.GetAllFilesForMod` (та же схема) `[1.6]`;
- файлы перевода: `LoadedLanguage.TryRegisterFileIfNew` — per-mod `HashSet` относительных
  путей `[1.6 · Verse/LoadedLanguage.cs#TryRegisterFileIfNew]`.

Файлы, начинающиеся с `.` или `._` (macOS-мусор), пропускаются `[1.6 · DirectXmlLoader.cs]`.
Сортировки по алфавиту внутри папки нет — порядок файлов определяется файловой системой.

### 1.5 Конвейер загрузки (1.6)

`[1.6 · Verse/LoadedModManager.cs#LoadAllActiveMods]`, ровно в этом порядке:

1. `InitializeMods` — ModContentPack по активному списку (см. 1.1), у каждого уже вычислен
   `foldersToLoadDescendingOrder`.
2. `LoadModContent` — текстуры/аудио/strings/asset bundles/сборки; мод без контента получает
   ошибку со списком использованных папок (`did not load any content. Following load folders...`).
3. `CreateModClasses` — инстансы `Mod`.
4. `LoadModXML` — по модам в loadOrder, `Defs/` каждой content-папки.
5. `CombineIntoUnifiedXML` — **все** Defs-узлы всех модов складываются в ОДИН документ
   `<Defs>` (map узел→источник). Корень каждого файла обязан называться `Defs`, иначе ошибка.
6. **`TKeySystem.Parse(unifiedXml)`** — новое в 1.6: разбор `TKey`-атрибутов дефов до патчей.
7. `ErrorCheckPatches` — `Patches/` каждой папки: корень обязан быть `<Patch>`, дети
   `<Operation>`; иначе ошибки `Unexpected document element in patch XML`.
8. `ApplyPatches` — **все** патчи всех модов применяются к объединённому XML в порядке
   loadOrder модов. Т.е. патч применён к сырому XML **до** разрешения наследования и **до**
   создания дефов: содержимое `<Defs>` после патчей — это то, что станет дефами.
9. `ParseAndProcessXML` — регистрация наследования (`XmlInheritance.TryRegister/Resolve`),
   затем создание дефов по узлам (в 1.6 по умолчанию новый десериализатор
   `DirectXmlToObjectNew.DefFromNodeNew`; вернуть старый можно флагом
   `legacy-xml-deserializer`). Узлы с `MayRequire`/`MayRequireAnyOf` пропускаются, если мод не активен.
10. `ClearCachedPatches` → `patch.Complete` → `XmlInheritance.Clear`.

Дубликат defName: `DefDatabase` логирует `Adding duplicate <DefType> name: X`
`[1.6 · Verse/DefDatabase.cs#Add, literal из DLL]` — обе записи остаются в БД, что для
переводчика означает неоднозначность «кто настоящий» (рефлексия инжекции идёт по
`GenDefDatabase.GetDefSilentFail`, берёт первую регистрацию).

### 1.6 Регистр путей (macOS)

- `About/About.xml` и `LoadFolders.xml` резолвятся **регистронезависимо** явно
  (`GenFile.ResolveCaseInsensitiveFilePath`: сначала точное имя, потом перебор файлов папки
  с `CurrentCultureIgnoreCase`) `[1.6 · Verse/GenFile.cs#ResolveCaseInsensitiveFilePath]`.
- Всё остальное — точные строки путей; дедуп-словари сравнивают пути **с учётом регистра**
  (`Defs/X.xml` и `defs/x.xml` — разные ключи). На APFS с регистронезависимым форматом ОС
  найдёт файл, но дедуп может не сработать; на чувствительных к регистру томах такие пути
  просто не существуют. RimLoc'у достаточно воспроизводить точное сравнение.

### 1.7 Валидационные коды игры для LoadFolders

`ModLoadFolders.GetIssueList` `[1.6]` — используется в UI модов; литералы из DLL:
`ModLoadFolderListEmpty`, `ModLoadFolderRepeatingFolder`, `ModLoadFolderMalformedVersion`,
`ModLoadFolderDefaultDeprecated`, `ModLoadFolderOutOfOrder`, `ModLoadFolderDoesntExist`
(папка из `<li>` не существует на диске), `ModLoadFolderDefinesUnsupportedGameVersion`
(версия-тег не входит в `<supportedVersions>` мода). Плюс свободный текст
`More than one value for a same version of ...`.

---

## 2. TRANSLATION REPORT — категории и их точный смысл

### 2.0 Где и как генерируется

- Класс `Verse.LanguageReportGenerator`, единственный триггер — кнопка главного меню
  (`SaveTranslationReport`); **в 1.6 файл пишется на РАБОЧИЙ СТОЛ**
  (`Environment.SpecialFolder.Desktop`; фолбэк `GenFilePaths.SaveDataFolderPath` только если
  Desktop не определился) `[1.6 · Verse/LanguageReportGenerator.cs#DoSaveTranslationReport]`.
  ⚠ Поправка к `testlab/research/rimworld-isolated-run.md` §4/§7: в 1.6 отчёт НЕ в
  `Config/` песочницы, а на Desktop пользователя.
- Для английского (default) языка без ошибок скан отклоняется: `Please activate a
  non-English language to scan.`
- Оракул всех сравнений — `LanguageDatabase.defaultLanguage` = язык с folderName `English`
  (ванильные EN Keyed/DefInjected + `Languages/English` каждого мода). Оракул «missing»
  строится от EN, а не от кода игры.
- Счётчик в логе `Translation data for language X has N errors. Generate translation report
  for more info.` = `activeLanguage.loadErrors.Count` + Σ `defInjectionPackage.loadErrors`
  ПОСЛЕ второй фазы инжекции `[1.6 · Verse/LoadedLanguage.cs#InjectIntoData_AfterImpliedDefs]`.

Заголовок отчёта: `Translation report for <FriendlyNameEnglish>`; при использовании
legacy-синтаксиса `<rep>` добавляется совет перейти на прямые пути. Секции разделов:
`========== <Название> (<число>) ==========`. Полный список секций в порядке вывода:

| # | Заголовок секции (дословно из DLL 1.6) | Что проверяется |
|---|---|---|
| 1 | `General load errors` | `activeLanguage.loadErrors`: XML-ошибки Keyed/DefInjected-файлов, дубликаты keyed в файле, папки DefInjected не-деф-типов (`dir X doesn't correspond to any def type. Skipping...`), устаревшие `CodeLinked`/`DefLinked` (`Translations aren't called ... any more`), ошибки WordInfo и пр. |
| 2 | `Def-injected translations load errors` | Σ `DefInjectionPackage.loadErrors` всех пакетов: все ошибки резолва путей (см. 2.2). |
| 3 | `Missing keyed translations` | Для каждого ключа **EN**-keyed, которого нет в активном языке (`HaveTextForKey`, плейсхолдеры не считаются): `Key 'EN-значение' (English file: Файл:строка)`. Если в активном языке стоит `TODO` — приписка `(placeholder exists in файл)`. |
| 4 | `Def-injected translations missing` | Обход **всех загруженных дефов** рефлексией (`DefInjectionUtility.ForEachPossibleDefInjection`): строковые поля и `IEnumerable<string>`, для которых нет инжекции по нормализованному пути или стоит `TODO`. Формат `DefType: путь 'EN-значение'`; для списков хинт `(hint: this list allows full-list translation by using <li> nodes)`. |
| 5 | `Backstory translations missing` | Идентификаторы `BackstoryDef`, отсутствующие в legacy-файле бэкстори (только если legacy-файлы вообще есть). |
| 6 | `Unnecessary def-injected translations (marked as NoTranslate)` | Инжекции на поля с `[NoTranslate]`/`[Unsaved]`/`[MayTranslate]` или без права перевода — заполняется тем же обходом, что №4. |
| 7 | `Def-injected translations using old, renamed defs (fixed automatically but can break in the next RimWorld version)` | Инжекции, где `path != nonBackCompatiblePath` (BackCompatibility переименовал def/путь): `Def has been renamed: A -> B, translation ... should be renamed as well.` или `Translation X should be renamed to Y`. |
| 8 | `Argument count mismatches (may or may not be incorrect)` | Для ключей, существующих в EN и в активном языке (не-плейсхолдерах): сравнение символов `{...}` (`SameSimpleGrammarResolverSymbols`). Проверяется только **вложение**: каждый EN-символ должен присутствовать в переводе; лишние символы перевода НЕ репортятся. |
| 9 | `Unnecessary keyed translations (will never be used)` | Ключи активного языка, которых нет в EN-keyed. |
| 10 | `Keyed translations matching English (maybe ok)` | Перевод == EN-значение (не-плейсхолдер). |
| 11 | `Backstory translations matching English (maybe ok)` | То же для legacy-бэкстори. |
| 12 | `Backstories translation using obsolete format (def injection is now enabled for backstories)` | Все записи legacy `Backstories/Backstories.xml` (формат устарел, бэкстори теперь DefInjected/BackstoryDef). |
| 13 | `Def-injected translations syntax suggestions` | `Consider using <suggested> instead of <current> for translation '...'`: игра предлагает канонический путь (списочные хэндлы вида `li-N`, TKey-пути). Секция выводится только если не пуста. |
| 14 | `TKey system errors` | `TKeySystem.loadErrors` — дубликаты TKey-путей (`Duplicate TKey: ...`) и др.; только если не пусто. |

Каждая секция обёрнута в try/catch и при падении пишет в лог
`Error while generating translation report (<категория>): ...` — отчёт не умирает целиком.

### 2.1 Загрузка Keyed

`[1.6 · Verse/LoadedLanguage.cs#LoadFromFile_Keyed]`, формат `<LanguageData><Key>значение</Key>`:

- Файлы: `Keyed/**/*.xml` (рекурсивно) из **каждой** языковой папки каждого мода.
- **Дубликат в одном файле** = ошибка `Duplicate keyed translation key: X in language Y`,
  берётся первый.
- **Дубликат между файлами/папками/модами** — ошибки нет: `keyedReplacements.SetOrAdd`,
  побеждает **последний загруженный** (моды в loadOrder, папки в descending, файлы в порядке ФС).
  Это асимметрия с Defs (там побеждает первый загруженный файл) — легко перепутать.
- Значение ровно `TODO` → `isPlaceholder = true`, значение считается пустым; `HaveTextForKey`
  без `allowPlaceholders` возвращает false (т.е. TODO = отсутствие перевода).
- Источник для отчёта: `Файл:строка` (`GetKeySourceFileAndLine`).
- Парсер — `DirectXmlLoaderSimple.ValuesFromXmlFile` (плоские key→value, номер строки).

### 2.2 Загрузка и резолв DefInjected

Папка: `DefInjected/<DefTypeName>/**/*.xml`; имя подпапки резолвится как тип:
`GenTypes.GetTypeInAnyAssembly(name)`, при неудаче и длине >3 — повтор с обрезанным
последним символом (для legacy-суффиксов). Неизвестная папка → loadError и пропуск
`[1.6 · Verse/LoadedLanguage.cs#LoadData]`. После всех файлов заводятся пустые пакеты для
**всех** деф-типов (`EnsureAllDefTypesHaveDefInjectionPackage`) — отчёт «missing» полон.

Форматы ключей (`[1.6 · Verse/DefInjectionPackage.cs#AddDataFromFile]`):

- `Путь.КПолю` — прямой элемент: `<ThingDef.X.label>…</…>`.
- Списки целиком: `<ThingDef.X.myList><li>…</li><li>…</li></…>` (full-list injection);
  дети не-`li` → ошибка `... has elements which are not 'li'`. XML-комментарии внутри списка
  сохраняются игрой (fullListInjectionComments) — комментарии к элементам легальны.
- Индексы: `.3` или `[3]` — скобки нормализуются (`[`→`.`, `]` вырезается).
- Списочные хэндлы: `.SomeHandle` / `.SomeHandle-2` — резолвятся в индекс через
  `TranslationHandleUtility`; в подсказках (секция 13) игра сама предлагает хэндл-форму.
- `.slateRef`-суффикс — легальный ключ.
- `<rep><path>…</path><trans>…</trans></rep>` — legacy, помечает `usedOldRepSyntax` (совет в шапке отчёта).
- `TODO` → плейсхолдер (инжекция не выполняется, но missing не репортится).
- Первая секция пути (defName) прогоняется через `BackCompatibility.BackCompatibleDefName`;
  спецкейс: у ConceptDef `.helpTexts.0` авто-переименовывается в `.helpText`.

Ошибки резолва (все — литералы DLL 1.6, попадают в секцию 2):

- `Key lacks a dot: X` — в ключе нет ни одной точки.
- `Duplicate def-injected translation key: X` — дубликат ключа в пакете (перезапись `SetOrAdd`,
  т.е. берётся последний; для full-list дубликат `Dictionary.Add` бросит и уйдёт в catch файла).
- `Duplicate def-injected translation key. Both A and B refer to the same field (suggestedPath)`
  — **между файлами**: два разных пути нормализовались в один (ловится после инжекции по
  `normalizedPath`).
- `Replacing the whole list and individual elements at the same time doesn't make sense...`
- `Found no <DefType> named X to match <path> (файл)` — несуществующий def (только во второй
  фазе; см. ниже).
- `Field X does not exist in type Y.` — несуществующее поле.
- `Translated untranslatable field ... [NoTranslate] ... will break the game.`
- `Translated untranslatable field (UnsavedAttribute) ...`
- `Translated non-string field ...` / `Translated non-List<string> ...` (несовпадение типа);
  для full-list без `[TranslationCanChangeCount]` — отдельная ошибка про атрибут.
- `Trying to translate X at index N but the list only has M entries (so max index is M-1).`
- `Couldn't inject <path> into <DefType> (файл): <причина>` — общий catch.

Инжекция двухфазная `[1.6 · Verse/PlayDataLoader.cs]`: `InjectIntoData_BeforeImpliedDefs`
(до генерации implied-дефов, `errorOnDefNotFound=false`) → резолв ссылок и генерация implied →
`InjectIntoData_AfterImpliedDefs` (`errorOnDefNotFound=true`; ошибки фазы 1 сбрасываются —
`InjectIntoDefs` начинается с `loadErrors.Clear()`, применённое не переapply'ивается).

### 2.3 Что считается «missing» в DefInjected (важно для паритета с отчётом)

`[1.6 · Verse/DefInjectionUtility.cs#ShouldCheckMissingInjection]` — поле попадает в секцию 4
отчёта только если:

1. деф не `generated` (implied/generated дефы не сканируются);
2. значение не пустое;
3. поле без `[NoTranslate]`/`[Unsaved]`/`[MayTranslate]`;
4. есть `[MustTranslate]` → всегда; есть `[MustTranslate_SlateRef]` → по правилу SlateRef;
   **иначе — только если строка содержит пробел** (`str.Contains(' ')`).
   Однословные лейблы (`Spine`, `Steel`) в missing НЕ попадают без MustTranslate.
5. `Def`-ссылки и вложения рекурсия пропускает; посещённое не обходит дважды.

Порядок полей детерминирован: Unsaved/NoTranslate первыми, затем `label`, `description`,
далее по имени (`FieldsInDeterministicOrder`) — порядок отчёта стабилен.

### 2.4 Аргументы/плейсхолдеры

- Игра проверяет **только количество символов `{...}`** для keyed (секция 8 отчёта) и то
  «в одну сторону» (все EN-символы есть в переводе). Для DefInjected проверок аргументов нет.
- Распознавание символа: `{Имя...}` с закрывающей `}`; суффиксы после `_` (именованный
  аргумент) и `?` (опциональность) отбрасываются — `{name_possessive?}` == `{name}`.
- Runtime-разрешение: `GrammarResolver` + функции в самих строках: `{lookup|ключ|таблица[|индекс]}`
  и `{replace|текст|"старое"-"новое"}` (`[1.6 · Verse/LanguageWorker.cs#ResolveFunction]`).
  Ошибки `lookup` (`Invalid argument number for 'lookup' function, expected 2 or 3...`)
  логируются в рантайме, в отчёт не попадают.

### 2.5 Backstories

- Новый формат — обычный DefInjected: `DefInjected/BackstoryDef/*.xml` (деф-тип существует).
- Legacy: `Languages/<lang>/Backstories/Backstories.xml` — элементы по идентификатору
  backstory, поля `title`, `titleFemale`, `titleShort`, `titleShortFemale`, `desc`
  `[1.6 · Verse/BackstoryTranslationUtility.cs]`. Ошибки: `Backstory not found matching
  identifier X`, `Backstory translation exactly matches default data: X`,
  `Translation doesn't correspond to any backstory: X`, `Obsolete backstory format: X`.
  Legacy попадает в секции 5/11/12 отчёта.

### 2.6 Strings (имена/слова)

`Strings/<Words|Names|WordParts>/**/*.txt` — плоские текстовые списки; дублирующийся
относительный путь между файлами **дополняет** существующий список
(`LoadFromFile_Strings`) `[1.6 · Verse/LoadedLanguage.cs]`. В txt-файлах `//`-комментарии
вырезаются (см. 3.3).

---

## 3. WordInfo и LanguageWorker

### 3.1 Выбор воркера

`LoadedLanguage.Worker` создаётся лениво из `info.languageWorkerClass`
`[1.6 · Verse/LoadedLanguage.cs#Worker]`; по умолчанию `LanguageWorker_Default`. Класс
задаётся в `LanguageInfo.xml` простым именем типа (резолв `GenTypes.GetTypeInAnyAssembly`).
Установленный русский `[install: LanguageInfo.xml из Russian (Русский).tar]`:
`<languageWorkerClass>LanguageWorker_Russian</languageWorkerClass>`,
`friendlyNameNative: Русский`, `friendlyNameEnglish: Russian / Русский`.

### 3.2 Где воркер востребован переводчиком

`LanguageWorker` (база) — статьи (`IndefiniteForm`/`DefiniteForm` keyed-хуки!), `Pluralize`,
`OrdinalNumber`, `ResolveNumCase` (форма «1/2-4/5+» по `num % 10`, исключение `10-19`),
`PostProcessed`. Русский: `TotalNumCaseCount = 3`, `ToTitleCase` = `GenText.ToTitleCaseSmart`,
`Pluralize` — сначала таблица `Plural`/`Case`, затем fallback-правила по роду и последним
буквам (хардкод в `LanguageWorker_Russian.PluralizeFallback` — включая исключения после
Г/К/Х/Ж/Ч/Ш/Щ/Ц). Следствие для RimLoc: род русского слова прямо влияет на согласование —
валидность `WordInfo/Gender/*.txt` критична.

### 3.3 Формат WordInfo

Каталог `WordInfo/` в корне языковой папки. Загрузка — `LanguageWordInfo.LoadFrom` +
ленивые LUT `[1.6 · Verse/LanguageWordInfo.cs]`.

**Род** — ровно три файла `[1.6 · LanguageWordInfo.LoadFrom]`:

- `WordInfo/Gender/Male.txt` → Gender.Male
- `WordInfo/Gender/Female.txt` → Gender.Female
- `WordInfo/Gender/Neuter.txt` → **Gender.None** (не отдельный enum! нейтральный род в игре
  кодируется как None)

Формат: одно слово/фраза на строку (без `;`), сравнение в нижнем регистре, первый вхождение
побеждает (`genders.ContainsKey` guard), дедуп файлов per-mod как у переводов. Пример из
установленного русского `[install]`:

```
// BodyDef
левый глаз
правый глаз
мизинец левой руки
```

`//`-комментарии и инлайновые `//` вырезаются на уровне `GenText.LinesFromString`
`[1.6 · Verse/GenText.cs#LinesFromString]` — актуально для **всех** `.txt` языковых файлов
(WordInfo, Strings).

**Lookup-таблицы** (`Case.txt`, `Plural.txt`, любые `<Имя>.txt`):

- Строка: `ключ;форма1;форма2;...` — разделитель `;`, ключ = нулевое поле в нижнем регистре;
  строки, где нет `;`, дают `Failed parsing lookup items from line ... in <файл>`.
  Дубликат ключа: первый побеждает, ошибки нет.
- Таблицы собираются из **всех** языковых папок активного языка всех модов (merge, не дедуп
  файлов) и регистрируются лениво по имени (`GetLookupTable` → `RegisterLut`), имя таблицы
  сравнивается в нижнем регистре; повторная регистрация имени = `Tried registering language
  look up table named X twice.`
- Пример `Plural.txt` `[install]`: `// ThingDef` + `пояс-щит; пояса-щиты`.
  Пример `Case.txt` `[install]`: 6 падежных форм через `;`
  `левое лёгкое; левого лёгкого; левому лёгкому; левое лёгкое; левым лёгким; левом лёгком`.
- Поиск: базовый `LanguageWorker.TryLookUp` — точный ключ (lowercase), повтор с вырезанными
  скобками. Русский `TryLookUp` — progressively-longer **фразовый** префикс: пробует
  совпадение по границам пробелов от конца («мизинец левой руки» целиком → «мизинец левой» →
  «мизинец»), найденная форма конкатенируется с несматченным хвостом.
- `Pluralize` берёт `форма1` из `Plural.txt` и капитализирует, если исходник с заглавной.

### 3.4 Выбор языка

`[1.6 · Verse/LanguageDatabase.cs#InitAllMetadata]`: языки регистрируются по именам папок
`Languages/*` всех модов; `folderName` vs `legacyFolderName` (имя до ` (` — так
`Russian (Русский)` и `Russian` — один язык). Активный язык = **точное** совпадение
`Prefs.LangFolderName` с folderName; нет — фолбэк на `English`. Официальные языки (27 имён в
`SupportedAutoSelectLanguages`) поставляются как **tar-архивы**
`Data/Core/Languages/<Имя>.tar` и читаются через виртуальную ФС `RimWorld.IO.TarDirectory` —
русский сейчас first-party, отдельный Workshop-пак для него не нужен.

---

## 4. Пять правил load semantics, которых RimLoc сейчас не учитывает

Сверено с `crates/rimloc-services/src/modview.rs`, `crates/rimloc-cli/src/version.rs`,
`crates/rimloc-services/src/scan.rs`, `crates/rimloc-validate/src/lib.rs` (2026-09-23).

1. **Languages/ существует в каждой content-папке, не только в корне.**
   `modview.rs` жёстко фиксирует `languages_dir = root/Languages`, но игра собирает
   `Languages/<lang>` из **всех** элементов `foldersToLoadDescendingOrder` — включая `1.6/`,
   `Common/` и условные папки (`LoadedLanguage.AllDirectories`). Мод с
   `1.6/Languages/Russian/...` сегодня RimLoc молча не сканирует. Фикс: сделать
   `languages_dirs` списком по content-папкам эффективного вида.

2. **Порядок папок — descending, с дедупом по относительному пути (первый выигрывает).**
   Игра разворачивает `<li>` (последний = приоритетный), добавляет `1.6 → Common → root`, и
   во всех загрузчиках первый встреченный относительный путь вытесняет остальные
   (`TryAdd`/`TryRegisterFileIfNew`). RimLoc сохраняет порядок `<li>` как есть и не дедуплицирует
   → при `Defs/X.xml` и в корне, и в `1.6/` извлекутся обе копии, а при перекрытии
   переводов/ключей победитель определится иначе, чем в игре. Для Keyed игра, наоборот,
   перезаписывает (последний выигрывает) — семантику перекрытий надо моделировать по типу
   контента: Defs — first-wins, Keyed — last-wins.

3. **Конвенционный фолбэк — это сумма папок, а не выбор одной.**
   Без LoadFolders.xml игра грузит `1.6/` **И** `Common/` **И** корень одновременно; RimLoc
   выбирает ровно один каталог (версию ИЛИ корень) и `Common/` игнорирует совсем. Плюс
   парсер в modview.rs не принимает: регистр тега (`<V1.6>`), `IfModActiveAll`,
   `IfModNotActive`, списки packageId через запятую, тег `default`, фолбэк «наибольший тег ≤
   запрошенной версии» (RimLoc берёт просто максимум). На кейсе-чувствительных томах ещё и
   чтение `LoadFolders.xml` нужно делать регистронезависимым (игра так и делает).

4. **Схемные правила DefInjected: TODO-плейсхолдеры и однословная эвристика missing.**
   Игра считает `TODO` отсутствующим переводом (и в keyed, и в DefInjected), а в отчёт
   «missing» добавляет строку только если она содержит пробел либо поле помечено
   `[MustTranslate]`; generated-дефы не сканируются; поля `[NoTranslate]`/`[Unsaved]` дают
   отдельную категорию «unnecessary». Без учёта этого `coverage`/`validate` RimLoc не
   совпадёт с числами `TranslationReport.txt` (иногда в выгодную сторону — однословные
   EN-лейблы RimLoc переведёт, а игра их missing'ом даже не посчитает). Минимальный шаг —
   распознавать `TODO`, список атрибутных полей вести как данные, сгенерированные из отчёта
   игры по версии.

5. **Валидация путей DefInjected без деф-схемы ловит только часть ошибок игры.**
   Игра валидирует: существование defName (после BackCompatible-переименований), тип поля,
   границы индексов списков, конфликт «full-list + элементы», кросс-файловую коллизию по
   нормализованному пути, TKey-нормализацию, `[i]`-скобки, списочные хэндлы `handle-N`,
   `.slateRef`. RimLoc-валидатор ограничен плейсхолдерами и дубликатами ключей — т.е.
   «зелёный» RimLoc-прогон не означает чистый игровой лог. Дешёвый частичный паритет:
   нормализовывать пути как игра (скобки→точки, `helpTexts.0`→`helpText` для ConceptDef),
   проверять «ключ без точки» и дубликаты, а полный оракул брать из реального
   `TranslationReport.txt` (кнопка в UI; в 1.6 он падает на **Desktop**, не в Config —
   поправить процедуру в `testlab/research/rimworld-isolated-run.md`).

Бонус-наблюдение для acceptance: счётчик в логе (`has N errors`) = `loadErrors` языка +
ошибки всех DefInjection-пакетов после второй фазы инжекции — это стабильный grep-маркер для
автотеста (уже используется в isolated-run), но сравнивать с ним coverage RimLoc можно только
после правил 1–4.

---

## 5. Быстрые факты для контекста

- Явные устаревшие имена: `CodeLinked` → `Keyed`, `DefLinked` → `DefInjected`
  (ошибка загрузки), `Backstories DELETE_ME` — legacy-константа бэкстори `[1.6]`.
- `Languages/<lang>/LangIcon.png` — иконка; `FriendlyName.txt` — альтернатива
  `LanguageInfo.xml/friendlyNameNative` `[1.6 · LoadedLanguage]`.
- Ключевые литералы лога для acceptance-грепа (все подтверждены в DLL 1.6):
  `Exception loading translation data from file`, `Error loading DefInjection from file`,
  `Duplicate keyed translation key:`, `Duplicate def-injected translation key:`,
  `Critical error while injecting translations into defs:`,
  `Exception loading language data. Rethrowing. Exception:`,
  `has ... errors. Generate translation report for more info.`,
  `Found no ... to match ...`, `Adding duplicate ... name:`.
