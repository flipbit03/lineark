use clap::Args;
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
}

const ISSUE_READ_QUERY: &str = "query IssueRead($id: String!) { issue(id: $id) { \
identifier title description priorityLabel url branchName \
createdAt updatedAt dueDate startedAt completedAt canceledAt \
estimate \
labels { nodes { id name } } \
assignee { id name } creator { id name } state { id name } \
team { id key name } project { id name } cycle { id number name } \
parent { id identifier } \
} }";

const ISSUE_SEARCH_ONE_QUERY: &str = "query IssueSearchOne($term: String!, $first: Int) { searchIssues(term: $term, first: $first) { nodes { \
identifier title description priorityLabel url branchName \
createdAt updatedAt dueDate startedAt completedAt canceledAt \
estimate \
labels { nodes { id name } } \
assignee { id name } creator { id name } state { id name } \
team { id key name } project { id name } cycle { id number name } \
parent { id identifier } \
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
                    .viewer()
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
    }
    Ok(())
}

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
    if identifier.contains('-') && !identifier.contains("00000") {
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
    if team_key.contains('-') && team_key.len() > 30 {
        return Ok(team_key.to_string());
    }
    let conn = client
        .teams(None, None, None, None, None)
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
