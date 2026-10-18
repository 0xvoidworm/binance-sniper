use anyhow::{Result, ensure};
use rust_decimal::Decimal as D;

use crate::{model::*, risk::*};

#[derive(Clone, Debug)]
pub struct Engine {
    state: State,
}

impl Engine {
    pub fn new(config: Config) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            state: State {
                equity: config.capital,
                day_start_equity: config.capital,
                config,
                clock_ms: 0,
                quote: None,
                realized_net: D::ZERO,
                day_index: None,
                halt_reason: None,
                active: None,
                completed_trades: 0,
                used_intents: vec![],
            },
        })
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    /// Invalid input produces an audit outcome and leaves all state unchanged.
    /// Use Journal::apply for durable event-id deduplication.
    pub fn apply(&mut self, envelope: &Envelope) -> Outcome {
        let mut next = self.clone();
        match next.transition(envelope) {
            Ok(effects) => {
                *self = next;
                Outcome {
                    accepted: true,
                    error: None,
                    effects,
                }
            }
            Err(err) => Outcome {
                accepted: false,
                error: Some(err.to_string()),
                effects: vec![],
            },
        }
    }

    fn transition(&mut self, envelope: &Envelope) -> Result<Vec<Effect>> {
        identifier(&envelope.id)?;
        ensure!(
            envelope.at_ms >= self.state.clock_ms && envelope.at_ms <= 4_102_444_800_000,
            "event time is out of order or outside supported range"
        );
        let day = envelope.at_ms / 86_400_000;
        if self.state.day_index != Some(day) {
            self.state.day_index = Some(day);
            self.state.day_start_equity = self.state.equity;
        }
        self.state.clock_ms = envelope.at_ms;
        let mut effects = vec![];
        match &envelope.event {
            Event::Quote { quote } => {
                self.state.config.validate_quote(quote, envelope.at_ms)?;
                ensure!(
                    self.state
                        .quote
                        .as_ref()
                        .is_none_or(|q| quote.observed_at_ms >= q.observed_at_ms),
                    "quote observation time regressed"
                );
                self.state.quote = Some(quote.clone());
            }
            Event::Enter {
                intent_id,
                side,
                stop,
                take_profit,
            } => {
                identifier(intent_id)?;
                ensure!(
                    self.state.halt_reason.is_none(),
                    "engine halted; new entries disabled"
                );
                ensure!(
                    self.state.active.is_none(),
                    "an order, position or cleanup is still active"
                );
                ensure!(
                    !self.state.used_intents.contains(intent_id),
                    "intent id already used"
                );
                ensure!(
                    self.state.used_intents.len() < 10_000,
                    "session intent capacity reached"
                );
                let quote = self
                    .state
                    .quote
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("quote required"))?;
                let plan =
                    self.state
                        .config
                        .plan(quote, envelope.at_ms, *side, *stop, *take_profit)?;
                ensure!(
                    plan.notional <= self.state.equity,
                    "insufficient paper equity for unleveraged allocation"
                );
                let total_room = sub(
                    self.state.equity,
                    sub(self.state.config.capital, self.state.config.max_total_loss)?,
                )?;
                let daily_room = sub(
                    self.state.equity,
                    sub(
                        self.state.day_start_equity,
                        self.state.config.max_daily_loss,
                    )?,
                )?;
                ensure!(
                    plan.budgeted_loss <= total_room.min(daily_room),
                    "remaining loss budget too small for this entry"
                );
                let order_id = format!("{intent_id}-entry");
                effects.push(Effect::PlaceEntry {
                    order_id: order_id.clone(),
                    side: *side,
                    qty: plan.qty,
                });
                self.state.active = Some(Trade {
                    intent_id: intent_id.clone(),
                    side: *side,
                    stop: *stop,
                    take_profit: *take_profit,
                    entry: new_order(order_id, plan.qty, envelope.at_ms),
                    plan,
                    stop_status: None,
                    take_profit_status: None,
                    protection_requested_at_ms: None,
                    cancel_entry_requested: false,
                    pending_exit: None,
                    exit: None,
                    cleanup_pending: false,
                });
                self.state.used_intents.push(intent_id.clone());
            }
            Event::EntryReport {
                order_id,
                status,
                fill,
                reconciled,
            } => {
                let mut trade = self.take_trade()?;
                validate_report(&trade.entry, order_id, *status, fill, *reconciled)?;
                let fee_delta = sub(fill.fee, trade.entry.fill.fee)?;
                self.state.realized_net = sub(self.state.realized_net, fee_delta)?;
                update_order(&mut trade.entry, *status, fill, envelope.at_ms);
                if fill.qty > D::ZERO && trade.stop_status.is_none() {
                    trade.stop_status = Some(Protection::Requested);
                    trade.take_profit_status = Some(Protection::Requested);
                    trade.protection_requested_at_ms = Some(envelope.at_ms);
                    for (suffix, trigger, kind) in [
                        ("sl", trade.stop, ExitReason::StopLoss),
                        ("tp", trade.take_profit, ExitReason::TakeProfit),
                    ] {
                        effects.push(Effect::PlaceProtection {
                            order_id: format!("{}-{suffix}", trade.intent_id),
                            side: opposite(trade.side),
                            trigger,
                            kind,
                            close_position: true,
                        });
                    }
                }
                // A real fill can exceed the planned risk due to slippage: record it,
                // latch a halt, and request reduction instead of discarding exposure.
                if fill.qty > D::ZERO {
                    let average = div(fill.notional, fill.qty)?;
                    let adverse = match trade.side {
                        Side::Long => average > trade.plan.worst_entry,
                        Side::Short => average < trade.plan.worst_entry,
                    };
                    if adverse || fill.notional > self.state.config.max_notional {
                        self.halt("entry fill exceeded planned price/notional", &mut effects);
                        trade.pending_exit.get_or_insert(ExitReason::RiskLimit);
                    }
                }
                self.state.active = Some(trade);
                self.advance_exit(&mut effects)?;
            }
            Event::EntryUncertain { order_id } => {
                let trade = self
                    .state
                    .active
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("no active trade"))?;
                ensure!(
                    trade.entry.id == *order_id && !trade.entry.status.terminal(),
                    "unknown or terminal entry"
                );
                query(&mut trade.entry, &mut effects);
            }
            Event::ProtectionReport { order_id, accepted } => {
                let trade = self
                    .state
                    .active
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("no active trade"))?;
                ensure!(!trade.cleanup_pending, "protection cleanup already pending");
                let status = if *order_id == format!("{}-sl", trade.intent_id) {
                    &mut trade.stop_status
                } else if *order_id == format!("{}-tp", trade.intent_id) {
                    &mut trade.take_profit_status
                } else {
                    anyhow::bail!("unknown protection order");
                };
                ensure!(status.is_some(), "protection was not requested");
                ensure!(
                    !(*status == Some(Protection::Failed) && *accepted),
                    "failed protection requires explicit recovery outside v1"
                );
                *status = Some(if *accepted {
                    Protection::Active
                } else {
                    Protection::Failed
                });
                if !accepted {
                    self.halt("protection rejected or failed", &mut effects);
                    self.request_exit(ExitReason::ProtectionFailure, &mut effects)?;
                }
            }
            Event::Close { reason } => {
                self.request_exit(*reason, &mut effects)?;
            }
            Event::ExitReport {
                order_id,
                status,
                fill,
                reconciled,
            } => {
                let mut trade = self.take_trade()?;
                let exit = trade
                    .exit
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("exit was not requested"))?;
                validate_report(exit, order_id, *status, fill, *reconciled)?;
                let qty_delta = sub(fill.qty, exit.fill.qty)?;
                let value_delta = sub(fill.notional, exit.fill.notional)?;
                let cost = mul(
                    qty_delta,
                    div(trade.entry.fill.notional, trade.entry.fill.qty)?,
                )?;
                let gross = match trade.side {
                    Side::Long => sub(value_delta, cost)?,
                    Side::Short => sub(cost, value_delta)?,
                };
                self.state.realized_net = sub(
                    add(self.state.realized_net, gross)?,
                    sub(fill.fee, exit.fill.fee)?,
                )?;
                update_order(exit, *status, fill, envelope.at_ms);
                if status.terminal() && fill.qty < exit.requested_qty {
                    self.halt(
                        "exit ended with residual exposure; manual reconciliation required",
                        &mut effects,
                    );
                }
                self.state.active = Some(trade);
                self.advance_exit(&mut effects)?;
            }
            Event::ExitUncertain { order_id } => {
                let trade = self
                    .state
                    .active
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("no active trade"))?;
                let exit = trade
                    .exit
                    .as_mut()
                    .ok_or_else(|| anyhow::anyhow!("exit was not requested"))?;
                ensure!(
                    exit.id == *order_id && !exit.status.terminal(),
                    "unknown or terminal exit"
                );
                query(exit, &mut effects);
            }
            Event::CleanupConfirmed { intent_id } => {
                let trade = self
                    .state
                    .active
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("no active trade"))?;
                ensure!(
                    trade.intent_id == *intent_id
                        && trade.cleanup_pending
                        && remaining(trade)? == D::ZERO
                        && trade.entry.status.terminal(),
                    "cleanup cannot complete before position and entry are closed"
                );
                self.state.completed_trades += u64::from(trade.entry.fill.qty > D::ZERO);
                self.state.active = None;
            }
            Event::Tick => {}
        }
        self.monitor(&mut effects)?;
        Ok(effects)
    }

    fn take_trade(&mut self) -> Result<Trade> {
        self.state
            .active
            .take()
            .ok_or_else(|| anyhow::anyhow!("no active trade"))
    }

    fn halt(&mut self, reason: &str, effects: &mut Vec<Effect>) {
        if self.state.halt_reason.is_none() {
            self.state.halt_reason = Some(reason.to_owned());
            effects.push(Effect::Halt {
                reason: reason.to_owned(),
            });
        }
    }

    fn request_exit(&mut self, reason: ExitReason, effects: &mut Vec<Effect>) -> Result<()> {
        let trade = self
            .state
            .active
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("no active trade"))?;
        trade.pending_exit.get_or_insert(reason);
        self.advance_exit(effects)
    }

    fn advance_exit(&mut self, effects: &mut Vec<Effect>) -> Result<()> {
        let mut trade = self.take_trade()?;
        if trade.entry.status.terminal() && remaining(&trade)? == D::ZERO {
            if trade.stop_status.is_none() {
                self.state.active = None;
                return Ok(());
            }
            if !trade.cleanup_pending {
                trade.cleanup_pending = true;
                effects.push(Effect::CancelProtections {
                    intent_id: trade.intent_id.clone(),
                    stop_id: format!("{}-sl", trade.intent_id),
                    take_profit_id: format!("{}-tp", trade.intent_id),
                });
            }
        } else if let Some(reason) = trade.pending_exit {
            if !trade.entry.status.terminal() {
                if !trade.entry.uncertain && !trade.cancel_entry_requested {
                    trade.cancel_entry_requested = true;
                    effects.push(Effect::CancelEntry {
                        order_id: trade.entry.id.clone(),
                    });
                }
            } else if trade.exit.is_none() {
                let qty = remaining(&trade)?;
                let order_id = format!("{}-exit", trade.intent_id);
                effects.push(Effect::PlaceExit {
                    order_id: order_id.clone(),
                    side: opposite(trade.side),
                    qty,
                    reason,
                    reduce_only: true,
                });
                trade.exit = Some(new_order(order_id, qty, self.state.clock_ms));
            }
        }
        self.state.active = Some(trade);
        Ok(())
    }

    fn monitor(&mut self, effects: &mut Vec<Effect>) -> Result<()> {
        let now = self.state.clock_ms;
        if let Some(trade) = self.state.active.as_mut() {
            for order in std::iter::once(&mut trade.entry).chain(trade.exit.iter_mut()) {
                if !order.status.terminal()
                    && now - order.requested_at_ms >= self.state.config.order_timeout_ms
                {
                    query(order, effects);
                }
            }
        }
        let protection_expired = self.state.active.as_ref().is_some_and(|t| {
            !t.cleanup_pending
                && t.protection_requested_at_ms
                    .is_some_and(|at| now - at >= self.state.config.protection_timeout_ms)
                && (t.stop_status != Some(Protection::Active)
                    || t.take_profit_status != Some(Protection::Active))
        });
        if protection_expired {
            self.halt("protection acknowledgement timed out", effects);
            self.request_exit(ExitReason::ProtectionFailure, effects)?;
        }
        let stale = self
            .state
            .quote
            .as_ref()
            .is_none_or(|q| now - q.observed_at_ms > self.state.config.max_quote_age_ms);
        if stale && self.state.active.is_some() {
            self.halt("market data stale with possible exposure", effects);
            self.request_exit(ExitReason::StaleData, effects)?;
        }
        let mut unrealized = D::ZERO;
        if let (Some(trade), Some(quote)) = (&self.state.active, &self.state.quote)
            && trade.entry.fill.qty > D::ZERO
        {
            let avg = div(trade.entry.fill.notional, trade.entry.fill.qty)?;
            let delta = match trade.side {
                Side::Long => sub(quote.bid, avg)?,
                Side::Short => sub(avg, quote.ask)?,
            };
            unrealized = mul(delta, remaining(trade)?)?;
        }
        self.state.equity = add(
            add(self.state.config.capital, self.state.realized_net)?,
            unrealized,
        )?;
        if sub(self.state.config.capital, self.state.equity)? >= self.state.config.max_total_loss
            || sub(self.state.day_start_equity, self.state.equity)?
                >= self.state.config.max_daily_loss
        {
            self.halt("total or UTC daily loss threshold reached", effects);
            if self.state.active.is_some() {
                self.request_exit(ExitReason::RiskLimit, effects)?;
            }
        }
        // Check again after fills too: a quote can cross a level while entry is
        // pending, before the first fill report reaches this core.
        if let (Some(trade), Some(quote)) = (&self.state.active, &self.state.quote)
            && !stale
            && remaining(trade)? > D::ZERO
            && trade.pending_exit.is_none()
        {
            let price = match trade.side {
                Side::Long => quote.bid,
                Side::Short => quote.ask,
            };
            let reason = match trade.side {
                Side::Long if price <= trade.stop => Some(ExitReason::StopLoss),
                Side::Long if price >= trade.take_profit => Some(ExitReason::TakeProfit),
                Side::Short if price >= trade.stop => Some(ExitReason::StopLoss),
                Side::Short if price <= trade.take_profit => Some(ExitReason::TakeProfit),
                _ => None,
            };
            if let Some(reason) = reason {
                self.request_exit(reason, effects)?;
            }
        }
        Ok(())
    }
}

