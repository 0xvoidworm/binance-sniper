from dataclasses import asdict, replace
import copy
import json

import pytest

from binance_sniper.domain import Candle
from binance_sniper.round01.data import DAY, H4
from binance_sniper.round01.engine import CONFIGS, COSTS, ChannelEngine, fill_costs, funding_charge, project_frame
from binance_sniper.round01.study import bootstrap_lower, economic_gates, summarize, run_partition


def candle(i, o=100., c=100., h=None, l=None, closed=True):
    return Candle("BTCUSDT", "4h", i * H4, (i + 1) * H4 - 1, o,
                  max(o, c) + 1 if h is None else h, min(o, c) - 1 if l is None else l, c, 10., closed)


def warmed(config=CONFIGS[0], costs=COSTS[0]):
    engine = ChannelEngine("BTCUSDT", config, costs)
    for i in range(60):
        engine.step(candle(i), trading=False)
    return engine


def signaled(direction=1, costs=COSTS[0]):
    engine = warmed(costs=costs)
    engine.step(candle(60, c=104. if direction == 1 else 96.))
    assert engine.pending["direction"] == direction
    assert engine.position is None
    return engine


@pytest.mark.parametrize("config", CONFIGS)
def test_prefix_invariance_and_projection(config):
    bars = [candle(i) for i in range(60)] + [candle(60, c=104), candle(61, o=104, c=106, l=103), candle(62, o=106, c=107, l=105), candle(63, o=107, c=98)]
    full = ChannelEngine("BTCUSDT", config, COSTS[0])
    states, frames = [], []
    for i, b in enumerate(bars):
        frames.append(full.step(b, trading=i >= 60))
        states.append(full.state())
    for length in range(60, len(bars) + 1):
        prefix = ChannelEngine("BTCUSDT", config, COSTS[0])
        for i, b in enumerate(bars[:length]):
            frame = prefix.step(b, trading=i >= 60)
        assert prefix.state() == states[length - 1]
        projected = project_frame(frame)
        for key, value in projected.items():
            assert value == frames[length - 1][key]
        projected["events"].append({"invented": True})
        assert prefix.last_frame["events"] == frames[length - 1]["events"]


def test_preview_and_premature_closed_candle_do_not_advance():
    engine = warmed()
    before = engine.state()
    assert engine.step(candle(60, c=1000, closed=False)) is None
    assert engine.state() == before
    with pytest.raises(ValueError, match="UNAVAILABLE"):
        engine.step(candle(60, c=104), available_at=61 * H4 - 1)
    assert engine.state() == before


def test_signal_uses_prior_channel_not_current_extreme_and_next_open_fill():
    engine = signaled()
    signal = engine.pending.copy()
    assert signal["signal_at"] == 61 * H4
    frame = engine.step(candle(61, o=107, c=108, h=109, l=106))
    entry = next(e for e in frame["events"] if e["kind"] == "ENTRY")
    assert entry["raw_entry"] == 107
    assert entry["entry_at"] == signal["signal_at"]
    assert entry["entry_fill"] > 107
    assert frame["data_available_at"] == 62 * H4


def test_intrabar_high_low_breakout_without_close_does_not_enter():
    engine = warmed()
    engine.step(candle(60, h=200, l=50))
    assert engine.pending is None and engine.position is None


@pytest.mark.parametrize("direction", (1, -1))
def test_stop_gap_uses_worse_available_open(direction):
    engine = signaled(direction)
    price = 104 if direction == 1 else 96
    engine.step(candle(61, o=price, c=price, h=price + 1, l=price - 1))
    stop = engine.position.initial_stop
    gap = stop - 8 if direction == 1 else stop + 8
    engine.step(candle(62, o=gap, c=gap))
    trade = engine.trades[0]
    assert trade["exit_reason"] == "GAP_STOP"
    assert trade["raw_exit"] == gap
    assert trade["net_r"] < -1


@pytest.mark.parametrize("direction", (1, -1))
def test_entry_and_stop_same_bar_conservative_funding(direction):
    engine = signaled(direction)
    price = 104 if direction == 1 else 96
    rates = [{"at": 61 * H4, "rate": -.001 * direction, "mark": price}]
    frame = engine.step(candle(61, o=price, c=price, h=120, l=80), rates)
    trade = engine.trades[0]
    assert trade["entry_bar_stop"]
    assert trade["funding_cost"] == 0  # uncertain credit is not granted
    assert trade["gross"] < 0
    assert trade["fee_cost"] > 0 and trade["slippage_cost"] > 0
    assert frame["events"][0]["kind"] == "ENTRY"
    assert frame["events"][1]["kind"] == "EXIT"


@pytest.mark.parametrize("direction", (1, -1))
@pytest.mark.parametrize("rate", (.001, -.001))
def test_funding_long_short_and_boundary_debit(direction, rate):
    normal = funding_charge(2, direction, rate, 100)
    assert normal == pytest.approx(200 * direction * rate)
    assert funding_charge(2, direction, rate, 100, True) == max(normal, 0)


@pytest.mark.parametrize("direction", (1, -1))
def test_fees_and_slippage_reconcile_fill_pnl(direction):
    base = fill_costs(100, 110, direction, 2, COSTS[0])
    stress = fill_costs(100, 110, direction, 2, COSTS[1])
    assert base["gross"] - base["slippage_cost"] == pytest.approx(2 * direction * (base["exit_fill"] - base["entry_fill"]))
    assert base["fee_cost"] == pytest.approx(.0005 * 2 * (base["entry_fill"] + base["exit_fill"]))
    assert stress["slippage_cost"] > base["slippage_cost"]
    assert stress["fee_cost"] > base["fee_cost"]


