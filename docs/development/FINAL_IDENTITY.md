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

# FINAL IDENTITY — rel25-candidate (2026-10-12)

Точная identity финального кандидата финальной бета-готовности
(волны R4-fix + R5). Пакет: `RimLoc-evidence/artifact-rel25-candidate/`.

## Канонические идентификаторы

```text
product_source_sha   1b46fd2b79fda6a948c6b6eb4a0e8953e93ea9f7
                     (HEAD main == origin/main на момент сборки)
binary_sha256        1d412a728b0fc4bd4e2da61a14ce6c5942bd03dfe251e82c1044ca0c2d3c7973
appzip_sha256        6ec84906e5c7eecc4322ece46d822f2673bbf3876a06ae6ec286186bc5222d25
app tree hash        b2a69d811f42c3e54d09799c1fb1ea22720ae2c1f8c0c8595fa122bc247c700a
dist tree hash       e3b795655f78f62f44be8c1a53bd0b2a50bb78c6bb9e18947a4b57c896ba6513
automation           internal target-r4-final, 61 wdio-маркер, дерево 1b46fd2
```

Mutable состояние (текущий HEAD main) в этом документе НЕ хранится —
только в metadata конкретного пакета и в release-state.

## Хронология RC-цикла

| SHA | Что это | Статус |
|---|---|---|
| `33457117` | rel24-candidate | УСТАРЕЛ |
| `fcb82d7` | продуктовая поверхность rel24 | в составе main |
| `7ee92c4` | R5/R4fix продукты (nav split, draft autosave, source lang, chat batch mgr, settings/about, yaml-rust2) | в составе main |
| `1b46fd2` | **бинарная база rel25** (10 final findings поверх 7ee92c4) | **product_source** |
| `3219e60` | docs-only поверх | текущий main |

## Верификация identity

- self-report бинаря совпадает с product_source_sha (strings).
- frontend gate: React R1 встроен, svelte-фолбэк отвергнут.
- preflight --gate exit 0, release-guard static+runtime PASS.
- MANIFEST.sha256 покрывает все файлы пакета evidence.


## Верификация identity

- self-report бинаря совпадает с `product_source_sha` (strings).
- `soak-preflight --gate --expect-sha256 … --expect-commit …` → exit 0.
- Canonical tree hashes (.app и dist) записаны в IDENTITY.md артефакта.
- MANIFEST.sha256 покрывает все файлы пакета evidence.
