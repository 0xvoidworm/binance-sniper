"""Causal closed-H4 channel research; all amounts are unitless index contributions.

No network, account sizing, order routing, background state or runtime hooks.
"""
from __future__ import annotations

from dataclasses import asdict, dataclass
import copy
import math

from binance_sniper.domain import Candle
from binance_sniper.round01.data import H4

FUNDING_POLICY_VERSION = "ownership-interval-v2"


@dataclass(frozen=True)
class Config:
    name: str
    entry_window: int
    exit_window: int
    atr_period: int = 20
    stop_atr: float = 2.0
    nominal: float = 0.5


CONFIGS = (Config("CHANNEL_20_10", 20, 10), Config("CHANNEL_55_20", 55, 20))


@dataclass(frozen=True)
class Costs:
    name: str
    fee_bps: float
    slippage_bps: float


COSTS = (Costs("BASE", 5, 2), Costs("STRESS", 7.5, 5))


@dataclass
class Position:
    trade_id: str
    direction: int
    signal_at: int
    entry_at: int
    raw_entry: float
    entry_fill: float
    qty_index: float
    atr_at_signal: float
    initial_stop: float
    funding_cost: float = 0.0


def fill_costs(raw_entry, raw_exit, direction, qty, costs):
    slip, fee = costs.slippage_bps / 10000, costs.fee_bps / 10000
    entry_fill = raw_entry * (1 + direction * slip)
    exit_fill = raw_exit * (1 - direction * slip)
    return {"entry_fill": entry_fill, "exit_fill": exit_fill,
            "gross": qty * direction * (raw_exit - raw_entry),
            "slippage_cost": qty * slip * (raw_entry + raw_exit),
            "fee_cost": qty * fee * (entry_fill + exit_fill)}


def funding_charge(qty, direction, rate, mark, ambiguous=False):
    charge = qty * direction * rate * mark
    return max(0.0, charge) if ambiguous else charge


def funding_ownership_policy(at, entry_at, exit_earliest_at=None, exit_latest_at=None):
    """Classify ownership without inventing intrabar fill/settlement order.

    Equal entry/exit timestamps and an unknown stop interval remain debit-only.
    Funding strictly outside the possible holding interval posts neither sign.
    """
    if (exit_earliest_at is None) != (exit_latest_at is None):
        raise ValueError("INCOMPLETE_EXIT_INTERVAL")
    if exit_earliest_at is not None and not entry_at <= exit_earliest_at <= exit_latest_at:
        raise ValueError("INVALID_EXIT_INTERVAL")
    if at < entry_at:
        return "BEFORE_ENTRY"
    if exit_latest_at is not None and at > exit_latest_at:
        return "AFTER_EXIT"
    if at == entry_at or (exit_earliest_at is not None and at >= exit_earliest_at):
        return "DEBIT_ONLY_BOUNDARY"
    return "NORMAL"