fn new_order(id: String, qty: D, at_ms: u64) -> Order {
    Order {
        id,
        requested_qty: qty,
        status: OrderStatus::Pending,
        fill: Fill::default(),
        uncertain: false,
        queried: false,
        requested_at_ms: at_ms,
    }
}

fn opposite(side: Side) -> Side {
    match side {
        Side::Long => Side::Short,
        Side::Short => Side::Long,
    }
}

pub fn remaining(trade: &Trade) -> Result<D> {
    sub(
        trade.entry.fill.qty,
        trade.exit.as_ref().map_or(D::ZERO, |o| o.fill.qty),
    )
}

fn query(order: &mut Order, effects: &mut Vec<Effect>) {
    order.uncertain = true;
    if !order.queried {
        order.queried = true;
        effects.push(Effect::QueryOrder {
            order_id: order.id.clone(),
        });
    }
}

fn update_order(order: &mut Order, status: OrderStatus, fill: &Fill, now: u64) {
    order.status = status;
    order.fill = fill.clone();
    order.uncertain = false;
    order.queried = false;
    order.requested_at_ms = now;
}

fn validate_report(
    order: &Order,
    id: &str,
    status: OrderStatus,
    fill: &Fill,
    reconciled: bool,
) -> Result<()> {
    ensure!(order.id == id, "unknown order id");
    ensure!(
        !order.uncertain || reconciled,
        "uncertain order requires authoritative reconciliation"
    );
    ensure!(
        status != OrderStatus::Pending,
        "pending is internal, not a broker report"
    );
    for value in [fill.qty, fill.notional, fill.fee] {
        number(value, true)?;
    }
    ensure!(
        fill.qty <= order.requested_qty,
        "fill exceeds requested quantity; reconcile externally"
    );
    ensure!(
        fill.qty >= order.fill.qty
            && fill.notional >= order.fill.notional
            && fill.fee >= order.fill.fee,
        "cumulative fill regressed"
    );
    ensure!(
        !order.status.terminal() || (status == order.status && *fill == order.fill),
        "terminal order report changed; reconcile externally"
    );
    ensure!(
        (fill.qty == D::ZERO) == (fill.notional == D::ZERO),
        "quantity and notional must both be zero or both positive"
    );
    if fill.qty == order.fill.qty {
        ensure!(
            fill.notional == order.fill.notional,
            "notional changed without a fill"
        );
    }
    if fill.qty > order.fill.qty {
        ensure!(
            fill.notional > order.fill.notional,
            "new quantity without new notional"
        );
    }
    match status {
        OrderStatus::New | OrderStatus::Rejected => ensure!(
            fill.qty == D::ZERO && fill.fee == D::ZERO,
            "unfilled status contains fills/fees"
        ),
        OrderStatus::PartiallyFilled => ensure!(
            fill.qty > D::ZERO && fill.qty < order.requested_qty,
            "partial status requires partial quantity"
        ),
        OrderStatus::Filled => ensure!(
            fill.qty == order.requested_qty,
            "filled status must cover requested quantity"
        ),
        _ => {}
    }
    Ok(())
}
