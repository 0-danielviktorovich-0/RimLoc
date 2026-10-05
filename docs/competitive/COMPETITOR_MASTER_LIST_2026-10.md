# COMPETITOR MASTER LIST 2026-10

Research: 2026-10-05, GitHub API. 14 verified + 3 new = 17 инструментов.
Полный отчёт субагента — в evidence директории.

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
