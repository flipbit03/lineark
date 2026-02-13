use clap::Args;
use lineark_sdk::Client;
use serde::Serialize;
use tabled::Tabled;

use crate::output::{self, Format};

/// Manage projects.
#[derive(Debug, Args)]
pub struct ProjectsCmd {
    #[command(subcommand)]
    pub action: ProjectsAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum ProjectsAction {
    /// List all projects.
    List,
}

#[derive(Debug, Serialize, Tabled)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub slug_id: String,
}

pub async fn run(cmd: ProjectsCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        ProjectsAction::List => {
            let conn = client
                .projects()
                .first(250)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let rows: Vec<ProjectRow> = conn
                .nodes
                .iter()
                .map(|p| ProjectRow {
                    id: p.id.clone().unwrap_or_default(),
                    name: p.name.clone().unwrap_or_default(),
                    slug_id: p.slug_id.clone().unwrap_or_default(),
                })
                .collect();

            output::print_table(&rows, format);
        }
    }
    Ok(())
}
