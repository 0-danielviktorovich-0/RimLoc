Pymorphy2 Local Service (FastAPI)

Lightweight HTTP service that exposes a tiny subset of pymorphy2 for RimLoc's `--provider pymorphy2` client.

Endpoints
- GET `/declension?text=слово` → JSON with nominative/genitive forms: `{ "nomn": "слово", "gent": "слова" }`

Quick Start
- python -m venv .venv && source .venv/bin/activate
- pip install -r requirements.txt
- uvicorn app:app --host 127.0.0.1 --port 8765

Env
- `PORT` (optional): server port (default 8765)

RimLoc usage
- export PYMORPHY_URL=http://127.0.0.1:8765
- rimloc-cli morph --root ./Mods/MyMod --provider pymorphy2 --lang ru --limit 50

Notes
- For nouns/short tokens we take the first pymorphy2 parse and try to inflect to `nomn` and `gent`. If inflection fails, source form is returned for that case.

