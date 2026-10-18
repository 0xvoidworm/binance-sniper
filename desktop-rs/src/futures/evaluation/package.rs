use super::{super::session_store::digest, schema::*};
use anyhow::{Context, Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

fn path_without_links(path: &Path) -> Result<()> {
    ensure!(path.is_absolute(), "Evaluation requires an absolute path");
    let mut prefix = PathBuf::new();
    for part in path.components() {
        ensure!(
            matches!(part, Component::RootDir | Component::Normal(_)),
            "Evaluation path traversal refused"
        );
        prefix.push(part);
        ensure!(
            !fs::symlink_metadata(&prefix)?.file_type().is_symlink(),
            "Evaluation symlink refused"
        );
    }
    Ok(())
}
pub fn private_directory(path: &Path) -> Result<()> {
    path_without_links(path)?;
    let m = fs::symlink_metadata(path)?;
    ensure!(
        m.is_dir() && m.permissions().mode() & 0o077 == 0,
        "Evaluation requires a private directory (0700)"
    );
    Ok(())
}
pub fn open_regular(path: &Path, max: u64) -> Result<File> {
    path_without_links(path)?;
    let m = fs::symlink_metadata(path)?;
    ensure!(
        m.is_file() && m.nlink() == 1 && m.len() <= max && m.permissions().mode() & 0o077 == 0,
        "Evaluation file type/size/permissions invalid"
    );
    // Reject symlinks in EVERY component at the kernel boundary, including a
    // concurrent rename after lstat. NONBLOCK prevents a swapped FIFO hanging
    // the validator. Unsupported openat2 fails closed (Fedora/Linux target).
    let f = File::from(rustix::fs::openat2(
        rustix::fs::CWD,
        path,
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::CLOEXEC
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::NONBLOCK,
        rustix::fs::Mode::empty(),
        rustix::fs::ResolveFlags::NO_SYMLINKS | rustix::fs::ResolveFlags::NO_MAGICLINKS,
    )?);
    let actual = f.metadata()?;
    let after = fs::symlink_metadata(path)?;
    ensure!(
        after.is_file()
            && actual.is_file()
            && actual.ino() == m.ino()
            && actual.dev() == m.dev()
            && after.ino() == m.ino()
            && after.dev() == m.dev()
            && actual.nlink() == 1
            && actual.permissions().mode() & 0o077 == 0
            && actual.len() <= max,
        "Evaluation file changed during open"
    );
    Ok(f)
}
pub fn read_private(path: &Path, max: u64) -> Result<Vec<u8>> {
    let f = open_regular(path, max)?;
    let before = f.metadata()?;
    let mut bytes = Vec::new();
    (&f).take(max + 1).read_to_end(&mut bytes)?;
    let after = f.metadata()?;
    let current = fs::symlink_metadata(path)?;
    ensure!(
        bytes.len() as u64 <= max
            && bytes.len() as u64 == before.len()
            && before.len() == after.len()
            && before.mtime() == after.mtime()
            && before.mtime_nsec() == after.mtime_nsec()
            && before.ctime() == after.ctime()
            && before.ctime_nsec() == after.ctime_nsec()
            && current.is_file()
            && current.ino() == before.ino()
            && current.dev() == before.dev(),
        "Evaluation file changed/oversized during read"
    );
    Ok(bytes)
}
fn canonical<T: DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T> {
    let value: T = serde_json::from_slice(bytes)
        .map_err(|_| anyhow::anyhow!("Evaluation schema malformed/unknown fields"))?;
    ensure!(
        serde_json::to_vec(&value)? == bytes,
        "Evaluation JSON must be canonical; duplicate keys/alternate encodings refused"
    );
    Ok(value)
}
fn content_files(packet: &Packet) -> Result<BTreeMap<String, Vec<u8>>> {
    let summary = serde_json::to_vec(&packet.summary)?;
    let summary: Summary = canonical(&summary)?;
    let mut rows = Vec::new();
    let mut verified = Vec::new();
    for trade in &packet.trades {
        let row = serde_json::to_vec(trade)?;
        verified.push(canonical::<Trade>(&row)?);
        rows.extend_from_slice(&row);
        rows.push(b'\n');
    }
    validate_content(&summary, &verified)?;
    Ok(BTreeMap::from([
        ("summary.json".into(), serde_json::to_vec(&summary)?),
        ("trades.jsonl".into(), rows),
        ("decisions.jsonl".into(), vec![]),
        ("health.jsonl".into(), vec![]),
    ]))
}
fn source_digest() -> String {
    digest(
        concat!(
            include_str!("mod.rs"),
            include_str!("schema.rs"),
            include_str!("source.rs"),
            include_str!("package.rs"),
            include_str!("proposal.rs"),
            include_str!("../session_store.rs"),
            include_str!("../diagnostics.rs"),
            include_str!("../recovery.rs"),
            include_str!("../../../Cargo.lock")
        )
        .as_bytes(),
    )
}
fn export_id(files: &BTreeMap<String, FileEntry>) -> Result<String> {
    Ok(format!(
        "evaluation-{}",
        digest(&serde_json::to_vec(files)?)
    ))
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    ensure!(
        bytes.len() as u64 <= MAX_FILE,
        "Evaluation output exceeds size cap"
    );
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(())
}
pub fn publish(path: &Path, packet: &Packet) -> Result<()> {
    publish_with(path, packet, write_new)
}
pub(super) fn publish_with(
    path: &Path,
    packet: &Packet,
    mut writer: impl FnMut(&Path, &[u8]) -> Result<()>,
) -> Result<()> {
    let parent = path.parent().context("Evaluation output parent missing")?;
    private_directory(parent)?;
    ensure!(
        matches!(path.components().next_back(), Some(Component::Normal(_))),
        "Evaluation output path invalid"
    );
    let files = content_files(packet)?;
    let mut entries = BTreeMap::new();
    for (name, bytes) in &files {
        entries.insert(
            name.clone(),
            FileEntry {
                bytes: bytes.len() as u64,
                sha256: digest(bytes),
                records: match name.as_str() {
                    "summary.json" => 1,
                    "trades.jsonl" => packet.trades.len() as u32,
                    _ => 0,
                },
            },
        );
    }
    let now = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
    ensure!(
        now >= packet.summary.finished_at_ms && now <= MAX_TIME,
        "Evaluation export clock invalid"
    );
    let manifest = Manifest {
        schema_version: 1,
        export_id: export_id(&entries)?,
        session_id: packet.summary.session_id.clone(),
        exporter_version: env!("CARGO_PKG_VERSION").into(),
        exporter_source_sha256: source_digest(),
        source_build_version: packet
            .summary
            .diagnostics
            .as_ref()
            .map(|d| d.build.label().into()),
        strategy: packet.summary.strategy,
        config_sha256: digest(&serde_json::to_vec(&packet.summary.economics)?),
        exported_at_ms: now,
        started_at_ms: packet.summary.started_at_ms,
        finished_at_ms: packet.summary.finished_at_ms,
        final_accounting: true,
        provenance: Provenance::LocalFinalSessionAndTradeArchivesNoExchangeRequery,
        missing_data: missing(&packet.trades, &packet.summary),
        files: entries,
    };
    let manifest_bytes = serde_json::to_vec(&manifest)?;
    ensure!(
        manifest_bytes.len() as u64 <= MAX_MANIFEST,
        "Evaluation manifest oversized"
    );
    // No overwrite/retry. An interrupted write keeps an explicitly incomplete
    // directory (no manifest); validator refuses it. Source state is never touched.
    fs::DirBuilder::new().mode(0o700).create(path)?;
    for (name, bytes) in files {
        writer(&path.join(name), &bytes)?;
    }
    writer(&path.join("manifest.pending"), &manifest_bytes)?;
    File::open(path)?.sync_all()?;
    fs::rename(path.join("manifest.pending"), path.join("manifest.json"))?;
    File::open(path)?.sync_all()?;
    File::open(parent)?.sync_all()?;
    validate(path)?;
    Ok(())
}
pub fn validate(path: &Path) -> Result<Validated> {
    private_directory(path)?;
    let expected: BTreeSet<_> = FILES
        .into_iter()
        .chain(["manifest.json"])
        .map(String::from)
        .collect();
    let mut actual = BTreeSet::new();
    for item in fs::read_dir(path)?.take(7) {
        let item = item?;
        let name = item
            .file_name()
            .into_string()
            .map_err(|_| anyhow::anyhow!("Evaluation filename invalid"))?;
        ensure!(
            expected.contains(&name) && item.file_type()?.is_file(),
            "Evaluation extra/missing/linked file"
        );
        actual.insert(name);
    }
    ensure!(actual == expected, "Evaluation package incomplete");
    let bytes = read_private(&path.join("manifest.json"), MAX_MANIFEST)?;
    let manifest: Manifest = canonical(&bytes)?;
    ensure!(
        manifest.schema_version == 1
            && manifest.session_id == "session-001"
            && manifest.final_accounting
            && matches!(
                manifest.exporter_version.as_str(),
                "0.6.8" | "0.6.9" | "0.6.10" | "0.6.11" | "0.6.12" | "0.6.13" | "0.6.14"
            )
            && hash_valid(&manifest.exporter_source_sha256)
            && hash_valid(&manifest.config_sha256)
            && manifest.files.len() == FILES.len()
            && manifest
                .files
                .keys()
                .all(|name| FILES.contains(&name.as_str()))
            && manifest.export_id == export_id(&manifest.files)?,
        "Evaluation manifest invalid/unsupported"
    );
    let mut contents = BTreeMap::new();
    for (name, entry) in &manifest.files {
        ensure!(
            entry.bytes <= MAX_FILE && hash_valid(&entry.sha256),
            "Evaluation file entry invalid"
        );
        let data = read_private(&path.join(name), MAX_FILE)?;
        ensure!(
            data.len() as u64 == entry.bytes && digest(&data) == entry.sha256,
            "Evaluation content hash/size mismatch"
        );
        contents.insert(name.as_str(), data);
    }
    let summary: Summary = canonical(&contents["summary.json"])?;
    let mut trades = Vec::new();
    let rows = &contents["trades.jsonl"];
    ensure!(
        rows.is_empty() || rows.last() == Some(&b'\n'),
        "Evaluation JSONL truncated"
    );
    if !rows.is_empty() {
        for row in rows[..rows.len() - 1].split(|b| *b == b'\n') {
            ensure!(
                trades.len() < MAX_TRADES,
                "Evaluation too many trade records"
            );
            trades.push(canonical::<Trade>(row)?);
        }
    }
    ensure!(
        contents["decisions.jsonl"].is_empty() && contents["health.jsonl"].is_empty(),
        "Evaluation v1 archive-only profile cannot claim unavailable telemetry"
    );
    ensure!(
        manifest.files["summary.json"].records == 1
            && manifest.files["trades.jsonl"].records as usize == trades.len()
            && manifest.files["decisions.jsonl"].records == 0
            && manifest.files["health.jsonl"].records == 0,
        "Evaluation record count mismatch"
    );
    validate_content(&summary, &trades)?;
    ensure!(
        manifest.session_id == summary.session_id
            && manifest.strategy == summary.strategy
            && manifest.started_at_ms == summary.started_at_ms
            && manifest.finished_at_ms == summary.finished_at_ms
            && manifest.exported_at_ms >= manifest.finished_at_ms
            && manifest.exported_at_ms <= MAX_TIME
            && manifest.config_sha256 == digest(&serde_json::to_vec(&summary.economics)?)
            && manifest.missing_data == missing(&trades, &summary)
            && manifest.source_build_version
                == summary.diagnostics.as_ref().map(|d| d.build.label().into())
            && (manifest.exporter_version != "0.6.8" || summary.diagnostics.is_none())
            && (manifest.exporter_version == "0.6.14"
                || (manifest.exporter_version == "0.6.13"
                    && summary
                        .diagnostics
                        .as_ref()
                        .is_none_or(|d| d.build != super::super::diagnostics::Build::V0614))
                || (manifest.exporter_version == "0.6.12"
                    && summary.diagnostics.as_ref().is_none_or(|d| {
                        !matches!(
                            d.build,
                            super::super::diagnostics::Build::V0613
                                | super::super::diagnostics::Build::V0614
                        )
                    }))
                || (manifest.exporter_version == "0.6.11"
                    && summary.diagnostics.as_ref().is_none_or(|d| {
                        !matches!(
                            d.build,
                            super::super::diagnostics::Build::V0612
                                | super::super::diagnostics::Build::V0613
                                | super::super::diagnostics::Build::V0614
                        )
                    }))
                || (manifest.exporter_version == "0.6.10"
                    && summary.diagnostics.as_ref().is_none_or(|d| !matches!(
                        d.build,
                        super::super::diagnostics::Build::V0611
                            | super::super::diagnostics::Build::V0612
                            | super::super::diagnostics::Build::V0613
                            | super::super::diagnostics::Build::V0614
                    )))
                || (summary.recovery.is_none()
                    && summary
                        .diagnostics
                        .as_ref()
                        .is_none_or(|d| d.build == super::super::diagnostics::Build::V069))),
        "Evaluation manifest/content mismatch"
    );
    Ok(Validated {
        manifest,
        manifest_digest: digest(&bytes),
        summary,
        trades,
    })
}
