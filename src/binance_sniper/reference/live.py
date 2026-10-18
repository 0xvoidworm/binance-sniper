"""Durable closed-bar reference observer. Never a trading executor."""
from __future__ import annotations

from dataclasses import asdict
import json
from pathlib import Path
import sqlite3

from binance_sniper.domain import Candle
from binance_sniper.reference.engine import DAY, Rules, evaluate_session, specification


class MarketView:
    """Market snapshots without running the retired pattern scanner."""
    def __init__(self, cache, settings):
        self.cache, self.settings = cache, settings
        self.reconnects = 0

    def evaluate(self, symbol):
        pass

    def evaluate_all(self):
        pass

    def snapshot(self):
        charts = {}
        for (symbol, timeframe), feed in self.cache.caches.items():
            preview = feed.preview_value()
            charts[f'{symbol}:{timeframe}'] = {
                'closed': [asdict(b) for b in feed.values()[-(400 if timeframe == '5m' else 80):]],
                'preview': asdict(preview) if preview else None}
        return {'symbols': {s: {'symbol': s, 'status': 'REFERENCE_ENGINE'} for s in self.settings.symbols},
                'charts': charts, 'candidates': [], 'events': [], 'zones': {}, 'outcomes': {},
                'feed_quality': self.cache.quality(), 'reconnects': self.reconnects,
                'active_engine': 'RANGE_BREAKOUT_REFERENCE_V1', 'legacy_scanner': 'RETIRED'}


