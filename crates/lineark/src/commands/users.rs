use clap::Args;
use lineark_sdk::Client;
use serde::Serialize;
use tabled::Tabled;

use crate::output::{self, Format};

/// Manage users.
#[derive(Debug, Args)]
pub struct UsersCmd {
    #[command(subcommand)]
    pub action: UsersAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum UsersAction {
    /// List all users in the organization.
    List {
        /// Only show active users.
        #[arg(long)]
        active: bool,
    },
}

#[derive(Debug, Serialize, Tabled)]
pub struct UserRow {
    pub id: String,
    pub name: String,
    pub email: String,
    pub active: bool,
}

pub async fn run(cmd: UsersCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        UsersAction::List { active } => {
            let conn = client
                .users()
                .last(250)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let rows: Vec<UserRow> = conn
                .nodes
                .iter()
                .filter(|u| !active || u.active.unwrap_or(false))
                .map(|u| UserRow {
                    id: u.id.clone().unwrap_or_default(),
                    name: u.display_name.clone().unwrap_or_default(),
                    email: u.email.clone().unwrap_or_default(),
                    active: u.active.unwrap_or(false),
                })
                .collect();

            output::print_table(&rows, format);
        }
    }
    Ok(())
}
