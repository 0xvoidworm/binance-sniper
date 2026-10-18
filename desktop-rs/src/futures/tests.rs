use super::{
    margin::*,
    outbox::*,
    protocol::*,
    transport::{self, Access, Connector, Exchange},
};
use binance_engine::{Filters, Quote, Side};
use reqwest::{Method, StatusCode};
use rust_decimal::Decimal as D;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::PermissionsExt};

fn d(n: &str) -> D {
    n.parse().unwrap()
}
fn tiers() -> Brackets {
    Brackets {
        tiers: vec![
            Tier {
                floor: D::ZERO,
                cap: d("1000"),
                max_leverage: 20,
                maintenance_rate: d("0.004"),
                cumulative: D::ZERO,
            },
            Tier {
                floor: d("1000"),
                cap: d("10000"),
                max_leverage: 10,
                maintenance_rate: d("0.01"),
                cumulative: d("6"),
            },
            Tier {
                floor: d("10000"),
                cap: d("100000"),
                max_leverage: 5,
                maintenance_rate: d("0.025"),
                cumulative: d("156"),
            },
        ],
    }
}
pub(crate) fn context() -> ContextData {
    ContextData {
        filters: Filters {
            min_price: d("0.01"),
            max_price: d("1000000"),
            tick_size: d("0.01"),
            min_qty: d("0.001"),
            max_qty: d("1000"),
            step_size: d("0.001"),
            min_notional: d("5"),
        },
        quote: Quote {
            bid: d("100"),
            ask: d("100.01"),
            observed_at_ms: 100000,
        },
        mark: d("100"),
        now: 100000,
        available: d("1000"),
        taker_rate: d("0.0006"),
        brackets: tiers(),
    }
}
fn budget() -> Budget {
    Budget {
        margin: d("50"),
        risk: d("10"),
        max_notional: d("1000"),
        leverage: 10,
        slippage_bps: d("1"),
        liquidation_buffer_bps: d("10"),
        funding_reserve: d("0.2"),
    }
}
fn entry() -> Command {
    Command::Market {
        symbol: Symbol::BTCUSDT,
        client_id: "bd-test-entry".into(),
        side: Side::Long,
        qty: d("0.002"),
        reduce_only: false,
    }
}

#[test]
fn accounting_queries_use_signed_get_and_bounded_pagination_contracts() {
    for (read, path) in [
        (Read::FeeBurn, "/fapi/v1/feeBurn"),
        (
            Read::OrderId {
                symbol: Symbol::BTCUSDT,
                order_id: 1,
            },
            "/fapi/v1/order",
        ),
        (
            Read::Trades {
                symbol: Symbol::BTCUSDT,
                order_id: 1,
                from_id: Some(1001),
            },
            "/fapi/v1/userTrades",
        ),
        (
            Read::Funding {
                symbol: Symbol::BTCUSDT,
                start: 100,
                end: 200,
                page: 2,
            },
            "/fapi/v1/income",
        ),
    ] {
        let spec = read.spec().unwrap();
        assert_eq!(spec.method, Method::GET);
        assert_eq!(spec.path, path);
        if path.ends_with("userTrades") {
            assert_eq!(spec.params["fromId"], "1001");
            assert_eq!(spec.params["limit"], "1000");
            assert!(!spec.params.contains_key("startTime"));
        }
        if path.ends_with("income") {
            assert_eq!(spec.params["incomeType"], "FUNDING_FEE");
            assert_eq!(spec.params["page"], "2");
        }
    }
    for read in [
        Read::OrderId {
            symbol: Symbol::BTCUSDT,
            order_id: 0,
        },
        Read::Trades {
            symbol: Symbol::BTCUSDT,
            order_id: 1,
            from_id: Some(0),
        },
        Read::Funding {
            symbol: Symbol::BTCUSDT,
            start: 1,
            end: 2,
            page: 21,
        },
        Read::Funding {
            symbol: Symbol::BTCUSDT,
            start: 2,
            end: 1,
            page: 1,
        },
    ] {
        assert!(read.spec().is_err());
    }
}
fn entry_response() -> Value {
    json!({"symbol":"BTCUSDT","clientOrderId":"bd-test-entry","orderId":17,"status":"FILLED","side":"BUY","positionSide":"BOTH","type":"MARKET","origQty":"0.002","executedQty":"0.002","cumQuote":"150.00","reduceOnly":false})
}
fn credentials() -> crate::vault::Credentials {
    crate::vault::Credentials::new(
        "dummy-public-key-test".into(),
        "dummy-secret-for-unit-test".into(),
    )
    .unwrap()
}

