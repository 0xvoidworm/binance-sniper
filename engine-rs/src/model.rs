use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub const SCHEMA: &str = "binance-engine-paper-v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub mode: Mode,
    pub market: Market,
    pub symbol: String,
    pub capital: Decimal,
    pub risk_per_trade: Decimal,
    pub max_notional: Decimal,
    pub max_daily_loss: Decimal,
    pub max_total_loss: Decimal,
    pub fee_bps: Decimal,
    pub slippage_bps: Decimal,
    pub max_spread_bps: Decimal,
    pub max_quote_age_ms: u64,
    pub order_timeout_ms: u64,
    pub protection_timeout_ms: u64,
    pub filters: Filters,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Paper,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Market {
    UsdmLinearOneWay,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Filters {
    pub min_price: Decimal,
    pub max_price: Decimal,
    pub tick_size: Decimal,
    pub min_qty: Decimal,
    pub max_qty: Decimal,
    pub step_size: Decimal,
    pub min_notional: Decimal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Long,
    Short,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quote {
    pub bid: Decimal,
    pub ask: Decimal,
    pub observed_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub id: String,
    pub at_ms: u64,
    pub event: Event,
}

/// Broker reports use cumulative quantities, notionals and fees in quote currency.
/// This normalized schema is deliberately not Binance's wire format.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Event {
    Quote {
        quote: Quote,
    },
    Enter {
        intent_id: String,
        side: Side,
        stop: Decimal,
        take_profit: Decimal,
    },
    EntryReport {
        order_id: String,
        status: OrderStatus,
        fill: Fill,
        reconciled: bool,
    },
    EntryUncertain {
        order_id: String,
    },
    ProtectionReport {
        order_id: String,
        accepted: bool,
    },
    Close {
        reason: ExitReason,
    },
    ExitReport {
        order_id: String,
        status: OrderStatus,
        fill: Fill,
        reconciled: bool,
    },
    ExitUncertain {
        order_id: String,
    },
    CleanupConfirmed {
        intent_id: String,
    },
    Tick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Pending,
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
}

impl OrderStatus {
    pub fn terminal(self) -> bool {
        matches!(
            self,
            Self::Filled | Self::Canceled | Self::Rejected | Self::Expired
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fill {
    pub qty: Decimal,
    pub notional: Decimal,
    pub fee: Decimal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitReason {
    StopLoss,
    TakeProfit,
    Manual,
    RiskLimit,
    ProtectionFailure,
    StaleData,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protection {
    Requested,
    Active,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub qty: Decimal,
    pub reference_price: Decimal,
    pub worst_entry: Decimal,
    pub budgeted_loss: Decimal,
    pub notional: Decimal,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub requested_qty: Decimal,
    pub status: OrderStatus,
    pub fill: Fill,
    pub uncertain: bool,
    pub queried: bool,
    pub requested_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trade {
    pub intent_id: String,
    pub side: Side,
    pub stop: Decimal,
    pub take_profit: Decimal,
    pub plan: Plan,
    pub entry: Order,
    pub stop_status: Option<Protection>,
    pub take_profit_status: Option<Protection>,
    pub protection_requested_at_ms: Option<u64>,
    pub cancel_entry_requested: bool,
    pub pending_exit: Option<ExitReason>,
    pub exit: Option<Order>,
    pub cleanup_pending: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub config: Config,
    pub clock_ms: u64,
    pub quote: Option<Quote>,
    pub realized_net: Decimal,
    pub equity: Decimal,
    pub day_index: Option<u64>,
    pub day_start_equity: Decimal,
    pub halt_reason: Option<String>,
    pub active: Option<Trade>,
    pub completed_trades: u64,
    pub used_intents: Vec<String>,
}

/// Outbox entries describe required work. No effect is automatically executed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Effect {
    PlaceEntry {
        order_id: String,
        side: Side,
        qty: Decimal,
    },
    QueryOrder {
        order_id: String,
    },
    CancelEntry {
        order_id: String,
    },
    PlaceProtection {
        order_id: String,
        side: Side,
        trigger: Decimal,
        kind: ExitReason,
        close_position: bool,
    },
    PlaceExit {
        order_id: String,
        side: Side,
        qty: Decimal,
        reason: ExitReason,
        reduce_only: bool,
    },
    CancelProtections {
        intent_id: String,
        stop_id: String,
        take_profit_id: String,
    },
    Halt {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub accepted: bool,
    pub error: Option<String>,
    pub effects: Vec<Effect>,
}
