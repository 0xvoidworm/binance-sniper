use super::{
    flow::{Intent, Next, Observation},
    margin,
    outbox::{self, Outbox},
    protocol::*,
    transport::{self, Exchange},
};
use binance_engine::{Quote, Side};
use rust_decimal::Decimal as D;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn d(s: &str) -> D {
    s.parse().unwrap()
}
fn intent() -> Intent {
    let context = super::tests::context();
    let budget = margin::Budget {
        margin: d("50"),
        risk: d("10"),
        max_notional: d("1000"),
        leverage: 10,
        slippage_bps: D::ONE,
        liquidation_buffer_bps: d("10"),
        funding_reserve: D::ZERO,
    };
    Intent {
        prefix: "bd-session-test".into(),
        symbol: Symbol::BTCUSDT,
        plan: margin::plan(&context, &budget, Side::Long, d("98"), d("104")).unwrap(),
        entry_deadline_ms: 105000,
    }
}
struct Broker {
    regular: BTreeMap<String, Value>,
    algos: BTreeMap<String, Value>,
    commands: Vec<Command>,
    position: D,
    isolated: bool,
    leverage: u32,
    partial: bool,
    reject_stop: bool,
    lose_entry_response: bool,
    lose_stop_response: bool,
    fill_during_cancel: bool,
}
impl Default for Broker {
    fn default() -> Self {
        Self {
            regular: BTreeMap::new(),
            algos: BTreeMap::new(),
            commands: vec![],
            position: D::ZERO,
            isolated: false,
            leverage: 1,
            partial: false,
            reject_stop: false,
            lose_entry_response: false,
            lose_stop_response: false,
            fill_during_cancel: false,
        }
    }
}
impl Broker {
    fn observation(&self) -> Observation {
        Observation {
            now_ms: 100000,
            as_of_ms: 100000,
            quote: Quote {
                bid: d("100"),
                ask: d("100.01"),
                observed_at_ms: 100000,
            },
            mark: d("100"),
            position_qty: self.position,
            isolated: self.isolated,
            leverage: self.leverage,
            compatible_account: true,
            unowned_orders: false,
        }
    }
}
impl Exchange for Broker {
    fn live_enabled(&self) -> bool {
        true
    }
    async fn read(&mut self, read: Read) -> transport::Result<Value> {
        match read {
            Read::Order { client_id, .. } => self
                .regular
                .get(&client_id)
                .cloned()
                .ok_or(transport::Error::NotFound),
            Read::Algo { client_id } => self
                .algos
                .get(&client_id)
                .cloned()
                .ok_or(transport::Error::NotFound),
            Read::Symbols => Ok(
                json!([{"symbol":"BTCUSDT","leverage":self.leverage,"marginType":if self.isolated{"ISOLATED"}else{"CROSSED"}}]),
            ),
            _ => panic!("unexpected read in local broker"),
        }
    }
    async fn write_once(&mut self, command: &Command) -> transport::Result<Value> {
        self.commands.push(command.clone());
        let order_id = self.commands.len() as u64;
        match command {
            Command::Isolated { .. } => {
                self.isolated = true;
                Ok(json!({"code":200}))
            }
            Command::Leverage { leverage, .. } => {
                self.leverage = *leverage;
                Ok(json!({"symbol":"BTCUSDT","leverage":leverage,"maxNotionalValue":"1000000"}))
            }
            Command::Market {
                symbol,
                client_id,
                side,
                qty,
                reduce_only,
            } => {
                let filled = if self.partial && !reduce_only {
                    (*qty / D::from(2)).round_dp(3)
                } else {
                    *qty
                };
                if *reduce_only {
                    assert_eq!(*side, Side::Short);
                    assert!(filled <= self.position);
                    self.position -= filled;
                } else {
                    self.position += filled;
                }
                let value = json!({"symbol":symbol.name(),"clientOrderId":client_id,"orderId":order_id,
                    "status":if filled==*qty{"FILLED"}else{"PARTIALLY_FILLED"},"executedQty":filled.to_string(),
                    "cumQuote":(filled*d("100.01")).to_string(),"origQty":qty.to_string(),"side":side_name(*side),"positionSide":"BOTH","type":"MARKET","reduceOnly":reduce_only});
                self.regular.insert(client_id.clone(), value.clone());
                if self.lose_entry_response && !reduce_only {
                    Err(transport::Error::Unknown)
                } else {
                    Ok(value)
                }
            }
            Command::Protect {
                symbol,
                client_id,
                position_side,
                kind,
                trigger,
            } => {
                if self.reject_stop && *kind == ProtectionKind::Stop {
                    return Err(transport::Error::Rejected(-2021));
                }
                let value = json!({"symbol":symbol.name(),"clientAlgoId":client_id,"algoId":order_id,"algoType":"CONDITIONAL",
                    "orderType":if *kind==ProtectionKind::Stop{"STOP_MARKET"}else{"TAKE_PROFIT_MARKET"},"algoStatus":"NEW","actualOrderId":"",
                    "side":side_name(opposite(*position_side)),"positionSide":"BOTH","workingType":"MARK_PRICE","triggerPrice":trigger.to_string(),"closePosition":true,"priceProtect":false});
                self.algos.insert(client_id.clone(), value.clone());
                if self.lose_stop_response && *kind == ProtectionKind::Stop {
                    Err(transport::Error::Unknown)
                } else {
                    Ok(value)
                }
            }
            Command::CancelOrder { client_id, .. } => {
                let value = self.regular.get_mut(client_id).unwrap();
                if self.fill_during_cancel && value["status"] != "FILLED" {
                    let original = margin::decimal(&value["origQty"]).unwrap();
                    let already = margin::decimal(&value["executedQty"]).unwrap();
                    self.position += original - already;
                    value["status"] = json!("FILLED");
                    value["executedQty"] = json!(original.to_string());
                    value["cumQuote"] = json!((original * d("100.01")).to_string());
                } else if value["status"] != "FILLED" {
                    value["status"] = json!("CANCELED");
                }
                Ok(value.clone())
            }
            Command::CancelAlgo { client_id } => {
                let value = self.algos.get_mut(client_id).unwrap();
                value["algoStatus"] = json!("CANCELED");
                Ok(json!({"code":"200","clientAlgoId":client_id,"algoId":value["algoId"]}))
            }
        }
    }
}

