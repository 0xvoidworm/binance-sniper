use crate::model::*;
use anyhow::{Result, anyhow, ensure};
use rust_decimal::Decimal as D;

pub(crate) fn add(a: D, b: D) -> Result<D> {
    a.checked_add(b)
        .ok_or_else(|| anyhow!("decimal overflow: add"))
}
pub(crate) fn sub(a: D, b: D) -> Result<D> {
    a.checked_sub(b)
        .ok_or_else(|| anyhow!("decimal overflow: subtract"))
}
pub(crate) fn mul(a: D, b: D) -> Result<D> {
    a.checked_mul(b)
        .ok_or_else(|| anyhow!("decimal overflow: multiply"))
}
pub(crate) fn div(a: D, b: D) -> Result<D> {
    a.checked_div(b)
        .ok_or_else(|| anyhow!("invalid decimal division"))
}
pub(crate) fn bps(value: D, rate: D) -> Result<D> {
    div(mul(value, rate)?, D::from(10_000))
}

pub(crate) fn number(value: D, allow_zero: bool) -> Result<()> {
    ensure!(
        value >= D::ZERO && (allow_zero || value > D::ZERO),
        "number must be positive (or explicitly zero)"
    );
    ensure!(
        value <= D::from(1_000_000_000_000u64) && value.scale() <= 12,
        "number exceeds magnitude/precision limit"
    );
    Ok(())
}

pub(crate) fn identifier(id: &str) -> Result<()> {
    ensure!(
        !id.is_empty()
            && id.len() <= 24
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
        "identifier must have 1..24 ASCII letters/digits/_/-"
    );
    Ok(())
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        identifier(&self.symbol)?;
        for v in [
            self.capital,
            self.risk_per_trade,
            self.max_notional,
            self.max_daily_loss,
            self.max_total_loss,
            self.filters.min_price,
            self.filters.max_price,
            self.filters.tick_size,
            self.filters.min_qty,
            self.filters.max_qty,
            self.filters.step_size,
            self.filters.min_notional,
        ] {
            number(v, false)?;
        }
        for v in [self.fee_bps, self.slippage_bps, self.max_spread_bps] {
            number(v, true)?;
        }
        ensure!(
            self.risk_per_trade <= self.max_daily_loss
                && self.max_daily_loss <= self.max_total_loss
                && self.max_total_loss <= self.capital,
            "risk limits must satisfy trade <= daily <= total <= capital"
        );
        ensure!(
            self.max_notional <= self.capital,
            "paper v1 has no leveraged margin model: notional must be <= capital"
        );
        ensure!(
            self.fee_bps <= D::from(100)
                && self.slippage_bps <= D::from(500)
                && self.max_spread_bps <= D::from(100),
            "cost/spread limit outside supported range"
        );
        ensure!(
            (1..=60_000).contains(&self.max_quote_age_ms)
                && (1..=60_000).contains(&self.order_timeout_ms)
                && (1..=60_000).contains(&self.protection_timeout_ms),
            "timeouts must be 1..60000 ms"
        );
        let f = &self.filters;
        ensure!(
            f.min_price <= f.max_price
                && f.min_qty <= f.max_qty
                && f.tick_size <= f.max_price
                && f.step_size <= f.max_qty,
            "invalid instrument bounds"
        );
        ensure!(
            f.min_notional <= self.max_notional,
            "min notional exceeds allocation"
        );
        Ok(())
    }

    pub fn validate_quote(&self, quote: &Quote, now: u64) -> Result<()> {
        number(quote.bid, false)?;
        number(quote.ask, false)?;
        ensure!(quote.bid <= quote.ask, "crossed bid/ask");
        ensure!(
            quote.bid >= self.filters.min_price && quote.ask <= self.filters.max_price,
            "quote outside instrument bounds"
        );
        ensure!(
            quote.observed_at_ms <= now && now - quote.observed_at_ms <= self.max_quote_age_ms,
            "quote is stale or from the future"
        );
        let spread = div(mul(sub(quote.ask, quote.bid)?, D::from(10_000))?, quote.bid)?;
        ensure!(spread <= self.max_spread_bps, "spread exceeds limit");
        Ok(())
    }

    fn price(&self, price: D) -> Result<()> {
        number(price, false)?;
        let f = &self.filters;
        ensure!(
            price >= f.min_price
                && price <= f.max_price
                && sub(price, f.min_price)?.checked_rem(f.tick_size) == Some(D::ZERO),
            "price violates tick filter"
        );
        Ok(())
    }

    pub fn plan(
        &self,
        quote: &Quote,
        now: u64,
        side: Side,
        stop: D,
        take_profit: D,
    ) -> Result<Plan> {
        self.validate_quote(quote, now)?;
        self.price(stop)?;
        self.price(take_profit)?;
        let reference = match side {
            Side::Long => quote.ask,
            Side::Short => quote.bid,
        };
        let worst_entry = match side {
            Side::Long => add(reference, bps(reference, self.slippage_bps)?)?,
            Side::Short => sub(reference, bps(reference, self.slippage_bps)?)?,
        };
        let distance = match side {
            Side::Long => {
                ensure!(
                    stop < quote.bid && take_profit > worst_entry,
                    "invalid long stop/target"
                );
                sub(worst_entry, stop)?
            }
            Side::Short => {
                ensure!(
                    stop > quote.ask && take_profit < worst_entry,
                    "invalid short stop/target"
                );
                sub(stop, worst_entry)?
            }
        };
        let exit_buffer = bps(stop, self.slippage_bps)?;
        let fee_buffer = bps(add(worst_entry, add(stop, exit_buffer)?)?, self.fee_bps)?;
        let per_unit = add(distance, add(exit_buffer, fee_buffer)?)?;
        // Use the higher entry price for a conservative short notional cap too.
        let notional_price = add(reference, bps(reference, self.slippage_bps)?)?;
        let raw = div(self.risk_per_trade, per_unit)?
            .min(div(self.max_notional, notional_price)?)
            .min(self.filters.max_qty);
        ensure!(
            raw >= self.filters.min_qty,
            "risk allocation below minimum quantity"
        );
        let steps = div(sub(raw, self.filters.min_qty)?, self.filters.step_size)?.floor();
        let qty = add(self.filters.min_qty, mul(steps, self.filters.step_size)?)?;
        let notional = mul(qty, notional_price)?;
        ensure!(
            mul(qty, worst_entry.min(reference))? >= self.filters.min_notional,
            "order below minimum notional"
        );
        Ok(Plan {
            qty,
            reference_price: reference,
            worst_entry,
            budgeted_loss: mul(qty, per_unit)?,
            notional,
        })
    }
}
