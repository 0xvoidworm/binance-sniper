use super::strategy::{self, Bar, INTERVAL, decimal};
use crate::market::{Market, Transport};
use anyhow::{Context, Result, ensure};
use binance_engine::{Filters, Quote};
use serde_json::Value;
use std::time::Duration;

#[derive(Clone)]
pub struct Batch {
    pub now: u64,
    pub quote: Quote,
    pub next_funding: u64,
    pub bars: Vec<Bar>,
    pub filters: Filters,
}
pub struct Public {
    transport: Transport,
    history: Vec<Bar>,
    filters: Option<Filters>,
    epoch: u64,
}
impl Public {
    pub fn new() -> Result<Self> {
        Ok(Self {
            transport: Transport::new()?,
            history: vec![],
            filters: None,
            epoch: 0,
        })
    }
    pub fn cooldown(&self) -> Duration {
        self.transport.public_cooldown()
    }
    async fn get(&mut self, path: &str, query: &[(&str, String)]) -> Result<Value> {
        self.transport.json(Market::Futures, path, query).await
    }
    pub async fn fetch(&mut self) -> Result<Batch> {
        // Fix the closed-bar cutoff BEFORE fetching klines. A cached forming
        // candle must never become final merely because wall time advanced.
        let before = self.get("time", &[]).await?["serverTime"]
            .as_u64()
            .context("Waktu sebelum candle invalid")?;
        let epoch = before / INTERVAL;
        if self.filters.is_none() || epoch != self.epoch {
            let info = self.get("exchangeInfo", &[]).await?;
            self.filters = Some(filters(&info)?);
            let history = self
                .get(
                    "klines",
                    &[
                        ("symbol", "BTCUSDT".into()),
                        ("interval", "5m".into()),
                        ("limit", "200".into()),
                    ],
                )
                .await?;
            self.history = strategy::bars(&history, before)?;
            self.epoch = epoch;
        }
        let book = self
            .get("ticker/bookTicker", &[("symbol", "BTCUSDT".into())])
            .await?;
        let premium = self
            .get("premiumIndex", &[("symbol", "BTCUSDT".into())])
            .await?;
        let time = self.get("time", &[]).await?;
        let now = time["serverTime"]
            .as_u64()
            .context("Waktu server invalid")?;
        ensure!(
            now >= before && now <= 4_102_444_800_000,
            "Waktu server di luar batas"
        );
        ensure!(
            book["symbol"] == "BTCUSDT" && premium["symbol"] == "BTCUSDT",
            "Simbol quote salah"
        );
        let timestamp = book["time"].as_u64().context("Timestamp quote hilang")?;
        ensure!(
            timestamp <= now && now - timestamp <= 5000,
            "Quote stale/future"
        );
        let premium_ms = premium["time"]
            .as_u64()
            .context("Timestamp funding hilang")?;
        ensure!(
            premium_ms <= now && now - premium_ms <= 10_000,
            "Data funding stale"
        );
        let next_funding = premium["nextFundingTime"]
            .as_u64()
            .context("Jadwal funding hilang")?;
        ensure!(next_funding > now, "Jadwal funding belum diperbarui");
        Ok(Batch {
            now,
            quote: Quote {
                bid: decimal(&book["bidPrice"])?,
                ask: decimal(&book["askPrice"])?,
                observed_at_ms: timestamp,
            },
            next_funding,
            bars: self.history.clone(),
            filters: self.filters.clone().context("Filter belum tersedia")?,
        })
    }
}
pub fn filters(value: &Value) -> Result<Filters> {
    let symbol = value["symbols"]
        .as_array()
        .context("Metadata simbol hilang")?
        .iter()
        .find(|s| s["symbol"] == "BTCUSDT")
        .context("BTCUSDT tidak ditemukan")?;
    ensure!(
        symbol["status"] == "TRADING"
            && symbol["contractType"] == "PERPETUAL"
            && symbol["quoteAsset"] == "USDT",
        "Kontrak BTC tidak didukung"
    );
    let rows = symbol["filters"].as_array().context("Filter hilang")?;
    let get = |name: &str| {
        rows.iter()
            .find(|f| f["filterType"] == name)
            .context("Filter wajib hilang")
    };
    let price = get("PRICE_FILTER")?;
    let lot = get("LOT_SIZE")?;
    let market = get("MARKET_LOT_SIZE")?;
    let step = decimal(&lot["stepSize"])?;
    let market_step = decimal(&market["stepSize"])?;
    ensure!(
        market_step.is_zero() || market_step == step,
        "Step market/lot berbeda; perlu pemetaan khusus"
    );
    Ok(Filters {
        min_price: decimal(&price["minPrice"])?,
        max_price: decimal(&price["maxPrice"])?,
        tick_size: decimal(&price["tickSize"])?,
        min_qty: decimal(&lot["minQty"])?.max(decimal(&market["minQty"])?),
        max_qty: decimal(&lot["maxQty"])?.min(decimal(&market["maxQty"])?),
        step_size: step,
        min_notional: decimal(&get("MIN_NOTIONAL")?["notional"])?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal as D;

    #[test]
    fn official_btc_metadata_uses_market_bounds_and_rejects_missing_filters() {
        let mut value: Value =
            serde_json::from_str(include_str!("../../fixtures/btc-futures-public.json")).unwrap();
        let parsed = filters(&value).unwrap();
        assert_eq!(parsed.tick_size, D::new(1, 1));
        assert_eq!(parsed.step_size, D::new(1, 3));
        assert_eq!(parsed.min_notional, D::from(50));
        let market = value["symbols"][0]["filters"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["filterType"] == "MARKET_LOT_SIZE")
            .unwrap();
        market["minQty"] = "0.003".into();
        market["maxQty"] = "0.025".into();
        let limits = filters(&value).unwrap();
        assert_eq!(limits.min_qty, D::new(3, 3));
        assert_eq!(limits.max_qty, D::new(25, 3));
        let market = value["symbols"][0]["filters"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["filterType"] == "MARKET_LOT_SIZE")
            .unwrap();
        market["stepSize"] = "0.002".into();
        assert!(filters(&value).is_err());
        value["symbols"][0]["filters"]
            .as_array_mut()
            .unwrap()
            .retain(|f| f["filterType"] != "PRICE_FILTER");
        assert!(filters(&value).is_err());
    }

    #[tokio::test]
    async fn public_transport_rejects_orders_accounts_and_arbitrary_urls_before_network() {
        let mut source = Public::new().unwrap();
        for path in [
            "order",
            "account",
            "../order",
            "https://example.com",
            "leverage",
            "algoOrder",
        ] {
            assert_eq!(
                source.get(path, &[]).await.unwrap_err().to_string(),
                "Endpoint publik ditolak"
            );
        }
    }
}
