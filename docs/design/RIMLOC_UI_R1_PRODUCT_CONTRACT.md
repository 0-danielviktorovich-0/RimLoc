# RIMLOC UI R1 PRODUCT CONTRACT

Канон продукта для React-кампании R1. Источник визуального языка:
`docs/design/LOVABLE_R1_ADAPTATION.md`. Источник функциональной правды:
`docs/design/CURRENT_SVELTE_BASELINE.md` (карта LIVE/PARTIAL/MOCK).
Профиль продукта: **DESKTOP WORKSTATION / LOCALIZATION TOOL**.

## Продуктовое ядро (JTBD)

Переводчик RimWorld-мода открывает мод → видит все переводимые строки →
переводит с поддержкой (валидация/глоссарий/контекст) → проверяет →
выпускает готовый мод-перевод. Core loop: **select → edit → confirm → next**.

## Экраны и их truth-источники

| Экран | Роль | Данные |
|---|---|---|
| Home | New translation · Open/update existing · Recent · Help translate RimLoc | project_list, recents |
| New translation (wizard) | источник → скан → цель → обзор → создание | build_project/create, Language Registry |
| Workspace (REPRESENTATIVE) | редактор: tree / entries / inspector | project_snapshot + apply intents + source_ref |
| Review/Checks | находки валидатора по категориям | project_validate |
| Existing | dry-run → классификация → apply | import_existing / apply_existing |
| Build/Export | сборка мода / обмен / импорт | project_build_mod / project_export |
| Glossary | CRUD терминов проекта | project_glossary* |
| Settings | внешний вид, проектные дефолты, провайдеры, честная capability-таблица | handshake + клиентские настройки |
| Diagnostics | операция → понятный итог → safe bundle | project_diagnose |
| Self-localization | «Help translate RimLoc» тем же редактором | selfloc_* |

## Визуальный контракт (минимум-планка R1)

- Токены/типографика/плотность — канон Lovable (см. ADAPTATION, раздел
  «Визуальная система»): OKLCH warm + wine, Literata-дисплей над Golos-UI,
  eyebrow/dot-язык, hairline-ритм, light/dark — равные цели качества.
- Workspace по умолчанию: LEFT (контекст проекта/файлов) / CENTER (инвентарь)
  / RIGHT (редактор/инспектор); resizable-панели с min/max и персистом.
- Виртуализация инвентаря (10k+ стресс), keyboard-first core loop.

## Честность

Каждая видимая возможность = LIVE / PARTIAL / DEMO / UNSUPPORTED, и это
ВИДНО. Демо-проект помечен. Никаких template-путей как фактов.

## Acceptance representative screen (§49) — до масштабирования

Реальный бэкенд-проект · light/dark · длинные строки · 10k+ стресс ·
resize · keyboard · selection · validation · inspector · (цели где live).
Провал любой планки = фикс системы, не масштабируем плохое.

## Gates

1. Deterministic layout/a11y probe (WDIO, background-only, §53).
2. E2E-джорни (§58) на живых данных.
3. Визуальный review скриншотов (light/dark, ключевые экраны).
4. Независимый ревьюер (§52).
5. Evidence-пакет §59 перед словом «DESIGN PASS».