class ReferenceMonitor:
    def __init__(self, state_dir: Path, settings, now: int):
        self.state_dir, self.settings, self.started = state_dir, settings, now
        self.rules = Rules()
        self.spec = specification(self.rules)
        self.key = self.spec['id']
        state_dir.mkdir(parents=True, exist_ok=True)
        self.db = sqlite3.connect(state_dir / 'reference.sqlite3', timeout=.2)
        self.db.row_factory = sqlite3.Row
        self.db.execute('PRAGMA journal_mode=WAL')
        self.db.executescript('''
        CREATE TABLE IF NOT EXISTS bars(symbol TEXT, at INTEGER, payload TEXT, PRIMARY KEY(symbol,at));
        CREATE TABLE IF NOT EXISTS sessions(engine TEXT, id TEXT, day INTEGER, status TEXT, payload TEXT,
          PRIMARY KEY(engine,id));
        ''')
        self.snapshot = {'status': 'STARTING', 'specification': self.spec, 'sessions': {}, 'recent': []}
        self.history = self._history()
        self.archived = self._archived()

    def _history(self):
        path = self.state_dir / 'reference-history.json'
        if not path.exists():
            return None
        report = json.loads(path.read_text(encoding='utf-8'))
        if report.get('specification', {}).get('id') != self.key:
            return {'status': 'SOURCE_CHANGED', 'note': 'Hasil terdahulu tidak cocok dengan versi engine ini.'}
        return report

    def _archived(self):
        path = self.state_dir / 'research.sqlite3'
        if not path.exists():
            return {'status': 'ARCHIVED', 'report': None}
        connection = sqlite3.connect(path.as_uri() + '?mode=ro', uri=True, timeout=.2)
        try:
            row = connection.execute("SELECT report FROM lab_trials WHERE status='COMPLETED' ORDER BY rowid DESC LIMIT 1").fetchone()
            return {'status': 'ARCHIVED', 'report': json.loads(row[0]) if row else None,
                    'shadow': {}, 'note': 'Strategi lama dihentikan. Hasil negatif dipertahankan, bukan mesin aktif.'}
        except sqlite3.OperationalError:
            return {'status': 'ARCHIVED', 'report': None}
        finally:
            connection.close()

    def capture(self, runtime, now):
        day = now // DAY * DAY
        sessions = {}
        with self.db:
            for symbol in self.settings.symbols:
                bars = [b for b in runtime.cache.candles(symbol, '5m') if b.closed and b.close_time < now]
                self.db.executemany('''INSERT INTO bars VALUES(?,?,?) ON CONFLICT(symbol,at)
                    DO UPDATE SET payload=excluded.payload WHERE bars.payload<>excluded.payload''',
                    [(symbol, b.open_time, json.dumps(asdict(b), sort_keys=True)) for b in bars])
                rows = self.db.execute('SELECT payload FROM bars WHERE symbol=? AND at>=? AND at<? ORDER BY at',
                                       (symbol, day, day+DAY)).fetchall()
                today = [Candle(**json.loads(r['payload'])) for r in rows]
                observed_until = min(now, today[-1].close_time+1) if today else min(now, day)
                result = evaluate_session(symbol, today, day, observed_until, self.rules)
                previous = self.db.execute('SELECT payload FROM sessions WHERE engine=? AND id=?',
                                           (self.key, result['id'])).fetchone()
                previous = json.loads(previous['payload']) if previous else None
                if previous and previous.get('entry') is not None:
                    result['origin'] = previous.get('origin', 'RECONSTRUCTED')
                else:
                    result['origin'] = ('LIVE_OBSERVED' if result.get('entry_bar_open', -1) >= self.started
                                        and now-result.get('last_bar_at', 0) < 360_000 else 'RECONSTRUCTED')
                result['observed_at'] = now
                if previous and previous['status'] in {'CLOSED', 'NO_BREAKOUT', 'AMBIGUOUS'}:
                    fields = ('status', 'entry', 'stop', 'exit', 'reason', 'after_cost_r')
                    revised = any(previous.get(k) != result.get(k) for k in fields)
                    result = {**previous, 'revision_warning': revised}
                self.db.execute('''INSERT INTO sessions VALUES(?,?,?,?,?) ON CONFLICT(engine,id)
                    DO UPDATE SET status=excluded.status,payload=excluded.payload''',
                    (self.key, result['id'], day, result['status'], json.dumps(result, sort_keys=True)))
                feed = runtime.cache.caches[(symbol, '5m')]
                result = {**result, 'data_age_ms': max(0, now-feed.quality.last_success_at) if feed.quality.last_success_at else None,
                          'decision_lag_ms': max(0, now-result['last_bar_at']) if result.get('last_bar_at') else None,
                          'observed_until': observed_until}
                sessions[symbol] = result
        recent = [json.loads(r['payload']) for r in self.db.execute(
            'SELECT payload FROM sessions WHERE engine=? ORDER BY day DESC,id LIMIT 60', (self.key,))]
        self.snapshot = {'status': 'READ_ONLY_MODEL', 'updated_at': now, 'started_at': self.started,
                         'specification': self.spec, 'sessions': sessions, 'recent': recent,
                         'history': self.history, 'legacy': {'status': 'ARCHIVED', 'live_loops': False},
                         'note': 'Model closed-M5, bukan fill/order broker. Funding live belum tersedia.'}

    def close(self):
        self.db.close()

    def chart(self, symbol, day, dataset=None):
        if symbol not in self.settings.symbols or day % DAY:
            return None
        if dataset:
            allowed = {d['id']: d for d in (self.history or {}).get('datasets', [])}
            source = allowed.get(dataset)
            if not source or not source['start'] <= day < source['end']:
                return None
            path = self.state_dir / 'research.sqlite3'
            connection = sqlite3.connect(path.as_uri()+'?mode=ro', uri=True, timeout=.2)
            connection.row_factory = sqlite3.Row
            try:
                rows = connection.execute("SELECT * FROM candles WHERE dataset=? AND symbol=? AND tf='5m' AND open_time>=? AND open_time<? ORDER BY open_time LIMIT 288", (dataset, symbol, day, day+DAY)).fetchall()
                bars = [Candle(symbol, '5m', r['open_time'], r['close_time'], r['o'], r['h'], r['l'], r['c'], r['v'], True) for r in rows]
                funding = [dict(r) for r in connection.execute('SELECT at,rate,mark FROM funding WHERE dataset=? AND symbol=? AND at>=? AND at<? ORDER BY at', (dataset, symbol, day, day+DAY))]
            finally:
                connection.close()
            if not bars or not funding:
                return None
            result = {**evaluate_session(symbol, bars, day, day+DAY, self.rules, funding),
                      'origin': 'HISTORICAL_RECONSTRUCTION', 'dataset': dataset}
        else:
            path = self.state_dir / 'reference.sqlite3'
            connection = sqlite3.connect(path.as_uri()+'?mode=ro', uri=True, timeout=.2)
            connection.row_factory = sqlite3.Row
            try:
                row = connection.execute('SELECT payload FROM sessions WHERE engine=? AND id=?', (self.key, f'{symbol}:{day}')).fetchone()
                rows = connection.execute('SELECT payload FROM bars WHERE symbol=? AND at>=? AND at<? ORDER BY at LIMIT 288', (symbol, day, day+DAY)).fetchall()
            finally:
                connection.close()
            if not row:
                return None
            result = json.loads(row['payload'])
            bars = [Candle(**json.loads(r['payload'])) for r in rows]
        # Preserve all observed session data including post-exit bars as explicitly hindsight evidence.
        return {'synthetic': False, 'historical': True, 'snapshot_at': bars[-1].close_time+1 if bars else result['as_of'],
                'symbols': {symbol: {}}, 'charts': {symbol+':5m': {'closed': [asdict(b) for b in bars], 'preview': None}},
                'feed_quality': {}, 'reference': {'status': 'HISTORICAL_RECONSTRUCTION', 'specification': self.spec,
                'sessions': {symbol: result}, 'recent': [result], 'history': self.history},
                'note': 'Rekonstruksi closed-M5, termasuk candle setelah exit. Bukan sinyal live atau order.'}
