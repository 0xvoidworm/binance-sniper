from __future__ import annotations

from contextlib import asynccontextmanager
from fastapi import FastAPI, WebSocket, HTTPException, Query
from fastapi.responses import HTMLResponse
from fastapi.staticfiles import StaticFiles
from pathlib import Path
from functools import lru_cache

from binance_sniper.replay import build_replay_snapshot


HTML = (Path(__file__).resolve().parents[2] / "web" / "index.html").read_text(encoding="utf-8")


@lru_cache(maxsize=1)
def replay_snapshot() -> dict:
    return build_replay_snapshot()


def build_app(state):
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
    async def index():
        return HTMLResponse(HTML)

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
