use clap::Args;
use lineark_sdk::generated::inputs::{IssueCreateInput, IssueUpdateInput};
use lineark_sdk::Client;
use serde::{Deserialize, Serialize};
use tabled::Tabled;

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

// ── List/search row (table + JSON) ──────────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct IssueListItem {
    identifier: Option<String>,
    title: Option<String>,
    priority_label: Option<String>,
    assignee: Option<NestedUser>,
    state: Option<NestedState>,
    team: Option<NestedTeam>,
}

#[derive(Debug, Serialize, Tabled)]
struct IssueRow {
    identifier: String,
    title: String,
    status: String,
    assignee: String,
    priority: String,
    team: String,
}

impl From<&IssueListItem> for IssueRow {
    fn from(i: &IssueListItem) -> Self {
        Self {
            identifier: i.identifier.clone().unwrap_or_default(),
            title: i.title.clone().unwrap_or_default(),
            status: i
                .state
                .as_ref()
                .and_then(|s| s.name.clone())
                .unwrap_or_default(),
            assignee: i
                .assignee
                .as_ref()
                .and_then(|a| a.name.clone())
                .unwrap_or_default(),
            priority: i.priority_label.clone().unwrap_or_default(),
            team: i
                .team
                .as_ref()
                .and_then(|t| t.key.clone())
                .unwrap_or_default(),
        }
    }
}

const ISSUE_LIST_QUERY: &str =
    "query Issues($first: Int, $filter: IssueFilter) { issues(first: $first, filter: $filter) { \
nodes { identifier title priorityLabel \
assignee { id name } state { id name } team { id key name } \
} pageInfo { hasNextPage endCursor } } }";

const ISSUE_SEARCH_QUERY: &str =
    "query IssueSearch($term: String!, $first: Int) { searchIssues(term: $term, first: $first) { \
nodes { identifier title priorityLabel \
assignee { id name } state { id name } team { id key name } \
} pageInfo { hasNextPage endCursor } } }";

// ── Detail (read) ───────────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct IssueDetail {
    pub identifier: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority_label: Option<String>,
    pub url: Option<String>,
    pub branch_name: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub due_date: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub canceled_at: Option<String>,
    pub estimate: Option<f64>,
    pub labels: Option<NestedLabelConnection>,
    pub assignee: Option<NestedUser>,
    pub creator: Option<NestedUser>,
    pub state: Option<NestedState>,
    pub team: Option<NestedTeam>,
    pub project: Option<NestedProject>,
    pub cycle: Option<NestedCycle>,
    pub parent: Option<NestedIssue>,
    pub attachments: Option<NestedAttachmentConnection>,
    pub relations: Option<NestedRelationConnection>,
    pub inverse_relations: Option<NestedRelationConnection>,
}

const ISSUE_READ_QUERY: &str = "query IssueRead($id: String!) { issue(id: $id) { \
identifier title description priorityLabel url branchName \
createdAt updatedAt dueDate startedAt completedAt canceledAt \
estimate \
labels { nodes { id name } } \
assignee { id name } creator { id name } state { id name } \
team { id key name } project { id name } cycle { id number name } \
parent { id identifier } \
attachments { nodes { id title url sourceType } } \
relations { nodes { id type relatedIssue { id identifier title } } } \
inverseRelations { nodes { id type issue { id identifier title } } } \
} }";

const ISSUE_SEARCH_ONE_QUERY: &str = "query IssueSearchOne($term: String!, $first: Int) { searchIssues(term: $term, first: $first) { nodes { \
identifier title description priorityLabel url branchName \
createdAt updatedAt dueDate startedAt completedAt canceledAt \
estimate \
labels { nodes { id name } } \
assignee { id name } creator { id name } state { id name } \
team { id key name } project { id name } cycle { id number name } \
parent { id identifier } \
attachments { nodes { id title url sourceType } } \
relations { nodes { id type relatedIssue { id identifier title } } } \
inverseRelations { nodes { id type issue { id identifier title } } } \
} } }";

