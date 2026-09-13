from __future__ import annotations

from fastapi import FastAPI, WebSocket
from fastapi.responses import HTMLResponse
from pathlib import Path


HTML = (Path(__file__).resolve().parents[2] / "web" / "index.html").read_text(encoding="utf-8")


def build_app(state):
    app = FastAPI(title="Binance Sniper", docs_url=None, redoc_url=None)

    @app.get("/", response_class=HTMLResponse)
    async def index():
        return HTMLResponse(HTML)

    @app.get("/api/state")
    async def api_state():
        return state.snapshot()

    @app.get("/api/account")
    async def api_account():
        return state.account

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
