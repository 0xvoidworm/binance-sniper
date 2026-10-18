use crate::{Engine, model::*};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

const MAX_BYTES: u64 = 16 * 1024 * 1024;
const MAX_RECORD_BYTES: usize = 64 * 1024;
const MAX_RECORDS: usize = 20_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Payload {
    Genesis {
        schema: String,
        config: Config,
    },
    Event {
        envelope: Envelope,
        outcome: Outcome,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Body {
    seq: u64,
    previous_hash: String,
    state_hash: String,
    payload: Payload,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    body: Body,
    hash: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Applied {
    pub duplicate: bool,
    pub outcome: Outcome,
}

/// One writer, synchronous append before state commit, deterministic replay.
/// Hashes detect accidental corruption; they are not signatures or tamper proofing.
pub struct Journal {
    file: File,
    engine: Engine,
    previous_hash: String,
    records: u64,
    seen: BTreeMap<String, (Envelope, Outcome)>,
    poisoned: bool,
}

impl Drop for Journal {
    fn drop(&mut self) {
        // Closing only this fd may leave a lock alive in a descriptor inherited
        // during concurrent process creation. Release this writer's lock before
        // its file is closed. A failed unlock still falls back to close-on-drop.
        let _ = self.file.unlock();
    }
}

impl Journal {
    pub fn create(path: &Path, config: Config) -> Result<Self> {
        let engine = Engine::new(config.clone())?;
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create_new(true)
            .open(path)
            .context("create new journal (existing files are never overwritten)")?;
        file.try_lock().context("journal already locked")?;
        let mut journal = Self {
            file,
            engine,
            previous_hash: String::new(),
            records: 0,
            seen: BTreeMap::new(),
            poisoned: false,
        };
        let state_hash = hash(journal.engine.state())?;
        journal.append(
            Payload::Genesis {
                schema: SCHEMA.to_owned(),
                config,
            },
            state_hash,
        )?;
        // Persist creation of the directory entry as well as the file contents.
        File::open(
            path.parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )?
        .sync_all()?;
        Ok(journal)
    }

    pub fn open(path: &Path) -> Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .open(path)
            .context("open existing journal")?;
        file.try_lock().context("journal already locked")?;
        ensure!(
            file.metadata()?.len() <= MAX_BYTES,
            "journal exceeds size limit"
        );
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        ensure!(
            bytes.last() == Some(&b'\n'),
            "empty or torn journal: explicit recovery required"
        );
        let lines: Vec<_> = bytes.split(|b| *b == b'\n').collect();
        ensure!(
            lines.len() - 1 <= MAX_RECORDS,
            "journal record limit exceeded"
        );
        let mut engine: Option<Engine> = None;
        let mut previous_hash = String::new();
        let mut seen = BTreeMap::new();
        for (seq, line) in lines[..lines.len() - 1].iter().enumerate() {
            ensure!(
                line.len() <= MAX_RECORD_BYTES,
                "journal record exceeds size limit"
            );
            let record: Record = serde_json::from_slice(line)
                .with_context(|| format!("invalid journal record {seq}"))?;
            ensure!(
                record.body.seq == seq as u64 && record.body.previous_hash == previous_hash,
                "journal sequence/hash chain mismatch at {seq}"
            );
            ensure!(
                hash(&record.body)? == record.hash,
                "journal checksum mismatch at {seq}"
            );
            match record.body.payload {
                Payload::Genesis { schema, config } => {
                    ensure!(
                        seq == 0 && schema == SCHEMA,
                        "unsupported journal schema/genesis"
                    );
                    engine = Some(Engine::new(config)?);
                }
                Payload::Event { envelope, outcome } => {
                    let e = engine
                        .as_mut()
                        .ok_or_else(|| anyhow::anyhow!("missing genesis"))?;
                    ensure!(!seen.contains_key(&envelope.id), "duplicate event record");
                    let replayed = e.apply(&envelope);
                    ensure!(
                        replayed == outcome,
                        "engine outcome diverged at record {seq}; explicit migration required"
                    );
                    seen.insert(envelope.id.clone(), (envelope, outcome));
                }
            }
            ensure!(
                hash(
                    engine
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("missing genesis"))?
                        .state()
                )? == record.body.state_hash,
                "replayed state diverged at {seq}"
            );
            previous_hash = record.hash;
        }
        Ok(Self {
            file,
            engine: engine.ok_or_else(|| anyhow::anyhow!("missing genesis"))?,
            previous_hash,
            records: (lines.len() - 1) as u64,
            seen,
            poisoned: false,
        })
    }

    pub fn state(&self) -> &State {
        self.engine.state()
    }
    pub fn records(&self) -> u64 {
        self.records
    }
    pub fn head_hash(&self) -> &str {
        &self.previous_hash
    }

    pub fn apply(&mut self, envelope: Envelope) -> Result<Applied> {
        ensure!(
            !self.poisoned,
            "journal writer poisoned by IO error; reopen and verify first"
        );
        if let Some((previous, outcome)) = self.seen.get(&envelope.id) {
            ensure!(
                *previous == envelope,
                "event id collision: payload or timestamp changed"
            );
            let mut outcome = outcome.clone();
            // Never redispatch an effect merely because its input was retried.
            outcome.effects.clear();
            return Ok(Applied {
                duplicate: true,
                outcome,
            });
        }
        let mut next = self.engine.clone();
        let outcome = next.apply(&envelope);
        self.append(
            Payload::Event {
                envelope: envelope.clone(),
                outcome: outcome.clone(),
            },
            hash(next.state())?,
        )?;
        self.engine = next;
        self.seen
            .insert(envelope.id.clone(), (envelope, outcome.clone()));
        Ok(Applied {
            duplicate: false,
            outcome,
        })
    }

    fn append(&mut self, payload: Payload, state_hash: String) -> Result<()> {
        ensure!(
            self.records < MAX_RECORDS as u64,
            "journal record limit reached; explicit rollover required"
        );
        let body = Body {
            seq: self.records,
            previous_hash: self.previous_hash.clone(),
            state_hash,
            payload,
        };
        let record = Record {
            hash: hash(&body)?,
            body,
        };
        let mut bytes = serde_json::to_vec(&record)?;
        ensure!(bytes.len() <= MAX_RECORD_BYTES, "journal record too large");
        bytes.push(b'\n');
        ensure!(
            self.file.metadata()?.len() + bytes.len() as u64 <= MAX_BYTES,
            "journal size limit reached; explicit rollover required"
        );
        self.poisoned = true;
        self.file
            .write_all(&bytes)
            .context("append journal; recovery required after IO failure")?;
        self.file
            .sync_all()
            .context("sync journal; recovery required after IO failure")?;
        self.previous_hash = record.hash;
        self.records += 1;
        self.poisoned = false;
        Ok(())
    }
}

pub fn hash(value: &impl Serialize) -> Result<String> {
    let digest = Sha256::digest(serde_json::to_vec(value)?);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_writer_releases_lock_even_with_an_inherited_descriptor() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("journal.jsonl");
        let config = serde_json::from_str(include_str!("../fixtures/paper.json")).unwrap();
        let journal = Journal::create(&path, config).unwrap();
        // try_clone shares the same open file description, as an inherited fd
        // briefly can between process creation and close-on-exec.
        let inherited = journal.file.try_clone().unwrap();
        assert!(Journal::open(&path).is_err());
        drop(journal);
        let reopened = Journal::open(&path).unwrap();
        assert!(inherited.metadata().is_ok());
        assert_eq!(reopened.records(), 1);
    }
}
