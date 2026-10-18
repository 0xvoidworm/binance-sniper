from __future__ import annotations

from contextlib import asynccontextmanager
from fastapi import FastAPI, WebSocket, HTTPException, Query
from fastapi.responses import HTMLResponse
from fastapi.staticfiles import StaticFiles
from pathlib import Path
from functools import lru_cache
import sqlite3

from binance_sniper.replay import build_replay_snapshot
from binance_sniper.reference.replay import build_reference_replay
from binance_sniper.tester.store import EvidenceStore


HTML = (Path(__file__).resolve().parents[2] / "web" / "index.html").read_text(encoding="utf-8")
RANGE_HTML = (Path(__file__).resolve().parents[2] / "web" / "range.html").read_text(encoding="utf-8")
TESTER_HTML = (Path(__file__).resolve().parents[2] / "web" / "tester.html").read_text(encoding="utf-8")
OBSERVER_HTML = (Path(__file__).resolve().parents[2] / "web" / "observe.html").read_text(encoding="utf-8")


@lru_cache(maxsize=1)
def replay_snapshot() -> dict:
    return build_replay_snapshot()


def build_app(state):
    tester = getattr(state, "tester_store", None) or EvidenceStore()
    @asynccontextmanager
    async def lifespan(app):
        if hasattr(state, "start"):
            await state.start()
        try:
            yield
        finally:
            if hasattr(state, "stop"):
                await state.stop()

    app = FastAPI(title="Binance Sniper", docs_url=None, redoc_url=None, lifespan=lifespan)
    app.mount("/assets", StaticFiles(directory=Path(__file__).resolve().parents[2] / "web" / "assets"), name="assets")

    @app.get("/", response_class=HTMLResponse)
    @app.get("/tester", response_class=HTMLResponse)
    async def index():
        return HTMLResponse(TESTER_HTML, headers={"Cache-Control": "no-store"})

    @app.get("/market", response_class=HTMLResponse)
    async def market():
        return HTMLResponse(RANGE_HTML)

    @app.get("/observe", response_class=HTMLResponse)
    async def observe():
        return HTMLResponse(OBSERVER_HTML, headers={"Cache-Control": "no-store"})

    @app.get("/api/observer")
    async def api_observer():
        return state.observer_snapshot() if hasattr(state, "observer_snapshot") else {"status": "UNAVAILABLE", "rows": []}

    def tester_read(operation):
        try:
            return operation()
        except (OSError, ValueError, sqlite3.Error, KeyError):
            raise HTTPException(status_code=503, detail="Tester evidence unavailable or integrity check failed") from None

    @app.get("/api/tester")
    def api_tester():
        return tester_read(tester.snapshot)

    @app.get("/api/tester/compare")
    def api_tester_compare(left: str = Query(pattern=r"^[a-f0-9]{64}$"), right: str = Query(pattern=r"^[a-f0-9]{64}$")):
        result = tester_read(lambda: tester.compare(left, right))
        if result is None:
            raise HTTPException(status_code=404, detail="Tester run not found")
        return result

    @app.get("/api/tester/runs/{run_id}")
    def api_tester_run(run_id: str):
        result = tester_read(lambda: tester.get(run_id))
        if result is None:
            raise HTTPException(status_code=404, detail="Tester run not found")
        return result

    @app.get('/api/reference')
    async def api_reference():
        return state.reference_snapshot() if hasattr(state, 'reference_snapshot') else {'status': 'UNAVAILABLE', 'sessions': {}}

    @app.get('/api/reference/replay')
    async def api_reference_replay():
        return build_reference_replay()

    @app.get('/api/reference/session')
    async def api_reference_session(symbol: str = Query(max_length=20), day: int = Query(ge=0, le=4_102_444_800_000), dataset: str | None = Query(default=None, max_length=100)):
        result = state.reference_chart(symbol, day, dataset) if hasattr(state, 'reference_chart') else None
        if result is None:
            raise HTTPException(status_code=404, detail='Recorded reference session not found')
        return result

    @app.get("/api/state")
    async def api_state():
        return state.snapshot()

    @app.get("/api/account")
    async def api_account():
        return state.account_snapshot() if hasattr(state, "account_snapshot") else state.account

    @app.get("/api/replay")
    async def api_replay():
        return replay_snapshot()

    @app.get("/api/research")
    async def api_research():
        return state.research_snapshot() if hasattr(state, "research_snapshot") else {"status":"UNAVAILABLE", "reports":[]}

    @app.get("/api/research/event")
    async def api_research_event(run_id: str = Query(max_length=100), candidate_id: str = Query(max_length=300)):
        result = state.research_event(run_id, candidate_id) if hasattr(state, "research_event") else None
        if result is None:
            raise HTTPException(status_code=404, detail="Recorded research event not found")
        return result

    @app.get("/api/lab")
    async def api_lab():
        return state.lab_snapshot() if hasattr(state, "lab_snapshot") else {"status":"UNAVAILABLE", "report":None, "shadow":{}}

    @app.get("/api/lab/chart")
    async def api_lab_chart(variant: str = Query(max_length=40), symbol: str = Query(max_length=20)):
        result = state.lab_chart(variant, symbol) if hasattr(state, "lab_chart") else None
        if result is None:
            raise HTTPException(status_code=404, detail="Unknown experimental variant or symbol")
        return result

    @app.get("/api/lab/events")
    async def api_lab_events(trial: str = Query(max_length=60), variant: str = Query(max_length=40)):
        result = state.lab_events(trial, variant) if hasattr(state, "lab_events") else None
        if result is None:
            raise HTTPException(status_code=404, detail="Unknown completed trial or variant")
        return result

    @app.websocket("/ws")
    async def socket(websocket: WebSocket):
        await websocket.accept()
        state.clients.add(websocket)
        try:
            await websocket.send_json(state.snapshot())
            while True:
                await websocket.receive_text()
        except Exception:
            pass
        finally:
            state.clients.discard(websocket)

    return app
