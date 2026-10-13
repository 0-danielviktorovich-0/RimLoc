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
release              rel26-draft-fixes
product_source       HEAD fix/draft-safety-structural-identity на момент
                     сборки (коммит-лестница ниже); точные SHA артефакта —
                     ТОЛЬКО в файлах пакета artifact-rel26-draft-fixes/
                     (IDENTITY.md, sha256.txt, appzip-sha256.txt,
                     MANIFEST.sha256) — единственный источник правды на
                     пакет; этот документ не дублирует пер-билд SHA,
                     чтобы его фиксация не двигала HEAD (урок 13,
                     расширенный rel26-циклом)
automation           ba-main SSD-зона; automation-React сборка для native
                     WDIO (testlab/build/build-automation-react.sh)
```

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
| `7db89dc` | failure patterns 16-17, хронология RELEASE_GATE | в фикс-ветке |
| `0e1f71b` | **бинарная база rel26** (identity-доки финализированы) | **product_source** |

## Верификация identity

- frontend gate: React R1 встроен, svelte-фолбэк отвергнут.
- tsc strict чист; cargo services lib 288/288; rimloc-gui 33/33; vitest 12/12.
- native WDIO (реальный пакет, automation-React): фазы 1+2 PASS — collision
  rows, независимые черновики, locale round trip, sidecar на диске,
  рестарт с честным restore-notice (drafts-native-e2e.txt).
- MANIFEST.sha256 покрывает все файлы пакета evidence.
