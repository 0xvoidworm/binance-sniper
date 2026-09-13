from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import math
from pathlib import Path

from binance_sniper.binance.public_rest import BinancePublicRest
from binance_sniper.config import Settings
from binance_sniper.market.bars import ClosedBarCache, INTERVAL_MS
from binance_sniper.research.store import ResearchStore, encoded

DAY=86_400_000
ROOT=Path(__file__).resolve().parents[3]


async def download(days=90):
    settings=Settings.load(ROOT/'config/settings.yaml')
    public=BinancePublicRest(settings.rest_base)
    store=ResearchStore(ROOT/'state/research.sqlite3')
    now=await public.server_time()
    end=now//DAY*DAY
    start=end-days*DAY
    key=f'binance-{start}-{end}-v1'
    existing=store.db.execute('SELECT status,manifest FROM datasets WHERE id=?',(key,)).fetchone()
    if existing and existing['status']=='COMPLETE':
        print(encoded({'dataset':key,'cached':True,'manifest':json.loads(existing['manifest'])}),flush=True)
        store.close();await public.close();return key
    manifest={'source':'https://fapi.binance.com/fapi/v1/klines','start_ms':start,'end_ms':end,'days':days,'warmup_bars':220,'feeds':{},'funding':{},'downloaded_at':now}
    store.dataset(key,'BINANCE_REST',start,end,'DOWNLOADING',manifest)
    async def pace():
        if (public.used_weight_1m or 0)>1200:
            await asyncio.sleep(61-(public.last_request_at//1000)%60)
        else:
            await asyncio.sleep(.15)
    try:
        for symbol in settings.symbols:
            for tf in settings.timeframes:
                interval=INTERVAL_MS[tf]
                lower=start-220*interval
                cached=store.bars(key,symbol,tf)
                cursor=cached[-1].open_time+interval if cached else lower
                parser=ClosedBarCache(symbol,tf)
                while cursor<end:
                    await pace()
                    for attempt in range(3):
                        try:
                            rows=await public.klines(symbol,tf,1000,cursor,end-1)
                            break
                        except Exception:
                            if attempt==2:raise
                            await asyncio.sleep(2**attempt)
                    if not rows:break
                    parsed=[parser._parse(row,True) for row in rows]
                    if any(c is None for c in parsed):raise ValueError('HISTORICAL_INVALID_CANDLE')
                    candles=[c for c in parsed if lower<=c.open_time and c.close_time<end]
                    if not candles or candles[-1].open_time<cursor:raise ValueError('HISTORICAL_PAGINATION_STALLED')
                    store.put_bars(key,candles)
                    cursor=candles[-1].open_time+interval
                bars=store.bars(key,symbol,tf)
                expected=(end-lower)//interval
                complete=len(bars)==expected and bars[0].open_time==lower and all(b.open_time-a.open_time==interval for a,b in zip(bars,bars[1:]))
                checksums=store.db.execute('SELECT checksum FROM candles WHERE dataset=? AND symbol=? AND tf=? ORDER BY open_time',(key,symbol,tf))
                checksum=hashlib.sha256(''.join(r[0] for r in checksums).encode()).hexdigest()
                manifest['feeds'][symbol+':'+tf]={'bars':len(bars),'expected':expected,'complete':complete,'sha256':checksum}
                if not complete:raise ValueError('HISTORICAL_COVERAGE_INCOMPLETE '+symbol+':'+tf)
                print(encoded({'feed':symbol+':'+tf,'bars':len(bars),'complete':complete}),flush=True)
                store.dataset(key,'BINANCE_REST',start,end,'DOWNLOADING',manifest)
            # Historical funding is public and only used in the outcome model.
            await pace()
            try:
                response=await public.client.get('/fapi/v1/fundingRate',params={'symbol':symbol,'startTime':start,'endTime':end-1,'limit':1000})
                public._record(response);response.raise_for_status();rows=response.json()
                if not isinstance(rows,list) or len(rows)>=1000:raise ValueError('FUNDING_COVERAGE_UNCERTAIN')
                values=[]
                for row in rows:
                    at,rate,mark=int(row['fundingTime']),float(row['fundingRate']),float(row['markPrice'])
                    if not math.isfinite(rate) or not math.isfinite(mark) or mark<=0:raise ValueError('FUNDING_SCHEMA_INVALID')
                    if start<=at<end:values.append((key,symbol,at,rate,mark))
                with store.db:store.db.executemany('INSERT OR IGNORE INTO funding VALUES(?,?,?,?,?)',values)
                manifest['funding'][symbol]={'status':'AVAILABLE','rows':len(values)}
            except Exception as exc:
                manifest['funding'][symbol]={'status':'UNAVAILABLE','reason':type(exc).__name__}
        manifest['sha256']=hashlib.sha256(encoded(manifest['feeds']).encode()).hexdigest()
        store.dataset(key,'BINANCE_REST',start,end,'COMPLETE',manifest)
        print(encoded({'dataset':key,'status':'COMPLETE','total_bars':sum(v['bars'] for v in manifest['feeds'].values())}),flush=True)
        return key
    except Exception as exc:
        manifest['error']=type(exc).__name__
        store.dataset(key,'BINANCE_REST',start,end,'FAILED',manifest)
        raise
    finally:
        store.close();await public.close()


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--days',type=int,default=90)
    args=parser.parse_args()
    if not 7<=args.days<=180:parser.error('days must be 7..180')
    asyncio.run(download(args.days))
