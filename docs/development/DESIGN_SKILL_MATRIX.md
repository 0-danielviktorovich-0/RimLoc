---
type: reference
status: current
tags:
  - project/rimloc
  - kind/design-governance
last-reviewed: 2026-09-30
related:
  - "[[AUTONOMOUS_ACCEPTANCE_ARCHITECTURE]]"
---

# DESIGN_SKILL_MATRIX — RimLoc

Inventory ФАКТИЧЕСКИ установленных дизайн-скиллов (сессия ZCode 2026-09-30,
источник — реальный список скиллов; имена не выдуманы). Мандат 03 §1.

## Выбор для RimLoc (desktop localization workstation)

**Primary: `ui-ux-pro-max`** — единственный установленный скилл, чьи инструкции
покрывают именно продуктовый/desktop UI: приоритизированные UX-правила (a11y CRITICAL →
touch → layout → typography → navigation), searchable-база (79 стилей, 192 палитры,
74 font-pairings, 119 UX-гайдлайнов). Репозиторий ссылался на него и раньше; аудит
использования: UI-кампания 29.09 (жюри 5 линз, топ-выбор) — соблюдался по сути.

**Complementary (использовать вместе):**
- `loop-design-check` — итеративная доводка «правка→скрин→критика» в дизайн-пассе;
- `minimalist-ui`, `high-end-visual-design` — линзы для финалистов T5-пакета;
- `theme-factory` — если пойдём в токены/темы глубже style-арсенала проекта;
- `webapp-testing` — при browser-режиме T2a (Playwright приёмы).

**Не применимы к RimLoc** (лендинг/маркетинговая территория, сами скиллы это
декларируют): design-taste-frontend (его описание прямо исключает дашборды/таблицы/
продуктовый UI в пользу ui-ux-pro-max), landing-design, canvas-design, algorithmic-art,
brandkit, imagegen-* (референсы), stitch-design-taste, gpt-taste (моушн лендингов),
image-to-code. Прошлые 5-линзовые прогоны это подтвердили эмпирически.

## Матрица по ролям мандата

| Роль мандата | Скилл | Статус |
|---|---|---|
| Product IA | ui-ux-pro-max (navigation/UX-гайды) | PRIMARY |
| Desktop/workstation UX | ui-ux-pro-max | PRIMARY |
| Visual design | high-end-visual-design + minimalist-ui (линзы) | COMPANION |
| Design system/tokens | theme-factory (+ tokens.css проекта) | COMPANION |
| Accessibility | ui-ux-pro-max (категория 1 CRITICAL) | PRIMARY |
| Interaction/motion | ui-ux-pro-max (anim) | PRIMARY |
| Frontend implementation | karpathy-guidelines/full-output (код-гигиена), не дизайн | SUPPORT |
| Visual review | loop-design-check + T5-пакет кадров + независимый ревьюер | COMPANION |

## Правило использования (мандат: «не упоминать в доке, а применять»)

Каждый дизайн-пасс волны обязан: (1) перед правкой — свериться с приоритет-таблицей
ui-ux-pro-max (категории 1/2/5 — a11y, interaction, layout); (2) после правки —
loop-design-check цикл на живых кадрах T5; (3) финалистов — линзы minimalist/high-end.
Выбранные скиллы грузятся в сессию реальным Skill-вызовом, не пересказом.