#[test]
fn leveraged_sizing_separates_margin_notional_risk_and_fees() {
    let ctx = context();
    let b = budget();
    let p = plan(&ctx, &b, Side::Long, d("98"), d("104")).unwrap();
    assert!(p.notional > b.margin);
    assert!(p.required_balance <= b.margin);
    assert!(p.budgeted_loss <= b.risk);
    assert_eq!(p.initial_margin, p.notional / D::from(10));
    assert!(p.fee_reserve > D::ZERO);
    assert!(p.liquidation_estimate < p.stop && p.stop < ctx.mark);
    let mut unleveraged = b.clone();
    unleveraged.leverage = 1;
    let q = plan(&ctx, &unleveraged, Side::Long, d("98"), d("104")).unwrap();
    assert!(q.notional < b.margin && q.qty < p.qty);
    let short = plan(&ctx, &b, Side::Short, d("102"), d("96")).unwrap();
    assert!(short.liquidation_estimate > short.stop && short.stop > ctx.mark);
    assert!(short.required_balance <= b.margin && short.budgeted_loss <= b.risk);
    let mut seven = b;
    seven.leverage = 7;
    assert!(plan(&ctx, &seven, Side::Long, d("98"), d("104")).is_ok());
}
#[test]
fn liquidation_solves_wallet_equity_equals_maintenance_across_tiers() {
    let b = tiers();
    b.validate().unwrap();
    for (side, entry) in [(Side::Long, d("105")), (Side::Short, d("95"))] {
        let qty = d("10");
        let cash = d("100");
        let lp = b.liquidation(qty, entry, cash, side).unwrap();
        let pnl = match side {
            Side::Long => qty * (lp - entry),
            Side::Short => qty * (entry - lp),
        };
        let mm = b.maintenance(qty * lp).unwrap();
        assert!((cash + pnl - mm).abs() < d("0.000000000001"));
        assert_ne!(b.tier(qty * entry).unwrap(), b.tier(qty * lp).unwrap());
    }
    assert_eq!(b.maintenance(d("1000")).unwrap(), d("4"));
    assert_eq!(b.maintenance(d("10000")).unwrap(), d("94"));
}
#[test]
fn margin_guards_reject_stale_bad_stop_leverage_tier_and_missing_funds() {
    let ctx = context();
    let b = budget();
    for case in [
        "stale",
        "future",
        "funds",
        "tick",
        "stop",
        "leverage",
        "liquidation",
        "fee",
        "funding",
    ] {
        let mut c = ctx.clone();
        let mut b = b.clone();
        let mut stop = d("98");
        match case {
            "stale" => c.now += 5001,
            "future" => c.quote.observed_at_ms += 1,
            "funds" => c.available = d("1"),
            "tick" => stop = d("98.001"),
            "stop" => stop = d("101"),
            "leverage" => b.leverage = 21,
            "liquidation" => {
                stop = d("80");
                b.leverage = 20;
            }
            "fee" => c.taker_rate = d("0.02"),
            "funding" => b.funding_reserve = b.risk,
            _ => unreachable!(),
        }
        assert!(plan(&c, &b, Side::Long, stop, d("104")).is_err(), "{case}");
    }
    let mut high = budget();
    high.margin = d("1000");
    high.risk = d("900");
    high.max_notional = d("5000");
    high.leverage = 20;
    assert!(plan(&ctx, &high, Side::Long, d("98"), d("104")).is_err());
}
#[test]
fn short_liquidation_does_not_assume_the_balance_reserve_is_extra_collateral() {
    let ctx = context();
    let b = budget();
    let p = plan(&ctx, &b, Side::Short, d("102"), d("96")).unwrap();
    let collateral =
        p.qty * p.worst_entry / D::from(p.leverage) - p.fee_reserve - p.funding_reserve;
    let equity = collateral + p.qty * (p.worst_entry - p.liquidation_estimate);
    let maintenance = ctx
        .brackets
        .maintenance(p.qty * p.liquidation_estimate)
        .unwrap();
    assert!((equity - maintenance).abs() < d("0.000000000001"));
    assert!(p.initial_margin > p.qty * p.worst_entry / D::from(p.leverage));
}

