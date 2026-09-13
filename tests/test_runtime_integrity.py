import asyncio
import os
import time

import pytest

import binance_sniper.app as app_module
from binance_sniper.app import Runtime
from binance_sniper.market.bars import ClosedBarCache


def row(open_time, close, close_time):
    return [open_time, str(close-.5), str(close+1), str(close-1), str(close), '10', close_time, '0', 1, '0', '0', '0']


def test_preview_only_rest_response_is_success_but_empty_is_not():
    cache=ClosedBarCache('BTCUSDT','5m')
    cache.warmup([row(0,100,299999)],now=400000)
    assert cache.update_rest([row(300000,101,599999)],500000) is False
    assert cache.quality.last_success_at==500000
    assert cache.quality.last_preview_at==500000
    assert cache.quality.last_error_class is None
    assert cache.update_rest([],510000) is False
    assert cache.quality.last_success_at==500000
    assert cache.quality.last_error_class=='REST_SCHEMA_INVALID'


def test_derived_transport_does_not_let_ws_silence_override_fresh_rest():
    runtime=Runtime.__new__(Runtime); runtime.ws_state='WS_SILENT'
    fallback={str(i):{'transport':'LIVE_REST_FALLBACK'} for i in range(24)}
    mixed={**fallback,'0':{'transport':'STALE'}}
    assert runtime._derived_transport(fallback)=='LIVE_REST_FALLBACK'
    assert runtime._derived_transport(mixed)=='DEGRADED'


def test_account_snapshot_age_is_computed_at_read_time(monkeypatch):
    runtime=Runtime.__new__(Runtime)
    runtime.account={'auth':'CONNECTED','snapshot_at':1000,'regular_orders':0}
    runtime.account_success_at=1000; runtime.account_error_class=None
    monkeypatch.setattr(app_module.time,'time',lambda:2.5)
    assert runtime.account_snapshot()['snapshot_age_ms']==1500
    runtime.account_error_class='TimeoutError'
    assert runtime.account_snapshot()['auth']=='STALE'


def test_stale_lock_is_recovered_but_live_lock_is_refused(tmp_path, monkeypatch):
    runtime=Runtime.__new__(Runtime); runtime.state_dir=tmp_path
    runtime.pid_file=tmp_path/'service.pid';runtime.lock_file=tmp_path/'service.lock'
    runtime.lock_file.write_text('999999',encoding='ascii')
    monkeypatch.setattr(runtime,'_pid_alive',lambda pid:False)
    runtime._claim_instance()
    assert runtime.lock_file.read_text(encoding='ascii')==str(os.getpid())
    runtime._release_instance()
    runtime.lock_file.write_text('1234',encoding='ascii')
    monkeypatch.setattr(runtime,'_pid_alive',lambda pid:True)
    with pytest.raises(RuntimeError,match='INSTANCE_ALREADY_RUNNING'): runtime._claim_instance()


@pytest.mark.asyncio
async def test_supervisor_records_and_restarts_failed_task(monkeypatch):
    runtime=Runtime.__new__(Runtime);runtime.task_registry={};runtime.stopping=False
    attempts=0
    async def no_wait(_): return None
    monkeypatch.setattr(app_module.asyncio,'sleep',no_wait)
    async def factory():
        nonlocal attempts
        attempts+=1
        if attempts==1: raise RuntimeError('boom')
        runtime.stopping=True
    await runtime._supervise('test',factory)
    assert attempts==2
    assert runtime.task_registry['test']['restarts']==1
    assert runtime.task_registry['test']['status']=='STOPPED'
