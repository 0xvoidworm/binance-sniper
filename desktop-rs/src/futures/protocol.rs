//! Typed USD-M production requests. No arbitrary routes or form fields.
use super::margin::decimal;
use crate::vault::Credentials;
use anyhow::{Context, Result, ensure};
use binance_engine::Side;
use hmac::{Hmac, KeyInit, Mac};
use reqwest::{
    Client, Method, Request, Url,
    header::{CONTENT_TYPE, HeaderName, HeaderValue},
};
use rust_decimal::Decimal as D;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use std::{collections::BTreeMap, fmt::Write};
use zeroize::Zeroizing;

pub const HOST: &str = "fapi.binance.com";
const BASE: &str = "https://fapi.binance.com";
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Symbol {
    BTCUSDT,
    ETHUSDT,
}
impl Symbol {
    pub fn name(self) -> &'static str {
        match self {
            Self::BTCUSDT => "BTCUSDT",
            Self::ETHUSDT => "ETHUSDT",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtectionKind {
    Stop,
    Target,
}
impl ProtectionKind {
    fn order_type(self) -> &'static str {
        match self {
            Self::Stop => "STOP_MARKET",
            Self::Target => "TAKE_PROFIT_MARKET",
        }
    }
}
pub fn side_name(side: Side) -> &'static str {
    match side {
        Side::Long => "BUY",
        Side::Short => "SELL",
    }
}
pub fn opposite(side: Side) -> Side {
    match side {
        Side::Long => Side::Short,
        Side::Short => Side::Long,
    }
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 36
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
}
fn quantity(qty: D) -> Result<String> {
    ensure!(
        qty > D::ZERO && qty <= D::from(1_000_000_000_000_u64) && qty.scale() <= 12,
        "Jumlah/harga order invalid"
    );
    Ok(qty.normalize().to_string())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", deny_unknown_fields)]
pub enum Command {
    Market {
        symbol: Symbol,
        client_id: String,
        side: Side,
        qty: D,
        reduce_only: bool,
    },
    Protect {
        symbol: Symbol,
        client_id: String,
        position_side: Side,
        kind: ProtectionKind,
        trigger: D,
    },
    CancelOrder {
        symbol: Symbol,
        client_id: String,
    },
    CancelAlgo {
        client_id: String,
    },
    Leverage {
        symbol: Symbol,
        leverage: u32,
    },
    Isolated {
        symbol: Symbol,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Read {
    Account,
    Positions,
    AccountConfig,
    Symbols,
    FeeBurn,
    OrderId {
        symbol: Symbol,
        order_id: u64,
    },
    Trades {
        symbol: Symbol,
        order_id: u64,
        from_id: Option<u64>,
    },
    Funding {
        symbol: Symbol,
        start: u64,
        end: u64,
        page: u32,
    },
    Brackets(Symbol),
    Commission(Symbol),
    OpenOrders(Symbol),
    OpenAlgos(Symbol),
    Order {
        symbol: Symbol,
        client_id: String,
    },
    Algo {
        client_id: String,
    },
}
pub struct Spec {
    pub(crate) method: Method,
    pub(crate) path: &'static str,
    pub(crate) params: BTreeMap<String, String>,
}
fn spec(method: Method, path: &'static str, params: Vec<(&str, String)>) -> Spec {
    Spec {
        method,
        path,
        params: params.into_iter().map(|(k, v)| (k.into(), v)).collect(),
    }
}
impl Read {
    pub fn spec(&self) -> Result<Spec> {
        let (path, params) = match self {
            Self::Account => ("/fapi/v3/account", vec![]),
            Self::Positions => ("/fapi/v3/positionRisk", vec![]),
            Self::AccountConfig => ("/fapi/v1/accountConfig", vec![]),
            Self::Symbols => ("/fapi/v1/symbolConfig", vec![]),
            Self::FeeBurn => ("/fapi/v1/feeBurn", vec![]),
            Self::OrderId { symbol, order_id } => {
                ensure!(*order_id > 0, "ID order invalid");
                (
                    "/fapi/v1/order",
                    vec![
                        ("symbol", symbol.name().into()),
                        ("orderId", order_id.to_string()),
                    ],
                )
            }
            Self::Trades {
                symbol,
                order_id,
                from_id,
            } => {
                ensure!(
                    *order_id > 0 && from_id.is_none_or(|n| n > 0),
                    "ID trade invalid"
                );
                let mut params = vec![
                    ("symbol", symbol.name().into()),
                    ("orderId", order_id.to_string()),
                    ("limit", "1000".into()),
                ];
                if let Some(id) = from_id {
                    params.push(("fromId", id.to_string()));
                }
                ("/fapi/v1/userTrades", params)
            }
            Self::Funding {
                symbol,
                start,
                end,
                page,
            } => {
                ensure!(
                    *start > 0
                        && end >= start
                        && end - start <= 7 * 86_400_000
                        && (1..=20).contains(page),
                    "Rentang funding invalid"
                );
                (
                    "/fapi/v1/income",
                    vec![
                        ("symbol", symbol.name().into()),
                        ("incomeType", "FUNDING_FEE".into()),
                        ("startTime", start.to_string()),
                        ("endTime", end.to_string()),
                        ("page", page.to_string()),
                        ("limit", "1000".into()),
                    ],
                )
            }
            Self::Brackets(s) => (
                "/fapi/v1/leverageBracket",
                vec![("symbol", s.name().into())],
            ),
            Self::Commission(s) => ("/fapi/v1/commissionRate", vec![("symbol", s.name().into())]),
            Self::OpenOrders(s) => ("/fapi/v1/openOrders", vec![("symbol", s.name().into())]),
            Self::OpenAlgos(s) => ("/fapi/v1/openAlgoOrders", vec![("symbol", s.name().into())]),
            Self::Order { symbol, client_id } => {
                ensure!(valid_id(client_id), "Client ID invalid");
                (
                    "/fapi/v1/order",
                    vec![
                        ("symbol", symbol.name().into()),
                        ("origClientOrderId", client_id.clone()),
                    ],
                )
            }
            Self::Algo { client_id } => {
                ensure!(valid_id(client_id), "Client algo ID invalid");
                (
                    "/fapi/v1/algoOrder",
                    vec![("clientAlgoId", client_id.clone())],
                )
            }
        };
        Ok(spec(Method::GET, path, params))
    }
}
impl Command {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Market {
                reduce_only: false, ..
            } => "entry",
            Self::Market {
                reduce_only: true, ..
            } => "exit",
            Self::Protect {
                kind: ProtectionKind::Stop,
                ..
            } => "stop",
            Self::Protect {
                kind: ProtectionKind::Target,
                ..
            } => "target",
            Self::CancelOrder { .. } => "cancel_order",
            Self::CancelAlgo { .. } => "cancel_algo",
            Self::Leverage { .. } => "leverage",
            Self::Isolated { .. } => "isolated",
        }
    }
    pub fn spec(&self) -> Result<Spec> {
        Ok(match self {
            Self::Market {
                symbol,
                client_id,
                side,
                qty,
                reduce_only,
            } => {
                ensure!(valid_id(client_id), "Client ID invalid");
                spec(
                    Method::POST,
                    "/fapi/v1/order",
                    vec![
                        ("symbol", symbol.name().into()),
                        ("newClientOrderId", client_id.clone()),
                        ("side", side_name(*side).into()),
                        ("positionSide", "BOTH".into()),
                        ("type", "MARKET".into()),
                        ("quantity", quantity(*qty)?),
                        ("reduceOnly", reduce_only.to_string()),
                        ("newOrderRespType", "RESULT".into()),
                    ],
                )
            }
            Self::Protect {
                symbol,
                client_id,
                position_side,
                kind,
                trigger,
            } => {
                ensure!(valid_id(client_id), "Client algo ID invalid");
                spec(
                    Method::POST,
                    "/fapi/v1/algoOrder",
                    vec![
                        ("symbol", symbol.name().into()),
                        ("clientAlgoId", client_id.clone()),
                        ("algoType", "CONDITIONAL".into()),
                        ("side", side_name(opposite(*position_side)).into()),
                        ("positionSide", "BOTH".into()),
                        ("type", kind.order_type().into()),
                        ("triggerPrice", quantity(*trigger)?),
                        ("workingType", "MARK_PRICE".into()),
                        ("closePosition", "true".into()),
                        ("priceProtect", "false".into()),
                    ],
                )
            }
            Self::CancelOrder { symbol, client_id } => {
                ensure!(valid_id(client_id), "Client ID invalid");
                spec(
                    Method::DELETE,
                    "/fapi/v1/order",
                    vec![
                        ("symbol", symbol.name().into()),
                        ("origClientOrderId", client_id.clone()),
                    ],
                )
            }
            Self::CancelAlgo { client_id } => {
                ensure!(valid_id(client_id), "Client algo ID invalid");
                spec(
                    Method::DELETE,
                    "/fapi/v1/algoOrder",
                    vec![("clientAlgoId", client_id.clone())],
                )
            }
            Self::Leverage { symbol, leverage } => {
                ensure!((1..=125).contains(leverage), "Leverage invalid");
                spec(
                    Method::POST,
                    "/fapi/v1/leverage",
                    vec![
                        ("symbol", symbol.name().into()),
                        ("leverage", leverage.to_string()),
                    ],
                )
            }
            Self::Isolated { symbol } => spec(
                Method::POST,
                "/fapi/v1/marginType",
                vec![
                    ("symbol", symbol.name().into()),
                    ("marginType", "ISOLATED".into()),
                ],
            ),
        })
    }
    pub fn reconciliation(&self) -> Read {
        match self {
            Self::Market {
                symbol, client_id, ..
            }
            | Self::CancelOrder { symbol, client_id } => Read::Order {
                symbol: *symbol,
                client_id: client_id.clone(),
            },
            Self::Protect { client_id, .. } | Self::CancelAlgo { client_id } => Read::Algo {
                client_id: client_id.clone(),
            },
            Self::Leverage { .. } | Self::Isolated { .. } => Read::Symbols,
        }
    }
    pub fn receipt(&self, v: &Value, reconciled: bool) -> Result<Receipt> {
        match self {
            Self::Market {
                symbol,
                client_id,
                side,
                qty,
                reduce_only,
            } => {
                let order = Order::parse(v, *symbol, client_id)?;
                ensure!(
                    v["side"] == side_name(*side)
                        && v["positionSide"] == "BOTH"
                        && v["type"] == "MARKET"
                        && decimal(&v["origQty"])? == *qty
                        && v["reduceOnly"].as_bool() == Some(*reduce_only),
                    "Order echo tidak cocok"
                );
                ensure!(order.executed_qty <= *qty, "Fill melebihi jumlah order");
                ensure!(
                    match order.status {
                        OrderStatus::Filled => order.executed_qty == *qty,
                        OrderStatus::New => order.executed_qty.is_zero(),
                        OrderStatus::PartiallyFilled =>
                            order.executed_qty > D::ZERO && order.executed_qty < *qty,
                        _ => true,
                    },
                    "Status dan jumlah fill tidak konsisten"
                );
                Ok(Receipt::Order(order))
            }
            Self::Protect {
                symbol,
                client_id,
                position_side,
                kind,
                trigger,
            } => {
                let algo = Algo::parse(v, client_id)?;
                ensure!(
                    v["symbol"] == symbol.name()
                        && v["algoType"] == "CONDITIONAL"
                        && v["orderType"] == kind.order_type()
                        && v["side"] == side_name(opposite(*position_side))
                        && v["positionSide"] == "BOTH"
                        && v["closePosition"].as_bool() == Some(true)
                        && v["workingType"] == "MARK_PRICE"
                        && v["priceProtect"].as_bool() == Some(false)
                        && decimal(&v["triggerPrice"])? == *trigger,
                    "Proteksi echo tidak cocok"
                );
                Ok(Receipt::Algo(algo))
            }
            Self::CancelOrder { symbol, client_id } => {
                let order = Order::parse(v, *symbol, client_id)?;
                ensure!(
                    order.status.terminal(),
                    "Order belum terminal setelah cancel"
                );
                Ok(Receipt::Order(order))
            }
            Self::CancelAlgo { client_id } => {
                if !reconciled {
                    ensure!(
                        code(v) == Some(200)
                            && v["clientAlgoId"] == *client_id
                            && v["algoId"].as_u64().is_some_and(|n| n > 0),
                        "Ack cancel algo invalid"
                    );
                    // ACK is not proof that a concurrent trigger did not fill.
                    Ok(Receipt::CancelAccepted)
                } else {
                    let algo = Algo::parse(v, client_id)?;
                    ensure!(algo.terminal(), "Algo belum terminal setelah cancel");
                    Ok(Receipt::Algo(algo))
                }
            }
            Self::Leverage { symbol, leverage } => {
                let row = if reconciled {
                    symbol_config(v, *symbol)?
                } else {
                    v
                };
                ensure!(
                    row["symbol"] == symbol.name()
                        && row["leverage"].as_u64() == Some(u64::from(*leverage)),
                    "Leverage belum terkonfirmasi"
                );
                if !reconciled {
                    ensure!(
                        decimal(&row["maxNotionalValue"])? > D::ZERO,
                        "Batas leverage hilang"
                    );
                }
                Ok(Receipt::Configured)
            }
            Self::Isolated { symbol } => {
                if reconciled {
                    ensure!(
                        symbol_config(v, *symbol)?["marginType"] == "ISOLATED",
                        "Margin belum isolated"
                    );
                } else {
                    ensure!(v["code"].as_i64() == Some(200), "Ack margin invalid");
                }
                Ok(Receipt::Configured)
            }
        }
    }
}
pub fn code(value: &Value) -> Option<i64> {
    value["code"]
        .as_i64()
        .or_else(|| value["code"].as_str().and_then(|s| s.parse().ok()))
}
fn symbol_config(v: &Value, symbol: Symbol) -> Result<&Value> {
    let rows = v.as_array().context("Konfigurasi simbol invalid")?;
    let selected: Vec<_> = rows
        .iter()
        .filter(|r| r["symbol"] == symbol.name())
        .collect();
    ensure!(selected.len() == 1, "Konfigurasi simbol hilang/duplikat");
    Ok(selected[0])
}
impl Spec {
    pub fn signed(
        &self,
        client: &Client,
        credentials: &Credentials,
        timestamp: u64,
    ) -> Result<Request> {
        ensure!(
            timestamp > 0 && timestamp <= 4_102_444_800_000,
            "Timestamp invalid"
        );
        let mut url = Url::parse(&format!("{BASE}{}", self.path))?;
        {
            let mut query = url.query_pairs_mut();
            for (k, v) in &self.params {
                query.append_pair(k, v);
            }
            query.append_pair("recvWindow", "5000");
            query.append_pair("timestamp", &timestamp.to_string());
        }
        let unsigned = Zeroizing::new(url.query().context("Query kosong")?.to_owned());
        let signature = sign(credentials.secret.as_bytes(), unsigned.as_bytes())?;
        url.query_pairs_mut().append_pair("signature", &signature);
        let body = Zeroizing::new(url.query().context("Query kosong")?.to_owned());
        if self.method != Method::GET {
            url.set_query(None);
        }
        let mut key = HeaderValue::from_str(&credentials.key)
            .map_err(|_| anyhow::anyhow!("API key invalid"))?;
        key.set_sensitive(true);
        let mut builder = client
            .request(self.method.clone(), url)
            .header(HeaderName::from_static("x-mbx-apikey"), key);
        if self.method != Method::GET {
            builder = builder
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(body.to_string());
        }
        let request = builder
            .build()
            .map_err(|_| anyhow::anyhow!("Gagal membentuk request Futures"))?;
        self.guard(&request)?;
        Ok(request)
    }
    pub fn guard(&self, request: &Request) -> Result<()> {
        let u = request.url();
        ensure!(
            request.method() == self.method
                && u.scheme() == "https"
                && u.host_str() == Some(HOST)
                && u.port_or_known_default() == Some(443)
                && u.username().is_empty()
                && u.password().is_none()
                && u.fragment().is_none()
                && u.path() == self.path
                && !request.headers().contains_key("cookie")
                && !request.headers().contains_key("authorization")
                && request
                    .headers()
                    .get("x-mbx-apikey")
                    .is_some_and(|v| v.is_sensitive()),
            "Request Futures di luar kontrak"
        );
        let encoded = if self.method == Method::GET {
            u.query().context("Query hilang")?
        } else {
            ensure!(
                u.query().is_none(),
                "Mutation tidak boleh memakai signed URL"
            );
            ensure!(
                request
                    .headers()
                    .get(CONTENT_TYPE)
                    .is_some_and(|v| v == "application/x-www-form-urlencoded"),
                "Content-type invalid"
            );
            std::str::from_utf8(
                request
                    .body()
                    .and_then(|b| b.as_bytes())
                    .context("Body hilang")?,
            )
            .context("Body invalid")?
        };
        let decoded = Url::parse(&format!("https://local.invalid/?{encoded}"))?;
        let pairs: Vec<_> = decoded
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        let mut map: BTreeMap<_, _> = pairs.iter().cloned().collect();
        ensure!(map.len() == pairs.len(), "Parameter duplikat");
        ensure!(
            map.remove("recvWindow").as_deref() == Some("5000"),
            "recvWindow invalid"
        );
        let timestamp = map
            .remove("timestamp")
            .and_then(|t| t.parse::<u64>().ok())
            .context("Timestamp invalid")?;
        ensure!(
            timestamp > 0 && timestamp <= 4_102_444_800_000,
            "Timestamp invalid"
        );
        let signature = map.remove("signature").context("Signature hilang")?;
        ensure!(
            signature.len() == 64
                && signature.bytes().all(|b| b.is_ascii_hexdigit())
                && map == self.params,
            "Form Futures tidak cocok"
        );
        Ok(())
    }
}
fn sign(secret: &[u8], query: &[u8]) -> Result<Zeroizing<String>> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret).map_err(|_| anyhow::anyhow!("Signing invalid"))?;
    mac.update(query);
    let mut out = Zeroizing::new(String::with_capacity(64));
    for byte in mac.finalize().into_bytes() {
        write!(out, "{byte:02x}")?;
    }
    Ok(out)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
    ExpiredInMatch,
}
impl OrderStatus {
    pub fn terminal(self) -> bool {
        !matches!(self, Self::New | Self::PartiallyFilled)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Order {
    pub exchange_id: u64,
    pub client_id: String,
    pub status: OrderStatus,
    pub executed_qty: D,
    pub quote_qty: D,
}
impl Order {
    pub fn parse(v: &Value, symbol: Symbol, id: &str) -> Result<Self> {
        ensure!(
            v["symbol"] == symbol.name() && v["clientOrderId"] == id,
            "Identitas order tidak cocok"
        );
        let out = Self {
            exchange_id: v["orderId"]
                .as_u64()
                .filter(|n| *n > 0)
                .context("Order ID hilang")?,
            client_id: id.into(),
            status: serde_json::from_value(v["status"].clone())
                .map_err(|_| anyhow::anyhow!("Status order tidak dikenal"))?,
            executed_qty: decimal(&v["executedQty"])?,
            quote_qty: decimal(&v["cumQuote"])?,
        };
        ensure!(
            (out.executed_qty.is_zero() && out.quote_qty.is_zero())
                || (out.executed_qty > D::ZERO && out.quote_qty > D::ZERO),
            "Nilai fill invalid"
        );
        ensure!(
            out.status != OrderStatus::Filled || out.executed_qty > D::ZERO,
            "Filled tanpa jumlah"
        );
        Ok(out)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Algo {
    pub exchange_id: u64,
    pub client_id: String,
    pub status: String,
    pub actual_order_id: Option<u64>,
}
impl Algo {
    pub fn parse(v: &Value, id: &str) -> Result<Self> {
        ensure!(v["clientAlgoId"] == id, "Identitas algo tidak cocok");
        let status = v["algoStatus"].as_str().context("Status algo hilang")?;
        ensure!(
            matches!(
                status,
                "NEW"
                    | "TRIGGERING"
                    | "TRIGGERED"
                    | "FINISHED"
                    | "CANCELED"
                    | "REJECTED"
                    | "EXPIRED"
            ),
            "Status algo tidak dikenal"
        );
        let actual_order_id = match v.get("actualOrderId") {
            Some(Value::String(s)) if s.is_empty() => None,
            Some(Value::String(s)) => Some(
                s.parse::<u64>()
                    .ok()
                    .filter(|id| *id > 0)
                    .context("Child order invalid")?,
            ),
            None if status == "NEW" => None,
            _ => anyhow::bail!("Child order hilang/invalid"),
        };
        Ok(Self {
            exchange_id: v["algoId"]
                .as_u64()
                .filter(|n| *n > 0)
                .context("Algo ID hilang")?,
            client_id: id.into(),
            status: status.into(),
            actual_order_id,
        })
    }
    pub fn terminal(&self) -> bool {
        matches!(
            self.status.as_str(),
            "FINISHED" | "CANCELED" | "REJECTED" | "EXPIRED"
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Receipt {
    Order(Order),
    Algo(Algo),
    Configured,
    CancelAccepted,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hmac_matches_rfc4231_test_case_one() {
        assert_eq!(
            &*sign(&[0x0b; 20], b"Hi There").unwrap(),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }
}