#[test]
fn bracket_parser_requires_exact_continuity_and_never_guesses_custom_coefficient() {
    let mut value = json!({"symbol":"BTCUSDT","brackets":[
        {"bracket":1,"initialLeverage":20,"notionalFloor":0,"notionalCap":1000,"maintMarginRatio":0.004,"cum":0},
        {"bracket":2,"initialLeverage":10,"notionalFloor":1000,"notionalCap":10000,"maintMarginRatio":0.01,"cum":6},
        {"bracket":3,"initialLeverage":5,"notionalFloor":10000,"notionalCap":100000,"maintMarginRatio":0.025,"cum":156}
    ]});
    assert_eq!(Brackets::parse(&value, "BTCUSDT").unwrap(), tiers());
    assert_eq!(
        Brackets::parse(&json!([value.clone()]), "BTCUSDT").unwrap(),
        tiers()
    );
    value["brackets"][0]["initialLeverage"] = json!(150);
    assert_eq!(
        Brackets::parse(&value, "BTCUSDT").unwrap().tiers[0].max_leverage,
        150
    );
    assert!(
        Command::Leverage {
            symbol: Symbol::BTCUSDT,
            leverage: 126
        }
        .spec()
        .is_err()
    );
    value["notionalCoef"] = json!(1.5);
    assert!(Brackets::parse(&value, "BTCUSDT").is_err());
    value.as_object_mut().unwrap().remove("notionalCoef");
    value["brackets"][1]["cum"] = json!(5);
    assert!(Brackets::parse(&value, "BTCUSDT").is_err());
    assert!(decimal(&json!("NaN")).is_err());
    assert!(decimal(&json!("-1")).is_err());
    assert_eq!(
        decimal(&serde_json::from_str::<Value>("0.000000000001").unwrap()).unwrap(),
        d("0.000000000001")
    );
}
#[test]
fn real_order_wire_contract_uses_algo_close_all_and_reduce_only_exit() {
    let client = crate::market::client().build().unwrap();
    let c = credentials();
    let command = entry();
    let s = command.spec().unwrap();
    let request = s.signed(&client, &c, 100000).unwrap();
    assert_eq!(request.method(), Method::POST);
    assert_eq!(request.url().path(), "/fapi/v1/order");
    assert!(request.url().query().is_none());
    assert!(request.headers()["x-mbx-apikey"].is_sensitive());
    let body = std::str::from_utf8(request.body().unwrap().as_bytes().unwrap()).unwrap();
    assert!(
        body.contains("quantity=0.002")
            && body.contains("reduceOnly=false")
            && body.contains("signature=")
    );
    let exit = Command::Market {
        symbol: Symbol::BTCUSDT,
        client_id: "bd-test-exit".into(),
        qty: d("0.002"),
        reduce_only: true,
        side: Side::Short,
    };
    assert_eq!(exit.spec().unwrap().params["reduceOnly"], "true");
    for kind in [ProtectionKind::Stop, ProtectionKind::Target] {
        let protection = Command::Protect {
            symbol: Symbol::BTCUSDT,
            client_id: "bd-stop".into(),
            position_side: Side::Long,
            kind,
            trigger: d("70000"),
        };
        let s = protection.spec().unwrap();
        assert_eq!(s.path, "/fapi/v1/algoOrder");
        assert_eq!(s.params["closePosition"], "true");
        assert_eq!(s.params["workingType"], "MARK_PRICE");
        assert_eq!(s.params["side"], "SELL");
        assert!(!s.params.contains_key("quantity") && !s.params.contains_key("reduceOnly"));
        s.guard(&s.signed(&client, &c, 100000).unwrap()).unwrap();
    }
    let query = Read::Algo {
        client_id: "bd-stop".into(),
    }
    .spec()
    .unwrap();
    assert!(!query.params.contains_key("symbol"));
}
#[test]
fn request_guard_rejects_route_header_method_body_and_duplicate_tampering() {
    let client = crate::market::client().build().unwrap();
    let spec = entry().spec().unwrap();
    for case in ["host", "path", "method", "cookie", "body", "query"] {
        let mut r = spec.signed(&client, &credentials(), 100000).unwrap();
        match case {
            "host" => r.url_mut().set_host(Some("example.com")).unwrap(),
            "path" => r.url_mut().set_path("/fapi/v1/transfer"),
            "method" => *r.method_mut() = Method::GET,
            "cookie" => {
                r.headers_mut()
                    .insert("cookie", reqwest::header::HeaderValue::from_static("x=y"));
            }
            "body" => {
                let mut bytes = r.body().unwrap().as_bytes().unwrap().to_vec();
                bytes.extend_from_slice(b"&quantity=999");
                *r.body_mut() = Some(bytes.into());
            }
            "query" => r.url_mut().set_query(Some("leverage=125")),
            _ => unreachable!(),
        }
        assert!(spec.guard(&r).is_err(), "{case}");
    }
    let bad = Command::CancelAlgo {
        client_id: "bad&signature=x".into(),
    };
    assert!(bad.spec().is_err());
}
#[test]
fn acknowledgements_never_invent_fills_and_cancel_fill_races_stay_visible() {
    let mut v = entry_response();
    let cmd = entry();
    let Receipt::Order(filled) = cmd.receipt(&v, false).unwrap() else {
        panic!()
    };
    assert_eq!(filled.executed_qty, d("0.002"));
    v["status"] = json!("NEW");
    v["executedQty"] = json!("0");
    v["cumQuote"] = json!("0");
    let Receipt::Order(new) = cmd.receipt(&v, false).unwrap() else {
        panic!()
    };
    assert!(!new.status.terminal() && new.executed_qty.is_zero());
    v["status"] = json!("FILLED");
    assert!(cmd.receipt(&v, false).is_err());
    let cancel = Command::CancelOrder {
        symbol: Symbol::BTCUSDT,
        client_id: "bd-test-entry".into(),
    };
    let Receipt::Order(race) = cancel.receipt(&entry_response(), true).unwrap() else {
        panic!()
    };
    assert_eq!(race.status, OrderStatus::Filled);
    let cancel = Command::CancelAlgo {
        client_id: "bd-stop".into(),
    };
    assert_eq!(
        cancel
            .receipt(
                &json!({"algoId":42,"clientAlgoId":"bd-stop","code":"200"}),
                false
            )
            .unwrap(),
        Receipt::CancelAccepted
    );
}
#[test]
fn timeout_server_error_and_rate_limit_do_not_become_definite_rejection() {
    for s in [
        StatusCode::REQUEST_TIMEOUT,
        StatusCode::INTERNAL_SERVER_ERROR,
        StatusCode::SERVICE_UNAVAILABLE,
    ] {
        assert_eq!(
            transport::classify(s, None, true),
            Err(transport::Error::Unknown)
        );
    }
    assert_eq!(
        transport::classify(StatusCode::OK, Some(-1007), true),
        Err(transport::Error::Unknown)
    );
    assert_eq!(
        transport::classify(StatusCode::TOO_MANY_REQUESTS, Some(-1003), true),
        Err(transport::Error::RateLimit)
    );
    assert_eq!(
        transport::classify(StatusCode::BAD_REQUEST, Some(-2019), true),
        Err(transport::Error::Rejected(-2019))
    );
    assert_eq!(
        transport::classify(StatusCode::BAD_REQUEST, Some(-2013), false),
        Err(transport::Error::NotFound)
    );
}

