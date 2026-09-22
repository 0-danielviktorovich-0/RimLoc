# Запуск RimWorld в изолированном disposable-профиле на macOS

QA-research для acceptance-тестирования собранных RimLoc переводов. Только чтение: игра не запускалась, `/Applications/RimWorld.app` и продакшн-профиль не изменялись.

Дата: 2026-09-23 · Стенд: macOS (Apple M4 Pro), RimWorld GOG 1.6.

## 0. Установленный стенд (проверено локально)

| Факт | Значение | Источник |
|---|---|---|
| Версия игры | `1.6.4871 rev573` | `~/Library/Application Support/RimWorld/Config/ModsConfig.xml`, `<version>` |
| Сборка assembly | `1.6.9676.17433` | строка версии в `Assembly-CSharp.dll` (UTF-16 литерал) |
| Unity player | 2022.3.35f1 (011206c7a712) | `Info.plist`: `CFBundleGetInfoString` |
| Bundle id | `ludeon.rimworld` | `Info.plist`: `CFBundleIdentifier` |
| Исполняемый файл | `/Applications/RimWorld.app/Contents/MacOS/RimWorld by Ludeon Studios` | `ls Contents/MacOS/` |
| Контент DLC | `/Applications/RimWorld.app/Data/{Core,Royalty,Ideology,Biotech,Anomaly,Odyssey}` | `ls /Applications/RimWorld.app/Data/` |
| Локальные моды | `/Applications/RimWorld.app/Mods/` (внутри .app) | `ls`; подтверждено логом Player.log: `CameraPlus: Found mod at /Applications/RimWorld.app/Mods/867467808` |

`/Applications/RimWorld.app/Source/` — частичный моддерский сабсет (43 `.cs`: ThingComps, JobDrivers). Код командной строки там **отсутствует** (grep по `savedatafolder|logfile|CommandLine` — пусто). Первоисточник флагов — скомпилированный `Assembly-CSharp.dll` (проверено извлечением строковых литералов) и публичные декомпилы (см. ниже).

## 1. Где RimWorld хранит config/saves/prefs на macOS

**Корень данных (Save data folder): `~/Library/Application Support/RimWorld/`** — фактическое содержимое этой машины:

```
~/Library/Application Support/RimWorld/
├── Config/            ← ModsConfig.xml, Prefs.xml, KeyPrefs.xml, Knowledge.xml,
│                        LastPlayedVersion.txt, настройки модов (Mod_*.xml),
│                        TranslationReport.txt (создаётся по запросу)
├── Saves/
├── HugsLib/  CameraPlus/  MissileGirl/   ← рабочие папки модов
```

Пользовательской папки `Mods` здесь **нет и не создаётся** (`ls` — отсутствует).

