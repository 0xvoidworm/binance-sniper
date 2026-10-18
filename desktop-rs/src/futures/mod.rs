//! Autonomous Futures sessions, durable execution, accounting and read-only preflight.
pub mod automatic;
pub mod diagnostics;
pub mod evaluation;
pub mod exit;
pub mod flow;
pub mod ledger;
pub mod live;
pub mod margin;
pub mod outbox;
pub mod planning;
pub mod protocol;
mod quotes;
mod readiness;
pub mod recovery;
mod session_input;
pub mod session_store;
pub mod supervisor;
pub mod transport;
pub use readiness::{Command as ReadinessCommand, Feed, Ready, Snapshot, probe};

#[cfg(test)]
mod flow_tests;
#[cfg(test)]
mod session_tests;
#[cfg(test)]
pub(crate) mod tests;