struct Fake {
    writes: usize,
    reads: usize,
    live: bool,
    write: transport::Result<Value>,
    read: transport::Result<Value>,
}
impl Exchange for Fake {
    fn live_enabled(&self) -> bool {
        self.live
    }
    async fn write_once(&mut self, _: &Command) -> transport::Result<Value> {
        self.writes += 1;
        self.write.clone()
    }
    async fn read(&mut self, _: Read) -> transport::Result<Value> {
        self.reads += 1;
        self.read.clone()
    }
}
fn fake() -> Fake {
    Fake {
        writes: 0,
        reads: 0,
        live: true,
        write: Ok(entry_response()),
        read: Ok(entry_response()),
    }
}
#[tokio::test]
async fn durable_outbox_prevents_duplicates_and_reconciles_unknown_without_resend() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("orders.json");
    let mut outbox = Outbox::open(&path).unwrap();
    let mut exchange = fake();
    exchange.write = Err(transport::Error::Unknown);
    assert!(matches!(
        dispatch(&mut exchange, &mut outbox, "session-entry", entry())
            .await
            .unwrap(),
        Dispatch::Uncertain(_)
    ));
    assert_eq!(exchange.writes, 1);
    assert!(
        dispatch(
            &mut exchange,
            &mut outbox,
            "different-dispatch-same-order",
            entry()
        )
        .await
        .is_err()
    );
    assert_eq!(exchange.writes, 1);
    assert_eq!(
        outbox.record("session-entry").unwrap().status,
        Status::Uncertain
    );
    assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
    drop(outbox);
    let mut outbox = Outbox::open(&path).unwrap();
    assert!(matches!(
        dispatch(&mut exchange, &mut outbox, "session-entry", entry())
            .await
            .unwrap(),
        Dispatch::Existing(_)
    ));
    exchange.read = Err(transport::Error::NotFound);
    assert!(matches!(
        reconcile(&mut exchange, &mut outbox, "session-entry")
            .await
            .unwrap(),
        Dispatch::Uncertain(transport::Error::NotFound)
    ));
    exchange.read = Ok(entry_response());
    let Dispatch::Recorded(record) = reconcile(&mut exchange, &mut outbox, "session-entry")
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(record.reconciled);
    assert_eq!(record.status, Status::Acknowledged);
    assert_eq!(exchange.writes, 1);
    assert_eq!(exchange.reads, 2);
}
#[tokio::test]
async fn reconciliation_cannot_erase_fill_or_regress_a_terminal_cancel() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("orders.json");
    let mut box_ = Outbox::open(&path).unwrap();
    let mut exchange = fake();
    let mut partial = entry_response();
    partial["status"] = json!("PARTIALLY_FILLED");
    partial["executedQty"] = json!("0.001");
    partial["cumQuote"] = json!("70");
    exchange.write = Ok(partial.clone());
    dispatch(&mut exchange, &mut box_, "entry", entry())
        .await
        .unwrap();
    let persisted = fs::read(&path).unwrap();
    let mut stale = partial.clone();
    stale["status"] = json!("NEW");
    stale["executedQty"] = json!("0");
    stale["cumQuote"] = json!("0");
    exchange.read = Ok(stale);
    assert!(reconcile(&mut exchange, &mut box_, "entry").await.is_err());
    assert_eq!(fs::read(&path).unwrap(), persisted);
    let mut canceled = partial.clone();
    canceled["status"] = json!("CANCELED");
    exchange.write = Ok(canceled.clone());
    dispatch(
        &mut exchange,
        &mut box_,
        "cancel",
        Command::CancelOrder {
            symbol: Symbol::BTCUSDT,
            client_id: "bd-test-entry".into(),
        },
    )
    .await
    .unwrap();
    exchange.read = Ok(partial);
    assert!(reconcile(&mut exchange, &mut box_, "entry").await.is_err());
    exchange.read = Ok(canceled);
    assert!(matches!(
        reconcile(&mut exchange, &mut box_, "entry").await.unwrap(),
        Dispatch::Recorded(_)
    ));
    drop(box_);
    let mut box_ = Outbox::open(&path).unwrap();
    exchange.read = Ok(entry_response());
    assert!(reconcile(&mut exchange, &mut box_, "entry").await.is_err());
    assert_eq!(exchange.writes, 2);
}

