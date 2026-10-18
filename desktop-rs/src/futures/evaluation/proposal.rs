//! Output is data for human review only. There is intentionally no runner/apply.
use super::{
    package,
    schema::{Validated, hash_valid},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeSet, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EvidenceFile {
    Summary,
    Trades,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fact {
    file: EvidenceFile,
    record: Option<u32>,
    field: String,
    value: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Candidate {
    StudyTargetAndHoldingHorizonOffline,
    CollectMissingObservations,
    NoChange,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Proposed,
    InsufficientData,
    Rejected,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Usage {
    calls: u32,
    input_tokens: u32,
    output_tokens: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Proposal {
    schema_version: u32,
    proposal_id: String,
    export_digest: String,
    evaluator_version: String,
    model: String,
    prompt_sha256: String,
    question: String,
    facts: Vec<Fact>,
    inferences: Vec<String>,
    missing_data: Vec<super::schema::Missing>,
    hypothesis: String,
    candidate_change: Candidate,
    invariants: Vec<Invariant>,
    test_protocol: String,
    pass_fail_criteria: String,
    usage: Usage,
    status: Status,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Invariant {
    NoLiveOrders,
    NoRiskIncrease,
    NoAutoApply,
    NoProfitOr24hClaim,
}
fn short_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 96
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b))
}
fn text_ok(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 2048 && !s.chars().any(|c| c.is_control())
}
pub fn validate(packet: &Validated, path: &Path) -> Result<()> {
    let bytes = package::read_private(path, 65_536)?;
    let p: Proposal = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("Proposal schema invalid; content not echoed"))?;
    ensure!(
        p.schema_version == 1
            && p.export_digest == packet.manifest_digest
            && hash_valid(&p.prompt_sha256)
            && short_id(&p.proposal_id)
            && short_id(&p.evaluator_version)
            && short_id(&p.model),
        "Proposal identity/version invalid"
    );
    ensure!(
        [
            &p.question,
            &p.hypothesis,
            &p.test_protocol,
            &p.pass_fail_criteria
        ]
        .iter()
        .all(|s| text_ok(s))
            && p.inferences.len() <= 8
            && p.inferences.iter().all(|s| text_ok(s)),
        "Proposal text too long/invalid"
    );
    ensure!(
        !p.facts.is_empty()
            && p.facts.len() <= 64
            && p.usage.calls <= 6
            && p.usage.input_tokens <= 40_000
            && p.usage.output_tokens <= 8_000,
        "Proposal limits exceeded"
    );
    // These are declared usage, NOT a trusted meter or permission to spend money.
    ensure!(
        p.missing_data == packet.manifest.missing_data,
        "Proposal must retain all missing-evidence flags"
    );
    let invariants: BTreeSet<_> = p.invariants.iter().copied().collect();
    ensure!(
        p.invariants.len() == 4
            && invariants
                == BTreeSet::from([
                    Invariant::NoLiveOrders,
                    Invariant::NoRiskIncrease,
                    Invariant::NoAutoApply,
                    Invariant::NoProfitOr24hClaim,
                ]),
        "Proposal safety invariants missing"
    );
    let summary = serde_json::to_value(&packet.summary)?;
    let trades = packet
        .trades
        .iter()
        .map(serde_json::to_value)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut seen = BTreeSet::new();
    for f in &p.facts {
        ensure!(
            f.field.len() <= 48
                && !f.field.is_empty()
                && f.field.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
            "Proposal field reference invalid"
        );
        let row = match f.file {
            EvidenceFile::Summary => {
                ensure!(f.record.is_none(), "Proposal summary reference invalid");
                &summary
            }
            EvidenceFile::Trades => trades
                .get(
                    f.record
                        .and_then(|n| n.checked_sub(1))
                        .ok_or_else(|| anyhow::anyhow!("Proposal trade reference missing"))?
                        as usize,
                )
                .ok_or_else(|| anyhow::anyhow!("Proposal trade reference invalid"))?,
        };
        let expected = row
            .get(&f.field)
            .ok_or_else(|| anyhow::anyhow!("Proposal field does not exist"))?;
        ensure!(
            (expected.is_string() || expected.is_boolean() || expected.as_u64().is_some())
                && expected == &f.value,
            "Proposal fact differs from verified evidence"
        );
        ensure!(
            seen.insert((
                matches!(f.file, EvidenceFile::Trades),
                f.record,
                f.field.clone()
            )),
            "Proposal duplicate evidence reference"
        );
    }
    // Prose/inference truth still needs a human reviewer. Model status never
    // authorizes a config change, market action, code execution or deployment.
    Ok(())
}
