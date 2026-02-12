use clap::Args;
use lineark_sdk::Client;
use serde::Serialize;
use tabled::Tabled;

use crate::output::{self, Format};

/// Manage issue labels.
#[derive(Debug, Args)]
pub struct LabelsCmd {
    #[command(subcommand)]
    pub action: LabelsAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum LabelsAction {
    /// List all issue labels.
    List,
}

#[derive(Debug, Serialize, Tabled)]
pub struct LabelRow {
    pub id: String,
    pub name: String,
    pub color: String,
}

pub async fn run(cmd: LabelsCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        LabelsAction::List => {
            let conn = client
                .issue_labels(None, None, Some(250), None, None)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let rows: Vec<LabelRow> = conn
                .nodes
                .iter()
                .map(|l| LabelRow {
                    id: l.id.clone().unwrap_or_default(),
                    name: l.name.clone().unwrap_or_default(),
                    color: l.color.clone().unwrap_or_default(),
                })
                .collect();

            output::print_table(&rows, format);
        }
    }
    Ok(())
}