#[tokio::test]
async fn crash_before_send_lock_corruption_and_malformed_ack_are_blocked() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("orders.json");
    let mut outbox = Outbox::open(&path).unwrap();
    assert!(Outbox::open(&path).is_err());
    outbox.reserve("crash", entry()).unwrap();
    drop(outbox);
    let mut outbox = Outbox::open(&path).unwrap();
    let mut exchange = fake();
    dispatch(&mut exchange, &mut outbox, "crash", entry())
        .await
        .unwrap();
    assert_eq!(exchange.writes, 0);
    let bad = Command::Leverage {
        symbol: Symbol::BTCUSDT,
        leverage: 3,
    };
    assert!(
        dispatch(&mut exchange, &mut outbox, "crash", bad)
            .await
            .is_err()
    );
    assert_eq!(exchange.writes, 0);
    exchange.write = Ok(json!({"status":"FILLED"}));
    let mut fresh_entry = entry();
    if let Command::Market { client_id, .. } = &mut fresh_entry {
        *client_id = "bd-malformed-ack".into();
    }
    assert!(matches!(
        dispatch(&mut exchange, &mut outbox, "bad-ack", fresh_entry)
            .await
            .unwrap(),
        Dispatch::Uncertain(_)
    ));
    drop(outbox);
    let mut store: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    store["hash"] = json!("broken");
    fs::write(&path, serde_json::to_vec(&store).unwrap()).unwrap();
    assert!(Outbox::open(&path).is_err());
}
#[tokio::test]
async fn readonly_connector_and_driver_refuse_mutations_before_network() {
    let mut connector = Connector::new(credentials(), Access::ReadOnly).unwrap();
    assert_eq!(
        connector.write_once(&entry()).await,
        Err(transport::Error::Disabled)
    );
    let dir = tempfile::tempdir().unwrap();
    let mut outbox = Outbox::open(&dir.path().join("orders.json")).unwrap();
    assert!(
        dispatch(&mut connector, &mut outbox, "denied", entry())
            .await
            .is_err()
    );
    assert!(outbox.records().is_empty());
}

