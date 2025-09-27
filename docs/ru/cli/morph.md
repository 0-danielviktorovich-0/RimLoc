---
title: Команда Morph
---

# Команда Morph

Генерирует файлы `_Case.xml`, `_Plural.xml` и `_Gender.xml` в `Languages/<lang>/Keyed` с помощью морфологического провайдера.

Провайдеры:
- `dummy` (по умолчанию) — простые офлайн-эвристики
- `morpher` — Morpher WS3 API (требуется `MORPHER_TOKEN`)
- `pymorphy2` — локальный REST‑сервис (см. `scripts/pymorphy2_local/`)

## Использование

```bash
rimloc-cli morph --root <MOD> [--provider dummy|morpher|pymorphy2] \
  [--lang <CODE>|--lang-dir <DIR>] [--filter-key-regex <RE>] [--limit N] \
  [--game-version <VER>] [--timeout-ms 1500] [--cache-size 1024] \
  [--pymorphy-url http://127.0.0.1:8765]
```

## Опции

| Опция | Описание |
|-------|----------|
| `--root <MOD>` | Корень мода RimWorld. |
| `--provider` | `dummy` (по умолчанию), `morpher` или `pymorphy2`. |
| `--lang <CODE>` / `--lang-dir <DIR>` | Целевой язык по ISO‑коду или явной папке. |
| `--filter-key-regex <RE>` | Обрабатывать только ключи, подходящие под регулярное выражение (для Keyed). |
| `--limit N` | Ограничить число обрабатываемых ключей. |
| `--game-version <VER>` | Работать в конкретной подпапке версии (`1.6` или `v1.6`). |
| `--timeout-ms` | Таймаут HTTP для запросов к провайдеру (по умолчанию 1500 мс). |
| `--cache-size` | Ёмкость LRU‑кэша ответов провайдера (по умолчанию 1024). |
| `--pymorphy-url` | Переопределяет `PYMORPHY_URL` (URL локального сервиса pymorphy2). |

## Выходные файлы

- `_Case.xml` содержит как минимум `Nominative` и `Genitive`. Если провайдер возвращает больше, добавляются `Dative`, `Accusative`, `Instrumental`, `Prepositional`.
- `_Plural.xml` и `_Gender.xml` формируются эвристиками (в дальнейшем могут быть уточнены).

## Примеры

Dummy (офлайн):

```bash
rimloc-cli morph --root ./Mods/MyMod --provider dummy --filter-key-regex '.*label$' --limit 100
```

Morpher WS3 (онлайн):

```bash
export MORPHER_TOKEN=...
rimloc-cli morph --root ./Mods/MyMod --provider morpher --lang ru --limit 50
```

Локальный pymorphy2:

```bash
(cd scripts/pymorphy2_local && python -m venv .venv && source .venv/bin/activate && pip install -r requirements.txt && uvicorn app:app --port 8765)
rimloc-cli morph --root ./Mods/MyMod --provider pymorphy2 --lang ru --limit 50 --timeout-ms 2000
```

Быстрая проверка сервиса через curl:

```bash
curl "http://127.0.0.1:8765/declension?text=мама"
# -> {"nomn":"мама","gent":"мамы","datv":"маме","accs":"маму","ablt":"мамой","loct":"маме"}
```

Другие примеры:

- Обрабатывать только ключи, оканчивающиеся на `label`, и ограничить 100 записями:

```bash
rimloc-cli morph --root ./Mods/MyMod --provider morpher \
  --filter-key-regex '.*label$' --limit 100 --lang ru
```

- Выбрать подпапку версии и увеличить кэш:

```bash
rimloc-cli morph --root ./Mods/MyMod --provider pymorphy2 \
  --game-version v1.6 --cache-size 4096 --timeout-ms 2500 --lang ru
```

Ограничения (dummy):
- Множественное число и пол определяются эвристически; нестандартные случаи (исключения, аббревиатуры) могут ошибаться.
- Для итогового результата лучше использовать онлайн/локальный морфологический провайдер.
