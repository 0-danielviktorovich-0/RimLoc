---
title: Morph
---

# Morph Command

Generate `_Case.xml`, `_Plural.xml`, and `_Gender.xml` under `Languages/<lang>/Keyed` using a morphology provider.

Supported providers:
- `dummy` (default) — simple heuristics offline
- `morpher` — Morpher WS3 API (requires `MORPHER_TOKEN`)
- `pymorphy2` — local REST microservice (see `scripts/pymorphy2_local/`)

## Usage

```bash
rimloc-cli morph --root <MOD> [--provider dummy|morpher|pymorphy2] \
  [--lang <CODE>|--lang-dir <DIR>] [--filter-key-regex <RE>] [--limit N] \
  [--game-version <VER>] [--timeout-ms 1500] [--cache-size 1024] \
  [--pymorphy-url http://127.0.0.1:8765]
```

## Options

| Option | Description |
|--------|-------------|
| `--root <MOD>` | RimWorld mod root. |
| `--provider` | `dummy` (default), `morpher`, or `pymorphy2`. |
| `--lang <CODE>` / `--lang-dir <DIR>` | Target language by ISO code or explicit folder. |
| `--filter-key-regex <RE>` | Process only keys matching regex (applies to Keyed keys). |
| `--limit N` | Limit number of keys processed (useful for sampling). |
| `--game-version <VER>` | Operate under a specific version subfolder (`1.6` or `v1.6`). |
| `--timeout-ms` | HTTP timeout for provider requests (default: 1500ms). |
| `--cache-size` | LRU cache capacity for provider responses (default: 1024). |
| `--pymorphy-url` | Override `PYMORPHY_URL` for the local pymorphy2 service. |

## Output

- `_Case.xml` includes at least `Nominative` and `Genitive`. If the provider supplies more, `Dative`, `Accusative`, `Instrumental`, `Prepositional` are added as available.
- `_Plural.xml` and `_Gender.xml` are generated using heuristics (and may be refined later).

## Examples

Dummy provider (offline):

```bash
rimloc-cli morph --root ./Mods/MyMod --provider dummy --filter-key-regex '.*label$' --limit 100
```

Morpher WS3 (online):

```bash
export MORPHER_TOKEN=... 
rimloc-cli morph --root ./Mods/MyMod --provider morpher --lang ru --limit 50
```

Local pymorphy2:

```bash
(cd scripts/pymorphy2_local && python -m venv .venv && source .venv/bin/activate && pip install -r requirements.txt && uvicorn app:app --port 8765)
rimloc-cli morph --root ./Mods/MyMod --provider pymorphy2 --lang ru --limit 50 --timeout-ms 2000
```

Quick service check with curl:

```bash
curl "http://127.0.0.1:8765/declension?text=мама"
# -> {"nomn":"мама","gent":"мамы","datv":"маме","accs":"маму","ablt":"мамой","loct":"маме"}
```