// ── Shared nested types ─────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct NestedUser {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct NestedTeam {
    pub id: Option<String>,
    pub key: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct NestedState {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct NestedProject {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct NestedCycle {
    pub id: Option<String>,
    pub number: Option<f64>,
    pub name: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct NestedLabelConnection {
    pub nodes: Vec<NestedLabel>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct NestedLabel {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct NestedIssue {
    pub id: Option<String>,
    pub identifier: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct NestedAttachmentConnection {
    pub nodes: Vec<NestedAttachment>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct NestedAttachment {
    pub id: Option<String>,
    pub title: Option<String>,
    pub url: Option<String>,
    pub source_type: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct NestedRelationConnection {
    pub nodes: Vec<NestedRelation>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct NestedRelation {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub relation_type: Option<String>,
    pub related_issue: Option<NestedIssue>,
    pub issue: Option<NestedIssue>,
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
            let mut filter = serde_json::Map::new();
            if !show_done {
                filter.insert(
                    "state".into(),
                    serde_json::json!({ "type": { "nin": ["completed", "canceled"] } }),
                );
            }
            if let Some(ref team_key) = team {
                let team_id = resolve_team_id(client, team_key).await?;
                filter.insert(
                    "team".into(),
                    serde_json::json!({ "id": { "eq": team_id } }),
                );
            }
            if mine {
                let viewer = client
                    .whoami()
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;
                let viewer_id = viewer
                    .id
                    .ok_or_else(|| anyhow::anyhow!("Could not determine authenticated user ID"))?;
                filter.insert(
                    "assignee".into(),
                    serde_json::json!({ "id": { "eq": viewer_id } }),
                );
            }
            let variables = serde_json::json!({
                "first": limit,
                "filter": filter,
            });
            let conn = client
                .execute_connection::<IssueListItem>(ISSUE_LIST_QUERY, variables, "issues")
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let items: Vec<&IssueListItem> = conn.nodes.iter().collect();
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
            // searchIssues doesn't support IssueFilter, so filter client-side.
            let variables = serde_json::json!({ "term": query, "first": limit });
            let conn: lineark_sdk::Connection<IssueListItem> = client
                .execute_connection(ISSUE_SEARCH_QUERY, variables, "searchIssues")
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            let items = filter_done(&conn.nodes, show_done);
            print_issue_list(&items, format);
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

            let payload = client
                .issue_create(input)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            check_success(&payload)?;
            let issue = payload.get("issue").cloned().unwrap_or_default();
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

            let payload = client
                .issue_update(input, issue_id)
                .await
                .map_err(|e| anyhow::anyhow!("{}", e))?;

            check_success(&payload)?;
            let issue = payload.get("issue").cloned().unwrap_or_default();
            output::print_one(&issue, format);
        }
    }
    Ok(())
}

// TODO(phase2): query workflowStates types instead of hardcoding state names
const DONE_STATES: &[&str] = &["Done", "Canceled", "Cancelled", "Duplicate"];

fn filter_done(items: &[IssueListItem], show_done: bool) -> Vec<&IssueListItem> {
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

fn print_issue_list(items: &[&IssueListItem], format: Format) {
    match format {
        Format::Json => {
            let json = serde_json::to_string_pretty(items).unwrap_or_default();
            println!("{json}");
        }
        Format::Human => {
            let rows: Vec<IssueRow> = items.iter().map(|i| IssueRow::from(*i)).collect();
            output::print_table(&rows, format);
        }
    }
}

/// Read a single issue by identifier (e.g. E-929) or UUID, with full nested details.
async fn read_issue(client: &Client, identifier: &str) -> anyhow::Result<IssueDetail> {
    if uuid::Uuid::parse_str(identifier).is_ok() {
        // Direct query by UUID.
        let variables = serde_json::json!({ "id": identifier });
        return client
            .execute(ISSUE_READ_QUERY, variables, "issue")
            .await
            .map_err(|e| anyhow::anyhow!("{}", e));
    }
    if identifier.contains('-') {
        // searchIssues is fuzzy — it may return a different issue if no exact match exists.
        // We fetch a small page and verify the identifier matches exactly.
        let variables = serde_json::json!({ "term": identifier, "first": 5 });
        let conn: lineark_sdk::Connection<IssueDetail> = client
            .execute_connection(ISSUE_SEARCH_ONE_QUERY, variables, "searchIssues")
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        conn.nodes
            .into_iter()
            .find(|issue| {
                issue
                    .identifier
                    .as_deref()
                    .is_some_and(|id| id.eq_ignore_ascii_case(identifier))
            })
            .ok_or_else(|| anyhow::anyhow!("Issue '{}' not found", identifier))
    } else {
        let variables = serde_json::json!({ "id": identifier });
        client
            .execute(ISSUE_READ_QUERY, variables, "issue")
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}

/// Resolve a team key (e.g., "E", "ENG") to a team UUID.
/// If the input already looks like a UUID, return it as-is.
async fn resolve_team_id(client: &Client, team_key: &str) -> anyhow::Result<String> {
    if uuid::Uuid::parse_str(team_key).is_ok() {
        return Ok(team_key.to_string());
    }
    let conn = client
        .teams()
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    for team in &conn.nodes {
        if team
            .key
            .as_deref()
            .is_some_and(|k| k.eq_ignore_ascii_case(team_key))
        {
            return Ok(team.id.clone().unwrap_or_default());
        }
    }
    Err(anyhow::anyhow!("Team '{}' not found", team_key))
}

/// Resolve a workflow state name to its UUID for a given team.
async fn resolve_state_id(
    client: &Client,
    team_id: &str,
    state_name: &str,
) -> anyhow::Result<String> {
    let filter = serde_json::json!({ "team": { "id": { "eq": team_id } } });
    let variables = serde_json::json!({ "first": 50, "filter": filter });
    let conn = client
        .execute_connection::<serde_json::Value>(
            "query WorkflowStates($first: Int, $filter: WorkflowStateFilter) { workflowStates(first: $first, filter: $filter) { nodes { id name } pageInfo { hasNextPage endCursor } } }",
            variables,
            "workflowStates",
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    for node in &conn.nodes {
        let name = node.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name.eq_ignore_ascii_case(state_name) {
            return Ok(node
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string());
        }
    }
    let available: Vec<String> = conn
        .nodes
        .iter()
        .filter_map(|n| n.get("name").and_then(|v| v.as_str()).map(String::from))
        .collect();
    Err(anyhow::anyhow!(
        "Status '{}' not found for this team. Available: {}",
        state_name,
        available.join(", ")
    ))
}

/// Resolve an issue identifier (e.g., ENG-123) to a UUID.
/// If the input already looks like a UUID, return it as-is.
async fn resolve_issue_id(client: &Client, identifier: &str) -> anyhow::Result<String> {
    if uuid::Uuid::parse_str(identifier).is_ok() {
        return Ok(identifier.to_string());
    }
    let variables = serde_json::json!({ "term": identifier, "first": 5 });
    let conn: lineark_sdk::Connection<serde_json::Value> = client
        .execute_connection(
            "query IssueIdResolve($term: String!, $first: Int) { searchIssues(term: $term, first: $first) { nodes { id identifier } pageInfo { hasNextPage endCursor } } }",
            variables,
            "searchIssues",
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    conn.nodes
        .iter()
        .find(|n| {
            n.get("identifier")
                .and_then(|v| v.as_str())
                .is_some_and(|id| id.eq_ignore_ascii_case(identifier))
        })
        .and_then(|n| n.get("id").and_then(|v| v.as_str()).map(String::from))
        .ok_or_else(|| anyhow::anyhow!("Issue '{}' not found", identifier))
}

/// Check the `success` field in a mutation payload.
fn check_success(payload: &serde_json::Value) -> anyhow::Result<()> {
    if payload.get("success").and_then(|v| v.as_bool()) != Some(true) {
        return Err(anyhow::anyhow!(
            "Mutation failed: {}",
            serde_json::to_string_pretty(payload).unwrap_or_default()
        ));
    }
    Ok(())
}
