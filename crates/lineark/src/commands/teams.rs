use clap::Args;
use lineark_sdk::Client;
use serde::Serialize;
use tabled::Tabled;

use crate::output::{self, Format};

/// Manage teams.
#[derive(Debug, Args)]
pub struct TeamsCmd {
    #[command(subcommand)]
    pub action: TeamsAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum TeamsAction {
    /// List all teams.
    List,
}

#[derive(Debug, Serialize, Tabled)]
pub struct TeamRow {
    pub id: String,
    pub key: String,
    pub name: String,
}

pub async fn run(cmd: TeamsCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        TeamsAction::List => {
            let conn = client
                .teams(None, None, Some(250), None, None)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let rows: Vec<TeamRow> = conn
                .nodes
                .iter()
                .map(|t| TeamRow {
                    id: t.id.clone().unwrap_or_default(),
                    key: t.key.clone().unwrap_or_default(),
                    name: t.name.clone().unwrap_or_default(),
                })
                .collect();

            output::print_table(&rows, format);
        }
    }
    Ok(())
}
