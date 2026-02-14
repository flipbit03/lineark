use clap::Args;
use lineark_sdk::generated::inputs::{
    IssueCreateInput, IssueFilter, IssueUpdateInput, WorkflowStateFilter,
};
use lineark_sdk::generated::types::{
    Issue, IssueRelation, IssueRelationConnection, IssueSearchResult, User, WorkflowState,
};
use lineark_sdk::{Client, GraphQLFields};
use serde::{Deserialize, Serialize};
use tabled::Tabled;

use super::helpers::{resolve_issue_id, resolve_team_id};
use crate::output::{self, Format};

/// Manage issues.
#[derive(Debug, Args)]
pub struct IssuesCmd {
    #[command(subcommand)]
    pub action: IssuesAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum IssuesAction {
    /// List all issues across the workspace (all teams, statuses, and assignees), newest first. Use --mine to show only your issues. Done/canceled issues are hidden by default.
    List {
        /// Maximum number of issues to return (max 250).
        #[arg(long, default_value = "50", value_parser = clap::value_parser!(i64).range(1..=250))]
        limit: i64,
        /// Filter by team key (e.g., E) or team UUID.
        #[arg(long)]
        team: Option<String>,
        /// Show only issues assigned to the authenticated user.
        #[arg(long, default_value = "false")]
        mine: bool,
        /// Include done and canceled issues (hidden by default).
        #[arg(long, default_value = "false")]
        show_done: bool,
    },
    /// Show full details for a single issue, including assignee, state, labels, and description.
    Read {
        /// Issue identifier (e.g., E-929) or UUID.
        identifier: String,
    },
    /// Full-text search across issue titles and descriptions. Done/canceled issues are hidden by default.
    Search {
        /// Search query text.
        query: String,
        /// Maximum number of results (max 250).
        #[arg(long, default_value = "25", value_parser = clap::value_parser!(i64).range(1..=250))]
        limit: i64,
        /// Include done and canceled issues (hidden by default).
        #[arg(long, default_value = "false")]
        show_done: bool,
    },
    /// Create a new issue. Returns the created issue.
    ///
    /// Examples:
    ///   lineark issues create "Fix the bug" --team ENG
    ///   lineark issues create "Add feature" --team ENG --priority 2 --description "Details here"
    ///   lineark issues create "Urgent fix" --team ENG --priority 1 --labels Bug,Frontend
    Create {
        /// Issue title.
        title: String,
        /// Team key (e.g., ENG) or UUID. Required.
        #[arg(long)]
        team: String,
        /// Assignee user UUID.
        #[arg(long)]
        assignee: Option<String>,
        /// Comma-separated label UUIDs.
        #[arg(long, value_delimiter = ',')]
        labels: Option<Vec<String>>,
        /// Priority: 0=none, 1=urgent, 2=high, 3=medium, 4=low.
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..=4))]
        priority: Option<i64>,
        /// Issue description (markdown).
        #[arg(long)]
        description: Option<String>,
        /// Parent issue identifier (e.g., ENG-123) or UUID.
        #[arg(long)]
        parent: Option<String>,
        /// Initial status name (resolved against team's workflow states).
        #[arg(long)]
        status: Option<String>,
    },
    /// Archive an issue.
    ///
    /// Examples:
    ///   lineark issues archive ENG-123
    Archive {
        /// Issue identifier (e.g., ENG-123) or UUID.
        identifier: String,
    },
    /// Unarchive a previously archived issue.
    ///
    /// Examples:
    ///   lineark issues unarchive ENG-123
    Unarchive {
        /// Issue identifier (e.g., ENG-123) or UUID.
        identifier: String,
    },
    /// Delete (trash) an issue. Use --permanently to delete permanently.
    ///
    /// Examples:
    ///   lineark issues delete ENG-123
    ///   lineark issues delete ENG-123 --permanently
    Delete {
        /// Issue identifier (e.g., ENG-123) or UUID.
        identifier: String,
        /// Permanently delete the issue instead of trashing it.
        #[arg(long, default_value = "false")]
        permanently: bool,
    },
    /// Update an existing issue. Returns the updated issue.
    ///
    /// Examples:
    ///   lineark issues update ENG-123 --status "In Progress"
    ///   lineark issues update ENG-123 --priority 1 --assignee USER-UUID
    ///   lineark issues update ENG-123 --labels LABEL1,LABEL2 --label-by adding
    Update {
        /// Issue identifier (e.g., ENG-123) or UUID.
        identifier: String,
        /// New status name (resolved against the issue's team workflow states).
        #[arg(long)]
        status: Option<String>,
        /// Priority: 0=none, 1=urgent, 2=high, 3=medium, 4=low.
        #[arg(long, value_parser = clap::value_parser!(i64).range(0..=4))]
        priority: Option<i64>,
        /// Comma-separated label UUIDs. Behavior depends on --label-by.
        #[arg(long, value_delimiter = ',')]
        labels: Option<Vec<String>>,
        /// How to apply --labels: "replacing" (default), "adding", or "removing".
        #[arg(long, default_value = "replacing")]
        label_by: LabelMode,
        /// Remove all labels from the issue.
        #[arg(long, default_value = "false")]
        clear_labels: bool,
        /// Assignee user UUID.
        #[arg(long)]
        assignee: Option<String>,
        /// Parent issue identifier (e.g., ENG-123) or UUID.
        #[arg(long)]
        parent: Option<String>,
        /// New title.
        #[arg(long)]
        title: Option<String>,
        /// New description (markdown).
        #[arg(long)]
        description: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum LabelMode {
    /// Replace all existing labels with the specified ones.
    Replacing,
    /// Add labels to the existing set.
    Adding,
    /// Remove the specified labels from the existing set.
    Removing,
}

// ── Flattened row (table display + JSON list output) ────────────────────────

#[derive(Debug, Serialize, Tabled)]
#[serde(rename_all = "camelCase")]
struct IssueRow {
    identifier: String,
    title: String,
    #[serde(rename = "priorityLabel")]
    #[tabled(rename = "priority")]
    priority_label: String,
    #[tabled(rename = "status")]
    state: String,
    assignee: String,
    team: String,
    #[tabled(skip)]
    url: String,
}

impl From<&IssueSummary> for IssueRow {
    fn from(i: &IssueSummary) -> Self {
        Self {
            identifier: i.identifier.clone().unwrap_or_default(),
            title: i.title.clone().unwrap_or_default(),
            priority_label: i.priority_label.clone().unwrap_or_default(),
            state: i
                .state
                .as_ref()
                .and_then(|s| s.name.clone())
                .unwrap_or_default(),
            assignee: i
                .assignee
                .as_ref()
                .and_then(|a| a.name.clone())
                .unwrap_or_default(),
            team: i
                .team
                .as_ref()
                .and_then(|t| t.key.clone())
                .unwrap_or_default(),
            url: i.url.clone().unwrap_or_default(),
        }
    }
}

impl From<&SearchSummary> for IssueRow {
    fn from(i: &SearchSummary) -> Self {
        Self {
            identifier: i.identifier.clone().unwrap_or_default(),
            title: i.title.clone().unwrap_or_default(),
            priority_label: i.priority_label.clone().unwrap_or_default(),
            state: i
                .state
                .as_ref()
                .and_then(|s| s.name.clone())
                .unwrap_or_default(),
            assignee: i
                .assignee
                .as_ref()
                .and_then(|a| a.name.clone())
                .unwrap_or_default(),
            team: i
                .team
                .as_ref()
                .and_then(|t| t.key.clone())
                .unwrap_or_default(),
            url: i.url.clone().unwrap_or_default(),
        }
    }
}

// ── IssueSummary — lean type for `issues list` with nested fields ────────

/// Lean issue type for list views — scalars + the nested fields we display.
#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Issue)]
#[serde(rename_all = "camelCase", default)]
pub struct IssueSummary {
    pub id: Option<String>,
    pub identifier: Option<String>,
    pub title: Option<String>,
    pub priority: Option<f64>,
    pub priority_label: Option<String>,
    pub url: Option<String>,
    #[graphql(nested)]
    pub state: Option<StateRef>,
    #[graphql(nested)]
    pub assignee: Option<UserRef>,
    #[graphql(nested)]
    pub team: Option<TeamRef>,
}

/// Lean search result type for `issues search` with nested fields.
#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = IssueSearchResult)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchSummary {
    pub id: Option<String>,
    pub identifier: Option<String>,
    pub title: Option<String>,
    pub priority: Option<f64>,
    pub priority_label: Option<String>,
    pub url: Option<String>,
    #[graphql(nested)]
    pub state: Option<StateRef>,
    #[graphql(nested)]
    pub assignee: Option<UserRef>,
    #[graphql(nested)]
    pub team: Option<TeamRef>,
}

