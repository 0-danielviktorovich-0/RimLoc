---
type: reference
status: current
tags:
  - project/rimloc
  - kind/release-identity
last-reviewed: 2026-10-07
related:
  - "[[RELEASE_GATE]]"
  - "[[RELEASE_CONVERGENCE_STATE]]"
---

# FINAL IDENTITY — rel23-final (2026-10-07)

Точная identity финального release-candidate. Никаких формулировок
«код практически идентичен» — только sha. Полная история артефакта:
`RimLoc-evidence/artifact-rel23-final/IDENTITY.md`.

## Канонические идентификаторы

```text
product_source_sha   857b0d2a51b97e6e541cdf8f61d00683b966bd04
                     (HEAD main == origin/main; прод-код относительно
                      предыдущего RC отличается и только им)
evidence/docs_sha    6c3ae49 (FINAL_IDENTITY/state-доки поверх 857b0d2;
                     docs-only — бинарная поверхность неизменна:
                      git diff 857b0d2..6c3ae49 -- crates/ gui/ пуст)
binary_sha256        7781dd395b8e7c83332da9590a92972e6e93c285cef717200c9c3117471ae4bf
                     (RimLoc GUI.app/Contents/MacOS/rimloc-gui, self-report
                      857b0d2a… без -dirty)
automation_sha256    см. automation-артефакт той же сборочной линии
                     (feature automation-bridge, 61 wdio-маркер; sha
                      фиксируется в evidence при каждом пересборe —
                      текущий прогон WDIO 59/59 в IDENTITY.md)
appzip_sha256        2d0f165213f700b66c58c75cfbe3c8110f9e0e1f05af623fcda887c4fa0cee20
tree_hashes          app `1ba23156…` · frontend-react/dist `388896b5…`
```

## Цепочка SHAs этого RC-цикла (хронология, без неоднозначности)

| SHA | Что это | Статус |
|---|---|---|
| `af06a4e` | rel22-rc (respin 1): R2-волна + гейты | УСТАРЕЛ |
| `f99fc19` | фикс шва локалей (mapSnapshot/commit) | в составе main |
| `ac896e6` | docs: гейт-отчёт respin 2 | в составе main |
| `292d72e` | PR #85: svelte-dist для CodeQL (C5) | в составе main |
| `41c63e0` | R3-интеграция (C2–C6) + panels-v4 фикс | в составе main |
| `857b0d2` | **ФИНАЛ продукта**: доки C-гейтов; бинарь собран с него | **product_source** |
| `6c3ae49` | docs-only: FINAL_IDENTITY + state (бинарная поверхность та же) | **Текущий main** |

Бинарные отличия 2305d564 → 7781dd39 = исходники `292d72e..857b0d2`
(C2–C6 гейты + dependencies), проверяемо: `git diff 2305d564-base..857b0d2 -- crates/ gui/`.

## Верификация identity

- self-report бинаря совпадает с `product_source_sha` (strings).
- `soak-preflight --gate --expect-sha256 … --expect-commit …` → exit 0.
- Canonical tree hashes (.app и dist) записаны в IDENTITY.md артефакта.
- MANIFEST.sha256 покрывает все файлы пакета evidence.
