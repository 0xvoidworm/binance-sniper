//! Bounded USD-M depth snapshot: exchange timestamps, never a fabricated local quote time.
use super::margin;
use anyhow::{Context, Result, ensure};
use binance_engine::Quote;
use rust_decimal::Decimal as D;
use serde_json::Value;
#[cfg(test)]
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Book {
    pub id: u64,
    pub event_ms: u64,
    pub transaction_ms: u64,
    bids: Vec<(D, D)>,
    asks: Vec<(D, D)>,
}
impl Book {
    #[cfg(test)]
    pub fn parse(v: &Value, now: u64, previous: Option<&Self>) -> Result<Self> {
        let clock = super::transport::ClockWindow::new(now, now, Instant::now())
            .map_err(anyhow::Error::new)?;
        Self::parse_with_clock(v, clock, previous)
    }
    pub(crate) fn parse_with_clock(
        v: &Value,
        clock: super::transport::ClockWindow,
        previous: Option<&Self>,
    ) -> Result<Self> {
        let stamp = |name: &str| {
            v[name]
                .as_u64()
                .filter(|x| *x > 0)
                .context("Timestamp/ID depth hilang atau invalid")
        };
        let id = stamp("lastUpdateId")?;
        let event_ms = stamp("E")?;
        let transaction_ms = stamp("T")?;
        ensure!(
            transaction_ms <= event_ms,
            "Urutan timestamp depth invalid; event_ms={event_ms}; transaction_ms={transaction_ms}"
        );
        let levels = |name: &str, descending: bool| -> Result<Vec<(D, D)>> {
            let rows = v[name].as_array().context("Level depth hilang")?;
            ensure!(
                !rows.is_empty() && rows.len() <= 5,
                "Depth harus berisi 1–5 level per sisi"
            );
            let mut parsed = Vec::with_capacity(rows.len());
            for row in rows {
                let row = row.as_array().context("Level depth invalid")?;
                ensure!(row.len() == 2, "Level depth harus price/qty");
                let (price, qty) = (margin::decimal(&row[0])?, margin::decimal(&row[1])?);
                ensure!(
                    price > D::ZERO && qty > D::ZERO,
                    "Price/qty depth harus positif"
                );
                if let Some((last, _)) = parsed.last() {
                    ensure!(
                        if descending {
                            price < *last
                        } else {
                            price > *last
                        },
                        "Urutan/duplikasi depth invalid"
                    );
                }
                parsed.push((price, qty));
            }
            Ok(parsed)
        };
        let book = Self {
            id,
            event_ms,
            transaction_ms,
            bids: levels("bids", true)?,
            asks: levels("asks", false)?,
        };
        ensure!(
            book.bids[0].0 < book.asks[0].0,
            "Depth terkunci/bersilangan"
        );
        if let Some(old) = previous {
            ensure!(
                id >= old.id && event_ms >= old.event_ms && transaction_ms >= old.transaction_ms,
                "Depth bergerak mundur; last_update_id={id}; previous_id={}",
                old.id
            );
            if id == old.id {
                ensure!(
                    book.bids == old.bids
                        && book.asks == old.asks
                        && transaction_ms == old.transaction_ms,
                    "Depth berubah tanpa ID pembaruan baru"
                );
            }
        }
        // Validate shape, prices and replay before temporal classification. A
        // value inside the measured clock interval is not accepted for an
        // order: it requests a fresh synchronization cycle instead.
        clock.validate_age("depth_event", event_ms, 5000)?;
        clock.validate_age("depth", transaction_ms, 5000)?;
        Ok(book)
    }
    pub fn quote(&self) -> Quote {
        Quote {
            bid: self.bids[0].0,
            ask: self.asks[0].0,
            observed_at_ms: self.transaction_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn value() -> Value {
        json!({"lastUpdateId":12345,"E":10000,"T":9999,"bids":[["80349.3","0.991"],["80349.2","0.023"]],"asks":[["80349.4","5.809"],["80349.5","0.004"]]})
    }
    #[test]
    fn r11_only_age_is_recoverable_and_invalid_stale_books_still_stop() {
        use super::super::recovery::transient;
        let old = Book::parse(&value(), 10001, None).unwrap();
        let stale = Book::parse(&value(), 17266, Some(&old)).unwrap_err();
        assert!(transient(&stale.context("depth read")));
        for age in [4999, 5000, 5001, 7267] {
            assert_eq!(Book::parse(&value(), 9999 + age, None).is_ok(), age <= 5000);
        }
        for mode in [
            "future",
            "reversed-time",
            "replay",
            "changed",
            "crossed",
            "schema",
        ] {
            let mut v = value();
            match mode {
                "future" => v["E"] = json!(18000),
                "reversed-time" => v["T"] = json!(10001),
                "replay" => v["lastUpdateId"] = json!(12344),
                "changed" => v["bids"][0][1] = json!("2"),
                "crossed" => v["bids"][0][0] = json!("90000"),
                "schema" => v["bids"] = json!([]),
                _ => unreachable!(),
            }
            let error = Book::parse(&v, 17266, Some(&old)).unwrap_err();
            assert!(!transient(&error), "{mode}: {error}");
        }
        assert!(!transient(&anyhow::anyhow!(
            "Data depth usang; age_ms=7267; limit_ms=5000"
        )));
    }
    #[test]
    fn snapshot_preserves_exchange_time_and_rejects_stale_future_or_inconsistent_depth() {
        let book = Book::parse(&value(), 10001, None).unwrap();
        assert_eq!(book.quote().observed_at_ms, 9999);
        assert_eq!(book.quote().bid.to_string(), "80349.3");
        assert!(Book::parse(&value(), 14999, Some(&book)).is_ok());
        assert!(Book::parse(&value(), 15000, None).is_err());
        for field in ["E", "T", "lastUpdateId"] {
            for n in [0, -1] {
                let mut v = value();
                v[field] = json!(n);
                assert!(Book::parse(&v, 10001, None).is_err());
            }
        }
        let mut v = value();
        v["E"] = json!(10002);
        assert!(Book::parse(&v, 10001, None).is_err());
        v = value();
        v["T"] = json!(10001);
        assert!(Book::parse(&v, 10001, None).is_err());
        v = value();
        v["E"] = json!(16000);
        assert!(
            Book::parse(&v, 16001, None).is_err(),
            "fresh output must not hide an old transaction"
        );
        for bad in [
            json!([]),
            json!([["1", "0"]]),
            json!([["80349.5", "1"]]),
            json!([["80349.3", "1"], ["80349.3", "1"]]),
            json!([["80349.2", "1"], ["80349.3", "1"]]),
        ] {
            v = value();
            v["bids"] = bad;
            assert!(Book::parse(&v, 10001, None).is_err());
        }
    }
    #[test]
    fn replayed_or_changed_snapshot_cannot_reset_freshness_or_regress_sequence() {
        let old = Book::parse(&value(), 10001, None).unwrap();
        let mut v = value();
        v["lastUpdateId"] = json!(12344);
        assert!(Book::parse(&v, 10001, Some(&old)).is_err());
        v = value();
        v["bids"][0][1] = json!("2");
        assert!(Book::parse(&v, 10001, Some(&old)).is_err());
        v["lastUpdateId"] = json!(12346);
        v["E"] = json!(10001);
        v["T"] = json!(10000);
        assert!(Book::parse(&v, 10002, Some(&old)).is_ok());
        assert!(Book::parse(&value(), 15001, Some(&old)).is_err());
    }
    #[tokio::test]
    #[ignore = "bounded public Binance GET comparison; no credentials or orders"]
    async fn compare_public_book_ticker_and_depth() {
        use crate::market::{Market, Transport};
        let mut source = Transport::new().unwrap();
        let mut previous = None;
        for sample in 1..=20 {
            let old = source
                .json(
                    Market::Futures,
                    "ticker/bookTicker",
                    &[("symbol", "BTCUSDT".into())],
                )
                .await
                .unwrap();
            let depth = source
                .json(
                    Market::Futures,
                    "depth",
                    &[("symbol", "BTCUSDT".into()), ("limit", "5".into())],
                )
                .await
                .unwrap();
            let now = source.json(Market::Futures, "time", &[]).await.unwrap()["serverTime"]
                .as_u64()
                .unwrap();
            let book = Book::parse(&depth, now, previous.as_ref()).unwrap();
            println!(
                "PUBLIC_QUOTE_COMPARE; sample={sample}; ticker_age_ms={}; depth_event_age_ms={}; depth_transaction_age_ms={}; depth_bid={}; depth_ask={}; mutations=0",
                i128::from(now) - i128::from(old["time"].as_u64().unwrap()),
                now - book.event_ms,
                now - book.transaction_ms,
                book.quote().bid,
                book.quote().ask
            );
            previous = Some(book);
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
}
