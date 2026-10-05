# COMPETITOR MASTER LIST 2026-10

Research: 2026-10-05, GitHub API. 14 verified + 3 new = 17 инструментов.
Полный отчёт субагента — в evidence директории.
**Актуальное состояние (прогоны + discovery) — в секции «Реконсиляция 2026-10-05» внизу и в `TIER_A_COMPLETION_MATRIX_2026-10.md` (24 строки).**

## Ключевые выводы
- Три имени-омонима RimTrans — разные родословные (не форки)
- 3 репо без лицензии (Text Grabber, inkitter, kelvinauta)
- AutonomoAI/rimworld-autonomous-translator — пустышка (исключить)
- Живой сегмент 2026: Translation Forge, Remis, RimWorld Translator Grabber GUI, Mod Translation Toolkit

## Tier A — Direct RimWorld Competitors (14 verified + 3 new)

| # | Инструмент | Репо | Лицензия | Язык | Активность | Ключевые особенности |
|---|---|---|---|---|---|---|
| 1 | Text Grabber | kamikadza13/Text-grabber | НЕТ | Python | v1.7.5 (2026-08) | Русское сообщество, извлечение + готовая папка перевод-мода |
| 2 | RimLangKit | OneCodeUnit/RimLangKit | Apache-2.0 | C#/.NET 9 | v3.6.4 (2025-09) | Русский, загрузчик перевода + набор переводчика (морфология, склонения) |
| 3 | RimTrans (RimWorld-zh) | RimWorld-zh/RimTrans | MIT | C# WPF | v0.18.2 (2018) НЕАКТИВЕН | Канонический китайский, генерация шаблонов |
| 4 | RimTrans (Aironsoft) | Aironsoft/RimTrans | MIT | C# | v0.21.9 (2021) НЕАКТИВЕН | Самостоятельный «наследник», не форк |
| 5 | RimTrans (inkitter) | inkitter/RimTrans | НЕТ | C# | v1.0 (2017) МЁРТВ | Исторический |
| 6 | RimTranslate | winterheart/RimTranslate | GPL-3.0 | Python | 2024-04 СПИТ | Def→PO→DefInjected, PO-воркфлоу |
| 7 | Translation Forge | Momaomao8787/Translation-Forge | MIT | Python 3.11+ | v0.8.7 (2026-09) АКТИВЕН | zh-TW/EN, создание пакета, missing detection, CSV/XML |
| 8 | rimworld-mod-translator | laskinss27-cmyk/rimworld-mod-translator | MIT | Python+Sheets | v4.0.0 (2026-08) | Русский EN/RU, Google Sheets workflow, безопасная сборка |
| 9 | RimWorldModTranslator | NicoriciN89/RimWorldModTranslator | Apache-2.0 | Python | v1.5.2 (2026-07) | **ПОЛНОСТЬЮ ОФЛАЙН**: Argos MT + глоссарий + Ollama LLM + сборка мода |
| 10 | Rimworld Mod Translator | kelvinauta/Rimworld-Mod-Translator | НЕТ | JS/Node | 2024 СПИТ | DeepL API |
| 11 | Mod Translation Toolkit | DrizztGaming/Mod-Translation-Toolkit | MIT | PowerShell | v0.10.26 (2026-09) АКТИВЕН | Multi-game (RW/Kenshi/PZ), EN→PL терминология, Workshop Dashboard |
| 12 | RimWorld AI Translator | chance496/RimWorldAiTranslator | MIT | C# | v1.1.0 (2026-07) | **Корейский** целевой, AI + Google фолбэк, глоссарий, ревью |
| 13 | RimWorld Autonomous Translator | AutonomoAI/rimworld-autonomous-translator | НЕТ | — | ПУСТО | ⚠️ UNCERTAIN — только README, кода нет |
| 14 | Remis | Drlinglong/Remis | AGPL-3.0 | Tauri 2+React+FastAPI | v3.2.1 (2026-09) АКТИВЕН | **Multi-game, local-first, TM, валидация, агенты** — strategic comparator |
| N1 | RW Translator Grabber GUI | doktorravlik-svg/RimWorld-Translator-Grabber-GUI | MIT | Python 3.14 | 2026-06 АКТИВЕН | Русский, **8+ MT движков** (Google→Argos), PyMorphy3, SQLite кеш |
| N2 | RimTransAI | mmjio-xy/RimTransAI | GPL-3.0 | C#/Avalonia | v1.3.0 (2026-07) | Китайский, **Mono.Cecil-рефлексия** + LLM батч |
| N3 | RWModTranslator (JP) | etejasdgjjjj532/RimWorldModTranslator | MIT | Python | 2026-08 | Японский |

## Pre-beta impact
| Инструмент | Блокер | HIGH | MEDIUM | ROADMAP |
|---|---|---|---|---|
| Text Grabber | — | Извлечение | — | — |
| RimLangKit | — | Морфология, загрузчик | — | — |
| Remis | — | TM, multi-game, local-first | — | — |
| Translation Forge | — | Missing detection | — | — |
| NicoriciN89 | — | Офлайн MT | — | — |
| Grabber GUI | — | MT fallback chain | — | — |
| Mod Toolkit | — | Терминология | — | — |

---

## Реконсиляция 2026-10-05

Полная матрица завершённости с прогонами — **`TIER_A_COMPLETION_MATRIX_2026-10.md`** (24 строки, без пропусков). Дифф-отчёты wave 2/3 — в `differential/`. Ниже — синхронизация этого списка с результатами практических прогонов и discovery; история таблицы выше сохранена как есть.