Механизм из декомпила (`Verse/GenFilePaths.cs`, [josh-m/RW-Decompile](https://github.com/josh-m/RW-Decompile)):

```csharp
public static string SaveDataFolderPath {
    get {
        if (GenFilePaths.saveDataPath == null) {
            string text;
            if (GenCommandLine.TryGetCommandLineArg("savedatafolder", out text)) { ... }
            else if (UnityData.platform == RuntimePlatform.OSXPlayer ...) {
                DirectoryInfo parent = Directory.GetParent(UnityData.persistentDataPath);
                string path = Path.Combine(parent.ToString(), "RimWorld");
                if (!Directory.Exists(path)) Directory.CreateDirectory(path);
                GenFilePaths.saveDataPath = path;
            }
            ...
```

То есть на macOS дефолт = `GetParent(persistentDataPath) + "/RimWorld"`, что совпадает с наблюдаемым `~/Library/Application Support/RimWorld`.

Содержимое `Config/` (реальные файлы, прочитаны только):

- `ModsConfig.xml` — `<version>`, `<activeMods><li>packageId</li>…</activeMods>` и `<loadFolders>`-секция загрузочного порядка. Это единственный рычаг управления составом модов.
- `Prefs.xml` — среди прочего `langFolderName` (на этой машине `Russian (Русский)`), `logVerbose`, `devMode`, `pauseOnError`. Язык можно выставить в песочнице **до** запуска, редактированием этого файла.

Артефакты Unity **вне** save data (важно для teardown):

| Путь | Что там |
|---|---|
| `~/Library/Logs/Ludeon Studios/RimWorld by Ludeon Studios/Player.log` (+ `Player-prev.log`) | главный лог, путь наблюдается на этой машине |
| `~/Library/Preferences/ludeon.rimworld.plist` | Unity PlayerPrefs — только настройки экрана (`Screenmanager *`, `unity.player_session*`). Проверено `plutil -p` |
| `~/Library/Application Support/ludeon.rimworld/` | пустая папка по bundle id (создаёт Unity/crash reporter) |

## 2. CLI-флаги

### 2.1 Механика разбора аргументов (первоисточник — декомпил `Verse/GenCommandLine.cs`, [josh-m/RW-Decompile](https://github.com/josh-m/RW-Decompile))

```csharp
public static bool CommandLineArgPassed(string key) {
    // match: arg == key  ИЛИ  arg == "-" + key   (без учёта регистра)
}
public static bool TryGetCommandLineArg(string key, out string value) {
    // match ТОЛЬКО форма key=value / -key=value  (split по '=', ровно 2 части)
}
```

Классы `GenCommandLine`, `SaveDataFolderCommand`, литералы `savedatafolder`, `quicktest`, `autoload`, `autostart`, `autohack`, `ModsConfig.xml` подтверждены извлечением UTF-16-литералов из `/Applications/RimWorld.app/Contents/Resources/Data/Managed/Assembly-CSharp.dll` этой установки (1.6).

При старте игра пишет аргументы в лог — `Verse/Root.cs` (декомпил [RimWorld-zh/RimWorld-Decompile](https://github.com/RimWorld-zh/RimWorld-Decompile), строки 91–94):

```csharp
Log.Message("Command line arguments: " + GenText.ToSpaceList(commandLineArgs.Skip(1)), false);
```

### 2.2 Подтверждённые RimWorld-флаги

| Флаг | Эффект | Доказательство |
|---|---|---|
| `-savedatafolder=<path>` (также `savedatafolder=<path>`) | полностью переопределяет корень данных (Config, Saves) | `GenFilePaths.SaveDataFolderPath` (декомпил, цитата в §1); литерал в DLL 1.6 |
| `quicktest` / `-quicktest` | с главного меню автоматически грузит сцену `Play` (вход в игру без кликов) | `Verse/QuickStarter.cs`: `if (GenCommandLine.CommandLineArgPassed("quicktest") && !quickStarted && GenScene.InEntryScene) { quickStarted = true; SceneManager.LoadScene("Play"); }` |
| `autoload`, `autostart`, `autohack` | литералы присутствуют в DLL 1.6; места использования в доступных декомпилах не найдены — считаем недокументированными, в тестах **не используем** | извлечение литералов Assembly-CSharp.dll |

Флагов вида `-mod=...` / `-modlist=...` **не существует** (нет в литералах DLL): состав модов управляется только `ModsConfig.xml`.

### 2.3 Unity player-флаги (игра на Unity 2022.3.35f1)

Источник: [Unity Manual, Standalone Player command line arguments (2022.3)](https://docs.unity3d.com/2022.3/Documentation/Manual/PlayerCommandLineArguments.html). macOS включён, оговорок по `-batchmode` для macOS нет:

- `-batchmode` — «Run the application in 'headless' mode. In this mode, the application doesn't display anything or accept user input».
- `-nographics` — в batch-режиме Unity не инициализирует графическое устройство.
- `-logFile <path>` — задаёт путь лога плеера; «To output to the console, specify `-` for the path name».
- macOS-only: `-force-metal`, `-force-low-power-device`.
- **На macOS НЕ поддерживаются**: `-single-instance` (важно: параллельные экземпляры технически возможны) и `-popupwindow`.

### 2.4 Живое доказательство комбинации — мод RimWorld Multiplayer

`Source/Client/Networking/HostUtil.cs` из [rwmt/Multiplayer](https://github.com/rwmt/Multiplayer/blob/master/Source/Client/Networking/HostUtil.cs) запускает **второй полный экземпляр игры** (Arbiter) так:

```csharp
string args = $"-batchmode -nographics -arbiter -logfile arbiter_log.txt -connect=127.0.0.1:{...}";
if (GenCommandLine.TryGetCommandLineArg("savedatafolder", out string saveDataFolder))
    args += $" \"-savedatafolder={saveDataFolder}\"";
```

Это одновременно подтверждает: валидность форм `-savedatafolder=<path>` и `-logfile <path>`, работу `-batchmode -nographics` на настоящем RimWorld и то, что `GenCommandLine.TryGetCommandLineArg("savedatafolder", …)` действительно матчит этот флаг. (`-arbiter`, `-connect` — флаги самого мода, не vanilla.)

## 3. Можно ли указать альтернативный каталог данных

**Да, и это главный механизм изоляции.** `-savedatafolder=<path>` переопределяет весь корень данных: `Config/` (включая `ModsConfig.xml`, `Prefs.xml`), `Saves/`, рабочие папки модов. Папка создаётся автоматически (декомпил §1). Игра при этом пишет в лог однозначный маркер:

```
Save data folder overridden to <path>
```

(литерал из `GenFilePaths.SaveDataFolderPath`, декомпил).

**Что НЕ переезжает:**

1. **Папка модов.** `GenFilePaths.CoreModsFolderPath = Path.Combine(parent(UnityData.dataPath), "Mods")` — вычисляется из расположения бандла и от `savedatafolder` не зависит. На этой установке это `/Applications/RimWorld.app/Mods` (подтверждено записями Player.log). `ModLister.RebuildModList` (декомпил) сканирует только `CoreModsFolderPath` + Steam Workshop («Adding mods from mods folder:» … «Adding mods from Steam:»). То есть добавить тестовый мод **только через `-savedatafolder` нельзя** — он не будет найден. Решение — §6 (клон .app).
2. **Unity PlayerPrefs** (`~/Library/Preferences/ludeon.rimworld.plist`) — адресуются bundle identifier'ом, рантайм-перенаправления у Unity player нет. Клон с тем же bundle id будет писать в тот же plist (только screen-настройки — приемлемый шум, см. риски §6).

«PlayerDataPath»-перенаправления вида `-playerDataPath` у Standalone player нет (в списке аргументов Unity Manual отсутствует).

## 4. Лог и формат ошибок локализации

**Дефолтный путь на macOS** (наблюдается на этой машине):

```
~/Library/Logs/Ludeon Studios/RimWorld by Ludeon Studios/Player.log   (+ Player-prev.log)
```

Переопределяется флагом `-logfile <path>` (Unity docs §2.3; Multiplayer Arbiter использует `-logfile arbiter_log.txt`). С `-logFile -` лог идёт в stdout — удобно для CI-пайплайна.

**Как выглядят ошибки локализации** — два уровня доказательств:

Реальная строка из Player.log этой машины (строка 820, чтение):

```
Translation data for language Russian / Русский has 1042 errors. Generate translation report for more info.
```

Код, который её печатает — `Verse/LoadedLanguage.cs` (декомпил, строка ~503): `" errors. Generate translation report for more info."` с счётчиком ошибок до этого.

Другие форматы загрузочных ошибок из того же класса (цитаты литералов декомпила/DLL):

```
Exception loading translation data from file …
Critical error while injecting translations into defs: …
Duplicate def-injected translation key: …
Duplicate keyed translation key: …
Exception loading language data. Rethrowing. Exception: …
```

**Детальный отчёт по ключам** пишется в `TranslationReport.txt` в **корне save data folder** (`Verse/LanguageReportGenerator.cs`: `Path.Combine(GenFilePaths.SaveDataFolderPath, "TranslationReport.txt")`). Триггер — только UI: кнопка «Save translation report» на главном меню (появляется для языка, отличного от английского; в dev-режиме тоже — `RimWorld/MainMenuDrawer.cs`, строки 216–222, 374–377). CLI/API-триггера нет, per-key лог по умолчанию не пишется; `Prefs.xml → <logVerbose>True</logVerbose>` добавляет в лог отладочную сводку построения списка модов (`ModLister`, «Adding mods from mods folder: …»).

## 5. Реалистичная автоматизация

**Headless-запуск валиден.** `-batchmode -nographics` доказанно работает на RimWorld: Multiplayer Arbiter — это полноценный второй экземпляр игры, живущий headless (§2.4), включая ветку `RuntimePlatform.OSXPlayer`. RimWorld грузит все дефы/языки/переводы при старте, **до** главного меню, поэтому для acceptance «перевод загружается без ошибок» достаточно дойти до меню в batch-режиме и распарсить лог.

Ограничения и оговорки:

- Окно при `-batchmode` может всё равно появляться: тред «[1.1] RimWorld opens a window when running in batchmode and loading a savegame» (Ludeon Forums, Zombiefied, 06.03.2020). На macOS xvfb-обхода нет. Fallback — обычный оконный запуск с `-logfile`: для acceptance-логов окно не мешает, закрывается по таймауту.
- `-nographics` может споткнуться о код инициализации GUI/текстур в сцене меню (Arbiter проходит через Play-scene, не через меню). Требуется один эмпирический прогон; маркеры успеха — см. ниже. Если меню в `-nographics` не грузится — запускать с графикой, парсинг лога идентичен.
- Batch-экземпляр сам не завершается → нужен watchdog: таймаут + `SIGTERM`; `-logfile` пишется инкрементально, лог парсится после убийства процесса.
- `quicktest` можно добавить, чтобы автоматически провалиться глубже меню в реальную игру, но для проверки загрузки переводов это опционально (и тяжелее: генерация мира).

Контрольные маркеры для grep в логе прогона:

| Маркер | Что подтверждает |
|---|---|
| `Save data folder overridden to <sandbox>` | песочница применилась, прод не тронут |
| `Command line arguments: …` | флаги дошли до игры |
| `Translation data for language … has N errors` | суммарное состояние загрузки языка (N сравнивать с baseline) |
| `Exception loading translation data`, `Critical error while injecting translations`, `Duplicate … translation key` | конкретные поломки сборки перевода |
| обычные деф-ошибки загрузки (`Failed to find …`, `Exception parsing …`) | дефы мода/патчей совместимы |

Программного CLI-триггера для `TranslationReport.txt` нет (только кнопка в UI, §4) — key-level отчёт в полностью автоматическом режиме сегодня недоступен без компаньон-мода (возможное будущее расширение RimLoc: мини-мод, вызывающий `LanguageReportGenerator.SaveTranslationReport()` на буте; вне скоупа этого исследования).

## 6. Дизайн процедуры изолированного acceptance

Песочница = disposable каталог внутри `testlab/run/<timestamp>/` + **клон .app** (APFS clonefile: `cp -c -R` мгновенный и почти не занимает место — copy-on-write).

```
testlab/run/<ts>/
├── RimWorldTest.app/        ← cp -c клона /Applications/RimWorld.app (disposable)
│   └── Mods/                ← сюда кладём тестовый мод/перевод (или симлинк на него)
├── data/                    ← будущий -savedatafolder; игра создаст Config/Saves сама,
│   └── Config/
│       ├── ModsConfig.xml   ← pre-write: активный список = база + наш мод
│       └── Prefs.xml        ← pre-write: langFolderName=Russian (Русский), logVerbose=True
└── run.log                  ← путь для -logfile
```

Процедура:

1. `cp -c -R /Applications/RimWorld.app testlab/run/<ts>/RimWorldTest.app`.
2. Установить проверяемый мод в `RimWorldTest.app/Mods/<packageId>/` (это **клон**, не оригинал). Уже установленные моды оригинала попадают в клон автоматически.
3. Создать `data/Config/ModsConfig.xml` (активные: `ludeon.rimworld`, DLC, зависимости, наш мод) и `Prefs.xml` с нужным языком. Структуру брать с реальных файлов §1.
4. Запустить напрямую бинарь клона:

   ```bash
   ".../RimWorldTest.app/Contents/MacOS/RimWorld by Ludeon Studios" \
     -savedatafolder=testlab/run/<ts>/data \
     -logfile=testlab/run/<ts>/run.log \
     [-batchmode -nographics]
   ```

5. Watchdog: таймаут (успех = в логе появился маркер языка/«Rebuilding mods list» и нет новых строк N секунд) → `SIGTERM` → парсинг `run.log` по маркерам §4/§5.
6. Teardown: каталог прогона удаляется целиком; ничего в системе не остаётся, кроме шума в `ludeon.rimworld.plist`.

**Категорически не трогать:**

- `/Applications/RimWorld.app` целиком, включая `Mods/`, `Data/`, `Source/` — только чтение; запись внутрь .app также ломает кодовую подпись.
- Продакшн-профиль `~/Library/Application Support/RimWorld/` — особенно `Saves/` и `Config/ModsConfig.xml`. Все запуски тестов — только с `-savedatafolder`; прод-`ModsConfig.xml` не редактируется (в песочнице он свой).
- `~/Library/Logs/…` — только чтение.

**Риски:**

- Клон наследует bundle id `ludeon.rimworld` → Unity PlayerPrefs plist (`~/Library/Preferences/ludeon.rimworld.plist`) общий: при запуске клона обновятся только screen/session-ключи (наблюдаемое содержимое — только они). Известный допустимый шум; альтернативы без пересборки bundle id нет.
- Добавление файлов в `Mods/` клона формально инвалидирует его подпись; локально созданный клон без quarantine-атрибута запускается — проверить эмпирически первым прогоном.
- Не запускать две копии одновременно без нужды (обе пишут один plist; `-single-instance` на macOS всё равно нет, конфликта процессов не будет, но и параллелизм тут не нужен).
- Если batch-прогон зависает до записи маркеров — сначала проверить `run.log` на ранние Unity-ошибки; при недостижимости меню в `-nographics` перейти на оконный запуск (§5).

## 7. Ручной smoke-test чеклист (если прогон автоматизацией не удался)

Условия те же: клон .app + `-savedatafolder` в песочницу, русский язык. Скриншоты — `Cmd+Shift+4`.

1. **Главное меню**: подписи кнопок переведены, нет голых ключей вида `MainMenuNewGame` или английских остатков; скрин меню.
2. **Наведение**: тултипы кнопок меню показывают переведённые описания.
3. **Мод-менеджер**: в списке модов видно название и описание проверяемого мода; страница настроек мода (`Mod settings`) — переведённые лейблы; скрин.
4. **Игра** (можно с `quicktest` для быстрого входа): имена дефов в интерфейсе (инспектор поселенца, вкладки, предметы), входящие сообщения/letters, подсказки при наведении на объекты; скрины 2–3 показательных экранов.
5. **Лог после выхода**: нет красных `Exception loading translation data` / `Critical error while injecting translations`; строка `has N errors` совпадает с ожидаемым baseline (0 или зафиксированное число).
6. **TranslationReport**: кнопка «Сохранить отчёт о переводе» на главном меню → проверить `data/TranslationReport.txt` в песочнице (список missing/unused/error ключей — прямой вход для фикса сборки RimLoc).

## Источники

Локальные (только чтение): `Info.plist`, `Assembly-CSharp.dll` (литералы), `boot.config`, `app.info`, `~/Library/Application Support/RimWorld/Config/*`, `~/Library/Logs/Ludeon Studios/RimWorld by Ludeon Studios/Player.log`, `~/Library/Preferences/ludeon.rimworld.plist`.

Декомпилы (публичные зеркала кода игры): [josh-m/RW-Decompile](https://github.com/josh-m/RW-Decompile) — `Verse/GenCommandLine.cs`, `Verse/GenFilePaths.cs`, `Verse/QuickStarter.cs`, `Verse/LoadedLanguage.cs`, `Verse/LanguageReportGenerator.cs`; [RimWorld-zh/RimWorld-Decompile](https://github.com/RimWorld-zh/RimWorld-Decompile) — `Verse/ModLister.cs`, `Verse/Prefs.cs`, `RimWorld/MainMenuDrawer.cs`, `Verse/Root.cs`.

Живое использование флагов: [rwmt/Multiplayer, Source/Client/Networking/HostUtil.cs](https://github.com/rwmt/Multiplayer/blob/master/Source/Client/Networking/HostUtil.cs).

Документация: [Unity 2022.3 Manual — Standalone Player command line arguments](https://docs.unity3d.com/2022.3/Documentation/Manual/PlayerCommandLineArguments.html).

Сообщество: тред «[1.1] RimWorld opens a window when running in batchmode and loading a savegame», Ludeon Forums, 06.03.2020; обсуждение «Changing config folder on Linux?» (Ludeon Forums, 2017) — `-savedatafolder` переносит и конфиги, не только сейвы.
