"""Parse English MT5 HTML reports, preserving broker-local timestamps.

HTML is never executed or served. Missing/unsupported data fails closed; a report
being internally consistent is not evidence that a strategy is profitable live.
"""
from __future__ import annotations

from datetime import datetime
from decimal import Decimal, InvalidOperation
from hashlib import sha256
from html.parser import HTMLParser
import json
import re

MAX_BYTES = 25 * 1024 * 1024
MAX_ROWS = 100_000


def canonical(value) -> str:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False)


def digest(value) -> str:
    return sha256(canonical(value).encode()).hexdigest()


def decode(raw: bytes) -> str:
    if not raw or len(raw) > MAX_BYTES:
        raise ValueError("Report must be nonempty and at most 25 MiB")
    if raw.startswith((b"\xff\xfe", b"\xfe\xff")):
        return raw.decode("utf-16")
    if b"\x00" in raw[:128]:
        raise ValueError("UTF-16 report needs a byte-order mark")
    return raw.decode("utf-8-sig")


class Rows(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.rows, self.row, self.cell = [], None, None
        self.ignored = 0

    def handle_starttag(self, tag, attrs):
        if tag in {"script", "style"}:
            self.ignored += 1
        if self.ignored:
            return
        if tag == "tr":
            if self.row is not None:
                raise ValueError("Nested or incomplete report row")
            self.row = []
        elif tag in {"td", "th"} and self.row is not None:
            if self.cell is not None:
                raise ValueError("Incomplete report cell")
            self.cell = []
        elif tag == "br" and self.cell is not None:
            self.cell.append(" ")

    def handle_data(self, data):
        if not self.ignored and self.cell is not None:
            self.cell.append(data)

    def handle_endtag(self, tag):
        if tag in {"script", "style"}:
            self.ignored = max(0, self.ignored - 1)
            return
        if self.ignored:
            return
        if tag in {"td", "th"} and self.cell is not None:
            value = " ".join("".join(self.cell).split())
            if len(value) > 16_384:
                raise ValueError("Oversized report cell")
            self.row.append(value)
            self.cell = None
        elif tag == "tr" and self.row is not None:
            if self.cell is not None:
                raise ValueError("Incomplete report cell")
            self.rows.append(self.row)
            self.row = None
            if len(self.rows) > MAX_ROWS:
                raise ValueError("Report row limit exceeded")


def number(value: str) -> Decimal:
    # MT5 English exports use decimal dots and optional space grouping.
    if not re.fullmatch(r"[+-]?(?:\d+|\d{1,3}(?: \d{3})+)(?:\.\d+)?", value):
        raise ValueError("Unsupported or non-finite report number")
    try:
        result = Decimal(value.replace(" ", ""))
    except InvalidOperation as exc:
        raise ValueError("Invalid report number") from exc
    if not result.is_finite() or abs(result) > Decimal("1e15"):
        raise ValueError("Report number out of range")
    return result


def count(value: str) -> int:
    result = number(value)
    if result < 0 or result != result.to_integral_value():
        raise ValueError("Invalid report count")
    return int(result)


def stamp(value: str) -> str:
    return datetime.strptime(value, "%Y.%m.%d %H:%M:%S").isoformat()


SETTINGS = {"Expert", "Symbol", "Period", "Currency", "Initial Deposit", "Leverage"}
NUMBERS = {"Total Net Profit": "net_profit", "Gross Profit": "gross_profit",
           "Gross Loss": "gross_loss", "Profit Factor": "profit_factor",
           "Total Trades": "total_trades", "Total Deals": "total_deals",
           "Bars": "bars", "Ticks": "ticks", "Symbols": "symbols_requested"}
TEXT_METRICS = {"History Quality": "history_quality", "Balance Drawdown Maximal": "balance_dd_maximal",
                "Balance Drawdown Relative": "balance_dd_relative", "Equity Drawdown Maximal": "equity_dd_maximal",
                "Equity Drawdown Relative": "equity_dd_relative"}
DEAL_HEADER = ["Time", "Deal", "Symbol", "Type", "Direction", "Volume", "Price", "Order", "Commission", "Swap", "Profit", "Balance", "Comment"]


def parse_report(raw: bytes) -> dict:
    text = decode(raw)
    if not re.search(r"</html>\s*$", text, re.I) or "Strategy Tester Report" not in text:
        raise ValueError("Incomplete or unsupported MT5 English tester report")
    parser = Rows()
    parser.feed(text)
    parser.close()
    if parser.row is not None or parser.cell is not None:
        raise ValueError("Incomplete report table")
    settings, metrics, inputs, labels = {}, {}, [], set()
    server, in_inputs, section, header, deals = None, False, "summary", None, []
    errors, warnings = [], []
    for row in parser.rows:
        if len(row) == 1:
            if re.fullmatch(r".+ \(Build \d+\)", row[0]) and server is None:
                server = row[0]
            if row[0] in {"Orders", "Deals"}:
                section, in_inputs = row[0], False
                continue
        if section == "Deals":
            if row == DEAL_HEADER:
                header = row
                continue
            # Totals/footer rows have an empty first cell, not a deal timestamp.
            if not row or not row[0]:
                continue
            if header is None or len(row) != len(header):
                raise ValueError("Unsupported or incomplete deal table")
            item = dict(zip(header, row))
            if item["Type"] not in {"buy", "sell", "balance"}:
                raise ValueError("Unsupported deal type; reconciliation would be incomplete")
            trading = item["Type"] in {"buy", "sell"}
            if trading and (not re.fullmatch(r"[A-Za-z0-9_.#&+\-]{1,40}", item["Symbol"]) or item["Direction"] not in {"in", "out", "in/out", "out by"}):
                raise ValueError("Unsupported deal symbol or direction")
            deal = {"time": stamp(item["Time"]), "deal": count(item["Deal"]),
                    "symbol": item["Symbol"] if trading else "", "type": item["Type"],
                    "direction": item["Direction"] if trading else "",
                    "order": count(item["Order"]) if trading else None,
                    "volume": float(number(item["Volume"])) if trading else None,
                    "price": float(number(item["Price"])) if trading else None}
            for key in ("Commission", "Swap", "Profit", "Balance"):
                deal[key.lower()] = float(number(item[key]))
            if trading and (deal["volume"] <= 0 or deal["price"] <= 0):
                raise ValueError("Non-positive deal volume or price")
            # Do not expose arbitrary EA comments, input values, account names or HTML.
            deals.append(deal)
        elif section == "summary":
            if row and row[0] == "Inputs:":
                inputs.extend(row[1:])
                in_inputs = True
                continue
            if in_inputs and len(row) >= 2 and not row[0] and any(row[1:]):
                inputs.extend(row[1:])
                continue
            in_inputs = False
            for index in range(len(row) - 1):
                label = row[index].removesuffix(":")
                if not row[index].endswith(":") or label not in SETTINGS | NUMBERS.keys() | TEXT_METRICS.keys():
                    continue
                if label in labels:
                    raise ValueError("Duplicate report summary field")
                labels.add(label)
                value = row[index + 1]
                if label in SETTINGS:
                    settings[label] = value
                elif label in TEXT_METRICS:
                    metrics[TEXT_METRICS[label]] = value
                else:
                    integer = label in {"Total Trades", "Total Deals", "Bars", "Ticks", "Symbols"}
                    # MT5 can report undefined PF when there are no losing trades.
                    metrics[NUMBERS[label]] = None if label == "Profit Factor" and value in {"", "∞", "inf"} else (count(value) if integer else float(number(value)))
    required = {"Expert", "Symbol", "Period", "Currency", "Initial Deposit"}
    if not required <= settings.keys() or not {"net_profit", "total_deals", "total_trades"} <= metrics.keys() or header is None:
        raise ValueError("Missing required summary or Deals table; English HTML export required")
    period = re.fullmatch(r"([A-Z0-9]+) \((\d{4}\.\d{2}\.\d{2}) - (\d{4}\.\d{2}\.\d{2})\)", settings["Period"])
    if not period:
        raise ValueError("Unsupported tester period")
    timeframe, start, end = period.groups()
    start, end = (datetime.strptime(x, "%Y.%m.%d").date().isoformat() for x in (start, end))
    if start > end or not re.fullmatch(r"[A-Z]{3,8}", settings["Currency"]):
        raise ValueError("Invalid period or currency")
    initial = number(settings["Initial Deposit"])
    if initial <= 0:
        raise ValueError("Initial deposit must be positive")
    trading = [d for d in deals if d["type"] in {"buy", "sell"}]
    if len({d["deal"] for d in deals}) != len(deals):
        errors.append("DUPLICATE_DEAL_ID")
    if any(a["time"] > b["time"] for a, b in zip(deals, deals[1:])):
        errors.append("NON_CHRONOLOGICAL_DEALS")
    if any(not start <= d["time"][:10] <= end for d in deals):
        errors.append("DEAL_OUTSIDE_REPORTED_PERIOD")
    if len(trading) != metrics["total_deals"]:
        errors.append("DEAL_COUNT_MISMATCH")
    balances = [d for d in deals if d["type"] == "balance"]
    if not deals or deals[0]["type"] != "balance" or len(balances) != 1 or Decimal(str(balances[0]["profit"])) != initial:
        errors.append("INITIAL_DEPOSIT_OR_CASHFLOW_UNSUPPORTED")
    previous = Decimal(0)
    for d in deals:
        delta = sum((Decimal(str(d[k])) for k in ("profit", "commission", "swap")), Decimal(0))
        if abs(previous + delta - Decimal(str(d["balance"]))) > Decimal("0.02"):
            errors.append("BALANCE_LEDGER_MISMATCH")
            break
        previous = Decimal(str(d["balance"]))
    net = sum((Decimal(str(d[k])) for d in trading for k in ("profit", "commission", "swap")), Decimal(0))
    if abs(net - Decimal(str(metrics["net_profit"]))) > Decimal("0.02"):
        errors.append("NET_PROFIT_MISMATCH")
    if metrics["total_trades"] > metrics["total_deals"]:
        errors.append("TRADE_COUNT_EXCEEDS_DEALS")
    if not inputs:
        warnings.append("REPORT_INPUTS_NOT_AVAILABLE")
    if not server:
        warnings.append("SERVER_BUILD_NOT_AVAILABLE")
    warnings.extend(["BALANCE_CURVE_IS_NOT_EQUITY", "HISTORY_QUALITY_IS_NOT_REAL_TICK_COVERAGE",
                     "REPORT_CONSISTENCY_IS_NOT_AUTHENTICITY_OR_PROFITABILITY"])
    if trading and all(d["commission"] == 0 for d in trading):
        warnings.append("ZERO_REPORTED_COMMISSION_NOT_VERIFIED_BROKER_COST")
    metrics.update(initial_deposit=float(initial), ledger_net=float(net), final_balance=deals[-1]["balance"] if deals else None,
                   commission=sum(d["commission"] for d in trading), swap=sum(d["swap"] for d in trading))
    return {"schema_version": 1, "parser": "mt5-english-html-v1", "source_sha256": sha256(raw).hexdigest(),
            "expert": settings["Expert"], "symbol": settings["Symbol"], "timeframe": timeframe,
            "start": start, "end": end, "currency": settings["Currency"], "leverage": settings.get("Leverage"),
            "server_build": server, "inputs_sha256": digest(inputs) if inputs else None,
            "metrics": metrics, "deals": deals, "traded_symbols": sorted({d["symbol"] for d in trading}),
            "time_basis": "BROKER_LOCAL_UNCONVERTED", "errors": sorted(set(errors)), "warnings": warnings,
            "validation": "RECONCILIATION_FAILED" if errors else "REPORT_RECONCILED",
            "availability": {"deals": True, "balance": not errors, "equity_curve": False,
                             "candles": False, "entry_zones": False, "mfe_mae_per_trade": False}}
