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
    /// Create a new issue label.
    ///
    /// Examples:
    ///   lineark labels create "Bug" --color "#eb5757"
    ///   lineark labels create "Feature" --team ENG --color "#4ea7fc" --description "Feature requests"
    ///   lineark labels create "Category" --group --color "#000000"
    ///   lineark labels create "Sub-label" --parent PARENT-UUID --color "#ffffff"
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
        /// Parent label UUID (makes this a sub-label; parent must be a group).
        #[arg(long)]
        parent: Option<String>,
        /// Create as a group label (required before other labels can use it as --parent).
        #[arg(long, default_value = "false")]
        group: bool,
    },
    /// Update an existing issue label.
    ///
    /// Examples:
    ///   lineark labels update LABEL-UUID --name "Renamed"
    ///   lineark labels update LABEL-UUID --color "#00ff00" --description "Updated"
    ///   lineark labels update LABEL-UUID --group          # promote to group
    ///   lineark labels update LABEL-UUID --no-group       # demote (must have no children)
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
        /// New parent label UUID (parent must be a group).
        #[arg(long)]
        parent: Option<String>,
        /// Remove the parent label relationship.
        #[arg(long, default_value = "false", conflicts_with = "parent")]
        clear_parent: bool,
        /// Promote to group (--group) or demote to plain label (--no-group).
        /// Demoting fails if the label still has children.
        #[arg(long, action = clap::ArgAction::Set, default_missing_value = "true", num_args = 0..=1)]
        group: Option<bool>,
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

/// Lean label type that includes team, parent, and group status.
#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = IssueLabel)]
#[serde(rename_all = "camelCase", default)]
struct LabelSummary {
    pub id: Option<String>,
    pub name: Option<String>,
    pub color: Option<String>,
    pub is_group: Option<bool>,
    #[graphql(nested)]
    pub team: Option<LabelTeamRef>,
    #[graphql(nested)]
    pub parent: Option<Box<LabelParentRef>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = lineark_sdk::generated::types::Team)]
#[serde(rename_all = "camelCase", default)]
struct LabelTeamRef {
    pub key: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = IssueLabel)]
#[serde(rename_all = "camelCase", default)]
struct LabelParentRef {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Tabled)]
pub struct LabelRow {
    pub name: String,
    pub color: String,
    pub is_label_group: String,
    pub team: String,
    pub parent_label: String,
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

fn label_to_row(l: &LabelSummary) -> LabelRow {
    LabelRow {
        name: l.name.clone().unwrap_or_default(),
        color: l.color.clone().unwrap_or_default(),
        is_label_group: if l.is_group.unwrap_or(false) {
            "yes".to_string()
        } else {
            String::new()
        },
        team: l
            .team
            .as_ref()
            .and_then(|t| t.key.clone())
            .unwrap_or_default(),
        parent_label: l
            .parent
            .as_ref()
            .and_then(|p| p.name.clone())
            .unwrap_or_default(),
    }
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

            // Sort: groups first (with their children right after), then ungrouped labels.
            let labels = &conn.nodes;
            let mut rows: Vec<LabelRow> = Vec::with_capacity(labels.len());

            // Collect group labels and their children.
            let mut used_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
            for g in labels.iter().filter(|l| l.is_group.unwrap_or(false)) {
                let gid = g.id.clone().unwrap_or_default();
                let gname = g.name.clone().unwrap_or_default();
                used_ids.insert(gid.clone());
                rows.push(label_to_row(g));
                // Children of this group, sorted by name.
                let mut children: Vec<&LabelSummary> = labels
                    .iter()
                    .filter(|l| {
                        l.parent
                            .as_ref()
                            .and_then(|p| p.name.as_deref())
                            .is_some_and(|n| n == gname)
                    })
                    .collect();
                children.sort_by(|a, b| {
                    a.name
                        .as_deref()
                        .unwrap_or("")
                        .cmp(b.name.as_deref().unwrap_or(""))
                });
                for c in children {
                    used_ids.insert(c.id.clone().unwrap_or_default());
                    rows.push(label_to_row(c));
                }
            }

            // Remaining ungrouped labels (no parent, not a group).
            let mut rest: Vec<&LabelSummary> = labels
                .iter()
                .filter(|l| !used_ids.contains(l.id.as_deref().unwrap_or("")))
                .collect();
            rest.sort_by(|a, b| {
                a.name
                    .as_deref()
                    .unwrap_or("")
                    .cmp(b.name.as_deref().unwrap_or(""))
            });
            for l in rest {
                rows.push(label_to_row(l));
            }

            output::print_table(&rows, format);
        }
        LabelsAction::Create {
            name,
            team,
            color,
            description,
            parent,
            group,
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
                is_group: if group { Some(true) } else { None },
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
            clear_parent,
            group,
        } => {
            if name.is_none()
                && color.is_none()
                && description.is_none()
                && parent.is_none()
                && !clear_parent
                && group.is_none()
            {
                return Err(anyhow::anyhow!(
                    "No update fields provided. Use --name, --color, --description, --parent, --clear-parent, --group, or --no-group."
                ));
            }

            let input = IssueLabelUpdateInput {
                name,
                color,
                description,
                parent_id: parent,
                is_group: group,
                ..Default::default()
            };

            // When --clear-parent is used, send `parentId: null` to the API.
            // The generated input uses skip_serializing_if so None omits the field.
            let label = if clear_parent {
                let mut input_val = serde_json::to_value(&input)?;
                input_val
                    .as_object_mut()
                    .unwrap()
                    .insert("parentId".to_string(), serde_json::Value::Null);
                let variables = serde_json::json!({ "input": input_val, "id": id });
                let sel = <LabelRef as GraphQLFields>::selection();
                let query = format!(
                    "mutation($input: IssueLabelUpdateInput!, $id: String!) {{ issueLabelUpdate(input: $input, id: $id) {{ success issueLabel {{ {sel} }} }} }}"
                );
                let payload: serde_json::Value = client
                    .execute(&query, variables, "issueLabelUpdate")
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                serde_json::from_value::<LabelRef>(
                    payload.get("issueLabel").cloned().unwrap_or_default(),
                )?
            } else {
                client
                    .issue_label_update::<LabelRef>(None, input, id)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?
            };

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