#[tokio::test]
async fn automatic_entry_protection_and_close_need_no_per_order_click() {
    for mode in [
        "normal",
        "partial",
        "stop-reject",
        "lost-response",
        "lost-stop",
        "partial-stop-reject",
        "cancel-race",
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("outbox.json");
        let mut box_ = Outbox::open(&path).unwrap();
        let mut broker = Broker::default();
        let intent = intent();
        broker.partial = matches!(mode, "partial" | "partial-stop-reject" | "cancel-race");
        broker.reject_stop = matches!(mode, "stop-reject" | "partial-stop-reject");
        broker.lose_entry_response = mode == "lost-response";
        broker.lose_stop_response = mode == "lost-stop";
        broker.fill_during_cancel = mode == "cancel-race";
        let mut stop = false;
        let mut complete = false;
        let mut reopened = false;
        for _ in 0..40 {
            match intent.next(&box_, &broker.observation(), stop).unwrap() {
                Next::Send { key, command } => {
                    outbox::dispatch(&mut broker, &mut box_, &key, command)
                        .await
                        .unwrap();
                    if ((mode == "lost-response" && key.ends_with("-entry"))
                        || (mode == "lost-stop" && key.ends_with("-sl")))
                        && !reopened
                    {
                        drop(box_);
                        box_ = Outbox::open(&path).unwrap();
                        reopened = true;
                    }
                }
                Next::Reconcile { key } => {
                    outbox::reconcile(&mut broker, &mut box_, &key)
                        .await
                        .unwrap();
                }
                Next::Watch { keys } => {
                    assert_eq!(keys.len(), 3);
                    assert!(broker.position > D::ZERO);
                    assert_eq!(broker.algos["bd-session-test-sl"]["algoStatus"], "NEW");
                    assert_eq!(broker.algos["bd-session-test-tp"]["algoStatus"], "NEW");
                    stop = true;
                }
                Next::NeedsAccounting {
                    entry_order_id,
                    exit_order_id,
                    child_order_ids,
                } => {
                    assert!(
                        entry_order_id > 0 && exit_order_id.is_some() && child_order_ids.is_empty()
                    );
                    assert!(broker.position.is_zero());
                    complete = true;
                    break;
                }
                other => panic!("unexpected local transition: {other:?}"),
            }
        }
        assert!(complete, "{mode}");
        assert_eq!(
            broker
                .commands
                .iter()
                .filter(|c| matches!(
                    c,
                    Command::Market {
                        reduce_only: false,
                        ..
                    }
                ))
                .count(),
            1
        );
        assert_eq!(
            broker
                .commands
                .iter()
                .filter(|c| matches!(
                    c,
                    Command::Market {
                        reduce_only: true,
                        ..
                    }
                ))
                .count(),
            1
        );
        assert_eq!(
            broker
                .commands
                .iter()
                .filter(|c| matches!(
                    c,
                    Command::Protect {
                        kind: ProtectionKind::Stop,
                        ..
                    }
                ))
                .count(),
            1
        );
        if matches!(mode, "partial" | "partial-stop-reject" | "cancel-race") {
            let stop = broker
                .commands
                .iter()
                .position(|c| {
                    matches!(
                        c,
                        Command::Protect {
                            kind: ProtectionKind::Stop,
                            ..
                        }
                    )
                })
                .unwrap();
            let cancel = broker
                .commands
                .iter()
                .position(|c| matches!(c, Command::CancelOrder { .. }))
                .unwrap();
            assert!(stop < cancel);
        }
        if matches!(mode, "stop-reject" | "partial-stop-reject") {
            assert!(!broker.commands.iter().any(|c| matches!(
                c,
                Command::Protect {
                    kind: ProtectionKind::Target,
                    ..
                }
            )));
        }
    }
}

