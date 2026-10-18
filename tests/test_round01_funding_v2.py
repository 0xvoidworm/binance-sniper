"""Ownership-time regressions. All candles/rates here are synthetic, never cache data."""
from dataclasses import asdict, replace

import pytest

from binance_sniper.domain import Candle
from binance_sniper.round01.data import H4, SYMBOLS
from binance_sniper.round01.engine import (
    CONFIGS, COSTS, FUNDING_POLICY_VERSION, ChannelEngine,
    funding_ownership_policy, project_frame,
)
from binance_sniper.round01 import study


def bar(i, price=100., *, high=None, low=None, close=None):
    close = price if close is None else close
    return Candle("BTCUSDT", "4h", i * H4, (i + 1) * H4 - 1, price,
                  max(price, close) + 1 if high is None else high,
                  min(price, close) - 1 if low is None else low, close, 10., True)


def ready(direction):
    engine = ChannelEngine("BTCUSDT", CONFIGS[0], COSTS[0])
    for i in range(60):
        engine.step(bar(i), trading=False)
    price = 100 + direction * 4
    engine.step(bar(60, close=price))
    assert engine.pending["direction"] == direction
    return engine, price


def row(at, rate, price):
    return {"at": at, "rate": rate, "mark": price}


@pytest.mark.parametrize("at,earliest,latest,policy", [
    (99, None, None, "BEFORE_ENTRY"),
    (100, None, None, "DEBIT_ONLY_BOUNDARY"),
    (101, None, None, "NORMAL"),
    (199, 200, 200, "NORMAL"),
    (200, 200, 200, "DEBIT_ONLY_BOUNDARY"),
    (201, 200, 200, "AFTER_EXIT"),
    (199, 200, 300, "NORMAL"),
    (200, 200, 300, "DEBIT_ONLY_BOUNDARY"),
    (250, 200, 300, "DEBIT_ONLY_BOUNDARY"),
    (300, 200, 300, "DEBIT_ONLY_BOUNDARY"),
    (301, 200, 300, "AFTER_EXIT"),
    (100, 100, 100, "DEBIT_ONLY_BOUNDARY"),
    (101, 100, 100, "AFTER_EXIT"),
])
def test_ownership_boundaries(at, earliest, latest, policy):
    assert funding_ownership_policy(at, 100, earliest, latest) == policy


@pytest.mark.parametrize("earliest,latest", [(None, 200), (200, None), (99, 200), (201, 200)])
def test_invalid_ownership_interval_rejected(earliest, latest):
    with pytest.raises(ValueError, match="EXIT_INTERVAL"):
        funding_ownership_policy(150, 100, earliest, latest)


@pytest.mark.parametrize("direction", (1, -1))
@pytest.mark.parametrize("rate", (.001, -.001))
def test_funding_before_entry_does_not_accrue_to_future_position(direction, rate):
    engine = ChannelEngine("BTCUSDT", CONFIGS[0], COSTS[0])
    for i in range(60):
        engine.step(bar(i), trading=False)
    price = 100 + direction * 4
    engine.step(bar(60, close=price), [row(61 * H4 - 1, rate, price)])
    assert engine.pending and engine.position is None
    assert engine.funding_ledger == []
    engine.step(bar(61, price))
    assert engine.position.funding_cost == 0


@pytest.mark.parametrize("direction", (1, -1))
@pytest.mark.parametrize("rate", (.001, -.001))
@pytest.mark.parametrize("offset", (0, 1, 999, 3_600_000))
def test_entry_timestamp_only_is_ambiguous_when_no_stop(direction, rate, offset):
    engine, price = ready(direction)
    candle = bar(61, price)
    rates = [row(candle.open_time + offset, rate, price)]
    frame = engine.step(candle, rates)
    charge = .5 * direction * rate
    expected = max(0., charge) if offset == 0 else charge
    assert engine.position.funding_cost == pytest.approx(expected)
    posting = engine.funding_ledger[0]
    assert posting["ambiguous"] == (offset == 0)
    assert posting["policy"] == ("DEBIT_ONLY_BOUNDARY" if offset == 0 else "NORMAL")
    assert posting["funding_policy_version"] == FUNDING_POLICY_VERSION
    assert project_frame(frame)["funding_policy_version"] == FUNDING_POLICY_VERSION
    saved = engine.state()
    assert engine.step(candle, rates) == frame
    assert engine.state() == saved  # duplicate delivery never doubles a debit or credit


@pytest.mark.parametrize("direction", (1, -1))
@pytest.mark.parametrize("rate", (.001, -.001))
@pytest.mark.parametrize("offset", (0, 1, 999, 3_600_000, H4 - 1))
def test_gap_open_never_posts_after_known_exit(direction, rate, offset):
    engine, price = ready(direction)
    engine.step(bar(61, price))
    gap_price = engine.position.initial_stop - direction * 8
    candle = bar(62, gap_price)
    rates = [row(candle.open_time + offset, rate, gap_price)]
    frame = engine.step(candle, rates)
    trade, posting = engine.trades[0], engine.funding_ledger[0]
    raw = .5 / price * direction * rate * gap_price
    expected = max(0., raw) if offset == 0 else 0.
    assert trade["exit_reason"] == "GAP_STOP"
    assert trade["raw_exit"] == gap_price
    assert trade["exit_earliest_at"] == trade["exit_latest_at"] == candle.open_time
    assert posting["policy"] == ("DEBIT_ONLY_BOUNDARY" if offset == 0 else "AFTER_EXIT")
    assert posting["raw_charge"] == pytest.approx(raw)
    assert posting["funding_cost"] == pytest.approx(expected)
    assert trade["funding_cost"] == pytest.approx(expected)
    assert trade["net"] == pytest.approx(trade["gross"] - trade["slippage_cost"] - trade["fee_cost"] - expected)
    assert frame["index_pnl_mtm"] == pytest.approx(trade["net"])
    assert trade["funding_policy_version"] == FUNDING_POLICY_VERSION
    saved = engine.state()
    assert engine.step(candle, rates) == frame
    assert engine.state() == saved


