//! GET-only activation checks. No session store or execution supervisor is loaded.
#![allow(dead_code)]

#[path = "../src/account.rs"]
mod account;
#[path = "../src/futures/margin.rs"]
mod margin;
#[path = "../src/market.rs"]
mod market;
// Preserve the existing exchange symbol identifiers in this diagnostic target.
#[allow(clippy::upper_case_acronyms)]
#[path = "../src/futures/protocol.rs"]
mod protocol;
#[path = "../src/futures/transport.rs"]
mod transport;
#[path = "../src/vault.rs"]
mod vault;

use anyhow::{Context, Result, ensure};
use protocol::{Read, Symbol};
use rust_decimal::Decimal as D;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use transport::{Access, Connector, Exchange};

fn main() -> Result<()> {
    let budget = std::env::args()
        .nth(1)
        .context("Usage: diagnose_activation <margin-budget-USDT>")?
        .parse::<D>()
        .context("Budget must be decimal")?;
    margin::number(budget, false)?;
    let credentials = vault::load().map_err(|e| anyhow::anyhow!(e.message()))?;
    let mut exchange = Connector::new(credentials, Access::ReadOnly)?;
    ensure!(!exchange.live_enabled(), "Diagnostic must be read-only");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(60), async {
                for snapshot in 1..=2 {
                    let balances = exchange.read(Read::Account).await?;
                    let positions = exchange.read(Read::Positions).await?;
                    let config = exchange.read(Read::AccountConfig).await?;
                    let symbols = exchange.read(Read::Symbols).await?;
                    let max_notional = symbols
                        .as_array()
                        .and_then(|rows| rows.iter().find(|s| s["symbol"] == "BTCUSDT"))
                        .and_then(|s| s.get("maxNotionalValue"))
                        .map(margin::decimal)
                        .transpose()?
                        .unwrap_or(D::ZERO);
                    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
                    let data = account::Data::parse(balances, positions, config, symbols, now)
                        .map_err(|e| anyhow::anyhow!(e.message()))?;
                    let symbol = data.symbol("BTCUSDT").context("BTCUSDT config missing")?;
                    let orders = exchange.read(Read::OpenOrders(Symbol::BTCUSDT)).await?;
                    let algos = exchange.read(Read::OpenAlgos(Symbol::BTCUSDT)).await?;
                    let fee = exchange.read(Read::FeeBurn).await?;
                    let fee_bnb = fee["feeBurn"].as_bool().context("Fee BNB flag missing")?;
                    let checks = [
                        ("account_can_trade", data.config.can_trade),
                        ("one_way", !data.config.dual_side_position),
                        ("single_asset_usdt", !data.config.multi_assets_margin),
                        ("auto_add_margin_off", !symbol.is_auto_add_margin),
                        ("no_open_positions", data.positions.iter().all(|p| p.position_amt.is_zero())),
                        ("no_btc_orders", orders.as_array().context("Order list invalid")?.is_empty()),
                        ("no_btc_algos", algos.as_array().context("Algo list invalid")?.is_empty()),
                        ("budget_fits_available_balance", budget <= data.balances.available_balance),
                        ("account_notional_positive", max_notional > D::ZERO),
                        ("fee_bnb_off", !fee_bnb),
                    ];
                    for (check, pass) in checks {
                        println!("snapshot={snapshot}; check={check}; pass={pass}");
                    }
                }
                println!("ACTIVATION_DIAGNOSIS_COMPLETE; snapshots=2; account_values=REDACTED; mutations=0");
                Ok::<(), anyhow::Error>(())
            })
            .await
            .context("Activation diagnosis timed out")?
        })
}
