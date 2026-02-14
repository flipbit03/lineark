use clap::Args;
use lineark_sdk::generated::inputs::IssueLabelFilter;
use lineark_sdk::generated::types::IssueLabel;
use lineark_sdk::{Client, GraphQLFields};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use super::helpers::resolve_team_id;
use crate::output::{self, Format};

/// Manage issue labels.
#[derive(Debug, Args)]
pub struct LabelsCmd {
    #[command(subcommand)]
    pub action: LabelsAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum LabelsAction {
    /// List all issue labels. Use --team to filter by team.
    List {
        /// Filter by team key (e.g., E) or team UUID.
        #[arg(long)]
        team: Option<String>,
    },
}

/// Lean label type that includes the parent team.
#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = IssueLabel)]
#[serde(rename_all = "camelCase", default)]
struct LabelSummary {
    pub id: Option<String>,
    pub name: Option<String>,
    pub color: Option<String>,
    #[graphql(nested)]
    pub team: Option<LabelTeamRef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = lineark_sdk::generated::types::Team)]
#[serde(rename_all = "camelCase", default)]
struct LabelTeamRef {
    pub key: Option<String>,
}

#[derive(Debug, Serialize, Tabled)]
pub struct LabelRow {
    pub id: String,
    pub name: String,
    pub color: String,
    pub team: String,
}

pub async fn run(cmd: LabelsCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        LabelsAction::List { team } => {
            let mut query = client.issue_labels::<LabelSummary>().first(250);

            if let Some(ref team_key) = team {
                let team_id = resolve_team_id(client, team_key).await?;
                let filter: IssueLabelFilter = serde_json::from_value(
                    serde_json::json!({ "team": { "id": { "eq": team_id } } }),
                )
                .expect("valid IssueLabelFilter");
                query = query.filter(filter);
            }

            let conn = query.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

            let rows: Vec<LabelRow> = conn
                .nodes
                .iter()
                .map(|l| LabelRow {
                    id: l.id.clone().unwrap_or_default(),
                    name: l.name.clone().unwrap_or_default(),
                    color: l.color.clone().unwrap_or_default(),
                    team: l
                        .team
                        .as_ref()
                        .and_then(|t| t.key.clone())
                        .unwrap_or_default(),
                })
                .collect();

            output::print_table(&rows, format);
        }
    }
    Ok(())
}
