import json

from binance_sniper.domain import Candle
from binance_sniper.replay import run_replay


EXPECTED_CHAIN = ["REGIME", "IMPULSE", "DETECTED", "ARMED", "TOUCHED", "TRIGGERED", "CANDIDATE", "COMPLETED", "OUTCOME"]


def chronological(snapshot):
    return list(reversed(snapshot["events"]))


def test_full_production_pipeline_replay_has_exact_semantic_chain(tmp_path):
    first = run_replay(tmp_path / "one")
    second = run_replay(tmp_path / "two")
    assert [event["to"] for event in chronological(first)] == EXPECTED_CHAIN
    assert chronological(first) == chronological(second)
    assert first["symbols"]["BTCUSDT"]["status"] == "COMPLETED"
    assert len(first["candidates"]) == 1
    candidate = first["candidates"][0]
    assert 0 < candidate["score"] <= 1
    assert set(candidate["score_components"]) == {"regime", "impulse", "pullback", "trigger", "room"}
    outcome = first["outcomes"][candidate["candidate_id"]]
    assert outcome["bars"] == 4
    assert outcome["status"] == "COMPLETED"
    assert round(outcome["mfe"], 6) == 2.1
    assert round(outcome["mae"], 6) == 0.4
    assert round(outcome["final_r"], 6) == 1.6
    assert outcome["first_hit"]["kind"] == "TARGET"


def test_future_candle_change_cannot_rewrite_events_at_or_before_trigger(tmp_path):
    base = run_replay(tmp_path / "base")
    trigger_time = next(event["time"] for event in chronological(base) if event["to"] == "TRIGGERED")

    def mutate_future(m5, trigger_index):
        candle = m5[trigger_index + 3]
        m5[trigger_index + 3] = Candle(candle.symbol, candle.timeframe, candle.open_time, candle.close_time, candle.open, 500, 1, 250, candle.volume, True)

    changed = run_replay(tmp_path / "changed", future_mutator=mutate_future)
    base_past = [event for event in chronological(base) if event["time"] <= trigger_time]
    changed_past = [event for event in chronological(changed) if event["time"] <= trigger_time]
    assert json.dumps(base_past, sort_keys=True) == json.dumps(changed_past, sort_keys=True)


def test_snapshot_is_pure_and_restart_keeps_completed_outcome(tmp_path):
    state_dir = tmp_path / "state"
    snapshot = run_replay(state_dir)
    zones_before = (state_dir / "zones.json").read_bytes()
    outcomes_before = (state_dir / "four-bar.json").read_bytes()
    again = run_replay(state_dir)
    assert (state_dir / "zones.json").read_bytes() == zones_before
    assert (state_dir / "four-bar.json").read_bytes() == outcomes_before
    assert again["candidates"] == snapshot["candidates"]
    assert again["outcomes"] == snapshot["outcomes"]
