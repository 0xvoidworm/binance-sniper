//! Exact realized results from exchange fills; no modeled profit substitutions.
use super::{
    margin,
    protocol::{OrderStatus, Read, Symbol, opposite, side_name},
    transport::Exchange,
};
use anyhow::{Context, Result, ensure};
use binance_engine::Side;
use rust_decimal::Decimal as D;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub fn signed(v: &Value) -> Result<D> {
    let text = v.as_str().context("Desimal ledger harus string")?;
    let n: D = text
        .parse()
        .map_err(|_| anyhow::anyhow!("Desimal ledger invalid"))?;
    margin::number(n.abs(), true)?;
    Ok(n)
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settlement {
    pub entry_qty: D,
    pub exit_qty: D,
    pub gross: D,
    pub fees: D,
    pub funding: D,
    pub net: D,
    pub fill_ids: Vec<u64>,
    pub funding_ids: Vec<u64>,
    pub last_fill_ms: u64,
}
pub struct Query<'a> {
    pub symbol: Symbol,
    pub side: Side,
    pub entry: u64,
    pub exits: &'a [u64],
    pub start: u64,
    pub end: u64,
    pub funding_due: &'a [u64],
}
pub async fn settle<E: Exchange>(exchange: &mut E, q: Query<'_>) -> Result<Settlement> {
    Ok(settle_detailed(exchange, q).await?.result)
}
pub(crate) struct Detailed {
    pub(crate) result: Settlement,
    pub(crate) filled_exit_orders: Vec<u64>,
}
pub(crate) async fn settle_detailed<E: Exchange>(
    exchange: &mut E,
    q: Query<'_>,
) -> Result<Detailed> {
    ensure!(
        q.entry > 0 && q.start > 0 && q.end >= q.start && q.end - q.start <= 7 * 86_400_000,
        "Rentang pembukuan invalid"
    );
    let mut result = Settlement {
        entry_qty: D::ZERO,
        exit_qty: D::ZERO,
        gross: D::ZERO,
        fees: D::ZERO,
        funding: D::ZERO,
        net: D::ZERO,
        fill_ids: vec![],
        funding_ids: vec![],
        last_fill_ms: 0,
    };
    let mut order_ids = BTreeSet::new();
    let mut filled_exit_orders = Vec::new();
    let mut fill_ids = BTreeSet::new();
    let mut entry_quote = D::ZERO;
    let mut exit_quote = D::ZERO;
    let mut first_entry_ms = u64::MAX;
    for id in std::iter::once(q.entry).chain(q.exits.iter().copied()) {
        ensure!(order_ids.insert(id), "Order pembukuan duplikat");
        let order = exchange
            .read(Read::OrderId {
                symbol: q.symbol,
                order_id: id,
            })
            .await?;
        let status: OrderStatus = serde_json::from_value(order["status"].clone())
            .map_err(|_| anyhow::anyhow!("Status fill ledger invalid"))?;
        let side = if id == q.entry {
            q.side
        } else {
            opposite(q.side)
        };
        ensure!(
            order["orderId"].as_u64() == Some(id)
                && order["symbol"] == q.symbol.name()
                && order["side"] == side_name(side)
                && order["positionSide"] == "BOTH"
                && status.terminal(),
            "Order ledger belum terminal/identitas berbeda"
        );
        let expected = margin::decimal(&order["executedQty"])?;
        let expected_quote = margin::decimal(&order["cumQuote"])?;
        let mut qty = D::ZERO;
        let mut quote = D::ZERO;
        let mut from_id = None;
        let mut complete = false;
        for _ in 0..20 {
            let response = exchange
                .read(Read::Trades {
                    symbol: q.symbol,
                    order_id: id,
                    from_id,
                })
                .await?;
            let rows = response.as_array().context("Daftar fill invalid")?;
            ensure!(rows.len() <= 1000, "Halaman fill melewati batas");
            let mut max_id = 0;
            for row in rows {
                let fill = row["id"].as_u64().context("ID fill hilang")?;
                let at = row["time"].as_u64().context("Waktu fill hilang")?;
                ensure!(
                    fill > 0
                        && fill_ids.insert(fill)
                        && from_id.is_none_or(|n| fill >= n)
                        && row["symbol"] == q.symbol.name()
                        && row["orderId"].as_u64() == Some(id)
                        && row["side"] == side_name(side)
                        && row["positionSide"] == "BOTH"
                        && row["commissionAsset"] == "USDT"
                        && at >= q.start
                        && at <= q.end,
                    "Fill duplikat/asing/biaya non-USDT"
                );
                let amount = margin::decimal(&row["qty"])?;
                let value = margin::decimal(&row["quoteQty"])?;
                ensure!(amount > D::ZERO && value > D::ZERO, "Fill nol invalid");
                qty += amount;
                quote += value;
                result.fees += signed(&row["commission"])?;
                let gross = signed(&row["realizedPnl"])?;
                ensure!(
                    id != q.entry || gross.is_zero(),
                    "Entry memiliki realized PnL; posisi awal tidak flat"
                );
                result.gross += gross;
                result.last_fill_ms = result.last_fill_ms.max(at);
                if id == q.entry {
                    first_entry_ms = first_entry_ms.min(at);
                }
                max_id = max_id.max(fill);
            }
            if rows.len() < 1000 {
                complete = true;
                break;
            }
            from_id = Some(max_id.checked_add(1).context("ID fill overflow")?);
        }
        ensure!(
            complete && qty == expected && (quote - expected_quote).abs() <= D::new(1, 8),
            "Fill/quote belum lengkap; jangan mulai trade baru"
        );
        if id == q.entry {
            result.entry_qty = qty;
            entry_quote = quote;
        } else {
            result.exit_qty += qty;
            exit_quote += quote;
            if qty > D::ZERO {
                filled_exit_orders.push(id);
            }
        }
    }
    ensure!(
        result.entry_qty > D::ZERO && result.entry_qty == result.exit_qty,
        "Jumlah entry/exit belum cocok"
    );
    let expected_gross = match q.side {
        Side::Long => exit_quote - entry_quote,
        Side::Short => entry_quote - exit_quote,
    };
    ensure!(
        (result.gross - expected_gross).abs() <= D::new(1, 6),
        "Realized PnL tidak cocok dengan fill"
    );
    let mut funding_ids = BTreeSet::new();
    let mut funding_times = vec![];
    let mut complete = false;
    for page in 1..=20 {
        let response = exchange
            .read(Read::Funding {
                symbol: q.symbol,
                start: q.start,
                end: q.end,
                page,
            })
            .await?;
        let rows = response.as_array().context("Riwayat funding invalid")?;
        ensure!(rows.len() <= 1000, "Halaman funding melewati batas");
        for row in rows {
            let id = row["tranId"].as_u64().context("ID funding hilang")?;
            let time = row["time"].as_u64().context("Waktu funding hilang")?;
            ensure!(
                funding_ids.insert(id)
                    && row["symbol"] == q.symbol.name()
                    && row["incomeType"] == "FUNDING_FEE"
                    && row["asset"] == "USDT"
                    && time >= q.start
                    && time <= q.end,
                "Riwayat funding duplikat/asing"
            );
            result.funding += signed(&row["income"])?;
            funding_times.push(time);
        }
        if rows.len() < 1000 {
            complete = true;
            break;
        }
    }
    ensure!(
        complete
            && q.funding_due
                .iter()
                .filter(|due| **due >= first_entry_ms && **due <= result.last_fill_ms)
                .all(|due| funding_times.iter().any(|t| t.abs_diff(*due) <= 30_000)),
        "Funding yang terlewati belum tercatat"
    );
    result.net = result.gross - result.fees + result.funding;
    result.fill_ids = fill_ids.into_iter().collect();
    result.funding_ids = funding_ids.into_iter().collect();
    Ok(Detailed {
        result,
        filled_exit_orders,
    })
}