### Классификации по словарю статусов (итог)
- **PRACTICALLY_RUN — 8**: Text Grabber (ev.5), RimLangKit (ev.6, ядро), RimTrans RimWorld-zh (ev.6, extractor), RimTranslate (ev.5), Translation Forge (ev.5), laskinss27 (ev.5, локальные стадии), NicoriciN89 (ev.5, scanner/patches), RimWorldAiTranslator (ev.6, ядро+их тесты 45/82 PASS).
- **BLOCKED_PLATFORM — 3**: RimTrans Aironsoft (net461 + Assembly-CSharp.dll с Windows-HintPath, 22×CS0246), inkitter (net45 WinForms, без лицензии), Mod Translation Toolkit (pwsh+WPF отсутствуют).
- **BLOCKED_DEPENDENCY — 1**: JalapenoLabs (единственная функция — платный OpenAI-вызов).
- **SOURCE_CONFIRMED_ONLY — 4**: Remis (ev.2, threat HIGH — НЕ запускался, вне объёма wave 3), RimTrans_PY (ev.2, новый), etejasdgjjjj532 (ev.2), TokcDK (ev.2).
- **DOC_ONLY — 4**: Grabber GUI (ev.1), RimTransAI (ev.1), AutonomoAI (ev.1, кода нет), rtl-tools (ev.1, новый).
- **NOT_MATERIALLY_RELEVANT — 4**: kelvinauta, rwmt (ложное срабатывание), lenhare (новый), Ludeon workflow (не тул).
- **IDENTITY_UNRESOLVED — 0.**

### Same-corpus: 8 из 24 прогнаны, NOT DONE — 16
Дифф-корзины (BOTH/RIMLOC_ONLY/COMPETITOR_ONLY/SEMANTIC) получены для: Text Grabber (1031/4/364/6), RimLangKit (286/1452/49/6), RimTrans-zh (664/370/12/4), RimTranslate (HugsLib: 71/163*/7/4), Translation Forge (Defs-слой VE: 196/0/6), NicoriciN89 (патч-фокус: 0 у RimLoc против 54 игроку-видимых), laskinss27 (181/83/1701/72), RWAT (709/325/7/4). Остальные 16 — см. матрицу §2: 7 feasible (Remis, Grabber GUI, RimTransAI, RimTrans_PY, etejasdgjjjj532, TokcDK, rtl-tools — очередь следующего practical-wave), 3 блокировка платформой, 1 блокировка зависимостью, 5 «не за чем/нечего запускать».

### Топ-находки против RimLoc (все подтверждены прогонами)
1. **MUST_FIX: патч-слой пуст** — learn-patches=0 и scan --with-patches Δ=0 на 3170653412 (25 патч-файлов), у конкурентов 54–163 строки; нужен полевой blacklist (урок N89: 46% шума).
2. **MUST_FIX: msgid-разметка** — `&lt;b&gt;…` → `b…/b` (4 ключа HugsLib; независимо нашли 3 лейна; RimTrans/RWAT возвращают корректно).
3. **HIGH**: дефолтный промах Defs вне version-папок (чинится `--defs-dir`, проверено 61 записью); дыры словаря (`all_fields` построен и не используется, `lib.rs:1501-1505`); мост learn-defs→export-po; индексированные стадии; фильтр чисел/цветов; баг `is_version_directory` на числовых workshop-id (probe-подтверждён).

### Исправления фактов в таблице выше
| Было | Стало (прогоны 2026-10-05) |
|---|---|
| RimTrans (RimWorld-zh): «C# WPF» | master — lerna-монорепо, ядро **TypeScript** (`@rimtrans/extractor`); C# WPF — legacy-тег v0.18.2.6; форков **23**, код мёртв с 2020-01 (dependabot до 2022-12) |
| RimTrans (Aironsoft): «самостоятельный наследник, не форк» | **форк старой C#-линии duduluu** (MIT © duduluu, README-атрибуция на RimWorld-zh/RimTrans), все коммиты за один день |
| RimWorldAiTranslator: «46 тест-файлов» | измерено **42** .cs в tests/ |
| RimTrans (inkitter): v1.0 (2017) | последний коммит **2017-05-16**, лицензии нет — подтверждено |
| kelvinauta: «DeepL API» | подтверждено: index.js 97 строк, DeepL-only, 3 тега, лицензии нет |
| Grabber GUI: MIT; RimTransAI: GPL-3.0 | подтверждено чтением файлов LICENSE (API-детект GitHub отдаёт null) |

### Новые строки (discovery 2026-10-05: `gh search repos` ×4 запроса + `gh api` по кандидатурам)
| # | Инструмент | Репо | Лицензия | Язык | Активность | Суть |
|---|---|---|---|---|---|---|
| N4 | RimTrans_PY | masakitenchi/RimTrans_PY | MIT (LICENSE прочитан) | Python | спит с 2024-08 | **третье независимое подтверждение патч-линии**: полные `<value>`-дефы из PatchOperationAdd c фильтром abstract + LoadFolders с IfActive/IfNotActive; SOURCE_CONFIRMED_ONLY |
| N5 | rimworld-rtl-translation-tools | mtimoustafa/rimworld-rtl-translation-tools | НЕТ | Ruby | спит с 2024-05 | пост-обработка RTL (контекстуализация арабских букв, реверс) — заметка для RTL-роадмапа RimLoc; DOC_ONLY |
| N6 | RimWorldTranslationTool | lenhare/RimWorldTranslationTool | НЕТ | Python | мёртв (1 день, 2024-07) | Google Cloud Translate + **закоммиченный google_credentials.json** — анти-паттерн; NOT_MATERIALLY_RELEVANT |

Footnote (не тула): `BetterRimworlds/Rimworld-Urdu` — контент-репо «via Autonomo AI» (подтверждает активность закрытого пайплайна AutonomoAI); остальные находки поиска — контент-репо и мёртвый шум (см. матрицу §5).
