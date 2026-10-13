---
type: reference
status: current
tags:
  - project/rimloc
  - kind/release-identity
last-reviewed: 2026-10-13
related:
  - "[[RELEASE_GATE]]"
  - "[[RELEASE_CONVERGENCE_STATE]]"
---

# FINAL IDENTITY — rel26-draft-fixes (2026-10-13)

Точная identity data-safety corrective кандидата (ответ на rel25-аудит:
BLOCKED — DATA-SAFETY FIXES REQUIRED). Пакет:
`RimLoc-evidence/artifact-rel26-draft-fixes/`.

## Канонические идентификаторы

```text
product_source_sha   7db89dc (fix/draft-safety-structural-identity @ 7db89dc)
binary_sha256        631f5c0… (полностью: artifact sha256.txt / appzip-sha256.txt)
appzip_sha256        015484d… (полностью: artifact appzip-sha256.txt)
automation           ba-main SSD-зона; automation-React сборка для native WDIO
                     (testlab/build/build-automation-react.sh)
```

Полные SHA — только в файлах артефакта (один источник правды на пакет;
урок 13 AUTONOMOUS_FAILURE_PATTERNS: никаких дублей identity).

## Хронология кандидатов

| SHA | Что это | Статус |
|---|---|---|
| `1b46fd2` | бинарная база rel25 | BLOCKED аудитом (data-safety) |
| `695dff9` | locale-scoped drafts + entryUid key-space + quiesce | в фикс-ветке |
| `dd1ba95` | vitest-сьют сценариев аудита (12/12) | в фикс-ветке |
| `4a674ad` | packer v3: параметризация + семантическая identity | в фикс-ветке |
| `f5b3d67` | RELEASE_GATE правда + failure patterns 13-15 | в фикс-ветке |
| `b301de5` | **tauri-адаптер: проводка drafts-команд** (дефект найден native WDIO) | в фикс-ветке |
| `0e2eec1` | native WDIO lane на automation-React поверхности | в фикс-ветке |
| `7db89dc` | **бинарная база rel26** (failure patterns 16-17, хронология) | **product_source** |

## Верификация identity

- frontend gate: React R1 встроен, svelte-фолбэк отвергнут.
- tsc strict чист; cargo services lib 288/288; rimloc-gui 33/33; vitest 12/12.
- native WDIO (реальный пакет, automation-React): фазы 1+2 PASS — collision
  rows, независимые черновики, locale round trip, sidecar на диске,
  рестарт с честным restore-notice (drafts-native-e2e.txt).
- MANIFEST.sha256 покрывает все файлы пакета evidence.
