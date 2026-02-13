use clap::Args;
use lineark_sdk::Client;
use serde::Serialize;
use tabled::Tabled;

use crate::output::{self, Format};

/// Manage cycles.
#[derive(Debug, Args)]
pub struct CyclesCmd {
    #[command(subcommand)]
    pub action: CyclesAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum CyclesAction {
    /// List cycles.
    List {
        /// Maximum number of cycles to return (max 250).
        #[arg(long, default_value = "50", value_parser = clap::value_parser!(i64).range(1..=250))]
        limit: i64,
    },
    /// Read a specific cycle.
    Read {
        /// Cycle ID.
        id: String,
    },
}

#[derive(Debug, Serialize, Tabled)]
pub struct CycleRow {
    pub id: String,
    pub number: String,
    pub name: String,
    pub starts_at: String,
    pub ends_at: String,
    pub is_active: bool,
}

pub async fn run(cmd: CyclesCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        CyclesAction::List { limit } => {
            let conn = client
                .cycles()
                .first(limit)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let rows: Vec<CycleRow> = conn
                .nodes
                .iter()
                .map(|c| CycleRow {
                    id: c.id.clone().unwrap_or_default(),
                    number: c.number.map(|n| n.to_string()).unwrap_or_default(),
                    name: c.name.clone().unwrap_or_default(),
                    starts_at: c.starts_at.map(|d| d.to_string()).unwrap_or_default(),
                    ends_at: c.ends_at.map(|d| d.to_string()).unwrap_or_default(),
                    is_active: c.is_active.unwrap_or(false),
                })
                .collect();

            output::print_table(&rows, format);
        }
        CyclesAction::Read { id } => {
            let cycle = client
                .cycle(id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output::print_one(&cycle, format);
        }
    }
    Ok(())
}
