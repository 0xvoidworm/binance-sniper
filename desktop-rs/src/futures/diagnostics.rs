//! Bounded, credential-free counters from actual decision branches.
//! Descriptive only: no risk settings, commands, or evaluator authority.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Build {
    #[serde(rename = "0.6.9")]
    V069,
    #[serde(rename = "0.6.10")]
    V0610,
    #[serde(rename = "0.6.11")]
    V0611,
    #[serde(rename = "0.6.12")]
    V0612,
    #[serde(rename = "0.6.13")]
    V0613,
    #[serde(rename = "0.6.14")]
    V0614,
}
impl Build {
    pub fn label(self) -> &'static str {
        match self {
            Self::V069 => "0.6.9",
            Self::V0610 => "0.6.10",
            Self::V0611 => "0.6.11",
            Self::V0612 => "0.6.12",
            Self::V0613 => "0.6.13",
            Self::V0614 => "0.6.14",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gate {
    NoPattern,
    LateSignal,
    Volatility,
    ConflictingSignal,
    Signal,
    Funding,
    Economics,
    Intent,
    RecoverySkip,
    DataFailure,
    SafetyFailure,
}
impl Gate {
    pub const ALL: [Self; 11] = [
        Self::NoPattern,
        Self::LateSignal,
        Self::Volatility,
        Self::ConflictingSignal,
        Self::Signal,
        Self::Funding,
        Self::Economics,
        Self::Intent,
        Self::RecoverySkip,
        Self::DataFailure,
        Self::SafetyFailure,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::NoPattern => "Tidak ada pola",
            Self::LateSignal => "Sinyal sudah terlambat",
            Self::Volatility => "ATR di luar batas",
            Self::ConflictingSignal => "Arah sinyal bertentangan",
            Self::Signal => "Sinyal arah ditemukan (belum order)",
            Self::Funding => "Ditahan menjelang funding",
            Self::Economics => "Rencana biaya/lot/risiko ditolak",
            Self::Intent => "Intent dibuat (belum bukti fill)",
            Self::RecoverySkip => "Pemulihan/revalidasi dilewati",
            Self::DataFailure => "Pembacaan data gagal",
            Self::SafetyFailure => "Pemeriksaan keselamatan gagal",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostics {
    pub build: Build,
    pub counts: [u32; 11],
    pub last_gate: Option<Gate>,
    pub saturated: bool,
}
impl Default for Diagnostics {
    fn default() -> Self {
        Self {
            build: Build::V0614,
            counts: [0; 11],
            last_gate: None,
            saturated: false,
        }
    }
}
impl Diagnostics {
    pub fn record(&mut self, gate: Gate) {
        let count = &mut self.counts[gate as usize];
        if let Some(next) = count.checked_add(1) {
            *count = next;
        } else {
            self.saturated = true;
        }
        self.last_gate = Some(gate);
    }
    pub fn evaluated_candles(&self) -> u64 {
        self.counts[..5].iter().map(|n| u64::from(*n)).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn p3_diagnostics_are_bounded_typed_and_do_not_count_polls_as_decisions() {
        let mut d = Diagnostics::default();
        assert_eq!(d.build.label(), env!("CARGO_PKG_VERSION"));
        for gate in Gate::ALL {
            d.record(gate);
        }
        assert_eq!(d.evaluated_candles(), 5);
        assert_eq!(d.counts, [1; 11]);
        d.counts[Gate::DataFailure as usize] = u32::MAX;
        d.record(Gate::DataFailure);
        assert!(d.saturated);
        assert_eq!(d.counts[Gate::DataFailure as usize], u32::MAX);
        let mut value = serde_json::to_value(&d).unwrap();
        value["build"] = "untrusted-secret".into();
        assert!(serde_json::from_value::<Diagnostics>(value).is_err());
    }
}