#[tokio::test]
async fn exchange_stop_trigger_cleans_sibling_and_requires_child_fill_accounting() {
    let dir = tempfile::tempdir().unwrap();
    let mut box_ = Outbox::open(&dir.path().join("outbox.json")).unwrap();
    let intent = intent();
    let mut broker = Broker::default();
    let mut complete = false;
    for _ in 0..40 {
        match intent.next(&box_, &broker.observation(), false).unwrap() {
            Next::Send { key, command } => {
                outbox::dispatch(&mut broker, &mut box_, &key, command)
                    .await
                    .unwrap();
            }
            Next::Reconcile { key } => {
                outbox::reconcile(&mut broker, &mut box_, &key)
                    .await
                    .unwrap();
            }
            Next::Watch { keys } => {
                let sl = broker.algos.get_mut("bd-session-test-sl").unwrap();
                sl["algoStatus"] = json!("FINISHED");
                sl["actualOrderId"] = json!("777");
                broker.position = D::ZERO;
                for key in keys {
                    outbox::reconcile(&mut broker, &mut box_, &key)
                        .await
                        .unwrap();
                }
            }
            Next::NeedsAccounting {
                exit_order_id,
                child_order_ids,
                ..
            } => {
                assert!(exit_order_id.is_none());
                assert_eq!(child_order_ids, vec![777]);
                complete = true;
                break;
            }
            other => panic!("unexpected local transition: {other:?}"),
        }
    }
    assert!(complete);
    assert!(!broker.commands.iter().any(|c| matches!(
        c,
        Command::Market {
            reduce_only: true,
            ..
        }
    )));
    assert_eq!(broker.algos["bd-session-test-tp"]["algoStatus"], "CANCELED");
}

#[test]
fn controller_blocks_stale_foreign_and_changed_price_before_entry() {
    let dir = tempfile::tempdir().unwrap();
    let box_ = Outbox::open(&dir.path().join("outbox.json")).unwrap();
    let i = intent();
    let broker = Broker {
        isolated: true,
        leverage: 10,
        ..Default::default()
    };
    for case in ["stale", "foreign", "position", "price", "deadline", "mode"] {
        let mut o = broker.observation();
        match case {
            "stale" => o.as_of_ms -= 5001,
            "foreign" => o.unowned_orders = true,
            "position" => o.position_qty = D::ONE,
            "price" => {
                o.quote.bid = d("101");
                o.quote.ask = d("101.01");
                o.mark = d("101");
            }
            "deadline" => {
                o.now_ms = i.entry_deadline_ms;
                o.as_of_ms = o.now_ms;
                o.quote.observed_at_ms = o.now_ms;
            }
            "mode" => o.compatible_account = false,
            _ => unreachable!(),
        }
        let outcome = i.next(&box_, &o, false);
        assert!(
            outcome.is_err() || matches!(outcome.unwrap(), Next::Halt(_)),
            "{case}"
        );
    }
    assert!(box_.records().is_empty());
}
