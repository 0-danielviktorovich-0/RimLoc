---
type: reference
status: current
tags:
  - project/rimloc
  - kind/testing-architecture
last-reviewed: 2026-09-30
related:
  - "[[AUTONOMOUS_ACCEPTANCE_ARCHITECTURE]]"
---

# ACCEPTANCE MATRIX — что чем доказываем

Живая таблица «возможность → тир(ы) → статус». Обновляется в каждой волне.
Тир-определения и инварианты — `RIMLOC_ACCEPTANCE_MANIFEST.yaml`.

## Статусы
LIVE (доказано на живом артефакте) · TEST (покрыто T0/T1) · MOCK (только мок) ·
GAP (нет покрытия) · BLOCKED (внешний блокер) · N/A

| Возможность / экран | T0 | T1 | T2 semantic | T3/T4 native | T5 visual | Статус |
|---|---|---|---|---|---|---|
| Создание проекта (pick_directory) | ✅ | ✅ | GAP | LIVE (release-джорни 27-29.09) | ✅ | LIVE |
| Открытие/reopen/recents (дедуп) | ✅ | ✅ | GAP | LIVE (managed 8→8) | ✅ | LIVE |
| Редактирование + persist-before-ack | ✅ | ✅ | GAP | — | — | TEST |
| Валидация проекта (+ плейсхолдеры selfloc, волна 6) | ✅ | ✅ | GAP | LIVE (экран проверки, кадры) | ✅ | LIVE |
| Экспорт перевода (гварды out-dir) | ✅ | ✅ | GAP | LIVE (reparse 2/2) | — | LIVE |
| Сборка мод-пакета (About/Languages) | ✅ | ✅ | GAP | LIVE (мод-пакет на диске) | — | LIVE |
| Диагностика (sanitized bundle) | ✅ | ✅ | GAP | LIVE | — | LIVE |
| Existing-flow (dry-run → apply) | ✅ | ✅ | GAP | — | — | TEST |
| Selfloc: карточка/Help-вход/дедуп | ✅ | ✅+ | GAP | LIVE (кадры r9c/w7) | ✅ | LIVE |
| Selfloc: вклад (бандл из GUI, волна 7) | ✅ | ✅ | GAP | частично (рендер блока LIVE, клик — GAP канала) | — | TEST+ |
| Selfloc: contribution build/apply CLI (SF-07/11/06 закрыты волной 8, 416 тестов) | ✅ | ✅ | — | — | — | TEST |
| Wizard-тур (9 шагов) | ✅ | ✅ | GAP | LIVE (шаг 1, кадр) | ✅ | LIVE |
| Review-экран счётчики | ✅ | ✅ | GAP | LIVE (кадр r1) | ✅ | LIVE |
| Геометрия: no-giant-blank-tail | — | ✅ | **T2a подключён (волна 10)** + фиксы волны 9 | — | — | LIVE(gate)+FIX |
| Фокус: tablist roving (workspace-вкладки) | — | ✅ | T2a: стрелки живут; бюджет 16 (fixme, виновник DevPanel-кнопка) | — | — | PARTIAL |
| Вложенный скролл/панель-контракт | — | частично | T2a scroll-инварианты; вкладки вылечены волной 9 | — | — | PARTIAL |
| Onboarding overlay-геометрия | — | GAP | — | — | — | GAP |
| Селектор языков (масштабируемость) | — | GAP | — | — | GAP | GAP (owner-наблюдение) |
| «(контракт)»/транспорт-термины из UI | — | GAP | — | — | — | **OPEN (livebadge и др.)** |
| TM / Глоссарий live | волна 13 (глоссарий в работе) | ✅(мок-тесты) | GAP | — | — | Глоссарий: WIP; TM: MOCK |
| Chat batch (no-API) | — | MOCK | GAP | — | — | MOCK |
| Multi-target live-контракт | — | частично | GAP | — | — | PARTIAL |
| Source Inspector live-данные | ✅ | ✅ | GAP (T2b) | ждёт REL-12 | — | TEST (волна 12: source_ref additive, live-ветка) |
| RimWorld игровая загрузка перевода | — | — | — | — | — | NOT-RUN (T6) |

## Owner-наблюдения 30.09 → классы (приоритет)

Из `owner-screenshots/` (01:16–01:39, после REL-11): классы blank-tail/nested-scroll/
терминов/фокуса помечены **выше** — это фронт волны 9 + инварианты T2.
Правило цели: каждый owner-дефект закрывается фиксом И инвариантом-гейтом.
