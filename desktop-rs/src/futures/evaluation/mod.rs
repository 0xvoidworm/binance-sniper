//! Offline, explicit evaluation boundary. Never called by the live worker.
//! No credentials, transport, mutation, or execution of evaluator output.
mod package;
mod proposal;
mod schema;
mod source;

use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct Cli {
    #[command(subcommand)]
    command: Action,
}
#[derive(Debug, Subcommand)]
enum Action {
    /// Export the latest FINAL session, locally; refuses an active writer/session.
    Export {
        #[arg(long)]
        source: PathBuf,
        /// A NEW package directory inside an existing private output directory.
        #[arg(long)]
        output: PathBuf,
    },
    /// Check schema, private permissions, hashes, relations and exact accounting.
    Validate { package: PathBuf },
    /// Check an untrusted proposal. Success means reviewable, NEVER approved.
    ValidateProposal { package: PathBuf, proposal: PathBuf },
}
impl Cli {
    pub fn run(self) -> Result<()> {
        // This entry point returns before any GUI, vault, network or live startup.
        match self.command {
            Action::Export { source, output } => {
                let packet = source::snapshot(&source)?;
                package::publish(&output, &packet)?;
                println!(
                    "EVALUATION_EXPORT_OK final=true trades={} net={} model_run=false",
                    packet.summary.completed_trades, packet.summary.net
                );
            }
            Action::Validate { package: path } => {
                let p = package::validate(&path)?;
                println!(
                    "EVALUATION_VALID final=true trades={} net={} missing_data={} sha256={} exchange_requery=false",
                    p.summary.completed_trades,
                    p.summary.net,
                    p.manifest.missing_data.len(),
                    p.manifest_digest
                );
            }
            Action::ValidateProposal {
                package: path,
                proposal: file,
            } => {
                let p = package::validate(&path)?;
                proposal::validate(&p, &file)?;
                println!(
                    "PROPOSAL_REVIEWABLE human_review_required=true approved=false auto_apply=false"
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
