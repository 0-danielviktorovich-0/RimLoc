<!-- mandate_id: g5-mock-live | wave: G5-W6 | scope: mock/live boundary, onboarding tour, demo project, scenario browser -->
# МАНДАТ: Mock/Live boundary + onboarding tour + scenario browser (G5-W6, 2026-09-24)

Сжатая персистенция (полный текст у владельца). Кумулятивно к G4/G5.

1. **Mock data явна**: dev-режим показывает бейдж «Demo data / Mock backend» (GOG 1.5 при
   реальной 1.6, Odyssey «unavailable» при установленном — примеры путаницы). Не clutter прод.
2. **Прод не шипит MockTransport**: после фриза — явный биндинг UI → RimLocClient → Tauri →
   canonical services; guard в xtask/CI против прод-пакета с MockTransport.
3. **Post-freeze GUI live-gate**: те же workflow на реальном бэкенде — discovery установок/
   версии/DLC/Workshop-модов, дружелюбные имена, создание проекта, языки, edit/persist/
   reopen, validate, build. Машина владельца (1.6 + Odyssey) = acceptance-кейс. Моки — не
   доказательство.
4. **Onboarding = anchored product tour** (не текстовая карусель): dim + подсветка реальных
   контролов, Back работает на каждом шаге, Skip/Next, прогресс.
5. **Состояние**: first launch → показать; completed/skipped → не повторять (persist);
   Help → Replay interface tour (видимо работает); опц. настройка contextual hints. E2E.
6. **Demo project**: bundled синтетическая фикстура RimLoc-owned (label, description,
   placeholder, Keyed, DefInjected, TKey/контекст, glossary, validation error, review,
   sourceChanged, мульти-таргет) — onboarding без Steam/RimWorld/Workshop.
7. **Guided tutorial**: Open Demo → выбрать строку → редактировать → Context → намеренная
   ошибка → Review → Validate → Build demo. Действия, не чтение. Финал: «Ваш первый
   перевод готов». Не писать в реальные директории.
8. **Demo safety**: изоляция от реальных проектов, детерминированный reset, не путаться в
   recent projects (маркировка), безопасно удаляется.
9. **No-mods empty state**: [Try Demo Project][Choose folder][Configure installation].
10. **Workshop Dark: оранжево-коричневый wash** — при финальном полирове нейтральный/
    тёплый-угольный фон, тепло в поверхностях/границах, amber — акценты. Идентичность
    не убирать. Решение владельца не требуется.
11. **Dev Scenario Browser** (только dev, интеграция со Style Lab): прямой доступ к
    сценариям — home/first-run, home/returning, wizard/{mod,base-game,dlc,language-pack,no-mods},
    workspace/{standard,multi-target,ai-running,narrow}, review/issues, build/{success,failure},
    settings/{rimworld,providers}, onboarding/*, diagnostics/bundle-preview. Те же фикстуры
    в визуальных/E2E тестах.
12. **REVIEW SCREEN MAP** в отчётах: экран · клик-путь · scenario id/URL — для запроса
    конкретных скриншотов.
13. **Не переписывать текущий GUI** — чинить mock/live ясность, онбординг, тулинг.

Принять в W-волны: W1-хвост (tour+replay-регрессия уже частично в W1), W6 = scenario
browser + demo project + mock-бейдж; live-gate = после FREEZE.
