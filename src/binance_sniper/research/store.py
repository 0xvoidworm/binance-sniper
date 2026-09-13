from __future__ import annotations

import hashlib
import json
import sqlite3
import time
from dataclasses import asdict
from pathlib import Path

from binance_sniper.domain import Candle


def encoded(value) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)


def digest(value) -> str:
    return hashlib.sha256(encoded(value).encode()).hexdigest()


class ResearchStore:
    def __init__(self, path: Path):
        path.parent.mkdir(parents=True, exist_ok=True)
        self.path = path
        self.db = sqlite3.connect(path, timeout=15)
        self.db.row_factory = sqlite3.Row
        self.db.execute("PRAGMA journal_mode=WAL")
        self.db.execute("PRAGMA busy_timeout=15000")
        version = self.db.execute("PRAGMA user_version").fetchone()[0]
        if version not in (0, 1):
            raise ValueError("UNSUPPORTED_RESEARCH_SCHEMA")
        self.db.executescript("""
        CREATE TABLE IF NOT EXISTS datasets(id TEXT PRIMARY KEY, source TEXT NOT NULL, start_ms INTEGER, end_ms INTEGER, status TEXT, manifest TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS candles(dataset TEXT, symbol TEXT, tf TEXT, open_time INTEGER, close_time INTEGER, o REAL, h REAL, l REAL, c REAL, v REAL, checksum TEXT, PRIMARY KEY(dataset,symbol,tf,open_time));
        CREATE TABLE IF NOT EXISTS funding(dataset TEXT, symbol TEXT, at INTEGER, rate REAL, mark REAL, PRIMARY KEY(dataset,symbol,at));
        CREATE TABLE IF NOT EXISTS runs(id TEXT PRIMARY KEY, dataset TEXT NOT NULL, mode TEXT NOT NULL, strategy TEXT NOT NULL, parameters TEXT NOT NULL, started_at INTEGER, status TEXT, report TEXT);
        CREATE TABLE IF NOT EXISTS decisions(run TEXT, symbol TEXT, at INTEGER, stage TEXT, regime TEXT, reason TEXT, features TEXT, PRIMARY KEY(run,symbol,at));
        CREATE TABLE IF NOT EXISTS signals(run TEXT, id TEXT, symbol TEXT, at INTEGER, payload TEXT NOT NULL, PRIMARY KEY(run,id));
        CREATE TABLE IF NOT EXISTS outcomes(run TEXT, id TEXT, status TEXT, payload TEXT NOT NULL, PRIMARY KEY(run,id));
        CREATE INDEX IF NOT EXISTS decisions_stage ON decisions(run,stage);
        CREATE INDEX IF NOT EXISTS signals_time ON signals(run,at);
        PRAGMA user_version=1;
        """)

    def close(self):
        self.db.close()

    def dataset(self, key, source, start, end, status, manifest):
        with self.db:
            self.db.execute("INSERT INTO datasets VALUES(?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET status=excluded.status,manifest=excluded.manifest,end_ms=excluded.end_ms", (key,source,start,end,status,encoded(manifest)))

    def put_bars(self, dataset: str, candles: list[Candle]):
        values = [(dataset,c.symbol,c.timeframe,c.open_time,c.close_time,c.open,c.high,c.low,c.close,c.volume,digest(asdict(c))) for c in candles if c.closed]
        with self.db:
            self.db.executemany("INSERT OR IGNORE INTO candles VALUES(?,?,?,?,?,?,?,?,?,?,?)", values)

    def bars(self, dataset, symbol, tf) -> list[Candle]:
        rows = self.db.execute("SELECT * FROM candles WHERE dataset=? AND symbol=? AND tf=? ORDER BY open_time", (dataset,symbol,tf))
        return [Candle(symbol,tf,r['open_time'],r['close_time'],r['o'],r['h'],r['l'],r['c'],r['v'],True) for r in rows]

    def funding_rows(self, dataset, symbol):
        return [dict(r) for r in self.db.execute("SELECT at,rate,mark FROM funding WHERE dataset=? AND symbol=? ORDER BY at",(dataset,symbol))]

    def run(self, key, dataset, mode, strategy, parameters):
        with self.db:
            self.db.execute("INSERT OR IGNORE INTO runs VALUES(?,?,?,?,?,?,?,NULL)",(key,dataset,mode,strategy,encoded(parameters),int(time.time()*1000),'RUNNING'))

    def decisions(self, rows):
        with self.db:
            self.db.executemany("INSERT OR IGNORE INTO decisions VALUES(?,?,?,?,?,?,?)", rows)

    def signal(self, run, payload):
        with self.db:
            self.db.execute("INSERT OR IGNORE INTO signals VALUES(?,?,?,?,?)",(run,payload['candidate_id'],payload['symbol'],payload['decision_time'],encoded(payload)))

    def outcome(self, run, key, payload):
        with self.db:
            self.db.execute("INSERT INTO outcomes VALUES(?,?,?,?) ON CONFLICT(run,id) DO UPDATE SET status=excluded.status,payload=excluded.payload WHERE outcomes.status='PENDING'",(run,key,payload['status'],encoded(payload)))

    def finish(self, run, report, status='COMPLETED'):
        with self.db:
            self.db.execute("UPDATE runs SET status=?,report=? WHERE id=?",(status,encoded(report),run))

    def latest_reports(self):
        rows=self.db.execute("SELECT id,mode,strategy,status,started_at,report FROM runs WHERE report IS NOT NULL ORDER BY started_at DESC LIMIT 12")
        return [{**{k:r[k] for k in ('id','mode','strategy','status','started_at')},'report':json.loads(r['report'])} for r in rows]

    def event_view(self, run, candidate_id):
        row = self.db.execute('SELECT s.payload,r.dataset,r.mode,o.payload result FROM signals s JOIN runs r ON r.id=s.run LEFT JOIN outcomes o ON o.run=s.run AND o.id=s.id WHERE s.run=? AND s.id=?',(run,candidate_id)).fetchone()
        if row is None:
            return None
        signal=json.loads(row['payload'])
        at=signal['decision_time']
        horizon=max(1,min(32,int(signal.get('horizon',4))))
        rows=self.db.execute('SELECT * FROM candles WHERE dataset=? AND symbol=? AND tf=? AND open_time>=? AND close_time<=? ORDER BY open_time LIMIT 82',
                             (row['dataset'],signal['symbol'],'5m',at+1-50*300000,at+horizon*300000))
        return {'mode':row['mode'],'signal':signal,'outcome':json.loads(row['result']) if row['result'] else None,
                'bars':[{'open_time':r['open_time'],'close_time':r['close_time'],'open':r['o'],'high':r['h'],'low':r['l'],'close':r['c']} for r in rows],
                'horizon':horizon,'note':f'Hindsight M5: sampai {horizon} candle setelah sinyal. Ini visual hasil, bukan data yang tersedia saat keputusan.'}