// ── IssueDetail — custom type for `issues read` with nested data ─────────

/// Lean issue type for `issues read` — scalars + the nested fields we display.
/// Zero-overfetch: the struct shape IS the query shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Issue)]
#[serde(rename_all = "camelCase", default)]
pub struct IssueDetail {
    pub id: Option<String>,
    pub identifier: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<f64>,
    pub priority_label: Option<String>,
    pub url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub archived_at: Option<String>,
    #[graphql(nested)]
    pub state: Option<StateRef>,
    #[graphql(nested)]
    pub assignee: Option<UserRef>,
    #[graphql(nested)]
    pub team: Option<TeamRef>,
    #[graphql(nested)]
    pub relations: Option<RelationConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = WorkflowState)]
#[serde(rename_all = "camelCase", default)]
pub struct StateRef {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = User)]
#[serde(rename_all = "camelCase", default)]
pub struct UserRef {
    pub id: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = lineark_sdk::generated::types::Team)]
#[serde(rename_all = "camelCase", default)]
pub struct TeamRef {
    pub id: Option<String>,
    pub name: Option<String>,
    pub key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = IssueRelationConnection)]
#[serde(rename_all = "camelCase", default)]
pub struct RelationConnection {
    #[graphql(nested)]
    pub nodes: Vec<RelationNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = IssueRelation)]