@pytest.mark.parametrize("direction", (1, -1))
@pytest.mark.parametrize("rate", (.001, -.001))
@pytest.mark.parametrize("entry_bar", (True, False))
@pytest.mark.parametrize("offset", (0, 1, 999, 3_600_000, H4 - 1))
def test_unknown_intrabar_stop_keeps_debits_and_denies_credits(direction, rate, entry_bar, offset):
    engine, price = ready(direction)
    i = 61
    if not entry_bar:
        engine.step(bar(i, price))
        i += 1
    candle = bar(i, price, high=120., low=80.)
    engine.step(candle, [row(candle.open_time + offset, rate, price)])
    trade, posting = engine.trades[0], engine.funding_ledger[0]
    assert trade["intrabar_time_unknown"] and trade["entry_bar_stop"] == entry_bar
    assert posting["policy"] == "DEBIT_ONLY_BOUNDARY" and posting["ambiguous"]
    assert trade["funding_cost"] == pytest.approx(max(0., .5 * direction * rate))
    assert trade["exit_earliest_at"] == candle.open_time
    assert trade["exit_latest_at"] == candle.close_time + 1


@pytest.mark.parametrize("direction", (1, -1))
@pytest.mark.parametrize("rate", (.001, -.001))
def test_held_funding_before_scheduled_exit_is_normal_and_next_bar_flat(direction, rate):
    engine, price = ready(direction)
    engine.step(bar(61, price))
    candle = bar(62, price)
    rates = [row(candle.open_time + offset, rate, price) for offset in (0, 999, H4 - 1)]
    frame = engine.step(candle, rates, force_close=True)
    trade = engine.trades[0]
    assert trade["exit_reason"] == "PARTITION_END"
    assert all(f["policy"] == "NORMAL" for f in engine.funding_ledger)
    assert trade["funding_cost"] == pytest.approx(3 * .5 * direction * rate)
    assert frame["index_pnl_mtm"] == pytest.approx(trade["net"])
    postings = list(engine.funding_ledger)
    engine.step(bar(63, price), [row(63 * H4, rate, price), row(63 * H4 + 1, rate, price)])
    assert engine.funding_ledger == postings  # no open position at next delivery


def synthetic_runner_data(direction, rate, offset):
    price = 100 + direction * 4
    candles = [bar(i) for i in range(60 - 220, 60)]
    # The synthetic open is deliberately beyond both the initial and channel stops.
    gap_price = 90. if direction == 1 else 110.
    candles += [bar(60, close=price), bar(61, price), bar(62, gap_price)]
    candles += [bar(i, gap_price) for i in range(63, 66)]
    return {"candles": {s: [asdict(replace(c, symbol=s)) for c in candles] for s in SYMBOLS},
            "funding": {s: [row(i * H4 + offset, rate if i == 62 else 0., gap_price)
                            for i in (60, 62, 64)] for s in SYMBOLS}}


@pytest.mark.parametrize("direction", (1, -1))
@pytest.mark.parametrize("rate", (.001, -.001))
@pytest.mark.parametrize("offset", (1, 999))
def test_runner_accepts_legal_jitter_and_excludes_post_gap_funding(monkeypatch, direction, rate, offset):
    monkeypatch.setitem(study.PARTITIONS, "SYNTHETIC", ("1970-01-11", "1970-01-12"))
    data = synthetic_runner_data(direction, rate, offset)
    result = study.run_partition(data, CONFIGS[0], COSTS[0], "SYNTHETIC")
    assert result["funding_policy_version"] == FUNDING_POLICY_VERSION
    gaps = [t for t in result["trades"] if t["exit_reason"] == "GAP_STOP"]
    assert len(gaps) == 2
    postings = [f for f in result["funding_ledger"] if f["at"] == 62 * H4 + offset]
    assert len(postings) == 2
    assert all(f["policy"] == "AFTER_EXIT" and f["funding_cost"] == 0 for f in postings)
    assert all(t["funding_cost"] == 0 for t in gaps)
    assert result["metrics"]["funding_cost"] == 0
    assert result["equity"][-1]["equity_index"] - 1 == pytest.approx(sum(t["net"] for t in result["trades"]))


@pytest.mark.parametrize("offset", (1000, 3_600_000))
def test_runner_rejects_jitter_outside_frozen_schedule(monkeypatch, offset):
    monkeypatch.setitem(study.PARTITIONS, "SYNTHETIC", ("1970-01-11", "1970-01-12"))
    with pytest.raises(ValueError, match="PARTITION_FUNDING_COVERAGE"):
        study.run_partition(synthetic_runner_data(1, .001, offset), CONFIGS[0], COSTS[0], "SYNTHETIC")


def test_v2_cannot_run_under_v1_protocol_or_create_trial_output(monkeypatch, tmp_path):
    def forbidden_read(_):
        pytest.fail("v1 dataset must not be read before policy-version rejection")
    monkeypatch.setattr(study, "verify_input", forbidden_read)
    output = tmp_path / "must-not-exist"
    with pytest.raises(ValueError, match="FUNDING_POLICY_PROTOCOL_MISMATCH"):
        study.run_study(tmp_path / "not-a-dataset", output)
    assert not output.exists()
