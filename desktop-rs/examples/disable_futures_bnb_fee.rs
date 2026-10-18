//! Explicitly authorized fee preference only; never starts a trading session.
#![allow(dead_code)]

#[path = "../src/account.rs"]
mod account;
#[path = "../src/futures/margin.rs"]
mod margin;
#[path = "../src/market.rs"]
mod market;
#[allow(clippy::upper_case_acronyms)]
#[path = "../src/futures/protocol.rs"]
mod protocol;
#[path = "../src/futures/transport.rs"]
mod transport;
#[path = "../src/vault.rs"]
mod vault;

use anyhow::{Context, Result, ensure};
use protocol::{Read, Spec};
use reqwest::Method;
use std::{collections::BTreeMap, time::Duration};
use transport::{Access, Connector, Exchange};

fn disable_spec() -> Spec {
    Spec {
        method: Method::POST,
        path: "/fapi/v1/feeBurn",
        params: BTreeMap::from([("feeBurn".into(), "false".into())]),
    }
}

async fn fee_enabled(reader: &mut Connector) -> Result<bool> {
    reader.read(Read::FeeBurn).await?["feeBurn"]
        .as_bool()
        .context("Fee BNB status missing; no assumption made")
}

fn main() -> Result<()> {
    ensure!(
        std::env::args().skip(1).collect::<Vec<_>>() == ["--disable"],
        "Usage after explicit user authorization: disable_futures_bnb_fee --disable"
    );
    let credentials = vault::load().map_err(|e| anyhow::anyhow!(e.message()))?;
    let mut reader = Connector::new(credentials.clone(), Access::ReadOnly)?;
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            tokio::time::timeout(Duration::from_secs(60), async {
                let before = fee_enabled(&mut reader).await?;
                println!("FUTURES_BNB_FEE_BEFORE; enabled={before}");
                if before {
                    let mut route = account::Transport::new()
                        .map_err(|e| anyhow::anyhow!(e.message()))?;
                    let (client, clock) = route.prepare().await
                        .map_err(|e| anyhow::anyhow!(e.message()))?;
                    let now = clock.signing_at(std::time::Instant::now())
                        .map_err(|e| anyhow::anyhow!(e.message()))?;
                    let request = disable_spec().signed(&client, &credentials, now)?;
                    // Exactly one mutation attempt. Never print requests, raw errors or responses.
                    match client.execute(request).await {
                        Ok(response) => {
                            let status = response.status();
                            match market::bounded_json(response).await {
                                Ok(value) => {
                                    let code = protocol::code(&value);
                                    if let Err(error) = transport::classify(status, code, true) {
                                        println!("FUTURES_BNB_FEE_POST; result={error}; retried=false");
                                    } else if code == Some(200) {
                                        println!("FUTURES_BNB_FEE_POST; accepted=true; retried=false");
                                    } else {
                                        println!("FUTURES_BNB_FEE_POST; ack_unrecognized=true; retried=false");
                                    }
                                }
                                Err(_) => println!("FUTURES_BNB_FEE_POST; result=UNKNOWN_RESPONSE; retried=false"),
                            }
                        }
                        Err(_) => println!("FUTURES_BNB_FEE_POST; result=UNKNOWN_NETWORK; retried=false"),
                    }
                }
                for snapshot in 1..=2 {
                    let enabled = fee_enabled(&mut reader).await?;
                    println!("FUTURES_BNB_FEE_VERIFY; snapshot={snapshot}; enabled={enabled}");
                    ensure!(!enabled, "Fee BNB remains ON; no mutation retry performed");
                }
                println!("FUTURES_BNB_FEE_OFF_CONFIRMED; mutation_attempts={}; orders_sent=0", u8::from(before));
                Ok::<(), anyhow::Error>(())
            })
            .await
            .context("Fee preference operation timed out; verify with GET before retrying")?
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_only_disables_futures_fee_bnb() {
        let credentials =
            vault::Credentials::new("dummy_public_key_123".into(), "dummy_secret_123456".into())
                .unwrap();
        let request = disable_spec()
            .signed(&reqwest::Client::new(), &credentials, 1_780_000_000_000)
            .unwrap();
        assert_eq!(request.method(), Method::POST);
        assert_eq!(
            request.url().as_str(),
            "https://fapi.binance.com/fapi/v1/feeBurn"
        );
        assert!(request.url().query().is_none());
        let body = std::str::from_utf8(request.body().unwrap().as_bytes().unwrap()).unwrap();
        let fields: BTreeMap<_, _> = body
            .split('&')
            .map(|part| part.split_once('=').unwrap())
            .collect();
        assert_eq!(fields.len(), 4);
        assert_eq!(fields["feeBurn"], "false");
        assert_eq!(fields["recvWindow"], "5000");
        assert!(fields.contains_key("timestamp"));
        assert!(fields.contains_key("signature"));
        assert!(request.headers()["x-mbx-apikey"].is_sensitive());
    }
}