#[serde(rename_all = "camelCase", default)]
pub struct RelationNode {
    pub id: Option<String>,
    pub r#type: Option<String>,
    #[graphql(nested)]
    pub related_issue: Option<RelatedIssueRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, GraphQLFields)]
#[graphql(full_type = Issue)]
#[serde(rename_all = "camelCase", default)]
pub struct RelatedIssueRef {
    pub id: Option<String>,
    pub identifier: Option<String>,
    pub title: Option<String>,
}

/// Lean result type for issue mutations.
#[derive(Debug, Default, Serialize, Deserialize, GraphQLFields)]
#[graphql(full_type = Issue)]
#[serde(rename_all = "camelCase", default)]
struct IssueRef {
    id: Option<String>,
    identifier: Option<String>,
}

// ── Command dispatch ────────────────────────────────────────────────────────

pub async fn run(cmd: IssuesCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        IssuesAction::List {
            limit,
            team,
            mine,
            show_done,
        } => {
            let mut filter_map = serde_json::Map::new();
            if !show_done {
                filter_map.insert(
                    "state".into(),
                    serde_json::json!({ "type": { "nin": ["completed", "canceled"] } }),
                );
            }
            if let Some(ref team_key) = team {
                let team_id = resolve_team_id(client, team_key).await?;
                filter_map.insert(
                    "team".into(),
                    serde_json::json!({ "id": { "eq": team_id } }),
                );
            }
            if mine {
                let viewer = client
                    .whoami::<User>()
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                let viewer_id = viewer
                    .id
                    .ok_or_else(|| anyhow::anyhow!("Could not determine authenticated user ID"))?;
                filter_map.insert(
                    "assignee".into(),
                    serde_json::json!({ "id": { "eq": viewer_id } }),
                );
            }

            let filter: IssueFilter = serde_json::from_value(serde_json::Value::Object(filter_map))
                .expect("valid IssueFilter");

            let conn = client
                .issues::<IssueSummary>()
                .filter(filter)
                .first(limit)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let items: Vec<&IssueSummary> = conn.nodes.iter().collect();
            print_issue_list(&items, format);
        }
        IssuesAction::Read { identifier } => {
            let issue = read_issue(client, &identifier).await?;
            output::print_one(&issue, format);
        }
        IssuesAction::Search {
            query,
            limit,
            show_done,
        } => {
            let conn = client
                .search_issues::<SearchSummary>(query)
                .first(limit)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let items = filter_done_search(&conn.nodes, show_done);
            print_search_list(&items, format);
        }
        IssuesAction::Create {
            title,
            team,
            assignee,
            labels,
            priority,
            description,
            parent,
            status,
        } => {
            let team_id = resolve_team_id(client, &team).await?;

            let state_id = match status {
                Some(ref name) => Some(resolve_state_id(client, &team_id, name).await?),
                None => None,
            };

            let parent_id = match parent {
                Some(ref p) => Some(resolve_issue_id(client, p).await?),
                None => None,
            };

            let input = IssueCreateInput {
                title: Some(title),
                team_id: Some(team_id),
                assignee_id: assignee,
                label_ids: labels,
                priority,
                description,
                parent_id,
                state_id,
                ..Default::default()
            };

            let issue = client
                .issue_create::<IssueRef>(input)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&issue, format);
        }
        IssuesAction::Archive { identifier } => {
            let issue_id = resolve_issue_id(client, &identifier).await?;

            let issue = client
                .issue_archive::<IssueRef>(None, issue_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&issue, format);
        }
        IssuesAction::Unarchive { identifier } => {
            let issue_id = resolve_issue_id(client, &identifier).await?;

            let issue = client
                .issue_unarchive::<IssueRef>(issue_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&issue, format);
        }
        IssuesAction::Delete {
            identifier,
            permanently,
        } => {
            let issue_id = resolve_issue_id(client, &identifier).await?;
            let permanently_delete = if permanently { Some(true) } else { None };

            let issue = client
                .issue_delete::<IssueRef>(permanently_delete, issue_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&issue, format);
        }
        IssuesAction::Update {
            identifier,
            status,
            priority,
            labels,
            label_by,
            clear_labels,
            assignee,
            parent,
            title,
            description,
        } => {
            if status.is_none()
                && priority.is_none()
                && labels.is_none()
                && !clear_labels
                && assignee.is_none()
                && parent.is_none()
                && title.is_none()
                && description.is_none()
            {
                return Err(anyhow::anyhow!(
                    "No update fields provided. Use --status, --priority, --assignee, --labels, --title, --description, or --parent to specify changes."
                ));
            }

            let issue_id = resolve_issue_id(client, &identifier).await?;

            // For status resolution, we need the team ID. Read the issue to get it.
            let state_id = match status {
                Some(ref name) => {
                    let issue_detail = read_issue(client, &identifier).await?;
                    let team_id = issue_detail
                        .team
                        .as_ref()
                        .and_then(|t| t.id.clone())
                        .ok_or_else(|| {
                            anyhow::anyhow!("Could not determine team for issue '{}'", identifier)
                        })?;
                    Some(resolve_state_id(client, &team_id, name).await?)
                }
                None => None,
            };

            let parent_id = match parent {
                Some(ref p) => Some(resolve_issue_id(client, p).await?),
                None => None,
            };

            // Build label fields based on mode.
            let (label_ids, added_label_ids, removed_label_ids) = if clear_labels {
                (Some(vec![]), None, None)
            } else if let Some(ids) = labels {
                match label_by {
                    LabelMode::Replacing => (Some(ids), None, None),
                    LabelMode::Adding => (None, Some(ids), None),
                    LabelMode::Removing => (None, None, Some(ids)),
                }
            } else {
                (None, None, None)
            };

            let input = IssueUpdateInput {
                title,
                description,
                assignee_id: assignee,
                priority,
                state_id,
                parent_id,
                label_ids,
                added_label_ids,
                removed_label_ids,
                ..Default::default()
            };

            let issue = client
                .issue_update::<IssueRef>(input, issue_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            output::print_one(&issue, format);
        }
    }
    Ok(())
}