class ChannelEngine:
    def __init__(self, symbol: str, config: Config, costs: Costs):
        self.symbol, self.config, self.costs = symbol, config, costs
        self.history: list[Candle] = []
        self.tr_seed: list[float] = []
        self.atr: float | None = None
        self.pending: dict | None = None
        self.position: Position | None = None
        self.realized = 0.0
        self.trades: list[dict] = []
        self.funding_ledger: list[dict] = []
        self.last_frame: dict | None = None

    def state(self):
        return copy.deepcopy({"atr": self.atr, "pending": self.pending,
            "position": asdict(self.position) if self.position else None,
            "realized": self.realized, "trades": self.trades, "funding": self.funding_ledger,
            "last_frame": self.last_frame})

    def _channels(self):
        cfg = self.config
        if len(self.history) < max(cfg.entry_window, cfg.exit_window, cfg.atr_period):
            return None
        e, x = self.history[-cfg.entry_window:], self.history[-cfg.exit_window:]
        return {"entry_high": max(b.high for b in e), "entry_low": min(b.low for b in e),
                "exit_high": max(b.high for b in x), "exit_low": min(b.low for b in x)}

    def _stop(self, channels):
        p = self.position
        if not p or not channels:
            return None, None
        channel = channels["exit_low"] if p.direction == 1 else channels["exit_high"]
        stop = max(p.initial_stop, channel) if p.direction == 1 else min(p.initial_stop, channel)
        reason = "CHANNEL_STOP" if stop == channel else "INITIAL_STOP"
        return stop, reason

    def _mark(self, price):
        if not self.position:
            return self.realized
        p = self.position
        amounts = fill_costs(p.raw_entry, price, p.direction, p.qty_index, self.costs)
        return self.realized + amounts["gross"] - amounts["slippage_cost"] - amounts["fee_cost"] - p.funding_cost

    def step(self, bar: Candle, funding=(), *, available_at=None, trading=True,
             allow_entry=True, force_close=False):
        """Consume one complete candle. Funding is accounting only, never a signal input.

        force_close is a preannounced partition boundary, not an implicit end-of-input.
        Duplicate delivery is idempotent only with exactly identical inputs; adapters
        replay from the beginning instead of persisting a new engine state.
        """
        if not bar.closed:
            return None
        now = bar.close_time + 1 if available_at is None else available_at
        if (bar.symbol != self.symbol or bar.timeframe != "4h" or bar.open_time % H4
                or bar.close_time != bar.open_time + H4 - 1 or now < bar.close_time + 1
                or not all(math.isfinite(v) for v in (bar.open, bar.high, bar.low, bar.close, bar.volume))
                or not 0 < bar.low <= min(bar.open, bar.close) <= max(bar.open, bar.close) <= bar.high or bar.volume < 0):
            raise ValueError("INVALID_OR_UNAVAILABLE_CLOSED_BAR")
        delivery = (bar, tuple((r["at"], r["rate"], r["mark"]) for r in funding), now, trading, allow_entry, force_close)
        if self.history and bar.open_time == self.history[-1].open_time:
            if delivery != self._last_delivery:
                raise ValueError("REVISION_OF_CONSUMED_BAR")
            return copy.deepcopy(self.last_frame)
        if self.history and bar.open_time != self.history[-1].open_time + H4:
            raise ValueError("NONCONTIGUOUS_BAR")
        if any(not bar.open_time <= r["at"] <= bar.close_time or not math.isfinite(r["rate"])
               or not math.isfinite(r["mark"]) or r["mark"] <= 0 for r in funding):
            raise ValueError("INVALID_FUNDING_INPUT")
        if len({r["at"] for r in funding}) != len(funding):
            raise ValueError("DUPLICATE_FUNDING_INPUT")
        if not trading and (self.position or self.pending):
            raise ValueError("WARMUP_WITH_ACTIVE_STATE")
        channels = self._channels()
        events = []
        entered = False
        if self.pending and trading:
            signal = self.pending
            self.pending = None
            if signal["signal_at"] != bar.open_time:
                raise ValueError("STALE_PENDING_SIGNAL")
            if allow_entry and not force_close:
                d, n = signal["direction"], signal["atr"]
                initial_stop = bar.open - d * self.config.stop_atr * n
                if n > 0 and initial_stop > 0:
                    p = Position(f"{self.symbol}-{signal['signal_at']}", d, signal["signal_at"], bar.open_time,
                                 bar.open, bar.open * (1 + d * self.costs.slippage_bps / 10000),
                                 self.config.nominal / bar.open, n, initial_stop)
                    self.position, entered = p, True
                    events.append({"kind": "ENTRY", **asdict(p), "fill_assumption": "NEXT_H4_OPEN_WITH_ADVERSE_COST"})
                else:
                    events.append({"kind": "ENTRY_CANCELLED", "reason": "INVALID_INITIAL_STOP", **signal})
            else:
                events.append({"kind": "ENTRY_CANCELLED", "reason": "PARTITION_END", **signal})
        stop, stop_reason = self._stop(channels)
        exit_price = exit_reason = None
        gap = False
        if self.position:
            p = self.position
            if stop is not None:
                gap = bar.open <= stop if p.direction == 1 else bar.open >= stop
                touch = bar.low <= stop if p.direction == 1 else bar.high >= stop
                if gap:
                    exit_price, exit_reason = bar.open, "GAP_STOP"
                elif touch:
                    exit_price, exit_reason = stop, stop_reason
            stopped = exit_reason is not None
            if force_close and not stopped:
                exit_price, exit_reason = bar.close, "PARTITION_END"
            exit_earliest_at = (bar.open_time if stopped else bar.close_time + 1) if exit_reason else None
            exit_latest_at = (bar.open_time if gap else bar.close_time + 1) if exit_reason else None
            for row in funding:
                policy = funding_ownership_policy(row["at"], p.entry_at, exit_earliest_at, exit_latest_at)
                ambiguous = policy == "DEBIT_ONLY_BOUNDARY"
                charge = (0.0 if policy in ("BEFORE_ENTRY", "AFTER_EXIT") else
                          funding_charge(p.qty_index, p.direction, row["rate"], row["mark"], ambiguous))
                record = {"trade_id": p.trade_id, "symbol": self.symbol, **row, "direction": p.direction,
                          "qty_index": p.qty_index, "ambiguous": ambiguous,
                          "raw_charge": p.qty_index * p.direction * row["rate"] * row["mark"], "funding_cost": charge,
                          "policy": policy, "funding_policy_version": FUNDING_POLICY_VERSION,
                          "entry_at": p.entry_at, "exit_earliest_at": exit_earliest_at, "exit_latest_at": exit_latest_at}
                self.funding_ledger.append(record)
                p.funding_cost += charge
            if exit_reason:
                amounts = fill_costs(p.raw_entry, exit_price, p.direction, p.qty_index, self.costs)
                net = amounts["gross"] - amounts["slippage_cost"] - amounts["fee_cost"] - p.funding_cost
                trade = {**asdict(p), "symbol": self.symbol, "mode": "HISTORICAL_REPLAY", **amounts,
                    "raw_exit": exit_price, "exit_reason": exit_reason, "stop_applied": stop,
                    "exit_bar_open": bar.open_time, "exit_recorded_at": bar.close_time + 1,
                    "exit_earliest_at": exit_earliest_at, "exit_latest_at": exit_latest_at,
                    "intrabar_time_unknown": stopped and not gap, "entry_bar_stop": entered and stopped,
                    "funding_policy_version": FUNDING_POLICY_VERSION,
                    "net": net, "net_r": net / (p.qty_index * self.config.stop_atr * p.atr_at_signal),
                    "cost_complete_for_model": True, "account_model_complete": False}
                self.trades.append(trade)
                self.realized += net
                events.append({"kind": "EXIT", **trade})
                self.position = None
        prev_close = self.history[-1].close if self.history else bar.close
        tr = max(bar.high - bar.low, abs(bar.high - prev_close), abs(bar.low - prev_close))
        if self.atr is None:
            self.tr_seed.append(tr)
            if len(self.tr_seed) == self.config.atr_period:
                self.atr = sum(self.tr_seed) / self.config.atr_period
        else:
            self.atr = ((self.config.atr_period - 1) * self.atr + tr) / self.config.atr_period
        if trading and not force_close and not self.position and channels and self.atr and self.atr > 0:
            direction = 1 if bar.close > channels["entry_high"] else (-1 if bar.close < channels["entry_low"] else 0)
            if direction:
                self.pending = {"direction": direction, "atr": self.atr, "signal_at": bar.close_time + 1,
                                "source_bar_open": bar.open_time, "channel_level": channels["entry_high"] if direction == 1 else channels["entry_low"]}
                events.append({"kind": "SIGNAL", **self.pending})
        self.history.append(bar)
        self._last_delivery = delivery
        next_stop, _ = self._stop(self._channels())
        frame = {"mode": "HISTORICAL_REPLAY", "engine": self.config.name, "scenario": self.costs.name,
            "funding_policy_version": FUNDING_POLICY_VERSION,
            "symbol": self.symbol, "data_available_at": bar.close_time + 1, "decision_at": bar.close_time + 1,
            "bar": asdict(bar), "channels_known_before_bar": channels, "stop_applied_this_bar": stop,
            "stop_for_next_bar": next_stop, "atr_after_close": self.atr,
            "position": asdict(self.position) if self.position else None, "pending": copy.deepcopy(self.pending),
            "events": events, "reason": "WARMUP" if not trading else ("HOLD_CHANNEL_VALID" if self.position else ("SIGNAL_PENDING_NEXT_OPEN" if self.pending else "FLAT_NO_SIGNAL")),
            "index_pnl_mtm": self._mark(bar.close), "realized_net": self.realized,
            "cost_complete_for_model": True, "account_model_complete": False}
        self.last_frame = copy.deepcopy(frame)
        return frame


def project_frame(frame):
    """Passive chart projection; engine data is the only rule source."""
    return copy.deepcopy({k: frame[k] for k in ("mode", "engine", "scenario", "funding_policy_version", "symbol", "data_available_at", "decision_at",
        "bar", "channels_known_before_bar", "stop_applied_this_bar", "stop_for_next_bar", "position", "pending", "events",
        "reason", "index_pnl_mtm", "cost_complete_for_model", "account_model_complete")})
