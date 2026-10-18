//! Isolated USDT one-way sizing. Liquidation is an estimate, not an exchange promise.
use anyhow::{Context, Result, ensure};
use binance_engine::{Filters, Quote, Side};
use rust_decimal::Decimal as D;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub fn decimal(v: &Value) -> Result<D> {
    let text = match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => anyhow::bail!("Tipe desimal Futures tidak valid"),
    };
    let n = text
        .parse::<D>()
        .or_else(|_| D::from_scientific(&text))
        .map_err(|_| anyhow::anyhow!("Desimal Futures tidak valid"))?;
    number(n, true)?;
    Ok(n)
}
pub(crate) fn number(n: D, zero: bool) -> Result<()> {
    ensure!(
        n >= D::ZERO
            && (zero || n > D::ZERO)
            && n <= D::from(1_000_000_000_000_u64)
            && n.scale() <= 12,
        "Angka Futures di luar batas"
    );
    Ok(())
}
fn mul(a: D, b: D) -> Result<D> {
    a.checked_mul(b).context("Overflow margin")
}
fn div(a: D, b: D) -> Result<D> {
    a.checked_div(b).context("Pembagian margin invalid")
}
fn add(a: D, b: D) -> Result<D> {
    a.checked_add(b).context("Overflow margin")
}
fn sub(a: D, b: D) -> Result<D> {
    a.checked_sub(b).context("Overflow margin")
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tier {
    pub floor: D,
    pub cap: D,
    pub max_leverage: u32,
    pub maintenance_rate: D,
    pub cumulative: D,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Brackets {
    pub tiers: Vec<Tier>,
}
impl Brackets {
    pub fn parse(value: &Value, symbol: &str) -> Result<Self> {
        let item = if let Some(rows) = value.as_array() {
            ensure!(rows.len() == 1, "Bracket harus berisi satu simbol");
            &rows[0]
        } else {
            value
        };
        ensure!(item["symbol"] == symbol, "Simbol bracket tidak cocok");
        // Custom coefficients need an explicit contract for whether returned
        // thresholds are pre-scaled; never silently multiply them twice.
        if let Some(coef) = item.get("notionalCoef") {
            ensure!(
                decimal(coef)? == D::ONE,
                "Bracket akun dengan koefisien khusus belum didukung"
            );
        }
        let rows = item["brackets"].as_array().context("Bracket akun hilang")?;
        ensure!(
            !rows.is_empty() && rows.len() <= 128,
            "Jumlah bracket invalid"
        );
        let mut tiers = Vec::new();
        for (i, r) in rows.iter().enumerate() {
            ensure!(
                r["bracket"].as_u64() == Some(i as u64 + 1),
                "Urutan bracket invalid"
            );
            tiers.push(Tier {
                floor: decimal(&r["notionalFloor"])?,
                cap: decimal(&r["notionalCap"])?,
                max_leverage: u32::try_from(
                    r["initialLeverage"]
                        .as_u64()
                        .context("Leverage bracket invalid")?,
                )?,
                maintenance_rate: decimal(&r["maintMarginRatio"])?,
                cumulative: decimal(&r["cum"])?,
            });
        }
        let brackets = Self { tiers };
        brackets.validate()?;
        Ok(brackets)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.tiers.is_empty() && self.tiers.len() <= 128,
            "Bracket kosong/terlalu besar"
        );
        ensure!(
            self.tiers[0].floor == D::ZERO && self.tiers[0].cumulative == D::ZERO,
            "Awal bracket invalid"
        );
        for (i, t) in self.tiers.iter().enumerate() {
            for n in [t.floor, t.cap, t.maintenance_rate, t.cumulative] {
                number(n, true)?;
            }
            ensure!(t.cap > t.floor, "BRACKET_NOTIONAL_BOUNDS");
            // Metadata may exceed the documented command cap (125). Reading
            // that tier does not authorize requesting leverage above 125.
            ensure!(
                (1..=1000).contains(&t.max_leverage),
                "BRACKET_LEVERAGE_BOUNDS"
            );
            ensure!(
                t.maintenance_rate > D::ZERO && t.maintenance_rate < D::ONE,
                "BRACKET_MAINTENANCE_RATE_BOUNDS"
            );
            if i > 0 {
                let prev = &self.tiers[i - 1];
                ensure!(
                    prev.cap == t.floor
                        && prev.max_leverage >= t.max_leverage
                        && prev.maintenance_rate <= t.maintenance_rate,
                    "Bracket gap/urutan invalid"
                );
                let expected = add(
                    prev.cumulative,
                    mul(t.floor, sub(t.maintenance_rate, prev.maintenance_rate)?)?,
                )?;
                ensure!(
                    expected == t.cumulative,
                    "Cumulative maintenance tidak konsisten"
                );
            }
        }
        Ok(())
    }
    pub fn tier(&self, notional: D) -> Result<&Tier> {
        self.tiers
            .iter()
            .enumerate()
            .find(|(i, t)| {
                notional >= t.floor
                    && (notional < t.cap || (*i + 1 == self.tiers.len() && notional == t.cap))
            })
            .map(|(_, t)| t)
            .context("Notional di luar bracket akun")
    }
    pub fn maintenance(&self, notional: D) -> Result<D> {
        let t = self.tier(notional)?;
        sub(mul(notional, t.maintenance_rate)?, t.cumulative)
    }
    pub fn liquidation(&self, qty: D, entry: D, cash: D, side: Side) -> Result<D> {
        for n in [qty, entry, cash] {
            ensure!(
                n > D::ZERO && n <= D::from(1_000_000_000_000_u64),
                "Input estimasi likuidasi invalid"
            );
        }
        let entry_notional = mul(qty, entry)?;
        for t in &self.tiers {
            let (numerator, denominator) = match side {
                Side::Long => (
                    sub(sub(entry_notional, cash)?, t.cumulative)?,
                    mul(qty, sub(D::ONE, t.maintenance_rate)?)?,
                ),
                Side::Short => (
                    add(add(entry_notional, cash)?, t.cumulative)?,
                    mul(qty, add(D::ONE, t.maintenance_rate)?)?,
                ),
            };
            if numerator <= D::ZERO {
                continue;
            }
            let price = div(numerator, denominator)?;
            if self
                .tier(mul(qty, price)?)
                .is_ok_and(|selected| selected == t)
            {
                return Ok(price);
            }
        }
        anyhow::bail!("Estimasi likuidasi di luar rentang bracket")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    pub margin: D,
    pub risk: D,
    pub max_notional: D,
    pub leverage: u32,
    pub slippage_bps: D,
    pub liquidation_buffer_bps: D,
    pub funding_reserve: D,
}
#[derive(Clone)]
pub struct ContextData {
    pub filters: Filters,
    pub quote: Quote,
    pub mark: D,
    pub now: u64,
    pub available: D,
    pub taker_rate: D,
    pub brackets: Brackets,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub side: Side,
    pub qty: D,
    pub worst_entry: D,
    pub stop: D,
    pub take_profit: D,
    pub notional: D,
    pub initial_margin: D,
    pub maintenance_margin: D,
    pub fee_reserve: D,
    pub funding_reserve: D,
    pub required_balance: D,
    pub budgeted_loss: D,
    pub liquidation_estimate: D,
    pub leverage: u32,
}

pub fn plan(ctx: &ContextData, budget: &Budget, side: Side, stop: D, tp: D) -> Result<Plan> {
    ctx.brackets.validate()?;
    for n in [
        budget.margin,
        budget.risk,
        budget.max_notional,
        ctx.quote.bid,
        ctx.quote.ask,
        ctx.mark,
        stop,
        tp,
    ] {
        number(n, false)?;
    }
    for n in [
        ctx.available,
        ctx.taker_rate,
        budget.slippage_bps,
        budget.liquidation_buffer_bps,
        budget.funding_reserve,
    ] {
        number(n, true)?;
    }
    ensure!(
        (1..=125).contains(&budget.leverage)
            && budget.risk <= budget.margin
            && budget.margin <= ctx.available
            && budget.funding_reserve < budget.risk
            && budget.slippage_bps <= D::from(500)
            && budget.liquidation_buffer_bps <= D::from(1000)
            && ctx.taker_rate <= D::new(1, 2),
        "Budget/saldo/biaya/leverage invalid"
    );
    ensure!(
        ctx.quote.bid <= ctx.quote.ask
            && ctx.quote.observed_at_ms <= ctx.now
            && ctx.now - ctx.quote.observed_at_ms <= 5000,
        "Quote invalid/usang"
    );
    ensure!(
        div(
            mul(sub(ctx.quote.ask, ctx.quote.bid)?, D::from(10_000))?,
            ctx.quote.bid
        )? <= D::from(5),
        "Spread terlalu besar"
    );
    let f = &ctx.filters;
    for n in [
        f.min_price,
        f.max_price,
        f.tick_size,
        f.min_qty,
        f.max_qty,
        f.step_size,
        f.min_notional,
    ] {
        number(n, false)?;
    }
    ensure!(
        f.min_price <= f.max_price && f.min_qty <= f.max_qty,
        "Filter Futures invalid"
    );
    for p in [stop, tp] {
        ensure!(
            p >= f.min_price && p <= f.max_price && p.checked_rem(f.tick_size) == Some(D::ZERO),
            "Stop/target tidak sesuai tick"
        );
    }
    ensure!(
        ctx.quote.bid >= f.min_price
            && ctx.quote.ask <= f.max_price
            && ctx.mark >= f.min_price
            && ctx.mark <= f.max_price,
        "Harga di luar filter"
    );
    let slip = div(budget.slippage_bps, D::from(10_000))?;
    let (entry, exit, distance) = match side {
        Side::Long => {
            let entry = mul(ctx.quote.ask, add(D::ONE, slip)?)?;
            let exit = mul(stop, sub(D::ONE, slip)?)?;
            ensure!(
                stop < ctx.quote.bid.min(ctx.mark) && tp > entry.max(ctx.mark),
                "Arah stop/target long invalid"
            );
            (entry, exit, sub(entry, exit)?)
        }
        Side::Short => {
            let entry = mul(ctx.quote.bid, sub(D::ONE, slip)?)?;
            let exit = mul(stop, add(D::ONE, slip)?)?;
            ensure!(
                stop > ctx.quote.ask.max(ctx.mark) && tp < entry.min(ctx.mark),
                "Arah stop/target short invalid"
            );
            (entry, exit, sub(exit, entry)?)
        }
    };
    let conservative_price = mul(ctx.quote.ask, add(D::ONE, slip)?)?;
    let fee_per_unit = mul(add(conservative_price, exit.max(tp))?, ctx.taker_rate)?;
    let risk_qty = div(
        sub(budget.risk, budget.funding_reserve)?,
        add(distance, fee_per_unit)?,
    )?;
    let margin_per_unit = add(
        div(conservative_price, D::from(budget.leverage))?,
        fee_per_unit,
    )?;
    let margin_qty = div(sub(budget.margin, budget.funding_reserve)?, margin_per_unit)?;
    let raw = risk_qty
        .min(margin_qty)
        .min(div(budget.max_notional, conservative_price)?)
        .min(f.max_qty);
    let qty = mul(div(raw, f.step_size)?.floor(), f.step_size)?.normalize();
    ensure!(
        qty >= f.min_qty
            && qty <= f.max_qty
            && mul(qty, entry.min(ctx.quote.bid))? >= f.min_notional,
        "Budget tidak mencapai minimum order"
    );
    let notional = mul(qty, conservative_price)?;
    let tier = ctx.brackets.tier(notional)?;
    ensure!(
        budget.leverage <= tier.max_leverage,
        "Leverage melebihi bracket notional"
    );
    let initial_margin = div(notional, D::from(budget.leverage))?;
    let fee_reserve = mul(qty, fee_per_unit)?;
    let required_balance = add(add(initial_margin, fee_reserve)?, budget.funding_reserve)?;
    let budgeted_loss = add(
        mul(qty, add(distance, fee_per_unit)?)?,
        budget.funding_reserve,
    )?;
    ensure!(
        required_balance <= budget.margin && budgeted_loss <= budget.risk,
        "Pembulatan melewati budget"
    );
    // The higher ask-side notional reserves enough balance for either side.
    // It must not invent extra isolated collateral for a lower-priced short fill.
    let liquidation_margin = div(mul(qty, entry)?, D::from(budget.leverage))?;
    let cash = sub(
        sub(liquidation_margin, fee_reserve)?,
        budget.funding_reserve,
    )?;
    let liquidation_estimate = ctx.brackets.liquidation(qty, entry, cash, side)?;
    let buffer = mul(entry, div(budget.liquidation_buffer_bps, D::from(10_000))?)?;
    ensure!(
        match side {
            Side::Long => stop > add(liquidation_estimate, buffer)?,
            Side::Short => stop < sub(liquidation_estimate, buffer)?,
        },
        "Stop terlalu dekat atau melewati estimasi likuidasi"
    );
    Ok(Plan {
        side,
        qty,
        worst_entry: entry,
        stop,
        take_profit: tp,
        notional,
        initial_margin,
        maintenance_margin: ctx.brackets.maintenance(notional)?,
        fee_reserve,
        funding_reserve: budget.funding_reserve,
        required_balance,
        budgeted_loss,
        liquidation_estimate,
        leverage: budget.leverage,
    })
}