// TODO(phase2): query workflowStates types instead of hardcoding state names
const DONE_STATES: &[&str] = &["Done", "Canceled", "Cancelled", "Duplicate"];

fn print_issue_list(items: &[&IssueSummary], format: Format) {
    let rows: Vec<IssueRow> = items.iter().map(|i| IssueRow::from(*i)).collect();
    output::print_table(&rows, format);
}

fn filter_done_search(items: &[SearchSummary], show_done: bool) -> Vec<&SearchSummary> {
    if show_done {
        items.iter().collect()
    } else {
        items
            .iter()
            .filter(|i| {
                let state = i
                    .state
                    .as_ref()
                    .and_then(|s| s.name.as_deref())
                    .unwrap_or("");
                !DONE_STATES.iter().any(|d| d.eq_ignore_ascii_case(state))
            })
            .collect()
    }
}

fn print_search_list(items: &[&SearchSummary], format: Format) {
    let rows: Vec<IssueRow> = items.iter().map(|i| IssueRow::from(*i)).collect();
    output::print_table(&rows, format);
}

/// Read a single issue by identifier (e.g. E-929) or UUID, with full nested details.
/// Uses [`IssueDetail`] — a custom type with exactly the nested fields we display.
async fn read_issue(client: &Client, identifier: &str) -> anyhow::Result<IssueDetail> {
    if uuid::Uuid::parse_str(identifier).is_ok() {
        return client
            .issue::<IssueDetail>(identifier.to_string())
            .await
            .map_err(|e| anyhow::anyhow!("{}", e));
    }
    if identifier.contains('-') {
        // searchIssues is fuzzy — search with full type, find the UUID, then fetch details.
        let conn = client
            .search_issues::<IssueSearchResult>(identifier)
            .first(5)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let id = conn
            .nodes
            .iter()
            .find(|issue| {
                issue
                    .identifier
                    .as_deref()
                    .is_some_and(|id| id.eq_ignore_ascii_case(identifier))
            })
            .and_then(|n| n.id.clone())
            .ok_or_else(|| anyhow::anyhow!("Issue '{}' not found", identifier))?;
        client
            .issue::<IssueDetail>(id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    } else {
        client
            .issue::<IssueDetail>(identifier.to_string())
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}

/// Resolve a workflow state name to its UUID for a given team.
async fn resolve_state_id(
    client: &Client,
    team_id: &str,
    state_name: &str,
) -> anyhow::Result<String> {
    let filter: WorkflowStateFilter =
        serde_json::from_value(serde_json::json!({ "team": { "id": { "eq": team_id } } }))
            .expect("valid WorkflowStateFilter");

    let conn = client
        .workflow_states::<WorkflowState>()
        .filter(filter)
        .first(50)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    for node in &conn.nodes {
        let name = node.name.as_deref().unwrap_or("");
        if name.eq_ignore_ascii_case(state_name) {
            return Ok(node.id.clone().unwrap_or_default());
        }
    }
    let available: Vec<String> = conn.nodes.iter().filter_map(|n| n.name.clone()).collect();
    Err(anyhow::anyhow!(
        "Status '{}' not found for this team. Available: {}",
        state_name,
        available.join(", ")
    ))
}
