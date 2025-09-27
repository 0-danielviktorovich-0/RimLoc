from __future__ import annotations

import os
from fastapi import FastAPI, Query
from fastapi.responses import JSONResponse
import uvicorn

try:
    import pymorphy2  # type: ignore
except Exception as e:  # pragma: no cover - local tool
    raise SystemExit(f"pymorphy2 import failed: {e}")


app = FastAPI(title="pymorphy2_local", version="0.1")
morph = pymorphy2.MorphAnalyzer()


@app.get("/health")
def health() -> dict[str, str]:  # pragma: no cover - trivial
    return {"status": "ok"}


@app.get("/declension")
def declension(text: str = Query(..., min_length=1, max_length=128)) -> JSONResponse:
    # Take first parse; try to inflect to Russian grammatical cases.
    # We always return the full set with fallback to input for missing forms.
    tags = ["nomn", "gent", "datv", "accs", "ablt", "loct"]
    out = {t: text for t in tags}
    try:
        p = morph.parse(text)[0]
        for t in tags:
            try:
                form = p.inflect({t})
                if form:
                    out[t] = form.word
            except Exception:
                # ignore individual inflection errors
                pass
        return JSONResponse(out)
    except Exception:
        # Graceful fallback: all cases equal to input
        return JSONResponse(out)


def main() -> None:  # pragma: no cover - local runner
    port = int(os.environ.get("PORT", "8765"))
    uvicorn.run("app:app", host="127.0.0.1", port=port, reload=False, log_level="info")


if __name__ == "__main__":  # pragma: no cover
    main()