#[tokio::test]
async fn order_batch_rejects_unrelated_invalid_or_excessive_queries_before_network() {
    let mut connector = Connector::new(credentials(), Access::ReadOnly).unwrap();
    let mut fixture = fake();
    let queries = [
        vec![Read::Account],
        vec![Read::Algo {
            client_id: "invalid id".into(),
        }],
        vec![
            Read::Order {
                symbol: Symbol::BTCUSDT,
                client_id: "valid-id".into()
            };
            17
        ],
    ];
    for reads in queries {
        for results in [
            connector.read_orders(&reads).await.results,
            fixture.read_orders(&reads).await.results,
        ] {
            assert!(!results.is_empty());
            assert!(results.iter().all(|r| *r == Err(transport::Error::Schema)));
        }
    }
    assert!(connector.read_orders(&[]).await.results.is_empty());
    assert_eq!(fixture.reads, 0);
    assert_eq!(fixture.writes, 0);
}

#[tokio::test]
async fn incomplete_mutation_receipt_never_invents_fill_and_notfound_never_resends() {
    for found in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orders.json");
        let mut exchange = fake();
        let mut response = entry_response();
        response.as_object_mut().unwrap().remove("cumQuote");
        exchange.write = Ok(response);
        if !found {
            exchange.read = Err(transport::Error::NotFound);
        }
        let mut outbox = Outbox::open(&path).unwrap();
        let result = dispatch(&mut exchange, &mut outbox, "entry", entry())
            .await
            .unwrap();
        assert_eq!(exchange.writes, 1);
        assert_eq!(exchange.reads, 1);
        if found {
            assert!(matches!(result,Dispatch::Recorded(ref r) if r.reconciled));
        } else {
            assert!(matches!(
                result,
                Dispatch::Uncertain(transport::Error::NotFound)
            ));
            assert!(outbox.record("entry").unwrap().receipt.is_none());
        }
        drop(outbox);
        let mut outbox = Outbox::open(&path).unwrap();
        dispatch(&mut exchange, &mut outbox, "entry", entry())
            .await
            .unwrap();
        assert_eq!(exchange.writes, 1);
    }
}
