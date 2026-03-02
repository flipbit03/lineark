use clap::Args;
use lineark_sdk::generated::inputs::{
    IssueLabelCreateInput, IssueLabelFilter, IssueLabelUpdateInput,
};
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
#[allow(clippy::large_enum_variant)]
pub enum LabelsAction {
    /// List all issue labels. Use --team to filter by team.
    List {
        /// Filter by team key (e.g., E) or team UUID.
        #[arg(long)]
        team: Option<String>,
    },
    /// Show full details for a single label.
    ///
    /// Examples:
    ///   lineark labels read LABEL-UUID
    Read {
        /// Label UUID.
        id: String,
    },
    /// Create a new issue label.
    ///
    /// Examples:
    ///   lineark labels create "Bug" --color "#eb5757"
    ///   lineark labels create "Feature" --team ENG --color "#4ea7fc" --description "Feature requests"
    ///   lineark labels create "Sub-label" --parent PARENT-UUID --color "#000000"
    Create {
        /// Label name.
        name: String,
        /// Team key, name, or UUID. Omit for a workspace-wide label.
        #[arg(long)]
        team: Option<String>,
        /// Label color (hex string, e.g. "#eb5757").
        #[arg(long)]
        color: Option<String>,
        /// Label description.
        #[arg(long)]
        description: Option<String>,
        /// Parent label UUID (makes this a sub-label).
        #[arg(long)]
        parent: Option<String>,
    },
    /// Update an existing issue label.
    ///
    /// Examples:
    ///   lineark labels update LABEL-UUID --name "Renamed"
    ///   lineark labels update LABEL-UUID --color "#00ff00" --description "Updated"
    Update {
        /// Label UUID.
        id: String,
        /// New label name.
        #[arg(long)]
        name: Option<String>,
        /// New label color (hex string).
        #[arg(long)]
        color: Option<String>,
        /// New label description.
        #[arg(long)]
        description: Option<String>,
        /// New parent label UUID.
        #[arg(long)]
        parent: Option<String>,
    },
    /// Delete an issue label.
    ///
    /// Examples:
    ///   lineark labels delete LABEL-UUID
    Delete {
        /// Label UUID.
        id: String,
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

/// Full label detail for `labels read`.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = IssueLabel)]
#[serde(rename_all = "camelCase", default)]
struct LabelDetail {
    pub id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub is_group: Option<bool>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
    #[graphql(nested)]
    pub team: Option<LabelTeamRef>,
    #[graphql(nested)]
    pub parent: Option<LabelParentRef>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = IssueLabel)]
#[serde(rename_all = "camelCase", default)]
struct LabelParentRef {
    pub id: Option<String>,
    pub name: Option<String>,
}

/// Lean result type for label mutations.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = IssueLabel)]
#[serde(rename_all = "camelCase", default)]
struct LabelRef {
    pub id: Option<String>,
    pub name: Option<String>,
    pub color: Option<String>,
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
        LabelsAction::Read { id } => {
            let label = client
                .issue_label::<LabelDetail>(id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            output::print_one(&label, format);
        }
        LabelsAction::Create {
            name,
            team,
            color,
            description,
            parent,
        } => {
            let team_id = match team {
                Some(ref t) => Some(resolve_team_id(client, t).await?),
                None => None,
            };

            let input = IssueLabelCreateInput {
                name: Some(name),
                color,
                description,
                parent_id: parent,
                team_id,
                ..Default::default()
            };

            let label = client
                .issue_label_create::<LabelRef>(None, input)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&label, format);
        }
        LabelsAction::Update {
            id,
            name,
            color,
            description,
            parent,
        } => {
            if name.is_none() && color.is_none() && description.is_none() && parent.is_none() {
                return Err(anyhow::anyhow!(
                    "No update fields provided. Use --name, --color, --description, or --parent."
                ));
            }

            let input = IssueLabelUpdateInput {
                name,
                color,
                description,
                parent_id: parent,
                ..Default::default()
            };

            let label = client
                .issue_label_update::<LabelRef>(None, input, id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&label, format);
        }
        LabelsAction::Delete { id } => {
            let result = client
                .issue_label_delete(id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&result, format);
        }
    }
    Ok(())
}