def test_new_bar_extremes_cannot_tighten_stop_inside_that_bar():
    engine = signaled()
    engine.step(candle(61, o=104, c=105, h=106, l=103))
    initial = engine.position.initial_stop
    frame = engine.step(candle(62, o=105, c=190, h=200, l=104))
    assert frame["stop_applied_this_bar"] == initial
    assert engine.position is not None  # not an invented high-minus-ATR trailing stop


def test_channel_stop_is_computed_before_bar_and_tightens():
    engine = signaled()
    for i in range(61, 72):
        engine.step(candle(i, o=106, c=106, h=107, l=103 + (i - 61) * .1))
    frame = engine.step(candle(72, o=106, c=102, h=107, l=101))
    assert frame["stop_applied_this_bar"] == pytest.approx(103.1)
    assert engine.trades[-1]["exit_reason"] == "CHANNEL_STOP"
    assert engine.trades[-1]["raw_exit"] == pytest.approx(103.1)


def test_gap_missing_duplicate_and_revision_behavior():
    engine = signaled()
    before = engine.state()
    with pytest.raises(ValueError, match="NONCONTIGUOUS"):
        engine.step(candle(62))
    assert engine.state() == before
    b = candle(61, o=104, c=104, l=103)
    frame = engine.step(b)
    after = engine.state()
    assert engine.step(b) == frame
    assert engine.state() == after
    with pytest.raises(ValueError, match="REVISION"):
        engine.step(replace(b, high=999))
    assert engine.state() == after


def test_partition_liquidation_is_explicit_and_not_end_of_prefix():
    engine = signaled()
    engine.step(candle(61, o=104, c=105, l=103))
    assert engine.position is not None
    frame = engine.step(candle(62, o=105, c=106, l=104), force_close=True)
    assert engine.position is None and engine.pending is None
    assert engine.trades[0]["exit_reason"] == "PARTITION_END"
    assert frame["index_pnl_mtm"] == pytest.approx(engine.trades[0]["net"])
    another = signaled()
    another.step(candle(61, o=104, c=104), force_close=True)
    assert another.trades == [] and another.position is None and another.pending is None


def test_stop_precedes_partition_close():
    engine = signaled()
    engine.step(candle(61, o=104, c=104, l=103))
    engine.step(candle(62, o=104, c=110, h=111, l=80), force_close=True)
    assert engine.trades[0]["exit_reason"] == "INITIAL_STOP"


def test_invalid_funding_or_duplicate_is_rejected_before_state_change():
    engine = signaled()
    before = engine.state()
    row = {"at": 61 * H4, "rate": .001, "mark": 100}
    with pytest.raises(ValueError, match="DUPLICATE_FUNDING"):
        engine.step(candle(61), [row, row])
    assert engine.state() == before
    with pytest.raises(ValueError, match="INVALID_FUNDING"):
        engine.step(candle(61), [{**row, "mark": float("nan")}])
    assert engine.state() == before


def test_funding_equity_summary_reconciliation_with_mtm_drawdown():
    engine = signaled()
    frames = []
    for i in range(61, 66):
        price = 104 if i < 63 else 102
        rows = [{"at": i * H4, "rate": .001, "mark": price}] if i == 62 else []
        frames.append(engine.step(candle(i, o=price, c=price, l=price - .5), rows, force_close=i == 65))
    # Summary requires complete UTC days; a one-day synthetic equity tape suffices.
    eq = [{"at": (i + 1) * H4, "equity_index": 1 + engine.trades[0]["net"]} for i in range(60, 66)]
    eq[0]["equity_index"] = 1.02
    summary, daily = summarize(engine.trades, eq, engine.funding_ledger, 60 * H4, 66 * H4)
    assert summary["funding_cost"] > 0
    assert summary["final_equity_index"] - 1 == pytest.approx(summary["net"])
    assert summary["max_mtm_drawdown_fraction"] > .01
    broken = copy.deepcopy(engine.trades)
    broken[0]["net"] += .1
    with pytest.raises(AssertionError, match="COMPONENT_RECONCILIATION"):
        summarize(broken, eq, engine.funding_ledger, 60 * H4, 66 * H4)


def test_bootstrap_is_deterministic_and_includes_zero_days():
    daily = [{"day": i * DAY, "net_change": .001 if i % 8 == 0 else 0} for i in range(60)]
    a = bootstrap_lower(daily)
    assert a == bootstrap_lower(json.loads(json.dumps(daily)))
    assert a >= 0 and a < .001


def test_gates_do_not_treat_positive_net_as_edge():
    base = {"trade_count": 40, "net": .02, "pf": 1.3, "per_symbol": {s: {"net": .01, "trade_count": 20} for s in ("BTCUSDT", "ETHUSDT")},
            "subperiod_net": [.02, -.01, .01], "max_mtm_drawdown_fraction": .05, "top3_positive_share": .8,
            "bootstrap_daily_mean_lower5": -.00001}
    result = economic_gates(base, base, 30, 10)
    assert not result["pass"] and result["sample_sufficient"]
    assert set(result["failed"]) == {"concentration", "bootstrap_lower5_positive"}


def test_missing_funding_fails_before_running_strategy(monkeypatch):
    from binance_sniper.round01 import study
    monkeypatch.setitem(study.PARTITIONS, "SYNTHETIC", ("1970-01-02", "1970-01-03"))
    data = {"candles": {s: [asdict(replace(candle(i), symbol=s)) for i in range(6 - 220, 12)] for s in ("BTCUSDT", "ETHUSDT")},
            "funding": {s: [] for s in ("BTCUSDT", "ETHUSDT")}}
    with pytest.raises(ValueError, match="FUNDING_COVERAGE"):
        run_partition(data, CONFIGS[0], COSTS[0], "SYNTHETIC")
